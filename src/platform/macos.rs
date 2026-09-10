//! Native NSPanel overlays. Overlay is never a regular NSWindow.

use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{class, define_class, msg_send, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType,
    NSColor, NSControlStateValueOff, NSControlStateValueOn, NSEvent, NSImage, NSMenu, NSMenuItem,
    NSPanel, NSScreen, NSStatusBar, NSStatusItem, NSTrackingArea, NSTrackingAreaOptions, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindowCollectionBehavior, NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSData, NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
    NSTimer,
};

use crate::app::{Action, Logic, Mode, PickKey};
use crate::config::{Config, Style, FROST_ALPHA};
use crate::hotkey::{format_combo, key_from_mac_vk, Hotkey};
use crate::i18n::Language;

const BORDER: f64 = 8.0;

struct Ivars {
    state: *mut MacState,
    display_id: u32,
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct BlackoutPanel;

    impl BlackoutPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            false
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            false
        }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct OverlayView;

    impl OverlayView {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, _event: Option<&NSEvent>) {
            pointer(self.ivars());
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: Option<&NSEvent>) {
            pointer(self.ivars());
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: Option<&NSEvent>) {
            click(self.ivars());
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            paint(self);
        }
    }
);

struct BindIvars {
    state: *mut MacState,
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[ivars = BindIvars]
    struct BindPanel;

    impl BindPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            true
        }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = BindIvars]
    struct BindView;

    impl BindView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: Option<&NSEvent>) {
            if let Some(event) = event {
                unsafe { handle_bind_key(&mut *self.ivars().state, event) }
            }
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: Option<&NSEvent>) {
            if let Some(event) = event {
                unsafe { handle_bind_keyup(&mut *self.ivars().state, event) }
            }
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: Option<&NSEvent>) {
            if let Some(event) = event {
                unsafe { handle_bind_flags(&mut *self.ivars().state, event) }
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: Option<&NSEvent>) {
            let Some(event) = event else {
                return;
            };
            unsafe {
                match popup_button_hit(self, event) {
                    Some(true) => commit_bind(&mut *self.ivars().state),
                    Some(false) => {
                        close_bind(&mut *self.ivars().state);
                        install_hotkey(&mut *self.ivars().state);
                    }
                    None => {}
                }
            }
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            unsafe {
                let state = &*self.ivars().state;
                let bounds = self.bounds();
                NSColor::colorWithCalibratedRed_green_blue_alpha(
                    0x3D as f64 / 255.0,
                    0xDC as f64 / 255.0,
                    0x97 as f64 / 255.0,
                    1.0,
                )
                .setFill();
                objc2_app_kit::NSBezierPath::fillRect(bounds);
                NSColor::colorWithCalibratedRed_green_blue_alpha(0.06, 0.06, 0.06, 1.0).setFill();
                let inset = NSRect::new(
                    NSPoint::new(2.0, 2.0),
                    NSSize::new(bounds.size.width - 4.0, bounds.size.height - 4.0),
                );
                objc2_app_kit::NSBezierPath::fillRect(inset);
                let hint = NSString::from_str(&state.config.ui_lang().bind_hint());
                let hint_rect = NSRect::new(
                    NSPoint::new(24.0, 16.0),
                    NSSize::new(bounds.size.width - 48.0, 100.0),
                );
                draw_popup_text(&hint, hint_rect, 0.94, 0.94, 0.94, 14.0, true, false);
                let preview = if state.bind_preview.is_empty() {
                    "..."
                } else {
                    state.bind_preview.as_str()
                };
                let combo = NSString::from_str(preview);
                let combo_rect = NSRect::new(
                    NSPoint::new(24.0, bounds.size.height - 112.0),
                    NSSize::new(bounds.size.width - 48.0, 48.0),
                );
                if state.bind_draft.is_some() {
                    draw_popup_text(
                        &combo,
                        combo_rect,
                        0x3D as f64 / 255.0,
                        0xDC as f64 / 255.0,
                        0x97 as f64 / 255.0,
                        16.0,
                        true,
                        true,
                    );
                } else {
                    draw_popup_text(&combo, combo_rect, 0.63, 0.63, 0.63, 16.0, true, true);
                }
                let (save_btn, cancel_btn) = popup_button_rects(bounds);
                stroke_bind_button(save_btn);
                stroke_bind_button(cancel_btn);
                let t = state.config.ui_lang().tr();
                let save = NSString::from_str(t.save_enter);
                let cancel = NSString::from_str(t.cancel_esc);
                if state.bind_draft.is_some() {
                    draw_popup_text(&save, save_btn, 0.94, 0.94, 0.94, 13.0, true, true);
                } else {
                    draw_popup_text(&save, save_btn, 0.63, 0.63, 0.63, 13.0, true, true);
                }
                draw_popup_text(&cancel, cancel_btn, 0.94, 0.94, 0.94, 13.0, true, true);
            }
        }
    }
);

define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[ivars = BindIvars]
    struct WipePanel;

    impl WipePanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            true
        }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = BindIvars]
    struct WipeView;

    impl WipeView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: Option<&NSEvent>) {
            let Some(event) = event else {
                return;
            };
            unsafe {
                let code = event.keyCode();
                if code == 53 {
                    close_wipe(&mut *self.ivars().state);
                    install_hotkey(&mut *self.ivars().state);
                } else if code == 36 {
                    commit_wipe(&mut *self.ivars().state);
                }
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: Option<&NSEvent>) {
            let Some(event) = event else {
                return;
            };
            unsafe {
                match popup_button_hit(self, event) {
                    Some(true) => commit_wipe(&mut *self.ivars().state),
                    Some(false) => {
                        close_wipe(&mut *self.ivars().state);
                        install_hotkey(&mut *self.ivars().state);
                    }
                    None => {}
                }
            }
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            unsafe {
                let state = &*self.ivars().state;
                let bounds = self.bounds();
                NSColor::colorWithCalibratedRed_green_blue_alpha(
                    0x3D as f64 / 255.0,
                    0xDC as f64 / 255.0,
                    0x97 as f64 / 255.0,
                    1.0,
                )
                .setFill();
                objc2_app_kit::NSBezierPath::fillRect(bounds);
                NSColor::colorWithCalibratedRed_green_blue_alpha(0.06, 0.06, 0.06, 1.0).setFill();
                let inset = NSRect::new(
                    NSPoint::new(2.0, 2.0),
                    NSSize::new(bounds.size.width - 4.0, bounds.size.height - 4.0),
                );
                objc2_app_kit::NSBezierPath::fillRect(inset);
                let lang = state.config.ui_lang();
                let title = NSString::from_str(lang.tr().wipe_title);
                let title_rect = NSRect::new(
                    NSPoint::new(24.0, 16.0),
                    NSSize::new(bounds.size.width - 48.0, 36.0),
                );
                draw_popup_text(&title, title_rect, 0.94, 0.94, 0.94, 16.0, true, true);
                let body = NSString::from_str(&crate::purge::dialog_body(lang));
                let body_rect = NSRect::new(
                    NSPoint::new(24.0, 56.0),
                    NSSize::new(bounds.size.width - 48.0, bounds.size.height - 140.0),
                );
                draw_popup_text(&body, body_rect, 0.94, 0.94, 0.94, 13.0, true, false);
                if !state.wipe_error.is_empty() {
                    let err = NSString::from_str(&state.wipe_error);
                    let err_rect = NSRect::new(
                        NSPoint::new(24.0, bounds.size.height - 76.0),
                        NSSize::new(bounds.size.width - 48.0, 20.0),
                    );
                    draw_popup_text(&err, err_rect, 0.94, 0.31, 0.31, 12.0, true, true);
                }
                let (ok_btn, cancel_btn) = popup_button_rects(bounds);
                stroke_bind_button(ok_btn);
                stroke_bind_button(cancel_btn);
                draw_popup_text(
                    &NSString::from_str(lang.tr().wipe_confirm),
                    ok_btn,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
                draw_popup_text(
                    &NSString::from_str(lang.tr().cancel_esc),
                    cancel_btn,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
            }
        }
    }
);

define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[ivars = BindIvars]
    struct AboutPanel;

    impl AboutPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            true
        }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = BindIvars]
    struct AboutView;

    impl AboutView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: Option<&NSEvent>) {
            let Some(event) = event else {
                return;
            };
            unsafe {
                let code = event.keyCode();
                if code == 53 || code == 36 {
                    close_about(&mut *self.ivars().state);
                }
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: Option<&NSEvent>) {
            let Some(event) = event else {
                return;
            };
            unsafe {
                match popup_button_hit(self, event) {
                    Some(true) => crate::about::open_github(),
                    Some(false) => close_about(&mut *self.ivars().state),
                    None => {
                        let point = popup_view_point(self, event);
                        let h = self.bounds().size.height;
                        if point.y >= h - 100.0 && point.y <= h - 72.0 {
                            crate::about::open_github();
                        }
                    }
                }
            }
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            unsafe {
                let bounds = self.bounds();
                NSColor::colorWithCalibratedRed_green_blue_alpha(
                    0x3D as f64 / 255.0,
                    0xDC as f64 / 255.0,
                    0x97 as f64 / 255.0,
                    1.0,
                )
                .setFill();
                objc2_app_kit::NSBezierPath::fillRect(bounds);
                NSColor::colorWithCalibratedRed_green_blue_alpha(0.06, 0.06, 0.06, 1.0).setFill();
                let inset = NSRect::new(
                    NSPoint::new(2.0, 2.0),
                    NSSize::new(bounds.size.width - 4.0, bounds.size.height - 4.0),
                );
                objc2_app_kit::NSBezierPath::fillRect(inset);
                let title_rect = NSRect::new(
                    NSPoint::new(24.0, 16.0),
                    NSSize::new(bounds.size.width - 48.0, 36.0),
                );
                draw_popup_text(
                    &NSString::from_str(crate::about::NAME),
                    title_rect,
                    0x3D as f64 / 255.0,
                    0xDC as f64 / 255.0,
                    0x97 as f64 / 255.0,
                    20.0,
                    true,
                    true,
                );
                let mut line = NSRect::new(
                    NSPoint::new(24.0, 56.0),
                    NSSize::new(bounds.size.width - 48.0, 24.0),
                );
                let lang = (*self.ivars().state).config.ui_lang();
                draw_popup_text(
                    &NSString::from_str(&lang.version_line(crate::about::VERSION)),
                    line,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
                line.origin.y += 24.0;
                draw_popup_text(
                    &NSString::from_str(&lang.author_line(crate::about::AUTHOR)),
                    line,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
                line.origin.y += 26.0;
                line.size.height = 16.0;
                for part in lang.credit_lines() {
                    draw_popup_text(
                        &NSString::from_str(part),
                        line,
                        0.47,
                        0.47,
                        0.47,
                        11.0,
                        true,
                        true,
                    );
                    line.origin.y += 16.0;
                }
                let license = NSRect::new(
                    NSPoint::new(24.0, bounds.size.height - 124.0),
                    NSSize::new(bounds.size.width - 48.0, 24.0),
                );
                draw_popup_text(
                    &NSString::from_str(&lang.license_line(crate::about::LICENSE)),
                    license,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
                let link = NSRect::new(
                    NSPoint::new(24.0, bounds.size.height - 96.0),
                    NSSize::new(bounds.size.width - 48.0, 24.0),
                );
                draw_popup_text(
                    &NSString::from_str(crate::about::GITHUB),
                    link,
                    0x3D as f64 / 255.0,
                    0xDC as f64 / 255.0,
                    0x97 as f64 / 255.0,
                    13.0,
                    true,
                    true,
                );
                let (github_btn, close_btn) = popup_button_rects(bounds);
                stroke_bind_button(github_btn);
                stroke_bind_button(close_btn);
                draw_popup_text(
                    &NSString::from_str("GitHub"),
                    github_btn,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
                draw_popup_text(
                    &NSString::from_str(lang.tr().close_esc),
                    close_btn,
                    0.94,
                    0.94,
                    0.94,
                    13.0,
                    true,
                    true,
                );
            }
        }
    }
);

struct DelegateIvars {
    state: *mut MacState,
}

define_class!(
    #[unsafe(super(objc2::runtime::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "BlackoutAppDelegate"]
    #[ivars = DelegateIvars]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            unsafe { start(self, &mut *self.ivars().state) }
        }

        #[unsafe(method(statusToggle:))]
        fn status_toggle(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe {
                let state = &mut *self.ivars().state;
                let action = state.logic.on_toggle(cursor_display(state));
                apply(state, action);
            }
        }

        #[unsafe(method(statusHotkey:))]
        fn status_hotkey(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            defer_sel(self, objc2::sel!(openBind:));
        }

        #[unsafe(method(openBind:))]
        fn open_bind(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe { begin_bind(&mut *self.ivars().state) }
        }

        #[unsafe(method(statusSolid:))]
        fn status_solid(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe { set_style(&mut *self.ivars().state, Style::Solid, self) }
        }

        #[unsafe(method(statusTint:))]
        fn status_tint(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe { set_style(&mut *self.ivars().state, Style::Tint, self) }
        }

        #[unsafe(method(statusFrost:))]
        fn status_frost(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe { set_style(&mut *self.ivars().state, Style::Frost, self) }
        }

        #[unsafe(method(statusAutostart:))]
        fn status_autostart(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe {
                let on = !crate::autostart::is_enabled();
                if crate::autostart::set_enabled(on).is_ok() {
                    if let Some(item) = &(*self.ivars().state).autostart_item {
                        item.setState(if on {
                            NSControlStateValueOn
                        } else {
                            NSControlStateValueOff
                        });
                    }
                }
            }
        }

        #[unsafe(method(statusWipe:))]
        fn status_wipe(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            defer_sel(self, objc2::sel!(openWipe:));
        }

        #[unsafe(method(openWipe:))]
        fn open_wipe(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe { begin_wipe(&mut *self.ivars().state) }
        }

        #[unsafe(method(statusAbout:))]
        fn status_about(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            defer_sel(self, objc2::sel!(openAbout:));
        }

        #[unsafe(method(openAbout:))]
        fn open_about(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe { begin_about(&mut *self.ivars().state) }
        }

        #[unsafe(method(pollPointer:))]
        fn poll_pointer(&self, _timer: Option<&NSTimer>) {
            unsafe { follow_cursor(&mut *self.ivars().state) }
        }

        #[unsafe(method(statusLang:))]
        fn status_lang(&self, sender: Option<&objc2::runtime::AnyObject>) {
            unsafe {
                let Some(sender) = sender else {
                    return;
                };
                let tag: isize = msg_send![sender, tag];
                let Some(&lang) = Language::ALL.get(tag as usize) else {
                    return;
                };
                let state = &mut *self.ivars().state;
                state.config.language = Some(lang);
                state.config.save();
                rebuild_status_menu(state, self);
                redraw_text_panels(state);
            }
        }

        #[unsafe(method(statusQuit:))]
        fn status_quit(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            unsafe {
                let app = NSApplication::sharedApplication((*self.ivars().state).mtm);
                app.terminate(None);
            }
        }
    }
);

struct Overlay {
    panel: Retained<BlackoutPanel>,
    view: Retained<OverlayView>,
    effect: Option<Retained<NSVisualEffectView>>,
    display_id: u32,
    frame: NSRect,
}

struct MacState {
    mtm: MainThreadMarker,
    logic: Logic,
    config: Config,
    overlays: Vec<Overlay>,
    status: Option<Retained<NSStatusItem>>,
    hotkey_ref: *mut std::ffi::c_void,
    pick_refs: Vec<*mut std::ffi::c_void>,
    pointer_anchor: Option<NSPoint>,
    bind: Option<Retained<BindPanel>>,
    bind_draft: Option<Hotkey>,
    bind_preview: String,
    bind_locked: bool,
    bind_held_key: Option<u16>,
    autostart_item: Option<Retained<NSMenuItem>>,
    wipe: Option<Retained<WipePanel>>,
    wipe_error: String,
    about: Option<Retained<AboutPanel>>,
    delegate: *const AppDelegate,
    pointer_timer: Option<Retained<NSTimer>>,
}

static SINGLETON: OnceLock<bool> = OnceLock::new();

pub fn run() -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("must run on the main thread")?;
    if SINGLETON.set(true).is_err() {
        return Ok(());
    }

    let config = Config::load();
    crate::autostart::refresh_path();
    let state = Box::leak(Box::new(MacState {
        mtm,
        logic: Logic::new(config.style),
        config,
        overlays: Vec::new(),
        status: None,
        hotkey_ref: std::ptr::null_mut(),
        pick_refs: Vec::new(),
        pointer_anchor: None,
        bind: None,
        bind_draft: None,
        bind_preview: String::new(),
        bind_locked: false,
        bind_held_key: None,
        autostart_item: None,
        wipe: None,
        wipe_error: String::new(),
        about: None,
        delegate: std::ptr::null(),
        pointer_timer: None,
    }));

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let delegate = AppDelegate::alloc(mtm).set_ivars(DelegateIvars { state });
    let delegate: Retained<AppDelegate> = unsafe { msg_send![super(delegate), init] };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
    Ok(())
}

fn defer_sel(this: &AppDelegate, selector: objc2::runtime::Sel) {
    unsafe {
        let _: () = msg_send![
            this,
            performSelector: selector,
            withObject: None::<&objc2::runtime::AnyObject>,
            afterDelay: 0.0
        ];
    }
}

unsafe fn ensure_pointer_poll(state: &mut MacState) {
    if state.pointer_timer.is_some() {
        return;
    }
    let Some(delegate) = state.delegate.as_ref() else {
        return;
    };
    state.pointer_timer = Some(
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            0.03,
            delegate,
            objc2::sel!(pollPointer:),
            None,
            true,
        ),
    );
}

fn stop_pointer_poll(state: &mut MacState) {
    if let Some(timer) = state.pointer_timer.take() {
        timer.invalidate();
    }
}

fn style_popup(panel: &NSPanel) {
    panel.setHidesOnDeactivate(false);
    panel.setFloatingPanel(true);
    panel.setBecomesKeyOnlyIfNeeded(false);
    panel.setLevel(1_002);
    panel.setHasShadow(true);
}

unsafe fn start(delegate: &AppDelegate, state: &mut MacState) {
    let mtm = state.mtm;
    state.delegate = delegate;
    build_overlays(mtm, state);
    build_status(mtm, state, delegate);
    install_hotkey(state);
    listen_screens(mtm, state);
}

unsafe fn build_overlays(mtm: MainThreadMarker, state: &mut MacState) {
    for overlay in state.overlays.drain(..) {
        overlay.panel.close();
    }

    let screens = NSScreen::screens(mtm);
    for screen in screens.iter() {
        let frame = screen.frame();
        let number = screen_id(&screen);
        let panel = make_panel(mtm, state, number, frame);
        let view = make_view(mtm, state, number, frame);
        panel.setContentView(Some(&view));
        panel.orderOut(None);
        state.overlays.push(Overlay {
            panel,
            view,
            effect: None,
            display_id: number,
            frame,
        });
    }
}

fn screen_id(screen: &NSScreen) -> u32 {
    // NSScreenNumber in device description.
    unsafe {
        let desc = screen.deviceDescription();
        let key = NSString::from_str("NSScreenNumber");
        if let Some(obj) = desc.objectForKey(&key) {
            let n: u32 = msg_send![&obj, unsignedIntValue];
            return n;
        }
    }
    0
}

unsafe fn make_panel(
    mtm: MainThreadMarker,
    state: &MacState,
    display_id: u32,
    frame: NSRect,
) -> Retained<BlackoutPanel> {
    let alloc = BlackoutPanel::alloc(mtm).set_ivars(Ivars {
        state: state as *const MacState as *mut MacState,
        display_id,
    });
    let panel: Retained<BlackoutPanel> = msg_send![
        super(alloc),
        initWithContentRect: frame,
        styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        backing: NSBackingStoreType::Buffered,
        defer: false
    ];
    panel.setFloatingPanel(true);
    panel.setBecomesKeyOnlyIfNeeded(false);
    panel.setHidesOnDeactivate(false);
    panel.setOpaque(false);
    panel.setHasShadow(false);
    panel.setIgnoresMouseEvents(false);
    panel.setAcceptsMouseMovedEvents(true);
    panel.setLevel(1_000); // NSScreenSaverWindowLevel
    panel.setSharingType(NSWindowSharingType::None);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::CanJoinAllApplications,
    );
    panel.setFrame_display(frame, true);
    panel
}

unsafe fn make_view(
    mtm: MainThreadMarker,
    state: &MacState,
    display_id: u32,
    frame: NSRect,
) -> Retained<OverlayView> {
    let alloc = OverlayView::alloc(mtm).set_ivars(Ivars {
        state: state as *const MacState as *mut MacState,
        display_id,
    });
    let view: Retained<OverlayView> =
        msg_send![super(alloc), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), frame.size)];
    view.setWantsLayer(true);
    let area = NSTrackingArea::initWithRect_options_owner_userInfo(
        NSTrackingArea::alloc(),
        view.bounds(),
        NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::MouseMoved
            | NSTrackingAreaOptions::ActiveAlways
            | NSTrackingAreaOptions::InVisibleRect,
        Some(&*view),
        None,
    );
    view.addTrackingArea(&area);
    view
}

fn pointer(ivars: &Ivars) {
    unsafe { follow_cursor(&mut *ivars.state) }
}

fn follow_cursor(state: &mut MacState) {
    if state.logic.mode != Mode::Pick {
        return;
    }
    if !state.logic.follow_pointer {
        let loc = NSEvent::mouseLocation();
        if let Some(anchor) = state.pointer_anchor {
            if (loc.x - anchor.x).abs() <= 6.0 && (loc.y - anchor.y).abs() <= 6.0 {
                return;
            }
        }
        state.pointer_anchor = None;
        state.logic.allow_pointer();
    }
    let Some(id) = cursor_display(state) else {
        return;
    };
    let action = state.logic.on_pointer(&id);
    unsafe { apply(state, action) }
}

fn click(ivars: &Ivars) {
    unsafe {
        let state = &mut *ivars.state;
        let id = ivars.display_id.to_string();
        let action = state.logic.on_click(&id);
        apply(state, action);
    }
}

fn paint(view: &OverlayView) {
    unsafe {
        let state = &*view.ivars().state;
        let id = view.ivars().display_id.to_string();
        let is_candidate = state.logic.mode == crate::app::Mode::Pick
            && state.logic.candidate.as_deref() == Some(&id);
        let bounds = view.bounds();
        if is_candidate {
            NSColor::clearColor().setFill();
            objc2_app_kit::NSBezierPath::fillRect(bounds);
            let color = NSColor::colorWithCalibratedRed_green_blue_alpha(
                0x3D as f64 / 255.0,
                0xDC as f64 / 255.0,
                0x97 as f64 / 255.0,
                1.0,
            );
            color.setStroke();
            let path = objc2_app_kit::NSBezierPath::bezierPathWithRect(bounds);
            path.setLineWidth(BORDER * 2.0);
            path.stroke();
        } else if state.logic.style == Style::Solid {
            NSColor::blackColor().setFill();
            objc2_app_kit::NSBezierPath::fillRect(bounds);
        } else {
            NSColor::colorWithCalibratedRed_green_blue_alpha(
                0.0,
                0.0,
                0.0,
                f64::from(FROST_ALPHA) / 255.0,
            )
            .setFill();
            objc2_app_kit::NSBezierPath::fillRect(bounds);
        }
    }
}

unsafe fn apply(state: &mut MacState, action: Action) {
    let rebuild = !matches!(action, Action::None | Action::RefreshPick { .. });
    match &action {
        Action::None => {}
        Action::HideAll => {
            stop_pointer_poll(state);
            set_pick_keys(state, false);
            for overlay in &state.overlays {
                overlay.panel.orderOut(None);
            }
        }
        Action::ShowPick { candidate } | Action::RefreshPick { candidate } => {
            ensure_pointer_poll(state);
            set_pick_keys(state, true);
            for overlay in &mut state.overlays {
                present(
                    overlay,
                    overlay.display_id.to_string() == *candidate,
                    true,
                    state.logic.style,
                );
            }
        }
        Action::ShowLocked { cinema } => {
            stop_pointer_poll(state);
            set_pick_keys(state, false);
            for overlay in &mut state.overlays {
                if overlay.display_id.to_string() == *cinema {
                    overlay.panel.orderOut(None);
                } else {
                    present(overlay, false, false, state.logic.style);
                }
            }
        }
    }
    if rebuild {
        if let Some(delegate) = state.delegate.as_ref() {
            rebuild_status_menu(state, delegate);
        }
    }
}

unsafe fn present(overlay: &mut Overlay, candidate: bool, _pick: bool, style: Style) {
    overlay.panel.setFrame_display(overlay.frame, true);
    apply_frost(overlay, !candidate && style == Style::Frost);
    overlay.view.setNeedsDisplay(true);
    overlay.panel.orderFrontRegardless();
}

unsafe fn apply_frost(overlay: &mut Overlay, on: bool) {
    if on {
        if overlay.effect.is_none() {
            let effect = NSVisualEffectView::initWithFrame(
                NSVisualEffectView::alloc(overlay.view.mtm()),
                overlay.view.bounds(),
            );
            effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            effect.setMaterial(NSVisualEffectMaterial::FullScreenUI);
            effect.setState(NSVisualEffectState::Active);
            effect.setAutoresizingMask(
                objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                    | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            overlay.view.addSubview_positioned_relativeTo(
                &effect,
                objc2_app_kit::NSWindowOrderingMode::Below,
                None,
            );
            overlay.effect = Some(effect);
        }
    } else if let Some(effect) = overlay.effect.take() {
        effect.removeFromSuperview();
    }
}

unsafe fn build_status(mtm: MainThreadMarker, state: &mut MacState, delegate: &AppDelegate) {
    let bar = NSStatusBar::systemStatusBar();
    let item = bar.statusItemWithLength(objc2_app_kit::NSSquareStatusItemLength);
    if let Some(button) = item.button(mtm) {
        if let Some(image) = status_image(mtm) {
            button.setImage(Some(&image));
            button.setTitle(&NSString::from_str(""));
        } else {
            button.setTitle(&NSString::from_str("●"));
        }
        button.setToolTip(Some(&NSString::from_str("Blackout")));
    }
    state.status = Some(item);
    rebuild_status_menu(state, delegate);
}

unsafe fn rebuild_status_menu(state: &mut MacState, delegate: &AppDelegate) {
    let Some(item) = state.status.as_ref() else {
        return;
    };
    let mtm = state.mtm;
    let lang = state.config.ui_lang();
    let t = lang.tr();
    let menu = NSMenu::new(mtm);
    add_item(
        mtm,
        &menu,
        lang.toggle(state.logic.mode == Mode::Idle),
        objc2::sel!(statusToggle:),
        delegate,
    );
    let styles = NSMenu::new(mtm);
    let solid = add_item(mtm, &styles, t.solid, objc2::sel!(statusSolid:), delegate);
    let tint = add_item(mtm, &styles, t.tint, objc2::sel!(statusTint:), delegate);
    let frost = add_item(mtm, &styles, t.frost, objc2::sel!(statusFrost:), delegate);
    solid.setState(if state.logic.style == Style::Solid {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    tint.setState(if state.logic.style == Style::Tint {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    frost.setState(if state.logic.style == Style::Frost {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    let style_item = NSMenuItem::new(mtm);
    style_item.setTitle(&NSString::from_str(t.style));
    style_item.setSubmenu(Some(&styles));
    menu.addItem(&style_item);
    let settings = NSMenu::new(mtm);
    add_item(
        mtm,
        &settings,
        &lang.hotkey_item(&state.config.hotkey.to_string(), true),
        objc2::sel!(statusHotkey:),
        delegate,
    );
    let autostart = add_item(
        mtm,
        &settings,
        t.autostart,
        objc2::sel!(statusAutostart:),
        delegate,
    );
    autostart.setState(if crate::autostart::is_enabled() {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    state.autostart_item = Some(autostart);
    let languages = NSMenu::new(mtm);
    for (i, item_lang) in Language::ALL.iter().enumerate() {
        let entry = add_item(
            mtm,
            &languages,
            item_lang.endonym(),
            objc2::sel!(statusLang:),
            delegate,
        );
        let _: () = msg_send![&*entry, setTag: i as isize];
        entry.setState(if *item_lang == lang {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
    }
    let language_item = NSMenuItem::new(mtm);
    language_item.setTitle(&NSString::from_str(t.language));
    language_item.setSubmenu(Some(&languages));
    settings.addItem(&language_item);
    let sep: Retained<NSMenuItem> = unsafe { msg_send![class!(NSMenuItem), separatorItem] };
    settings.addItem(&sep);
    add_item(
        mtm,
        &settings,
        t.wipe,
        objc2::sel!(statusWipe:),
        delegate,
    );
    let settings_item = NSMenuItem::new(mtm);
    settings_item.setTitle(&NSString::from_str(t.settings));
    settings_item.setSubmenu(Some(&settings));
    menu.addItem(&settings_item);
    add_item(
        mtm,
        &menu,
        t.about,
        objc2::sel!(statusAbout:),
        delegate,
    );
    add_item(mtm, &menu, t.quit, objc2::sel!(statusQuit:), delegate);
    item.setMenu(Some(&menu));
}

unsafe fn redraw_text_panels(state: &MacState) {
    if let Some(panel) = &state.bind {
        panel.setTitle(&NSString::from_str(state.config.ui_lang().tr().bind_title));
        if let Some(view) = panel.contentView() {
            view.setNeedsDisplay(true);
        }
    }
    if let Some(panel) = &state.wipe {
        if let Some(view) = panel.contentView() {
            view.setNeedsDisplay(true);
        }
    }
    if let Some(panel) = &state.about {
        if let Some(view) = panel.contentView() {
            view.setNeedsDisplay(true);
        }
    }
}

fn status_image(_mtm: MainThreadMarker) -> Option<Retained<NSImage>> {
    let data = NSData::with_bytes(crate::icon_png::PNG_32);
    let image = NSImage::initWithData(NSImage::alloc(), &data)?;
    image.setSize(NSSize::new(18.0, 18.0));
    Some(image)
}

fn add_item(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    title: &str,
    action: objc2::runtime::Sel,
    delegate: &AppDelegate,
) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str(title));
    unsafe {
        item.setAction(Some(action));
        item.setTarget(Some(delegate));
    }
    menu.addItem(&item);
    item
}

fn set_style(state: &mut MacState, style: Style, delegate: &AppDelegate) {
    state.config.style = style;
    state.config.save();
    let action = state.logic.set_style(style);
    unsafe {
        apply(state, action);
        rebuild_status_menu(state, delegate);
    }
}

fn key_screen(mtm: MainThreadMarker) -> Option<Retained<NSScreen>> {
    NSScreen::mainScreen(mtm).or_else(|| NSScreen::screens(mtm).iter().next())
}

fn cursor_display(state: &MacState) -> Option<String> {
    let loc = NSEvent::mouseLocation();
    state
        .overlays
        .iter()
        .find(|o| {
            loc.x >= o.frame.origin.x
                && loc.y >= o.frame.origin.y
                && loc.x < o.frame.origin.x + o.frame.size.width
                && loc.y < o.frame.origin.y + o.frame.size.height
        })
        .map(|o| o.display_id.to_string())
        .or_else(|| state.overlays.first().map(|o| o.display_id.to_string()))
}

unsafe fn listen_screens(mtm: MainThreadMarker, state: &MacState) {
    let _ = (mtm, state);
    // NSApplication.didChangeScreenParametersNotification is handled on next toggle/rebuild.
}

unsafe fn install_hotkey(state: &mut MacState) {
    #[repr(C)]
    struct EventHotKeyID {
        signature: u32,
        id: u32,
    }
    #[repr(C)]
    struct EventTypeSpec {
        event_class: u32,
        event_kind: u32,
    }
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn RegisterEventHotKey(
            virtual_key: u32,
            modifiers: u32,
            id: EventHotKeyID,
            target: *mut std::ffi::c_void,
            options: u32,
            out: *mut *mut std::ffi::c_void,
        ) -> i32;
        fn UnregisterEventHotKey(hot: *mut std::ffi::c_void) -> i32;
        fn GetApplicationEventTarget() -> *mut std::ffi::c_void;
        fn InstallEventHandler(
            target: *mut std::ffi::c_void,
            handler: unsafe extern "C" fn(
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
            ) -> i32,
            count: u32,
            specs: *const EventTypeSpec,
            user: *mut std::ffi::c_void,
            out: *mut *mut std::ffi::c_void,
        ) -> i32;
    }

    if !state.hotkey_ref.is_null() {
        let _ = UnregisterEventHotKey(state.hotkey_ref);
        state.hotkey_ref = std::ptr::null_mut();
    }
    let Some(vk) = state.config.hotkey.mac_vk() else {
        return;
    };
    let id = EventHotKeyID {
        signature: u32::from_be_bytes(*b"BLKO"),
        id: 1,
    };
    let mut hot = std::ptr::null_mut();
    let target = GetApplicationEventTarget();
    static HANDLER_ONCE: std::sync::Once = std::sync::Once::new();
    HANDLER_ONCE.call_once(|| {
        let spec = EventTypeSpec {
            event_class: u32::from_be_bytes(*b"keyb"),
            event_kind: 5, // kEventHotKeyPressed
        };
        let mut handler = std::ptr::null_mut();
        let _ = InstallEventHandler(
            target,
            carbon_hotkey,
            1,
            &spec,
            state as *mut MacState as *mut _,
            &mut handler,
        );
    });
    if RegisterEventHotKey(vk, state.config.hotkey.mac_mods(), id, target, 0, &mut hot) == 0 {
        state.hotkey_ref = hot;
    }
}

unsafe extern "C" fn carbon_hotkey(
    _next: *mut std::ffi::c_void,
    event: *mut std::ffi::c_void,
    user: *mut std::ffi::c_void,
) -> i32 {
    #[repr(C)]
    struct EventHotKeyID {
        signature: u32,
        id: u32,
    }
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn GetEventParameter(
            event: *mut std::ffi::c_void,
            name: u32,
            wanted: u32,
            actual: *mut u32,
            size: u32,
            actual_size: *mut u32,
            data: *mut std::ffi::c_void,
        ) -> i32;
    }
    let mut hkid = EventHotKeyID {
        signature: u32::from_be_bytes(*b"BLKO"),
        id: 1,
    };
    let _ = GetEventParameter(
        event,
        u32::from_be_bytes(*b"----"),
        u32::from_be_bytes(*b"hkid"),
        std::ptr::null_mut(),
        std::mem::size_of::<EventHotKeyID>() as u32,
        std::ptr::null_mut(),
        &mut hkid as *mut _ as *mut _,
    );
    let state = &mut *(user as *mut MacState);
    let ids: Vec<String> = state
        .overlays
        .iter()
        .map(|o| o.display_id.to_string())
        .collect();
    let action = match hkid.id {
        2 => state.logic.on_pick_key(PickKey::Escape, &[]),
        3 => state.logic.on_pick_key(PickKey::Enter, &ids),
        4 => {
            state.pointer_anchor = Some(NSEvent::mouseLocation());
            state.logic.on_pick_key(PickKey::Left, &ids)
        }
        5 => {
            state.pointer_anchor = Some(NSEvent::mouseLocation());
            state.logic.on_pick_key(PickKey::Right, &ids)
        }
        id if (10..19).contains(&id) => {
            state.pointer_anchor = Some(NSEvent::mouseLocation());
            state
                .logic
                .on_pick_key(PickKey::Digit((id - 9) as u8), &ids)
        }
        _ => state.logic.on_toggle(cursor_display(state)),
    };
    apply(state, action);
    0
}

unsafe fn set_pick_keys(state: &mut MacState, on: bool) {
    #[repr(C)]
    struct EventHotKeyID {
        signature: u32,
        id: u32,
    }
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn RegisterEventHotKey(
            virtual_key: u32,
            modifiers: u32,
            id: EventHotKeyID,
            target: *mut std::ffi::c_void,
            options: u32,
            out: *mut *mut std::ffi::c_void,
        ) -> i32;
        fn UnregisterEventHotKey(hot: *mut std::ffi::c_void) -> i32;
        fn GetApplicationEventTarget() -> *mut std::ffi::c_void;
    }
    for hot in state.pick_refs.drain(..) {
        let _ = UnregisterEventHotKey(hot);
    }
    if !on {
        return;
    }
    let target = GetApplicationEventTarget();
    let signature = u32::from_be_bytes(*b"BLKO");
    let keys = [
        (2, 0x35),
        (3, 0x24),
        (4, 0x7B),
        (5, 0x7C),
        (10, 0x12),
        (11, 0x13),
        (12, 0x14),
        (13, 0x15),
        (14, 0x17),
        (15, 0x16),
        (16, 0x1A),
        (17, 0x1C),
        (18, 0x19),
    ];
    for (id, vk) in keys {
        let hid = EventHotKeyID { signature, id };
        let mut hot = std::ptr::null_mut();
        if RegisterEventHotKey(vk, 0, hid, target, 0, &mut hot) == 0 && !hot.is_null() {
            state.pick_refs.push(hot);
        }
    }
}

unsafe fn brand_green() -> Retained<NSColor> {
    NSColor::colorWithCalibratedRed_green_blue_alpha(
        0x3D as f64 / 255.0,
        0xDC as f64 / 255.0,
        0x97 as f64 / 255.0,
        1.0,
    )
}

unsafe fn stroke_bind_button(rect: NSRect) {
    brand_green().setStroke();
    let path = objc2_app_kit::NSBezierPath::bezierPathWithRect(rect);
    path.setLineWidth(1.0);
    path.stroke();
}

fn popup_button_rects(bounds: NSRect) -> (NSRect, NSRect) {
    let mid = bounds.size.width / 2.0;
    let y = bounds.size.height - 52.0;
    (
        NSRect::new(NSPoint::new(mid - 184.0, y), NSSize::new(176.0, 40.0)),
        NSRect::new(NSPoint::new(mid + 8.0, y), NSSize::new(176.0, 40.0)),
    )
}

fn popup_view_point(view: &NSView, event: &NSEvent) -> NSPoint {
    view.convertPoint_fromView(event.locationInWindow(), None)
}

fn popup_button_hit(view: &NSView, event: &NSEvent) -> Option<bool> {
    let point = popup_view_point(view, event);
    let bounds = view.bounds();
    let y0 = bounds.size.height - 52.0;
    let y1 = bounds.size.height - 12.0;
    if point.y < y0 || point.y > y1 {
        return None;
    }
    let mid = bounds.size.width / 2.0;
    if point.x >= mid - 184.0 && point.x < mid - 8.0 {
        Some(true)
    } else if point.x >= mid + 8.0 && point.x <= mid + 184.0 {
        Some(false)
    } else {
        None
    }
}

unsafe fn draw_popup_text(
    text: &NSString,
    rect: NSRect,
    r: f64,
    g: f64,
    b: f64,
    font_size: f64,
    center: bool,
    vcenter: bool,
) {
    let color = NSColor::colorWithCalibratedRed_green_blue_alpha(r, g, b, 1.0);
    let font: *mut objc2::runtime::AnyObject = msg_send![class!(NSFont), systemFontOfSize: font_size];
    let dict: *mut objc2::runtime::AnyObject = msg_send![class!(NSMutableDictionary), dictionary];
    let color_key = NSString::from_str("NSColor");
    let font_key = NSString::from_str("NSFont");
    let _: () = msg_send![dict, setObject: &*color, forKey: &*color_key];
    let _: () = msg_send![dict, setObject: font, forKey: &*font_key];
    // UsesLineFragmentOrigin | UsesFontLeading — otherwise drawInRect in a
    // flipped view pins the line to the right edge.
    const LINE_FRAG: isize = 1 | 2;
    let used: NSRect = msg_send![
        text,
        boundingRectWithSize: NSSize::new(rect.size.width, 10_000.0),
        options: LINE_FRAG,
        attributes: dict
    ];
    let w = used.size.width.min(rect.size.width).max(1.0);
    let h = used.size.height.min(rect.size.height).max(1.0);
    let dest = NSRect::new(
        NSPoint::new(
            if center {
                rect.origin.x + (rect.size.width - w) / 2.0
            } else {
                rect.origin.x
            },
            if vcenter {
                rect.origin.y + (rect.size.height - h) / 2.0
            } else {
                rect.origin.y
            },
        ),
        NSSize::new(w, h),
    );
    let _: () = msg_send![
        text,
        drawWithRect: dest,
        options: LINE_FRAG,
        attributes: dict
    ];
}

unsafe fn begin_bind(state: &mut MacState) {
    if state.wipe.is_some() {
        return;
    }
    close_about(state);
    if let Some(panel) = &state.bind {
        panel.makeKeyAndOrderFront(None);
        panel.orderFrontRegardless();
        return;
    }
    if !state.hotkey_ref.is_null() {
        #[link(name = "Carbon", kind = "framework")]
        extern "C" {
            fn UnregisterEventHotKey(hot: *mut std::ffi::c_void) -> i32;
        }
        let _ = UnregisterEventHotKey(state.hotkey_ref);
        state.hotkey_ref = std::ptr::null_mut();
    }

    let Some(screen) = key_screen(state.mtm) else {
        install_hotkey(state);
        return;
    };
    let visible = screen.visibleFrame();
    let size = NSSize::new(560.0, 280.0);
    state.bind_draft = None;
    state.bind_preview.clear();
    state.bind_locked = false;
    state.bind_held_key = None;
    let origin = NSPoint::new(
        visible.origin.x + (visible.size.width - size.width) / 2.0,
        visible.origin.y + (visible.size.height - size.height) / 2.0,
    );
    let frame = NSRect::new(origin, size);
    let alloc = BindPanel::alloc(state.mtm).set_ivars(BindIvars {
        state: state as *mut MacState,
    });
    let panel: Retained<BindPanel> = msg_send![
        super(alloc),
        initWithContentRect: frame,
        styleMask: NSWindowStyleMask::Titled,
        backing: NSBackingStoreType::Buffered,
        defer: false
    ];
    style_popup(&panel);
    panel.setTitle(&NSString::from_str(state.config.ui_lang().tr().bind_title));
    let view_alloc = BindView::alloc(state.mtm).set_ivars(BindIvars {
        state: state as *mut MacState,
    });
    let view: Retained<BindView> =
        msg_send![super(view_alloc), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), size)];
    panel.setContentView(Some(&view));
    panel.makeFirstResponder(Some(&view));
    panel.makeKeyAndOrderFront(None);
    panel.orderFrontRegardless();
    state.bind = Some(panel);
}

fn mac_mods(flags: usize) -> (bool, bool, bool, bool) {
    (
        flags & (1 << 18) != 0,
        flags & (1 << 19) != 0,
        flags & (1 << 17) != 0,
        flags & (1 << 20) != 0,
    )
}

fn mac_combo_held(state: &MacState, flags: usize) -> bool {
    let (ctrl, alt, shift, meta) = mac_mods(flags);
    ctrl || alt || shift || meta || state.bind_held_key.is_some()
}

fn redraw_bind(state: &MacState) {
    if let Some(panel) = &state.bind {
        if let Some(view) = panel.contentView() {
            view.setNeedsDisplay(true);
        }
    }
}

unsafe fn handle_bind_flags(state: &mut MacState, event: &NSEvent) {
    let flags: usize = msg_send![event, modifierFlags];
    let (ctrl, alt, shift, meta) = mac_mods(flags);
    if state.bind_locked {
        if !mac_combo_held(state, flags) {
            state.bind_locked = false;
        }
        return;
    }
    if state.bind_draft.is_some() && (ctrl || alt || shift || meta) {
        state.bind_draft = None;
        state.bind_preview.clear();
    }
    let live = format_combo(ctrl, alt, shift, meta, None);
    if !live.is_empty() {
        state.bind_preview = live;
    } else if let Some(draft) = state.bind_draft {
        state.bind_preview = draft.to_string();
    } else {
        state.bind_preview.clear();
    }
    redraw_bind(state);
}

unsafe fn handle_bind_keyup(state: &mut MacState, event: &NSEvent) {
    if state.bind_held_key == Some(event.keyCode()) {
        state.bind_held_key = None;
    }
    let flags: usize = msg_send![event, modifierFlags];
    if state.bind_locked {
        if !mac_combo_held(state, flags) {
            state.bind_locked = false;
        }
        return;
    }
    handle_bind_flags(state, event);
}

unsafe fn handle_bind_key(state: &mut MacState, event: &NSEvent) {
    let code = event.keyCode();
    if code == 53 {
        close_bind(state);
        install_hotkey(state);
        return;
    }
    let flags: usize = msg_send![event, modifierFlags];
    let (ctrl, alt, shift, meta) = mac_mods(flags);
    if code == 36 && !ctrl && !alt && !shift && !meta && state.bind_draft.is_some() {
        commit_bind(state);
        return;
    }
    if state.bind_locked {
        return;
    }
    if state.bind_draft.is_some() {
        state.bind_draft = None;
        state.bind_preview.clear();
    }
    let Some(key) = key_from_mac_vk(code) else {
        redraw_bind(state);
        return;
    };
    if let Some(hotkey) = Hotkey::new(ctrl, alt, shift, meta, key) {
        state.bind_draft = Some(hotkey);
        state.bind_preview = hotkey.to_string();
        state.bind_locked = true;
        state.bind_held_key = Some(code);
    } else {
        state.bind_preview = format_combo(ctrl, alt, shift, meta, Some(key));
    }
    redraw_bind(state);
}

unsafe fn commit_bind(state: &mut MacState) {
    let Some(hotkey) = state.bind_draft.take() else {
        return;
    };
    close_bind(state);
    state.config.hotkey = hotkey;
    state.config.save();
    install_hotkey(state);
    if let Some(item) = &state.status {
        if let Some(button) = item.button(state.mtm) {
            button.setToolTip(Some(&NSString::from_str("Blackout")));
        }
    }
    if let Some(delegate) = state.delegate.as_ref() {
        rebuild_status_menu(state, delegate);
    }
}

unsafe fn close_bind(state: &mut MacState) {
    if let Some(panel) = state.bind.take() {
        panel.close();
    }
}

unsafe fn begin_wipe(state: &mut MacState) {
    if let Some(panel) = &state.wipe {
        panel.makeKeyAndOrderFront(None);
        panel.orderFrontRegardless();
        return;
    }
    close_about(state);
    if state.bind.is_some() {
        close_bind(state);
    }
    let action = state.logic.dismiss();
    apply(state, action);
    if !state.hotkey_ref.is_null() {
        #[link(name = "Carbon", kind = "framework")]
        extern "C" {
            fn UnregisterEventHotKey(hot: *mut std::ffi::c_void) -> i32;
        }
        let _ = UnregisterEventHotKey(state.hotkey_ref);
        state.hotkey_ref = std::ptr::null_mut();
    }
    let Some(screen) = key_screen(state.mtm) else {
        install_hotkey(state);
        return;
    };
    let visible = screen.visibleFrame();
    let size = NSSize::new(560.0, 380.0);
    state.wipe_error.clear();
    let origin = NSPoint::new(
        visible.origin.x + (visible.size.width - size.width) / 2.0,
        visible.origin.y + (visible.size.height - size.height) / 2.0,
    );
    let frame = NSRect::new(origin, size);
    let alloc = WipePanel::alloc(state.mtm).set_ivars(BindIvars {
        state: state as *mut MacState,
    });
    let panel: Retained<WipePanel> = msg_send![
        super(alloc),
        initWithContentRect: frame,
        styleMask: NSWindowStyleMask::Titled,
        backing: NSBackingStoreType::Buffered,
        defer: false
    ];
    style_popup(&panel);
    panel.setTitle(&NSString::from_str("Blackout"));
    let view_alloc = WipeView::alloc(state.mtm).set_ivars(BindIvars {
        state: state as *mut MacState,
    });
    let view: Retained<WipeView> =
        msg_send![super(view_alloc), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), size)];
    panel.setContentView(Some(&view));
    panel.makeFirstResponder(Some(&view));
    panel.makeKeyAndOrderFront(None);
    panel.orderFrontRegardless();
    state.wipe = Some(panel);
}

unsafe fn close_wipe(state: &mut MacState) {
    if let Some(panel) = state.wipe.take() {
        panel.close();
    }
    state.wipe_error.clear();
}

unsafe fn begin_about(state: &mut MacState) {
    if state.wipe.is_some() {
        return;
    }
    if state.bind.is_some() {
        return;
    }
    if let Some(panel) = &state.about {
        panel.makeKeyAndOrderFront(None);
        panel.orderFrontRegardless();
        return;
    }
    let Some(screen) = key_screen(state.mtm) else {
        return;
    };
    let visible = screen.visibleFrame();
    let size = NSSize::new(560.0, 300.0);
    let origin = NSPoint::new(
        visible.origin.x + (visible.size.width - size.width) / 2.0,
        visible.origin.y + (visible.size.height - size.height) / 2.0,
    );
    let frame = NSRect::new(origin, size);
    let alloc = AboutPanel::alloc(state.mtm).set_ivars(BindIvars {
        state: state as *mut MacState,
    });
    let panel: Retained<AboutPanel> = msg_send![
        super(alloc),
        initWithContentRect: frame,
        styleMask: NSWindowStyleMask::Titled,
        backing: NSBackingStoreType::Buffered,
        defer: false
    ];
    style_popup(&panel);
    panel.setTitle(&NSString::from_str("Blackout"));
    let view_alloc = AboutView::alloc(state.mtm).set_ivars(BindIvars {
        state: state as *mut MacState,
    });
    let view: Retained<AboutView> =
        msg_send![super(view_alloc), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), size)];
    panel.setContentView(Some(&view));
    panel.makeFirstResponder(Some(&view));
    panel.makeKeyAndOrderFront(None);
    panel.orderFrontRegardless();
    state.about = Some(panel);
}

unsafe fn close_about(state: &mut MacState) {
    if let Some(panel) = state.about.take() {
        panel.close();
    }
}

unsafe fn commit_wipe(state: &mut MacState) {
    match crate::purge::wipe() {
        Ok(()) => {
            close_wipe(state);
            let app = NSApplication::sharedApplication(state.mtm);
            app.terminate(None);
        }
        Err(_) => {
            state.wipe_error = state.config.ui_lang().tr().wipe_failed.to_string();
            if let Some(panel) = &state.wipe {
                if let Some(view) = panel.contentView() {
                    view.setNeedsDisplay(true);
                }
            }
        }
    }
}
