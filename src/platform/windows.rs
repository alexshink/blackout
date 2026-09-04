use std::cell::Cell;
use std::mem::size_of;
use std::ptr::{null, null_mut};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, EndPaint, EnumDisplayMonitors, FillRect, FrameRect, GetMonitorInfoW,
    GetStockObject, SelectObject, SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLACK_BRUSH, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_QUALITY, DIB_RGB_COLORS, DT_CENTER,
    DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, HBRUSH, HDC, HGDIOBJ, HMONITOR, MONITORINFOEXW,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL,
    MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VIRTUAL_KEY, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8,
    VK_9, VK_CONTROL, VK_ESCAPE, VK_LEFT, VK_LWIN, VK_MENU, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_INFO,
    NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW,
    NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu,
    DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, KillTimer, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassExW,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TrackPopupMenu,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HICON, HWND_TOP, HWND_TOPMOST,
    IDC_ARROW, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, SM_CXSMICON,
    SM_CYSMICON, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, SW_SHOWNOACTIVATE, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_APP,
    WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ENDSESSION,
    WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCCREATE,
    WM_PAINT, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::app::{Action, Logic, Mode, PickKey};
use crate::config::{Config, Style, FROST_ALPHA};
use crate::hotkey::{format_combo, key_from_win_vk, Hotkey};
use crate::i18n::Language;

const HOST_CLASS: PCWSTR = w!("BlackoutHost");
const OVERLAY_CLASS: PCWSTR = w!("BlackoutOverlay");
const BIND_CLASS: PCWSTR = w!("BlackoutBind");
const WIPE_CLASS: PCWSTR = w!("BlackoutWipe");
const ABOUT_CLASS: PCWSTR = w!("BlackoutAbout");
const MUTEX_NAME: PCWSTR = w!("Local\\BlackoutCinema.single");

const WM_TRAY: u32 = WM_APP + 1;
const WM_EXTERNAL_TOGGLE: u32 = WM_APP + 2;
const WM_APPLY_HOTKEY: u32 = WM_APP + 3;
const NIN_SELECT: u32 = 0x0400;
const NIN_KEYSELECT: u32 = 0x0401;

const HOTKEY_TOGGLE: i32 = 1;
const HOTKEY_ESC: i32 = 2;
const HOTKEY_ENTER: i32 = 3;
const HOTKEY_LEFT: i32 = 4;
const HOTKEY_RIGHT: i32 = 5;
const HOTKEY_DIGIT_BASE: i32 = 10;

const TIMER_REASSERT: usize = 1;
const TIMER_HOTPLUG: usize = 2;
const TIMER_ZORDER_KICK: usize = 3;
const ZORDER_KICKS: u8 = 8;
const ZORDER_KICK_MS: u32 = 10;

const CMD_TOGGLE: usize = 1;
const CMD_QUIT: usize = 2;
const CMD_STYLE_SOLID: usize = 3;
const CMD_STYLE_FROST: usize = 4;
const CMD_HOTKEY: usize = 5;
const CMD_AUTOSTART: usize = 6;
const CMD_WIPE: usize = 7;
const CMD_STYLE_TINT: usize = 8;
const CMD_ABOUT: usize = 9;
const CMD_LANG_BASE: usize = 20;

const BORDER_DIP: i32 = 8;
const BORDER_R: u8 = 0x3D;
const BORDER_G: u8 = 0xDC;
const BORDER_B: u8 = 0x97;

const DWMSBT_NONE: u32 = 1;
const DWMSBT_TRANSIENTWINDOW: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Hidden,
    Veil,
    Candidate,
}

struct Overlay {
    hwnd: HWND,
    id: String,
    rect: RECT,
    applied: Cell<Option<(Role, Style)>>,
}

struct State {
    logic: Logic,
    config: Config,
    overlays: Vec<Overlay>,
    host: HWND,
    tray: NOTIFYICONDATAW,
    icon: HICON,
    pick_keys: bool,
    pointer_anchor: Option<POINT>,
    tray_ready_at: std::time::Instant,
    tray_last_toggle: std::time::Instant,
    bind_hwnd: HWND,
    bind_draft: Option<Hotkey>,
    bind_preview: String,
    bind_locked: bool,
    bind_win: bool,
    pending_hotkey: Option<Hotkey>,
    hotkey_ok: bool,
    zorder_kicks: u8,
    wipe_hwnd: HWND,
    wipe_error: String,
    about_hwnd: HWND,
}

pub fn run() -> Result<(), String> {
    unsafe { run_inner() }
}

unsafe fn run_inner() -> Result<(), String> {
    let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    let mutex = CreateMutexW(None, true, MUTEX_NAME).map_err(|e| e.to_string())?;
    if GetLastError() == ERROR_ALREADY_EXISTS {
        let _ = mutex;
        if let Ok(existing) = FindWindowW(HOST_CLASS, None) {
            let _ = PostMessageW(Some(existing), WM_EXTERNAL_TOGGLE, WPARAM(0), LPARAM(0));
        }
        return Ok(());
    }

    let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
    let cursor = LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?;

    let host_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(host_proc),
        hInstance: instance.into(),
        hCursor: cursor,
        lpszClassName: HOST_CLASS,
        ..Default::default()
    };
    if RegisterClassExW(&host_class) == 0 {
        return Err("RegisterClassExW host failed".into());
    }

    let overlay_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(overlay_proc),
        hInstance: instance.into(),
        hCursor: cursor,
        lpszClassName: OVERLAY_CLASS,
        hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
        ..Default::default()
    };
    if RegisterClassExW(&overlay_class) == 0 {
        return Err("RegisterClassExW overlay failed".into());
    }

    let bind_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(bind_proc),
        hInstance: instance.into(),
        hCursor: cursor,
        lpszClassName: BIND_CLASS,
        hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
        ..Default::default()
    };
    if RegisterClassExW(&bind_class) == 0 {
        return Err("RegisterClassExW bind failed".into());
    }

    let wipe_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wipe_proc),
        hInstance: instance.into(),
        hCursor: cursor,
        lpszClassName: WIPE_CLASS,
        hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
        ..Default::default()
    };
    if RegisterClassExW(&wipe_class) == 0 {
        return Err("RegisterClassExW wipe failed".into());
    }

    let about_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(about_proc),
        hInstance: instance.into(),
        hCursor: cursor,
        lpszClassName: ABOUT_CLASS,
        hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
        ..Default::default()
    };
    if RegisterClassExW(&about_class) == 0 {
        return Err("RegisterClassExW about failed".into());
    }

    let config = Config::load();
    crate::autostart::refresh_path();
    let icon = create_tray_icon();
    let mut state = Box::new(State {
        logic: Logic::new(config.style),
        config,
        overlays: Vec::new(),
        host: HWND(null_mut()),
        tray: NOTIFYICONDATAW::default(),
        icon,
        pick_keys: false,
        pointer_anchor: None,
        tray_ready_at: std::time::Instant::now() + std::time::Duration::from_millis(400),
        tray_last_toggle: std::time::Instant::now() - std::time::Duration::from_secs(1),
        bind_hwnd: HWND(null_mut()),
        bind_draft: None,
        bind_preview: String::new(),
        bind_locked: false,
        bind_win: false,
        pending_hotkey: None,
        hotkey_ok: false,
        zorder_kicks: 0,
        wipe_hwnd: HWND(null_mut()),
        wipe_error: String::new(),
        about_hwnd: HWND(null_mut()),
    });

    let host = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        HOST_CLASS,
        w!("Blackout"),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        Some(instance.into()),
        Some(state.as_mut() as *mut State as *const _),
    )
    .map_err(|e| e.to_string())?;
    state.host = host;

    add_tray(&mut state);
    rebuild_overlays(&mut state);
    register_toggle_hotkey(&mut state);
    if !state.hotkey_ok {
        let lang = state.config.ui_lang();
        let occupied = state.config.hotkey.to_string();
        tray_note(
            &mut state,
            lang.tr().hotkey_busy_title,
            &lang.hotkey_busy_startup(&occupied),
            true,
        );
    }

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    let _ = Shell_NotifyIconW(NIM_DELETE, &state.tray);
    if !state.icon.is_invalid() {
        let _ = DestroyIcon(state.icon);
    }
    let _ = mutex;
    Ok(())
}

unsafe fn state_from(hwnd: HWND) -> Option<&'static mut State> {
    let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if raw == 0 {
        None
    } else {
        Some(&mut *(raw as *mut State))
    }
}

unsafe extern "system" fn host_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(1);
    }

    let Some(state) = state_from(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };

    match msg {
        WM_HOTKEY => {
            handle_hotkey(state, wparam.0 as i32);
            LRESULT(0)
        }
        WM_EXTERNAL_TOGGLE => {
            let action = state.logic.on_toggle(cursor_display_id());
            apply(state, action);
            LRESULT(0)
        }
        WM_APPLY_HOTKEY => {
            apply_pending_hotkey(state);
            LRESULT(0)
        }
        WM_TRAY => {
            if std::time::Instant::now() >= state.tray_ready_at {
                // VERSION_4: event in LOWORD. One physical click can send
                // both NIN_SELECT and WM_LBUTTONUP — debounce so we don't
                // enter pick and immediately leave.
                match (lparam.0 as u32) & 0xFFFF {
                    NIN_SELECT | NIN_KEYSELECT | WM_LBUTTONUP | 0x0203 => {
                        tray_toggle(state);
                    }
                    WM_RBUTTONUP | WM_CONTEXTMENU => show_tray_menu(state),
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            handle_command(state, wparam.0 & 0xFFFF);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            let _ = SetTimer(Some(hwnd), TIMER_HOTPLUG, 400, None);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_HOTPLUG => {
            let _ = KillTimer(Some(hwnd), TIMER_HOTPLUG);
            rebuild_overlays(state);
            let ids = display_ids(state);
            let action = state.logic.restore_after_hotplug(&ids);
            apply(state, action);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_REASSERT => {
            reassert_topmost(state);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_ZORDER_KICK => {
            reassert_topmost(state);
            if state.zorder_kicks > 0 {
                state.zorder_kicks -= 1;
            }
            if state.zorder_kicks == 0 {
                let _ = KillTimer(Some(hwnd), TIMER_ZORDER_KICK);
            }
            LRESULT(0)
        }
        WM_DESTROY | WM_ENDSESSION => {
            teardown(state);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn overlay_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(1);
    }

    if msg == WM_MOUSEACTIVATE {
        return LRESULT(3); // MA_NOACTIVATE
    }

    let Some(state) = state_from(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };

    match msg {
        WM_MOUSEMOVE => {
            if pointer_moved_enough(state) {
                if let Some(id) = overlay_id(state, hwnd) {
                    let action = state.logic.on_pointer(&id);
                    apply(state, action);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(id) = overlay_id(state, hwnd) {
                let action = state.logic.on_click(&id);
                apply(state, action);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            paint_overlay(state, hwnd);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if let Some(overlay) = state.overlays.iter().find(|o| o.hwnd == hwnd) {
                let r = overlay.rect;
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOACTIVATE,
                );
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn overlay_id(state: &State, hwnd: HWND) -> Option<String> {
    state
        .overlays
        .iter()
        .find(|o| o.hwnd == hwnd)
        .map(|o| o.id.clone())
}

fn display_ids(state: &State) -> Vec<String> {
    state.overlays.iter().map(|o| o.id.clone()).collect()
}

unsafe fn handle_hotkey(state: &mut State, id: i32) {
    let action = match id {
        HOTKEY_TOGGLE => state.logic.on_toggle(cursor_display_id()),
        HOTKEY_ESC => state.logic.on_pick_key(PickKey::Escape, &[]),
        HOTKEY_ENTER => {
            let ids = display_ids(state);
            state.logic.on_pick_key(PickKey::Enter, &ids)
        }
        HOTKEY_LEFT => {
            let ids = display_ids(state);
            state.logic.on_pick_key(PickKey::Left, &ids)
        }
        HOTKEY_RIGHT => {
            let ids = display_ids(state);
            state.logic.on_pick_key(PickKey::Right, &ids)
        }
        d if (HOTKEY_DIGIT_BASE..HOTKEY_DIGIT_BASE + 9).contains(&d) => {
            let ids = display_ids(state);
            let digit = (d - HOTKEY_DIGIT_BASE + 1) as u8;
            state.logic.on_pick_key(PickKey::Digit(digit), &ids)
        }
        _ => Action::None,
    };
    if id == HOTKEY_LEFT
        || id == HOTKEY_RIGHT
        || (HOTKEY_DIGIT_BASE..HOTKEY_DIGIT_BASE + 9).contains(&id)
    {
        state.pointer_anchor = cursor_point();
    }
    apply(state, action);
}

fn tray_toggle(state: &mut State) {
    let now = std::time::Instant::now();
    if now.duration_since(state.tray_last_toggle) < std::time::Duration::from_millis(350) {
        return;
    }
    state.tray_last_toggle = now;
    // Clicking the tray makes Explorer/taskbar the foreground topmost window.
    // Overlays are WS_EX_NOACTIVATE, so they lose that z-band until we take
    // foreground back onto the host (same ritual as the tray menu). A 0×0
    // hidden window cannot become foreground; park a 1×1 off-screen.
    unsafe {
        let _ = SetWindowPos(
            state.host,
            None,
            -32000,
            -32000,
            1,
            1,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = ShowWindow(state.host, SW_SHOWNOACTIVATE);
        let _ = SetForegroundWindow(state.host);
        let action = state.logic.on_toggle(cursor_display_id());
        apply(state, action);
    }
}

unsafe fn handle_command(state: &mut State, cmd: usize) {
    match cmd {
        CMD_TOGGLE => {
            let action = state.logic.on_toggle(cursor_display_id());
            apply(state, action);
        }
        CMD_QUIT => {
            teardown(state);
            PostQuitMessage(0);
        }
        CMD_STYLE_SOLID => set_style(state, Style::Solid),
        CMD_STYLE_TINT => set_style(state, Style::Tint),
        CMD_STYLE_FROST => set_style(state, Style::Frost),
        CMD_HOTKEY => begin_bind(state),
        CMD_AUTOSTART => {
            if crate::autostart::set_enabled(!crate::autostart::is_enabled()).is_err() {
                let t = state.config.ui_lang().tr();
                tray_note(state, t.autostart, t.autostart_failed, true);
            }
        }
        CMD_WIPE => begin_wipe(state),
        CMD_ABOUT => begin_about(state),
        cmd => {
            if let Some(lang) = lang_from_cmd(cmd) {
                set_language(state, lang);
            }
        }
    }
}

fn lang_from_cmd(cmd: usize) -> Option<Language> {
    let i = cmd.checked_sub(CMD_LANG_BASE)?;
    Language::ALL.get(i).copied()
}

fn set_language(state: &mut State, lang: Language) {
    state.config.language = Some(lang);
    state.config.save();
    unsafe { invalidate_text_windows(state) };
}

unsafe fn invalidate_text_windows(state: &State) {
    for hwnd in [state.wipe_hwnd, state.about_hwnd, state.bind_hwnd] {
        if !hwnd.0.is_null() {
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, true);
        }
    }
}

fn set_style(state: &mut State, style: Style) {
    state.config.style = style;
    state.config.save();
    let action = state.logic.set_style(style);
    unsafe { apply(state, action) };
}

unsafe fn apply(state: &mut State, action: Action) {
    let style = state.logic.style;
    match action {
        Action::None => {}
        Action::HideAll => {
            set_pick_keys(state, false);
            state.zorder_kicks = 0;
            let _ = KillTimer(Some(state.host), TIMER_REASSERT);
            let _ = KillTimer(Some(state.host), TIMER_ZORDER_KICK);
            for overlay in &state.overlays {
                set_role(overlay, Role::Hidden, style);
            }
        }
        Action::ShowPick { candidate } => {
            show_pick_layout(state, &candidate, style);
            kick_zorder(state);
        }
        Action::RefreshPick { candidate } => {
            show_pick_layout(state, &candidate, style);
        }
        Action::ShowLocked { cinema } => {
            set_pick_keys(state, false);
            let _ = SetTimer(Some(state.host), TIMER_REASSERT, 500, None);
            for overlay in &state.overlays {
                let role = if overlay.id == cinema {
                    Role::Hidden
                } else {
                    Role::Veil
                };
                set_role(overlay, role, style);
            }
            kick_zorder(state);
        }
    }
}

unsafe fn show_pick_layout(state: &mut State, candidate: &str, style: Style) {
    set_pick_keys(state, true);
    let _ = SetTimer(Some(state.host), TIMER_REASSERT, 500, None);
    for overlay in &state.overlays {
        let role = if overlay.id == candidate {
            Role::Candidate
        } else {
            Role::Veil
        };
        set_role(overlay, role, style);
    }
}

unsafe fn set_role(overlay: &Overlay, role: Role, style: Style) {
    if overlay.applied.get() == Some((role, style)) {
        return;
    }
    overlay.applied.set(Some((role, style)));

    let hwnd = overlay.hwnd;
    let r = overlay.rect;
    reset_window_surface(hwnd);
    match role {
        Role::Hidden => {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        Role::Candidate => {
            ensure_layered(hwnd, true);
            paint_candidate(hwnd, &r);
            place_topmost(hwnd, &r);
        }
        Role::Veil => {
            if matches!(style, Style::Tint | Style::Frost) {
                ensure_layered(hwnd, true);
                set_window_alpha(hwnd, FROST_ALPHA);
                if style == Style::Frost {
                    set_frost_blur(hwnd, true);
                }
            }
            place_topmost(hwnd, &r);
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, true);
        }
    }
}

/// UpdateLayeredWindow and SetLayeredWindowAttributes cannot share a window.
/// After either API (or DWM acrylic), the layered bit must be stripped before
/// the next paint mode or the old surface sticks — frost stays frost, and
/// switching to Solid can leave a live blur instead of opaque black.
unsafe fn reset_window_surface(hwnd: HWND) {
    set_frost_blur(hwnd, false);
    ensure_layered(hwnd, false);
}

unsafe fn place_topmost(hwnd: HWND, r: &RECT) {
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        SWP_SHOWWINDOW | SWP_NOACTIVATE,
    );
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    restack_topmost(hwnd);
}

/// HWND_TOPMOST on a window that already has WS_EX_TOPMOST does not move it
/// above other topmost windows (the taskbar after a tray click). HWND_TOP
/// restacks within the topmost band without activating.
unsafe fn restack_topmost(hwnd: HWND) {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
    let _ = SetWindowPos(hwnd, Some(HWND_TOP), 0, 0, 0, 0, flags);
}

unsafe fn reassert_topmost(state: &State) {
    if state.logic.mode == Mode::Idle {
        return;
    }
    for overlay in &state.overlays {
        let hidden = state.logic.mode == Mode::Locked
            && state.logic.cinema.as_deref() == Some(overlay.id.as_str());
        if hidden {
            continue;
        }
        restack_topmost(overlay.hwnd);
    }
}

unsafe fn kick_zorder(state: &mut State) {
    reassert_topmost(state);
    // Explorer restacks the taskbar after our WM_TRAY handler returns.
    state.zorder_kicks = ZORDER_KICKS;
    let _ = SetTimer(Some(state.host), TIMER_ZORDER_KICK, ZORDER_KICK_MS, None);
}

fn cursor_point() -> Option<POINT> {
    let mut pt = POINT::default();
    unsafe {
        GetCursorPos(&mut pt).ok()?;
    }
    Some(pt)
}

fn pointer_moved_enough(state: &mut State) -> bool {
    if state.logic.follow_pointer {
        return true;
    }
    let Some(anchor) = state.pointer_anchor else {
        state.logic.allow_pointer();
        return true;
    };
    let Some(pt) = cursor_point() else {
        return false;
    };
    if (pt.x - anchor.x).abs() > 6 || (pt.y - anchor.y).abs() > 6 {
        state.pointer_anchor = None;
        state.logic.allow_pointer();
        true
    } else {
        false
    }
}

unsafe fn ensure_layered(hwnd: HWND, on: bool) {
    let mut ex = WINDOW_EX_STYLE(GetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
    ) as u32);
    if on {
        ex |= WS_EX_LAYERED;
    } else {
        ex &= WINDOW_EX_STYLE(!WS_EX_LAYERED.0);
    }
    SetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
        ex.0 as isize,
    );
    let _ = SetWindowPos(
        hwnd,
        None,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
    );
}

unsafe fn set_window_alpha(hwnd: HWND, alpha: u8) {
    let _ = windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
        hwnd,
        COLORREF(0),
        alpha,
        windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA,
    );
}

unsafe fn set_frost_blur(hwnd: HWND, on: bool) {
    let margins = MARGINS {
        cxLeftWidth: if on { -1 } else { 0 },
        cxRightWidth: if on { -1 } else { 0 },
        cyTopHeight: if on { -1 } else { 0 },
        cyBottomHeight: if on { -1 } else { 0 },
    };
    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    let kind: u32 = if on {
        DWMSBT_TRANSIENTWINDOW
    } else {
        DWMSBT_NONE
    };
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_SYSTEMBACKDROP_TYPE,
        &kind as *const u32 as *const _,
        size_of::<u32>() as u32,
    );
    set_win10_accent(hwnd, on);
}

unsafe fn set_win10_accent(hwnd: HWND, on: bool) {
    let module = windows::Win32::System::LibraryLoader::GetModuleHandleW(w!("user32.dll"));
    let Ok(module) = module else {
        return;
    };
    let Some(fn_ptr) = GetProcAddress(module, windows::core::s!("SetWindowCompositionAttribute"))
    else {
        return;
    };
    type SetWindowCompositionAttribute = unsafe extern "system" fn(HWND, *mut AccentArgs) -> i32;
    let func: SetWindowCompositionAttribute = std::mem::transmute(fn_ptr);

    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        flags: u32,
        color: u32,
        animation_id: u32,
    }
    #[repr(C)]
    struct AccentArgs {
        attribute: u32,
        data: *mut AccentPolicy,
        size: u32,
    }

    // 4 = ACCENT_ENABLE_ACRYLICBLURBEHIND, 0 = ACCENT_DISABLED
    let mut policy = AccentPolicy {
        accent_state: if on { 4 } else { 0 },
        flags: if on { 2 } else { 0 },
        color: if on {
            (u32::from(FROST_ALPHA) << 24) | 0x00_00_00_00
        } else {
            0
        },
        animation_id: 0,
    };
    let mut args = AccentArgs {
        attribute: 19,
        data: &mut policy,
        size: size_of::<AccentPolicy>() as u32,
    };
    let _ = func(hwnd, &mut args);
}

unsafe fn paint_overlay(state: &State, hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let brush = CreateSolidBrush(COLORREF(0));
    let mut rect = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    let _ = FillRect(hdc, &rect, brush);
    let _ = DeleteObject(brush.into());
    if state.logic.style == Style::Frost {
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, COLORREF(0x00202020));
    }
    let _ = EndPaint(hwnd, &ps);
}

unsafe fn paint_candidate(hwnd: HWND, monitor: &RECT) {
    let width = (monitor.right - monitor.left).max(1);
    let height = (monitor.bottom - monitor.top).max(1);
    let dpi = GetDpiForWindow(hwnd).max(96);
    let border = ((BORDER_DIP * dpi as i32) / 96).max(6);

    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut std::ffi::c_void = null_mut();
    let screen = windows::Win32::Graphics::Gdi::GetDC(None);
    let dib = CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, None, 0);
    let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen);
    let Ok(dib) = dib else {
        return;
    };
    if bits.is_null() {
        let _ = DeleteObject(dib.into());
        return;
    }

    let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
    let border_px = pixel(BORDER_R, BORDER_G, BORDER_B, 255);
    let fill_px = pixel(0, 0, 0, 1);
    for y in 0..height {
        for x in 0..width {
            let edge = x < border || y < border || x >= width - border || y >= height - border;
            pixels[(y * width + x) as usize] = if edge { border_px } else { fill_px };
        }
    }

    let screen_dc = windows::Win32::Graphics::Gdi::GetDC(None);
    let mem = CreateCompatibleDC(Some(screen_dc));
    let old = SelectObject(mem, dib.into());
    let mut src = windows::Win32::Foundation::POINT { x: 0, y: 0 };
    let mut dest = windows::Win32::Foundation::POINT {
        x: monitor.left,
        y: monitor.top,
    };
    let mut size = windows::Win32::Foundation::SIZE {
        cx: width,
        cy: height,
    };
    let mut blend = windows::Win32::Graphics::Gdi::BLENDFUNCTION {
        BlendOp: windows::Win32::Graphics::Gdi::AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: windows::Win32::Graphics::Gdi::AC_SRC_ALPHA as u8,
    };
    let _ = windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow(
        hwnd,
        Some(screen_dc),
        Some(&mut dest),
        Some(&mut size),
        Some(mem),
        Some(&mut src),
        COLORREF(0),
        Some(&mut blend),
        windows::Win32::UI::WindowsAndMessaging::ULW_ALPHA,
    );
    SelectObject(mem, HGDIOBJ(old.0));
    let _ = DeleteDC(mem);
    let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
    let _ = DeleteObject(dib.into());
}

fn pixel(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let r = (u16::from(r) * u16::from(a) / 255) as u8;
    let g = (u16::from(g) * u16::from(a) / 255) as u8;
    let b = (u16::from(b) * u16::from(a) / 255) as u8;
    u32::from(b) | (u32::from(g) << 8) | (u32::from(r) << 16) | (u32::from(a) << 24)
}

struct MonitorCollect(Vec<(String, RECT)>);

unsafe extern "system" fn monitor_enum(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> windows::core::BOOL {
    let out = &mut *(data.0 as *mut MonitorCollect);
    let mut info = MONITORINFOEXW {
        monitorInfo: windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut _).as_bool() {
        let name = widestr_to_string(&info.szDevice);
        out.0.push((name, info.monitorInfo.rcMonitor));
    }
    windows::core::BOOL(1)
}

fn widestr_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

unsafe fn rebuild_overlays(state: &mut State) {
    for overlay in state.overlays.drain(..) {
        let _ = DestroyWindow(overlay.hwnd);
    }

    let mut found = MonitorCollect(Vec::new());
    let _ = EnumDisplayMonitors(
        None,
        None,
        Some(monitor_enum),
        LPARAM(&mut found as *mut _ as isize),
    );
    found
        .0
        .sort_by(|a, b| a.1.left.cmp(&b.1.left).then(a.1.top.cmp(&b.1.top)));

    let instance = GetModuleHandleW(None).ok();
    for (id, rect) in found.0 {
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            OVERLAY_CLASS,
            w!(""),
            WS_POPUP,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            None,
            None,
            instance.map(Into::into),
            Some(state as *mut State as *const _),
        );
        if let Ok(hwnd) = hwnd {
            let _ = ShowWindow(hwnd, SW_HIDE);
            state.overlays.push(Overlay {
                hwnd,
                id,
                rect,
                applied: Cell::new(None),
            });
        }
    }
}

unsafe fn cursor_display_id() -> Option<String> {
    let mut pt = POINT::default();
    GetCursorPos(&mut pt).ok()?;
    let monitor = windows::Win32::Graphics::Gdi::MonitorFromPoint(
        pt,
        windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
    );
    let mut info = MONITORINFOEXW {
        monitorInfo: windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut _).as_bool() {
        Some(widestr_to_string(&info.szDevice))
    } else {
        None
    }
}

fn hotkey_mods(hk: Hotkey) -> HOT_KEY_MODIFIERS {
    let mut bits = MOD_NOREPEAT.0;
    if hk.ctrl {
        bits |= MOD_CONTROL.0;
    }
    if hk.alt {
        bits |= MOD_ALT.0;
    }
    if hk.shift {
        bits |= MOD_SHIFT.0;
    }
    if hk.meta {
        bits |= MOD_WIN.0;
    }
    HOT_KEY_MODIFIERS(bits)
}

unsafe fn register_toggle_hotkey(state: &mut State) {
    let _ = UnregisterHotKey(Some(state.host), HOTKEY_TOGGLE);
    let hk = state.config.hotkey;
    let vk = hk.win_vk();
    if vk == 0 {
        state.hotkey_ok = false;
        return;
    }
    let mods = hotkey_mods(hk);
    if RegisterHotKey(Some(state.host), HOTKEY_TOGGLE, mods, vk).is_ok() {
        state.hotkey_ok = true;
        return;
    }
    // MOD_NOREPEAT is optional; some environments reject it.
    let bits = mods.0 & !MOD_NOREPEAT.0;
    if bits == 0 {
        state.hotkey_ok = false;
        return;
    }
    state.hotkey_ok =
        RegisterHotKey(Some(state.host), HOTKEY_TOGGLE, HOT_KEY_MODIFIERS(bits), vk).is_ok();
}

unsafe fn begin_bind(state: &mut State) {
    if !state.wipe_hwnd.0.is_null() {
        let _ = SetForegroundWindow(state.wipe_hwnd);
        return;
    }
    if !state.about_hwnd.0.is_null() {
        let _ = DestroyWindow(state.about_hwnd);
    }
    if !state.bind_hwnd.0.is_null() {
        let _ = SetForegroundWindow(state.bind_hwnd);
        return;
    }
    let _ = UnregisterHotKey(Some(state.host), HOTKEY_TOGGLE);
    state.hotkey_ok = false;
    state.bind_draft = None;
    state.bind_preview.clear();
    state.bind_locked = false;
    state.bind_win = false;

    const W: i32 = 560;
    const H: i32 = 280;
    let (x, y) = bind_origin(W, H);
    let instance = GetModuleHandleW(None).ok();
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        BIND_CLASS,
        w!("Blackout"),
        WS_POPUP,
        x,
        y,
        W,
        H,
        None,
        None,
        instance.map(Into::into),
        Some(state as *mut State as *const _),
    );
    let Ok(hwnd) = hwnd else {
        register_toggle_hotkey(state);
        return;
    };
    state.bind_hwnd = hwnd;
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
}

fn bind_origin(width: i32, height: i32) -> (i32, i32) {
    let mut pt = POINT::default();
    unsafe {
        if GetCursorPos(&mut pt).is_ok() {
            let monitor = windows::Win32::Graphics::Gdi::MonitorFromPoint(
                pt,
                windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
            );
            let mut info = MONITORINFOEXW {
                monitorInfo: windows::Win32::Graphics::Gdi::MONITORINFO {
                    cbSize: size_of::<MONITORINFOEXW>() as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut _).as_bool() {
                let r = info.monitorInfo.rcMonitor;
                return (
                    r.left + (r.right - r.left - width) / 2,
                    r.top + (r.bottom - r.top - height) / 2,
                );
            }
        }
    }
    (200, 200)
}

unsafe fn begin_wipe(state: &mut State) {
    if !state.wipe_hwnd.0.is_null() {
        let _ = SetForegroundWindow(state.wipe_hwnd);
        return;
    }
    if !state.about_hwnd.0.is_null() {
        let _ = DestroyWindow(state.about_hwnd);
    }
    if !state.bind_hwnd.0.is_null() {
        cancel_bind(state, state.bind_hwnd);
    }
    let action = state.logic.dismiss();
    apply(state, action);
    let _ = UnregisterHotKey(Some(state.host), HOTKEY_TOGGLE);
    state.hotkey_ok = false;
    state.wipe_error.clear();

    const W: i32 = 560;
    const H: i32 = 380;
    let (x, y) = bind_origin(W, H);
    let instance = GetModuleHandleW(None).ok();
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        WIPE_CLASS,
        w!("Blackout"),
        WS_POPUP,
        x,
        y,
        W,
        H,
        None,
        None,
        instance.map(Into::into),
        Some(state as *mut State as *const _),
    );
    let Ok(hwnd) = hwnd else {
        register_toggle_hotkey(state);
        return;
    };
    state.wipe_hwnd = hwnd;
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
}

unsafe extern "system" fn wipe_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(1);
    }
    let Some(state) = state_from(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };
    match msg {
        WM_PAINT => {
            paint_wipe(state, hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = wparam.0 as u32;
            if vk == VK_ESCAPE.0 as u32 {
                cancel_wipe(state, hwnd);
            } else if vk == VK_RETURN.0 as u32 {
                commit_wipe(state, hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let x = (lparam.0 as u32 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 as u32 >> 16) & 0xFFFF) as i16 as i32;
            let mut client = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client);
            let inset = RECT {
                left: client.left + 2,
                top: client.top + 2,
                right: client.right - 2,
                bottom: client.bottom - 2,
            };
            let (ok, cancel) = bind_button_rects(inset);
            if rect_contains(ok, x, y) {
                commit_wipe(state, hwnd);
            } else if rect_contains(cancel, x, y) {
                cancel_wipe(state, hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            state.wipe_hwnd = HWND(null_mut());
            state.wipe_error.clear();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_wipe(state: &State, hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rect = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    let frame = CreateSolidBrush(COLORREF(0x0097DC3D));
    let _ = FillRect(hdc, &rect, frame);
    let _ = DeleteObject(frame.into());
    let inset = RECT {
        left: rect.left + 2,
        top: rect.top + 2,
        right: rect.right - 2,
        bottom: rect.bottom - 2,
    };
    let fill = CreateSolidBrush(COLORREF(0x00101010));
    let _ = FillRect(hdc, &inset, fill);
    let _ = DeleteObject(fill.into());
    let _ = SetBkMode(hdc, TRANSPARENT);

    let title = CreateFontW(
        -22,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let body = CreateFontW(
        -16,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let old = SelectObject(hdc, title.into());
    let _ = SetTextColor(hdc, COLORREF(0x00F0F0F0));
    let mut head = inset;
    head.left += 28;
    head.right -= 28;
    head.top += 18;
    head.bottom = head.top + 36;
    let lang = state.config.ui_lang();
    let mut title_text = wide(lang.tr().wipe_title);
    let _ = DrawTextW(hdc, &mut title_text, &mut head, DT_CENTER | DT_SINGLELINE);

    let _ = SelectObject(hdc, body.into());
    let mut text = inset;
    text.left += 28;
    text.right -= 28;
    text.top = head.bottom + 8;
    text.bottom = inset.bottom - 90;
    let mut body_text = wide(&crate::purge::dialog_body(lang));
    let _ = DrawTextW(hdc, &mut body_text, &mut text, DT_WORDBREAK);

    if !state.wipe_error.is_empty() {
        let mut err = inset;
        err.left += 28;
        err.right -= 28;
        err.top = inset.bottom - 88;
        err.bottom = inset.bottom - 68;
        let _ = SetTextColor(hdc, COLORREF(0x005050F0));
        let mut err_text = wide(&state.wipe_error);
        let _ = DrawTextW(hdc, &mut err_text, &mut err, DT_CENTER | DT_SINGLELINE);
        let _ = SetTextColor(hdc, COLORREF(0x00F0F0F0));
    }

    let button = CreateFontW(
        -16,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let _ = SelectObject(hdc, button.into());
    let (ok, cancel) = bind_button_rects(inset);
    paint_bind_button(hdc, ok, lang.tr().wipe_confirm, true);
    paint_bind_button(hdc, cancel, lang.tr().cancel_esc, true);

    let _ = SelectObject(hdc, old);
    let _ = DeleteObject(title.into());
    let _ = DeleteObject(body.into());
    let _ = DeleteObject(button.into());
    let _ = EndPaint(hwnd, &ps);
}

unsafe fn cancel_wipe(state: &mut State, hwnd: HWND) {
    let _ = DestroyWindow(hwnd);
    register_toggle_hotkey(state);
    refresh_tray_tip(state);
}

unsafe fn commit_wipe(state: &mut State, hwnd: HWND) {
    match crate::purge::wipe() {
        Ok(()) => {
            teardown(state);
            PostQuitMessage(0);
        }
        Err(_) => {
            let lang = state.config.ui_lang();
            state.wipe_error = lang.tr().wipe_failed.to_string();
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, true);
            tray_note(state, lang.tr().wipe_note_title, lang.tr().wipe_failed, true);
        }
    }
}

unsafe fn begin_about(state: &mut State) {
    if !state.wipe_hwnd.0.is_null() {
        let _ = SetForegroundWindow(state.wipe_hwnd);
        return;
    }
    if !state.bind_hwnd.0.is_null() {
        let _ = SetForegroundWindow(state.bind_hwnd);
        return;
    }
    if !state.about_hwnd.0.is_null() {
        let _ = SetForegroundWindow(state.about_hwnd);
        return;
    }

    const W: i32 = 560;
    const H: i32 = 300;
    let (x, y) = bind_origin(W, H);
    let instance = GetModuleHandleW(None).ok();
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        ABOUT_CLASS,
        w!("Blackout"),
        WS_POPUP,
        x,
        y,
        W,
        H,
        None,
        None,
        instance.map(Into::into),
        Some(state as *mut State as *const _),
    );
    let Ok(hwnd) = hwnd else {
        return;
    };
    state.about_hwnd = hwnd;
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
}

unsafe extern "system" fn about_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(1);
    }
    let Some(state) = state_from(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };
    match msg {
        WM_PAINT => {
            paint_about(state, hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = wparam.0 as u32;
            if vk == VK_ESCAPE.0 as u32 || vk == VK_RETURN.0 as u32 {
                close_about(state, hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let x = (lparam.0 as u32 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 as u32 >> 16) & 0xFFFF) as i16 as i32;
            let mut client = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client);
            let inset = RECT {
                left: client.left + 2,
                top: client.top + 2,
                right: client.right - 2,
                bottom: client.bottom - 2,
            };
            if rect_contains(about_link_rect(inset), x, y)
                || rect_contains(about_github_button(inset), x, y)
            {
                crate::about::open_github();
            } else if rect_contains(about_close_button(inset), x, y) {
                close_about(state, hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            state.about_hwnd = HWND(null_mut());
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_about(state: &State, hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rect = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    let frame = CreateSolidBrush(COLORREF(0x0097DC3D));
    let _ = FillRect(hdc, &rect, frame);
    let _ = DeleteObject(frame.into());
    let inset = RECT {
        left: rect.left + 2,
        top: rect.top + 2,
        right: rect.right - 2,
        bottom: rect.bottom - 2,
    };
    let fill = CreateSolidBrush(COLORREF(0x00101010));
    let _ = FillRect(hdc, &inset, fill);
    let _ = DeleteObject(fill.into());
    let _ = SetBkMode(hdc, TRANSPARENT);

    let title = CreateFontW(
        -28,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let body = CreateFontW(
        -16,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let credit = CreateFontW(
        -12,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let old = SelectObject(hdc, title.into());
    let _ = SetTextColor(hdc, COLORREF(0x0097DC3D));
    let mut head = inset;
    head.left += 28;
    head.right -= 28;
    head.top += 22;
    head.bottom = head.top + 40;
    let mut title_text = wide(crate::about::NAME);
    let _ = DrawTextW(hdc, &mut title_text, &mut head, DT_CENTER | DT_SINGLELINE);

    let _ = SelectObject(hdc, body.into());
    let _ = SetTextColor(hdc, COLORREF(0x00F0F0F0));
    let mut line = inset;
    line.left += 28;
    line.right -= 28;
    line.top = head.bottom + 8;
    line.bottom = line.top + 26;
    let lang = state.config.ui_lang();
    let mut version = wide(&lang.version_line(crate::about::VERSION));
    let _ = DrawTextW(hdc, &mut version, &mut line, DT_CENTER | DT_SINGLELINE);

    line.top = line.bottom;
    line.bottom = line.top + 26;
    let mut author = wide(&lang.author_line(crate::about::AUTHOR));
    let _ = DrawTextW(hdc, &mut author, &mut line, DT_CENTER | DT_SINGLELINE);

    let _ = SelectObject(hdc, credit.into());
    let _ = SetTextColor(hdc, COLORREF(0x00787878));
    line.top = line.bottom - 4;
    for part in lang.credit_lines() {
        line.bottom = line.top + 18;
        let mut credit_text = wide(part);
        let _ = DrawTextW(hdc, &mut credit_text, &mut line, DT_CENTER | DT_SINGLELINE);
        line.top = line.bottom;
    }
    let _ = SelectObject(hdc, body.into());
    let _ = SetTextColor(hdc, COLORREF(0x00F0F0F0));

    let mut license = about_license_rect(inset);
    let mut license_text = wide(&lang.license_line(crate::about::LICENSE));
    let _ = DrawTextW(
        hdc,
        &mut license_text,
        &mut license,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );

    let mut link = about_link_rect(inset);
    let _ = SetTextColor(hdc, COLORREF(0x0097DC3D));
    let mut url = wide(crate::about::GITHUB);
    let _ = DrawTextW(hdc, &mut url, &mut link, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    let _ = SetTextColor(hdc, COLORREF(0x00F0F0F0));

    let button = CreateFontW(
        -16,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let _ = SelectObject(hdc, button.into());
    paint_bind_button(hdc, about_github_button(inset), "GitHub", true);
    paint_bind_button(hdc, about_close_button(inset), lang.tr().close_esc, true);

    let _ = SelectObject(hdc, old);
    let _ = DeleteObject(title.into());
    let _ = DeleteObject(body.into());
    let _ = DeleteObject(credit.into());
    let _ = DeleteObject(button.into());
    let _ = EndPaint(hwnd, &ps);
}

fn about_link_rect(inset: RECT) -> RECT {
    let buttons_top = bind_button_rects(inset).0.top;
    RECT {
        left: inset.left + 28,
        top: buttons_top - 40,
        right: inset.right - 28,
        bottom: buttons_top - 18,
    }
}

fn about_license_rect(inset: RECT) -> RECT {
    let link = about_link_rect(inset);
    RECT {
        left: link.left,
        top: link.top - 22,
        right: link.right,
        bottom: link.top - 2,
    }
}

fn about_github_button(inset: RECT) -> RECT {
    bind_button_rects(inset).0
}

fn about_close_button(inset: RECT) -> RECT {
    bind_button_rects(inset).1
}

unsafe fn close_about(state: &mut State, hwnd: HWND) {
    let _ = DestroyWindow(hwnd);
    state.about_hwnd = HWND(null_mut());
}

unsafe extern "system" fn bind_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(1);
    }
    let Some(state) = state_from(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };
    match msg {
        WM_PAINT => {
            paint_bind(state, hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            handle_bind_key(state, hwnd, wparam.0 as u32);
            LRESULT(0)
        }
        WM_KEYUP | WM_SYSKEYUP => {
            handle_bind_keyup(state, hwnd, wparam.0 as u32);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let x = (lparam.0 as u32 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 as u32 >> 16) & 0xFFFF) as i16 as i32;
            handle_bind_click(state, hwnd, x, y);
            LRESULT(0)
        }
        WM_DESTROY => {
            state.bind_hwnd = HWND(null_mut());
            state.bind_draft = None;
            state.bind_preview.clear();
            state.bind_locked = false;
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_bind(state: &State, hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rect = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    let frame = CreateSolidBrush(COLORREF(0x0097DC3D));
    let _ = FillRect(hdc, &rect, frame);
    let _ = DeleteObject(frame.into());
    let inset = RECT {
        left: rect.left + 2,
        top: rect.top + 2,
        right: rect.right - 2,
        bottom: rect.bottom - 2,
    };
    let fill = CreateSolidBrush(COLORREF(0x00101010));
    let _ = FillRect(hdc, &inset, fill);
    let _ = DeleteObject(fill.into());
    let _ = SetBkMode(hdc, TRANSPARENT);

    let title = CreateFontW(
        -22,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let preview = CreateFontW(
        -32,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );

    let mut hint = inset;
    hint.left += 28;
    hint.right -= 28;
    hint.top += 20;
    hint.bottom = hint.top + 64;
    let old = SelectObject(hdc, title.into());
    let _ = SetTextColor(hdc, COLORREF(0x00F0F0F0));
    let lang = state.config.ui_lang();
    let mut hint_text = wide(&lang.bind_hint());
    let _ = DrawTextW(hdc, &mut hint_text, &mut hint, DT_CENTER | DT_WORDBREAK);

    let mut combo = inset;
    combo.left += 28;
    combo.right -= 28;
    combo.top = hint.bottom + 12;
    combo.bottom = inset.bottom - 76;
    let _ = SelectObject(hdc, preview.into());
    let preview_s = if state.bind_preview.is_empty() {
        "..."
    } else {
        state.bind_preview.as_str()
    };
    let mut preview_text = wide(preview_s);
    let _ = SetTextColor(
        hdc,
        if state.bind_draft.is_some() {
            COLORREF(0x0097DC3D)
        } else {
            COLORREF(0x00A0A0A0)
        },
    );
    let _ = DrawTextW(
        hdc,
        &mut preview_text,
        &mut combo,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    let button = CreateFontW(
        -16,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        0u32,
        w!("Segoe UI"),
    );
    let _ = SelectObject(hdc, button.into());
    let (save, cancel) = bind_button_rects(inset);
    paint_bind_button(hdc, save, lang.tr().save_enter, state.bind_draft.is_some());
    paint_bind_button(hdc, cancel, lang.tr().cancel_esc, true);

    let _ = SelectObject(hdc, old);
    let _ = DeleteObject(title.into());
    let _ = DeleteObject(preview.into());
    let _ = DeleteObject(button.into());
    let _ = EndPaint(hwnd, &ps);
}

fn bind_button_rects(client: RECT) -> (RECT, RECT) {
    const BTN_W: i32 = 176;
    const BTN_H: i32 = 40;
    const GAP: i32 = 16;
    let mid = (client.left + client.right) / 2;
    let top = client.bottom - 62;
    let save = RECT {
        left: mid - BTN_W - GAP / 2,
        top,
        right: mid - GAP / 2,
        bottom: top + BTN_H,
    };
    let cancel = RECT {
        left: mid + GAP / 2,
        top,
        right: mid + GAP / 2 + BTN_W,
        bottom: top + BTN_H,
    };
    (save, cancel)
}

fn rect_contains(rect: RECT, x: i32, y: i32) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

unsafe fn paint_bind_button(hdc: HDC, rect: RECT, label: &str, enabled: bool) {
    let border = CreateSolidBrush(COLORREF(0x0097DC3D));
    let _ = FrameRect(hdc, &rect, border);
    let _ = DeleteObject(border.into());
    let _ = SetTextColor(
        hdc,
        if enabled {
            COLORREF(0x00F0F0F0)
        } else {
            COLORREF(0x00A0A0A0)
        },
    );
    let mut text = wide(label);
    let mut r = rect;
    let _ = DrawTextW(
        hdc,
        &mut text,
        &mut r,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
}

unsafe fn handle_bind_click(state: &mut State, hwnd: HWND, x: i32, y: i32) {
    let mut client = RECT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client);
    let inset = RECT {
        left: client.left + 2,
        top: client.top + 2,
        right: client.right - 2,
        bottom: client.bottom - 2,
    };
    let (save, cancel) = bind_button_rects(inset);
    if rect_contains(save, x, y) {
        commit_bind(state, hwnd);
    } else if rect_contains(cancel, x, y) {
        cancel_bind(state, hwnd);
    }
}

fn current_bind_mods(state: &State) -> (bool, bool, bool, bool) {
    unsafe {
        let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
        let alt = GetKeyState(VK_MENU.0 as i32) < 0;
        let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
        (ctrl, alt, shift, state.bind_win)
    }
}

fn bind_keys_held(state: &State) -> bool {
    let (ctrl, alt, shift, meta) = current_bind_mods(state);
    if ctrl || alt || shift || meta {
        return true;
    }
    if let Some(hotkey) = state.bind_draft {
        let vk = hotkey.win_vk();
        if vk != 0 && unsafe { GetKeyState(vk as i32) < 0 } {
            return true;
        }
    }
    false
}

fn begin_new_bind_stroke(state: &mut State) {
    if state.bind_draft.is_some() {
        state.bind_draft = None;
        state.bind_preview.clear();
    }
}

unsafe fn refresh_bind_preview(state: &mut State, hwnd: HWND, vk: Option<u32>) {
    let (ctrl, alt, shift, meta) = current_bind_mods(state);
    let key = vk.and_then(key_from_win_vk);
    if let Some(key) = key {
        if let Some(hotkey) = Hotkey::new(ctrl, alt, shift, meta, key) {
            state.bind_draft = Some(hotkey);
            state.bind_preview = hotkey.to_string();
            state.bind_locked = true;
        } else {
            state.bind_preview = format_combo(ctrl, alt, shift, meta, Some(key));
        }
    } else {
        let live = format_combo(ctrl, alt, shift, meta, None);
        if !live.is_empty() {
            state.bind_preview = live;
        } else if let Some(draft) = state.bind_draft {
            state.bind_preview = draft.to_string();
        } else {
            state.bind_preview.clear();
        }
    }
    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, true);
}

unsafe fn handle_bind_key(state: &mut State, hwnd: HWND, vk: u32) {
    if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 {
        state.bind_win = true;
        if state.bind_locked {
            return;
        }
        begin_new_bind_stroke(state);
        refresh_bind_preview(state, hwnd, None);
        return;
    }
    if vk == VK_ESCAPE.0 as u32 {
        cancel_bind(state, hwnd);
        return;
    }
    let (ctrl, alt, shift, meta) = current_bind_mods(state);
    if vk == VK_RETURN.0 as u32 && !ctrl && !alt && !shift && !meta {
        commit_bind(state, hwnd);
        return;
    }
    if state.bind_locked {
        return;
    }
    begin_new_bind_stroke(state);
    refresh_bind_preview(state, hwnd, Some(vk));
}

unsafe fn handle_bind_keyup(state: &mut State, hwnd: HWND, vk: u32) {
    if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 {
        state.bind_win = false;
    }
    if state.bind_locked {
        if !bind_keys_held(state) {
            state.bind_locked = false;
        }
        return;
    }
    refresh_bind_preview(state, hwnd, None);
}

unsafe fn cancel_bind(state: &mut State, hwnd: HWND) {
    state.pending_hotkey = None;
    let _ = DestroyWindow(hwnd);
    register_toggle_hotkey(state);
    refresh_tray_tip(state);
}

unsafe fn commit_bind(state: &mut State, hwnd: HWND) {
    let Some(hotkey) = state.bind_draft else {
        return;
    };
    state.pending_hotkey = Some(hotkey);
    let host = state.host;
    let _ = DestroyWindow(hwnd);
    let _ = PostMessageW(Some(host), WM_APPLY_HOTKEY, WPARAM(0), LPARAM(0));
}

unsafe fn apply_pending_hotkey(state: &mut State) {
    let Some(hotkey) = state.pending_hotkey.take() else {
        return;
    };
    let previous = state.config.hotkey;
    let already_ours = hotkey == previous;
    state.config.hotkey = hotkey;
    register_toggle_hotkey(state);
    if state.hotkey_ok || already_ours {
        state.hotkey_ok = true;
        state.config.save();
        refresh_tray_tip(state);
        let lang = state.config.ui_lang();
        tray_note(
            state,
            lang.tr().hotkey_title,
            &lang.hotkey_saved(&hotkey.to_string()),
            false,
        );
    } else {
        state.config.hotkey = previous;
        register_toggle_hotkey(state);
        let lang = state.config.ui_lang();
        tray_note(
            state,
            lang.tr().hotkey_busy_title,
            &lang.hotkey_busy_retry(&hotkey.to_string()),
            true,
        );
    }
}

unsafe fn set_pick_keys(state: &mut State, on: bool) {
    if state.pick_keys == on {
        return;
    }
    state.pick_keys = on;
    let host = Some(state.host);
    if on {
        let _ = RegisterHotKey(host, HOTKEY_ESC, MOD_NOREPEAT, VK_ESCAPE.0 as u32);
        let _ = RegisterHotKey(host, HOTKEY_ENTER, MOD_NOREPEAT, VK_RETURN.0 as u32);
        let _ = RegisterHotKey(host, HOTKEY_LEFT, MOD_NOREPEAT, VK_LEFT.0 as u32);
        let _ = RegisterHotKey(host, HOTKEY_RIGHT, MOD_NOREPEAT, VK_RIGHT.0 as u32);
        let digits: [VIRTUAL_KEY; 9] = [VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9];
        for (i, vk) in digits.iter().enumerate() {
            let _ = RegisterHotKey(
                host,
                HOTKEY_DIGIT_BASE + i as i32,
                MOD_NOREPEAT,
                vk.0 as u32,
            );
        }
    } else {
        for id in HOTKEY_ESC..=HOTKEY_RIGHT {
            let _ = UnregisterHotKey(host, id);
        }
        for i in 0..9 {
            let _ = UnregisterHotKey(host, HOTKEY_DIGIT_BASE + i);
        }
    }
}

unsafe fn add_tray(state: &mut State) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: state.host,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: WM_TRAY,
        hIcon: state.icon,
        ..Default::default()
    };
    copy_tip(&mut nid.szTip, "Blackout");
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
    state.tray = nid;
}

fn copy_tip(buf: &mut [u16], text: &str) {
    buf.fill(0);
    let encoded: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let n = encoded.len().min(buf.len());
    buf[..n].copy_from_slice(&encoded[..n]);
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn refresh_tray_tip(state: &mut State) {
    copy_tip(&mut state.tray.szTip, "Blackout");
    state.tray.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    let _ = Shell_NotifyIconW(NIM_MODIFY, &state.tray);
}

unsafe fn tray_note(state: &mut State, title: &str, text: &str, warn: bool) {
    copy_tip(&mut state.tray.szTip, "Blackout");
    copy_tip(&mut state.tray.szInfoTitle, title);
    copy_tip(&mut state.tray.szInfo, text);
    state.tray.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP | NIF_INFO;
    state.tray.dwInfoFlags = if warn { NIIF_WARNING } else { NIIF_INFO };
    let _ = Shell_NotifyIconW(NIM_MODIFY, &state.tray);
}

unsafe fn show_tray_menu(state: &State) {
    let menu = CreatePopupMenu().unwrap_or_default();
    if menu.is_invalid() {
        return;
    }
    let lang = state.config.ui_lang();
    let t = lang.tr();
    let hk = state.config.hotkey.to_string();
    let mut labels = Vec::new();
    let toggle_s = lang.toggle_tab(state.logic.mode == Mode::Idle, &hk);
    append_menu(&mut labels, menu, MF_STRING, CMD_TOGGLE, &toggle_s);

    let styles = CreatePopupMenu().unwrap_or_default();
    if !styles.is_invalid() {
        let solid_flags = if state.logic.style == Style::Solid {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        let tint_flags = if state.logic.style == Style::Tint {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        let frost_flags = if state.logic.style == Style::Frost {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        append_menu(&mut labels, styles, solid_flags, CMD_STYLE_SOLID, t.solid);
        append_menu(&mut labels, styles, tint_flags, CMD_STYLE_TINT, t.tint);
        append_menu(&mut labels, styles, frost_flags, CMD_STYLE_FROST, t.frost);
        append_popup(&mut labels, menu, styles, t.style);
    }

    let settings = CreatePopupMenu().unwrap_or_default();
    if !settings.is_invalid() {
        let bind_s = lang.hotkey_item(&hk, state.hotkey_ok);
        append_menu(&mut labels, settings, MF_STRING, CMD_HOTKEY, &bind_s);
        let auto_flags = if crate::autostart::is_enabled() {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        append_menu(&mut labels, settings, auto_flags, CMD_AUTOSTART, t.autostart);
        let langs = CreatePopupMenu().unwrap_or_default();
        if !langs.is_invalid() {
            for (i, item) in Language::ALL.iter().enumerate() {
                let flags = if *item == lang {
                    MF_STRING | MF_CHECKED
                } else {
                    MF_STRING | MF_UNCHECKED
                };
                append_menu(&mut labels, langs, flags, CMD_LANG_BASE + i, item.endonym());
            }
            append_popup(&mut labels, settings, langs, t.language);
        }
        let _ = AppendMenuW(settings, MF_SEPARATOR, 0, PCWSTR(null()));
        append_menu(&mut labels, settings, MF_STRING, CMD_WIPE, t.wipe);
        append_popup(&mut labels, menu, settings, t.settings);
    }

    append_menu(&mut labels, menu, MF_STRING, CMD_ABOUT, t.about);
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR(null()));
    append_menu(&mut labels, menu, MF_STRING, CMD_QUIT, t.quit);

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(state.host);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, None, state.host, None);
    let _ = PostMessageW(Some(state.host), 0, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(menu);
    drop(labels);
}

fn append_menu(labels: &mut Vec<Vec<u16>>, menu: windows::Win32::UI::WindowsAndMessaging::HMENU, flags: windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS, id: usize, text: &str) {
    labels.push(wide(text));
    let label = labels.last().unwrap();
    let _ = unsafe { AppendMenuW(menu, flags, id, PCWSTR(label.as_ptr())) };
}

fn append_popup(labels: &mut Vec<Vec<u16>>, menu: windows::Win32::UI::WindowsAndMessaging::HMENU, popup: windows::Win32::UI::WindowsAndMessaging::HMENU, text: &str) {
    labels.push(wide(text));
    let label = labels.last().unwrap();
    let _ = unsafe { AppendMenuW(menu, MF_POPUP, popup.0 as usize, PCWSTR(label.as_ptr())) };
}

unsafe fn create_tray_icon() -> HICON {
    let size = GetSystemMetrics(SM_CXSMICON).max(16);
    let _ = GetSystemMetrics(SM_CYSMICON);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut std::ffi::c_void = null_mut();
    let screen = windows::Win32::Graphics::Gdi::GetDC(None);
    let dib = CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, None, 0);
    let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen);
    if let Ok(dib) = dib {
        if !bits.is_null() {
            let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize);
            crate::icon::draw_monitor_icon(pixels, size);
        }
        let mut icon_info = windows::Win32::UI::WindowsAndMessaging::ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: windows::Win32::Graphics::Gdi::HBITMAP(dib.0),
            hbmColor: windows::Win32::Graphics::Gdi::HBITMAP(dib.0),
        };
        if let Ok(icon) =
            windows::Win32::UI::WindowsAndMessaging::CreateIconIndirect(&mut icon_info)
        {
            return icon;
        }
        let _ = DeleteObject(dib.into());
    }
    windows::Win32::UI::WindowsAndMessaging::LoadIconW(
        None,
        windows::Win32::UI::WindowsAndMessaging::IDI_APPLICATION,
    )
    .unwrap_or_default()
}

unsafe fn teardown(state: &mut State) {
    set_pick_keys(state, false);
    let _ = UnregisterHotKey(Some(state.host), HOTKEY_TOGGLE);
    if !state.wipe_hwnd.0.is_null() {
        let _ = DestroyWindow(state.wipe_hwnd);
    }
    if !state.about_hwnd.0.is_null() {
        let _ = DestroyWindow(state.about_hwnd);
    }
    if !state.bind_hwnd.0.is_null() {
        let _ = DestroyWindow(state.bind_hwnd);
    }
    for overlay in state.overlays.drain(..) {
        let _ = DestroyWindow(overlay.hwnd);
    }
    let _ = Shell_NotifyIconW(NIM_DELETE, &state.tray);
}
