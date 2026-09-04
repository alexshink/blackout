use std::env;

use embedded_graphics::mono_font::iso_8859_5::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Drawable, OriginDimensions, Point, Size};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Pixel;
use x11rb::connection::Connection;
use x11rb::protocol::render::{self, Color, CreatePictureAux, PictOp, PictType, Picture};
use x11rb::protocol::xproto::{
    AtomEnum, ButtonPressEvent, ChangeGCAux, Colormap, ColormapAlloc, ConnectionExt, CreateGCAux,
    CreateWindowAux, EnterNotifyEvent, EventMask, Gcontext, GrabMode, KeyPressEvent, PropMode,
    Rectangle, Screen, Visualid, Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::COPY_DEPTH_FROM_PARENT;

use crate::app::{Action, Logic, PickKey};
use crate::config::{Config, Style, FROST_ALPHA};
use crate::hotkey::{Hotkey, Key};

const BORDER: i16 = 8;
const BIND_W: u16 = 560;
const BIND_H: u16 = 280;
const BIND_BTN_W: i16 = 176;
const BIND_BTN_H: i16 = 40;
const BIND_BTN_GAP: i16 = 16;
/// Esc, Enter, Left, Right, `1`…`9` — US keycodes, as in x11-session.md.
const PICK_CODES: &[u8] = &[9, 36, 113, 114, 10, 11, 12, 13, 14, 15, 16, 17, 18];
const PICK_MODS: [u16; 4] = [0, 2, 16, 18];

#[derive(Clone, Copy)]
struct Argb {
    visual: Visualid,
    format: render::Pictformat,
    depth: u8,
    colormap: Colormap,
}

struct Overlay {
    window: Window,
    gc: Gcontext,
    picture: Option<Picture>,
    id: String,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
}

struct X11State {
    logic: Logic,
    config: Config,
    overlays: Vec<Overlay>,
    root: Window,
    argb: Option<Argb>,
    toggle_code: u8,
    bind: Option<Window>,
    bind_draft: Option<Hotkey>,
    bind_preview: String,
    bind_locked: bool,
    bind_held: Vec<u8>,
    pick_grabbed: bool,
}

pub fn run() -> Result<(), String> {
    if env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland")
        && env::var("DISPLAY").is_err()
    {
        return Err(crate::i18n::Language::from_os().tr().wayland.into());
    }

    let (conn, screen_num) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let screen = conn.setup().roots[screen_num].clone();
    let config = Config::load();
    crate::autostart::refresh_path();
    let lang = config.ui_lang();
    let mut state = X11State {
        logic: Logic::new(config.style),
        config,
        overlays: Vec::new(),
        root: screen.root,
        argb: find_argb(&conn, screen_num, screen.root),
        toggle_code: 0,
        bind: None,
        bind_draft: None,
        bind_preview: String::new(),
        bind_locked: false,
        bind_held: Vec::new(),
        pick_grabbed: false,
    };

    rebuild(&conn, &screen, &mut state)?;
    if grab_hotkey(&conn, screen.root, &state.config.hotkey)
        .map(|code| {
            state.toggle_code = code;
        })
        .is_err()
    {
        eprintln!("{}", lang.x11_busy_startup(&state.config.hotkey.to_string()));
        begin_bind(&conn, &screen, &mut state, None)?;
    }

    eprintln!(
        "{}",
        lang.x11_ready(
            &state.config.hotkey.to_string(),
            &crate::config::config_path().display().to_string()
        )
    );

    loop {
        conn.flush().map_err(|e| e.to_string())?;
        let event = conn.wait_for_event().map_err(|e| e.to_string())?;
        handle(&conn, &screen, &mut state, event)?;
    }
}

fn xcolor(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color {
        red: u16::from(r) * 257,
        green: u16::from(g) * 257,
        blue: u16::from(b) * 257,
        alpha: u16::from(a) * 257,
    }
}

fn find_argb<C: Connection>(conn: &C, screen_num: usize, root: Window) -> Option<Argb> {
    let reply = render::ConnectionExt::render_query_pict_formats(conn)
        .ok()?
        .reply()
        .ok()?;
    let pict_screen = reply.screens.get(screen_num)?;
    let (visual, format, depth) = pict_screen.depths.iter().find_map(|depth| {
        if depth.depth != 32 {
            return None;
        }
        depth.visuals.iter().find_map(|vis| {
            let fmt = reply.formats.iter().find(|f| f.id == vis.format)?;
            if u8::from(fmt.type_) == u8::from(PictType::DIRECT)
                && fmt.depth == 32
                && fmt.direct.alpha_mask != 0
            {
                Some((vis.visual, fmt.id, 32u8))
            } else {
                None
            }
        })
    })?;
    let colormap = conn.generate_id().ok()?;
    conn.create_colormap(ColormapAlloc::NONE, colormap, root, visual)
        .ok()?
        .check()
        .ok()?;
    Some(Argb {
        visual,
        format,
        depth,
        colormap,
    })
}

fn rebuild<C: Connection>(conn: &C, screen: &Screen, state: &mut X11State) -> Result<(), String> {
    for overlay in state.overlays.drain(..) {
        if let Some(picture) = overlay.picture {
            let _ = render::ConnectionExt::render_free_picture(conn, picture);
        }
        let _ = conn.destroy_window(overlay.window);
        let _ = conn.free_gc(overlay.gc);
    }

    let resources = x11rb::protocol::randr::ConnectionExt::randr_get_screen_resources_current(
        conn,
        screen.root,
    )
    .ok()
    .and_then(|c| c.reply().ok());

    let mut monitors = Vec::new();
    if let Some(res) = resources {
        for (i, output) in res.outputs.iter().enumerate() {
            if let Some(info) = x11rb::protocol::randr::ConnectionExt::randr_get_output_info(
                conn,
                *output,
                res.config_timestamp,
            )
            .ok()
            .and_then(|c| c.reply().ok())
            {
                if info.connection != x11rb::protocol::randr::Connection::CONNECTED
                    || info.crtc == 0
                {
                    continue;
                }
                if let Some(crtc) = x11rb::protocol::randr::ConnectionExt::randr_get_crtc_info(
                    conn,
                    info.crtc,
                    res.config_timestamp,
                )
                .ok()
                .and_then(|c| c.reply().ok())
                {
                    if crtc.width == 0 || crtc.height == 0 {
                        continue;
                    }
                    monitors.push((
                        format!("OUTPUT-{i}"),
                        crtc.x,
                        crtc.y,
                        crtc.width,
                        crtc.height,
                    ));
                }
            }
        }
    }
    if monitors.is_empty() {
        monitors.push((
            "SCREEN-0".into(),
            0,
            0,
            screen.width_in_pixels,
            screen.height_in_pixels,
        ));
    }
    monitors.sort_by_key(|m| (m.1, m.2));

    let mut atom_opacity = None;
    if let Ok(cookie) = conn.intern_atom(false, b"_NET_WM_WINDOW_OPACITY") {
        if let Ok(reply) = cookie.reply() {
            atom_opacity = Some(reply.atom);
        }
    }

    for (id, x, y, w, h) in monitors {
        let window = conn.generate_id().map_err(|e| e.to_string())?;
        let mut aux = CreateWindowAux::new()
            .override_redirect(1)
            .background_pixel(0)
            .border_pixel(0)
            .event_mask(
                EventMask::BUTTON_PRESS
                    | EventMask::ENTER_WINDOW
                    | EventMask::POINTER_MOTION
                    | EventMask::EXPOSURE
                    | EventMask::VISIBILITY_CHANGE,
            );
        let (depth, visual) = if let Some(argb) = state.argb {
            aux = aux.colormap(argb.colormap);
            (argb.depth, argb.visual)
        } else {
            (COPY_DEPTH_FROM_PARENT, 0)
        };
        conn.create_window(
            depth,
            window,
            screen.root,
            x,
            y,
            w,
            h,
            0,
            WindowClass::INPUT_OUTPUT,
            visual,
            &aux,
        )
        .map_err(|e| e.to_string())?;

        let gc = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_gc(gc, window, &CreateGCAux::new().foreground(0).background(0))
            .map_err(|e| e.to_string())?;

        let picture = if let Some(argb) = state.argb {
            let picture = conn.generate_id().map_err(|e| e.to_string())?;
            render::ConnectionExt::render_create_picture(
                conn,
                picture,
                window,
                argb.format,
                &CreatePictureAux::new(),
            )
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
            Some(picture)
        } else {
            None
        };

        if let Some(atom) = atom_opacity {
            let opacity = u32::MAX;
            let _ = conn.change_property(
                PropMode::REPLACE,
                window,
                atom,
                AtomEnum::CARDINAL,
                32,
                1,
                &opacity.to_ne_bytes(),
            );
        }
        state.overlays.push(Overlay {
            window,
            gc,
            picture,
            id,
            x,
            y,
            w,
            h,
        });
    }
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn keysym_to_code<C: Connection>(conn: &C, keysym: u32) -> Result<u8, String> {
    let mapping = conn
        .get_keyboard_mapping(8, 248)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    for (i, syms) in mapping
        .keysyms
        .chunks(mapping.keysyms_per_keycode as usize)
        .enumerate()
    {
        if syms.contains(&keysym) || syms.contains(&(keysym & !0x20)) {
            return Ok((i as u8) + 8);
        }
    }
    Err("key not in keyboard map".into())
}

fn grab_hotkey<C: Connection>(conn: &C, root: Window, hotkey: &Hotkey) -> Result<u8, String> {
    let code = keysym_to_code(conn, hotkey.x11_keysym())?;
    let mut ok = false;
    for mask in hotkey.x11_mod_variants() {
        if conn
            .grab_key(
                true,
                root,
                mask.into(),
                code,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )
            .ok()
            .and_then(|c| c.check().ok())
            .is_some()
        {
            ok = true;
        }
    }
    conn.flush().map_err(|e| e.to_string())?;
    if ok {
        Ok(code)
    } else {
        Err("hotkey already grabbed".into())
    }
}

fn ungrab_hotkey<C: Connection>(conn: &C, root: Window, code: u8, hotkey: &Hotkey) {
    for mask in hotkey.x11_mod_variants() {
        let _ = conn.ungrab_key(code, root, mask.into());
    }
    let _ = conn.flush();
}

fn pointer_on_root<C: Connection>(conn: &C, root: Window) -> Option<(i16, i16)> {
    conn.query_pointer(root)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| (r.root_x, r.root_y))
}

fn popup_xy(state: &X11State, hint: Option<(i16, i16)>) -> (i16, i16) {
    let monitor = hint
        .and_then(|(x, y)| {
            state
                .overlays
                .iter()
                .find(|o| x >= o.x && y >= o.y && x < o.x + o.w as i16 && y < o.y + o.h as i16)
        })
        .or_else(|| state.overlays.first());
    match monitor {
        Some(o) => {
            let x = i32::from(o.x) + (i32::from(o.w) - i32::from(BIND_W)) / 2;
            let y = i32::from(o.y) + (i32::from(o.h) - i32::from(BIND_H)) / 2;
            (x as i16, y as i16)
        }
        None => (0, 0),
    }
}

fn begin_bind<C: Connection>(
    conn: &C,
    screen: &Screen,
    state: &mut X11State,
    hint: Option<(i16, i16)>,
) -> Result<(), String> {
    if state.bind.is_some() {
        return Ok(());
    }
    ungrab_pick_keys(conn, state);
    if state.toggle_code != 0 {
        ungrab_hotkey(conn, screen.root, state.toggle_code, &state.config.hotkey);
        state.toggle_code = 0;
    }
    state.bind_draft = None;
    state.bind_preview.clear();
    state.bind_locked = false;
    state.bind_held.clear();
    let w = BIND_W;
    let h = BIND_H;
    let hint = hint.or_else(|| pointer_on_root(conn, screen.root));
    let (x, y) = popup_xy(state, hint);
    let win = conn.generate_id().map_err(|e| e.to_string())?;
    conn.create_window(
        COPY_DEPTH_FROM_PARENT,
        win,
        screen.root,
        x as i16,
        y as i16,
        w,
        h,
        1,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new()
            .override_redirect(1)
            .event_mask(
                EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE
                    | EventMask::BUTTON_PRESS
                    | EventMask::EXPOSURE
                    | EventMask::VISIBILITY_CHANGE,
            )
            .background_pixel(0x101010)
            .border_pixel(0x3DDC97),
    )
    .map_err(|e| e.to_string())?;
    conn.map_window(win).map_err(|e| e.to_string())?;
    let _ = conn.grab_keyboard(true, win, 0u32, GrabMode::ASYNC, GrabMode::ASYNC);
    state.bind = Some(win);
    paint_bind(conn, state, win)?;
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn finish_bind<C: Connection>(
    conn: &C,
    screen: &Screen,
    state: &mut X11State,
    hotkey: Option<Hotkey>,
) -> Result<(), String> {
    if let Some(win) = state.bind.take() {
        let _ = conn.ungrab_keyboard(0u32);
        let _ = conn.destroy_window(win);
        let _ = conn.flush();
    }
    state.bind_draft = None;
    state.bind_preview.clear();
    state.bind_locked = false;
    state.bind_held.clear();
    if let Some(hotkey) = hotkey {
        let already_ours = hotkey == state.config.hotkey;
        match grab_hotkey(conn, screen.root, &hotkey) {
            Ok(code) => {
                state.config.hotkey = hotkey;
                state.config.save();
                state.toggle_code = code;
                eprintln!("{}", state.config.ui_lang().x11_saved(&hotkey.to_string()));
            }
            Err(_) if already_ours => {
                state.config.save();
                if let Ok(code) = grab_hotkey(conn, screen.root, &hotkey) {
                    state.toggle_code = code;
                }
                eprintln!("{}", state.config.ui_lang().x11_saved(&hotkey.to_string()));
            }
            Err(_) => {
                eprintln!(
                    "{}",
                    state
                        .config
                        .ui_lang()
                        .x11_busy_keep(&hotkey.to_string(), &state.config.hotkey.to_string())
                );
                if let Ok(code) = grab_hotkey(conn, screen.root, &state.config.hotkey) {
                    state.toggle_code = code;
                }
            }
        }
    } else if let Ok(code) = grab_hotkey(conn, screen.root, &state.config.hotkey) {
        state.toggle_code = code;
    }
    if state.logic.mode == crate::app::Mode::Pick {
        grab_pick_keys(conn, state);
    }
    Ok(())
}

fn hotkey_from_x11(ev: &KeyPressEvent) -> Option<Hotkey> {
    let key = match ev.detail {
        65 => Key::Space,
        10..=18 => Key::Char(char::from(b'1' + ev.detail - 10)),
        19 => Key::Char('0'),
        24..=33 | 38..=47 | 52..=58 => key_from_common_x11(ev.detail)?,
        67..=76 => Key::F(ev.detail - 66),
        95 => Key::F(11),
        96 => Key::F(12),
        _ => return None,
    };
    let bits = ev.state.bits();
    let ctrl = bits & 4 != 0;
    let alt = bits & 8 != 0;
    let shift = bits & 1 != 0;
    let meta = bits & 64 != 0;
    Hotkey::new(ctrl, alt, shift, meta, key)
}

fn key_from_common_x11(code: u8) -> Option<Key> {
    // US QWERTY keycodes — enough for bind capture; config stores the letter.
    const ROW1: &[u8] = b"QWERTYUIOP";
    const ROW2: &[u8] = b"ASDFGHJKL";
    const ROW3: &[u8] = b"ZXCVBNM";
    let c = if (24..=33).contains(&code) {
        ROW1[(code - 24) as usize]
    } else if (38..=46).contains(&code) {
        ROW2[(code - 38) as usize]
    } else if (52..=58).contains(&code) {
        ROW3[(code - 52) as usize]
    } else {
        return None;
    };
    Some(Key::Char(c as char))
}

fn handle<C: Connection>(
    conn: &C,
    screen: &Screen,
    state: &mut X11State,
    event: Event,
) -> Result<(), String> {
    match event {
        Event::KeyPress(ev) => handle_key(conn, screen, state, ev)?,
        Event::KeyRelease(ev) => handle_bind_release(state, ev.detail),
        Event::ButtonPress(ev) => handle_button(conn, screen, state, ev)?,
        Event::EnterNotify(ev) => handle_enter(conn, state, ev)?,
        Event::MotionNotify(ev) => {
            if let Some(id) = id_of(state, ev.event) {
                let action = state.logic.on_pointer(&id);
                apply(conn, state, action)?;
            }
        }
        Event::Expose(ev) => paint(conn, state, ev.window)?,
        Event::VisibilityNotify(ev) => {
            let _ = conn.configure_window(
                ev.window,
                &x11rb::protocol::xproto::ConfigureWindowAux::new()
                    .stack_mode(x11rb::protocol::xproto::StackMode::ABOVE),
            );
        }
        Event::MapNotify(_) | Event::ConfigureNotify(_) => {}
        _ => {
            // RandR notify arrives as generic; debounce rebuild on any unexpected gap.
            let _ = screen;
        }
    }
    Ok(())
}

fn handle_bind_release(state: &mut X11State, code: u8) {
    if state.bind.is_none() {
        return;
    }
    state.bind_held.retain(|&c| c != code);
    if state.bind_locked && state.bind_held.is_empty() {
        state.bind_locked = false;
    }
}

fn handle_key<C: Connection>(
    conn: &C,
    screen: &Screen,
    state: &mut X11State,
    ev: KeyPressEvent,
) -> Result<(), String> {
    if state.bind.is_some() {
        if !state.bind_held.contains(&ev.detail) {
            state.bind_held.push(ev.detail);
        }
        if ev.detail == 9 {
            return finish_bind(conn, screen, state, None);
        }
        if ev.detail == 36 {
            return finish_bind(conn, screen, state, state.bind_draft);
        }
        if state.bind_locked {
            return Ok(());
        }
        if state.bind_draft.is_some() {
            state.bind_draft = None;
            state.bind_preview.clear();
            if let Some(win) = state.bind {
                paint_bind(conn, state, win)?;
            }
        }
        if let Some(hotkey) = hotkey_from_x11(&ev) {
            state.bind_draft = Some(hotkey);
            state.bind_preview = hotkey.to_string();
            state.bind_locked = true;
            if let Some(win) = state.bind {
                paint_bind(conn, state, win)?;
            }
            eprintln!("{}", state.config.ui_lang().x11_bind_hint(&hotkey.to_string()));
        }
        return Ok(());
    }
    let relevant = ev.state.bits() & (1 | 4 | 8 | 64);
    if state.toggle_code != 0
        && ev.detail == state.toggle_code
        && relevant == state.config.hotkey.x11_mod_mask()
    {
        let id = cursor_id(state, ev.root_x, ev.root_y);
        let action = state.logic.on_toggle(id);
        apply(conn, state, action)?;
        return Ok(());
    }
    if state.logic.mode != crate::app::Mode::Pick {
        return Ok(());
    }
    let ids = state
        .overlays
        .iter()
        .map(|o| o.id.clone())
        .collect::<Vec<_>>();
    let key = match ev.detail {
        9 => Some(PickKey::Escape),
        36 => Some(PickKey::Enter),
        113 => Some(PickKey::Left),
        114 => Some(PickKey::Right),
        code if (10..=18).contains(&code) => Some(PickKey::Digit(code - 9)),
        _ => None,
    };
    if let Some(key) = key {
        let action = state.logic.on_pick_key(key, &ids);
        apply(conn, state, action)?;
    }
    Ok(())
}

fn handle_button<C: Connection>(
    conn: &C,
    screen: &Screen,
    state: &mut X11State,
    ev: ButtonPressEvent,
) -> Result<(), String> {
    if state.bind == Some(ev.event) && ev.detail == 1 {
        let (save, cancel) = bind_button_rects();
        if hit_rect(save, ev.event_x, ev.event_y) {
            return if state.bind_draft.is_some() {
                finish_bind(conn, screen, state, state.bind_draft)
            } else {
                Ok(())
            };
        }
        if hit_rect(cancel, ev.event_x, ev.event_y) {
            return finish_bind(conn, screen, state, None);
        }
        return Ok(());
    }
    if ev.detail == 3 {
        return begin_bind(conn, screen, state, Some((ev.root_x, ev.root_y)));
    }
    if let Some(id) = id_of(state, ev.event) {
        let action = state.logic.on_click(&id);
        apply(conn, state, action)?;
    }
    Ok(())
}

fn handle_enter<C: Connection>(
    conn: &C,
    state: &mut X11State,
    ev: EnterNotifyEvent,
) -> Result<(), String> {
    if let Some(id) = id_of(state, ev.event) {
        let action = state.logic.on_pointer(&id);
        apply(conn, state, action)?;
    }
    Ok(())
}

fn id_of(state: &X11State, window: Window) -> Option<String> {
    state
        .overlays
        .iter()
        .find(|o| o.window == window)
        .map(|o| o.id.clone())
}

fn cursor_id(state: &X11State, x: i16, y: i16) -> Option<String> {
    state
        .overlays
        .iter()
        .find(|o| x >= o.x && y >= o.y && x < o.x + o.w as i16 && y < o.y + o.h as i16)
        .map(|o| o.id.clone())
        .or_else(|| state.overlays.first().map(|o| o.id.clone()))
}

fn grab_pick_keys<C: Connection>(conn: &C, state: &mut X11State) {
    if state.pick_grabbed {
        return;
    }
    for &code in PICK_CODES {
        for mask in PICK_MODS {
            let _ = conn.grab_key(
                true,
                state.root,
                mask.into(),
                code,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            );
        }
    }
    let _ = conn.flush();
    state.pick_grabbed = true;
}

fn ungrab_pick_keys<C: Connection>(conn: &C, state: &mut X11State) {
    if !state.pick_grabbed {
        return;
    }
    for &code in PICK_CODES {
        for mask in PICK_MODS {
            let _ = conn.ungrab_key(code, state.root, mask.into());
        }
    }
    let _ = conn.flush();
    state.pick_grabbed = false;
}

fn apply<C: Connection>(conn: &C, state: &mut X11State, action: Action) -> Result<(), String> {
    match action {
        Action::None => {}
        Action::HideAll => {
            ungrab_pick_keys(conn, state);
            for overlay in &state.overlays {
                let _ = conn.unmap_window(overlay.window);
            }
        }
        Action::ShowPick { candidate } | Action::RefreshPick { candidate } => {
            grab_pick_keys(conn, state);
            let style = state.logic.style;
            for overlay in &state.overlays {
                map_overlay(conn, overlay, overlay.id == candidate, style)?;
            }
        }
        Action::ShowLocked { cinema } => {
            ungrab_pick_keys(conn, state);
            let style = state.logic.style;
            for overlay in &state.overlays {
                if overlay.id == cinema {
                    let _ = conn.unmap_window(overlay.window);
                } else {
                    map_overlay(conn, overlay, false, style)?;
                }
            }
        }
    }
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn map_overlay<C: Connection>(
    conn: &C,
    overlay: &Overlay,
    candidate: bool,
    style: Style,
) -> Result<(), String> {
    conn.configure_window(
        overlay.window,
        &x11rb::protocol::xproto::ConfigureWindowAux::new()
            .x(i32::from(overlay.x))
            .y(i32::from(overlay.y))
            .width(u32::from(overlay.w))
            .height(u32::from(overlay.h))
            .stack_mode(x11rb::protocol::xproto::StackMode::ABOVE),
    )
    .map_err(|e| e.to_string())?;
    conn.map_window(overlay.window).map_err(|e| e.to_string())?;
    let opacity = if overlay.picture.is_some() {
        u32::MAX
    } else if candidate {
        u32::MAX
    } else if matches!(style, Style::Tint | Style::Frost) {
        u32::from(FROST_ALPHA) << 24
    } else {
        u32::MAX
    };
    set_opacity(conn, overlay.window, opacity)?;
    if !candidate && style == Style::Frost {
        try_kwin_blur(conn, overlay.window);
    }
    paint_window(conn, overlay, candidate, style)?;
    Ok(())
}

fn frame_rects(w: u16, h: u16) -> [Rectangle; 4] {
    let b = BORDER;
    [
        Rectangle {
            x: 0,
            y: 0,
            width: w,
            height: b as u16,
        },
        Rectangle {
            x: 0,
            y: h as i16 - b,
            width: w,
            height: b as u16,
        },
        Rectangle {
            x: 0,
            y: 0,
            width: b as u16,
            height: h,
        },
        Rectangle {
            x: w as i16 - b,
            y: 0,
            width: b as u16,
            height: h,
        },
    ]
}

fn set_opacity<C: Connection>(conn: &C, window: Window, opacity: u32) -> Result<(), String> {
    if let Ok(atom) = conn.intern_atom(false, b"_NET_WM_WINDOW_OPACITY") {
        if let Ok(atom) = atom.reply() {
            let _ = conn.change_property(
                PropMode::REPLACE,
                window,
                atom.atom,
                AtomEnum::CARDINAL,
                32,
                1,
                &opacity.to_ne_bytes(),
            );
        }
    }
    Ok(())
}

fn try_kwin_blur<C: Connection>(conn: &C, window: Window) {
    if let Ok(atom) = conn.intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION") {
        if let Ok(atom) = atom.reply() {
            let _ = conn.change_property(
                PropMode::REPLACE,
                window,
                atom.atom,
                AtomEnum::CARDINAL,
                32,
                0,
                &[],
            );
        }
    }
}

fn paint<C: Connection>(conn: &C, state: &X11State, window: Window) -> Result<(), String> {
    if state.bind == Some(window) {
        return paint_bind(conn, state, window);
    }
    if let Some(overlay) = state.overlays.iter().find(|o| o.window == window) {
        let candidate = state.logic.mode == crate::app::Mode::Pick
            && state.logic.candidate.as_deref() == Some(overlay.id.as_str());
        paint_window(conn, overlay, candidate, state.logic.style)?;
    }
    Ok(())
}

fn paint_window<C: Connection>(
    conn: &C,
    overlay: &Overlay,
    candidate: bool,
    style: Style,
) -> Result<(), String> {
    if let Some(picture) = overlay.picture {
        return paint_argb(conn, overlay, picture, candidate, style);
    }
    if candidate {
        conn.change_gc(overlay.gc, &ChangeGCAux::new().foreground(0x000000))
            .map_err(|e| e.to_string())?;
        conn.poly_fill_rectangle(
            overlay.window,
            overlay.gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: overlay.w,
                height: overlay.h,
            }],
        )
        .map_err(|e| e.to_string())?;
        conn.change_gc(overlay.gc, &ChangeGCAux::new().foreground(0x3DDC97))
            .map_err(|e| e.to_string())?;
        conn.poly_fill_rectangle(overlay.window, overlay.gc, &frame_rects(overlay.w, overlay.h))
            .map_err(|e| e.to_string())?;
    } else {
        let color = if style == Style::Frost {
            0x050505
        } else {
            0x000000
        };
        conn.change_gc(overlay.gc, &ChangeGCAux::new().foreground(color))
            .map_err(|e| e.to_string())?;
        conn.poly_fill_rectangle(
            overlay.window,
            overlay.gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: overlay.w,
                height: overlay.h,
            }],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn paint_argb<C: Connection>(
    conn: &C,
    overlay: &Overlay,
    picture: Picture,
    candidate: bool,
    style: Style,
) -> Result<(), String> {
    let full = [Rectangle {
        x: 0,
        y: 0,
        width: overlay.w,
        height: overlay.h,
    }];
    if candidate {
        render::ConnectionExt::render_fill_rectangles(
            conn,
            PictOp::SRC,
            picture,
            xcolor(0, 0, 0, 1),
            &full,
        )
        .map_err(|e| e.to_string())?;
        render::ConnectionExt::render_fill_rectangles(
            conn,
            PictOp::SRC,
            picture,
            xcolor(0x3D, 0xDC, 0x97, 255),
            &frame_rects(overlay.w, overlay.h),
        )
        .map_err(|e| e.to_string())?;
    } else {
        let (r, g, b, a) = if style == Style::Solid {
            (0, 0, 0, 255)
        } else if style == Style::Frost {
            (5, 5, 5, FROST_ALPHA)
        } else {
            (0, 0, 0, FROST_ALPHA)
        };
        render::ConnectionExt::render_fill_rectangles(
            conn,
            PictOp::SRC,
            picture,
            xcolor(r, g, b, a),
            &full,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn bind_button_rects() -> (Rectangle, Rectangle) {
    let mid = BIND_W as i16 / 2;
    let top = BIND_H as i16 - 64;
    (
        Rectangle {
            x: mid - BIND_BTN_W - BIND_BTN_GAP / 2,
            y: top,
            width: BIND_BTN_W as u16,
            height: BIND_BTN_H as u16,
        },
        Rectangle {
            x: mid + BIND_BTN_GAP / 2,
            y: top,
            width: BIND_BTN_W as u16,
            height: BIND_BTN_H as u16,
        },
    )
}

fn hit_rect(rect: Rectangle, x: i16, y: i16) -> bool {
    x >= rect.x && x < rect.x + rect.width as i16 && y >= rect.y && y < rect.y + rect.height as i16
}

fn outline(rect: Rectangle) -> [Rectangle; 4] {
    [
        Rectangle {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: 1,
        },
        Rectangle {
            x: rect.x,
            y: rect.y + rect.height as i16 - 1,
            width: rect.width,
            height: 1,
        },
        Rectangle {
            x: rect.x,
            y: rect.y,
            width: 1,
            height: rect.height,
        },
        Rectangle {
            x: rect.x + rect.width as i16 - 1,
            y: rect.y,
            width: 1,
            height: rect.height,
        },
    ]
}

struct GlyphDots {
    dots: Vec<(i16, i16)>,
}

impl DrawTarget for GlyphDots {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if color.is_on()
                && point.x >= i32::from(i16::MIN)
                && point.y >= i32::from(i16::MIN)
                && point.x <= i32::from(i16::MAX)
                && point.y <= i32::from(i16::MAX)
            {
                self.dots.push((point.x as i16, point.y as i16));
            }
        }
        Ok(())
    }
}

impl OriginDimensions for GlyphDots {
    fn size(&self) -> Size {
        Size::new(u32::from(BIND_W), u32::from(BIND_H))
    }
}

fn draw_centered<C: Connection>(conn: &C, window: Window, gc: Gcontext, rect: Rectangle, text: &str) {
    if text.is_empty() {
        return;
    }
    let char_w = FONT_10X20.character_size.width as i16 + FONT_10X20.character_spacing as i16;
    let char_h = FONT_10X20.character_size.height as i16;
    let text_w = char_w * text.chars().count() as i16;
    let x = i32::from(rect.x) + (i32::from(rect.width) - i32::from(text_w)) / 2;
    let y = i32::from(rect.y) + (i32::from(rect.height) - i32::from(char_h)) / 2;
    let mut dots = GlyphDots { dots: Vec::new() };
    let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
    let _ = Text::with_baseline(text, Point::new(x, y), style, Baseline::Top).draw(&mut dots);
    const CHUNK: usize = 512;
    let rects: Vec<Rectangle> = dots
        .dots
        .into_iter()
        .map(|(px, py)| Rectangle {
            x: px,
            y: py,
            width: 1,
            height: 1,
        })
        .collect();
    for chunk in rects.chunks(CHUNK) {
        let _ = conn.poly_fill_rectangle(window, gc, chunk);
    }
}

fn paint_bind<C: Connection>(conn: &C, state: &X11State, window: Window) -> Result<(), String> {
    let gc = conn.generate_id().map_err(|e| e.to_string())?;
    conn.create_gc(
        gc,
        window,
        &CreateGCAux::new().foreground(0x101010).background(0x101010),
    )
    .map_err(|e| e.to_string())?;
    conn.poly_fill_rectangle(
        window,
        gc,
        &[Rectangle {
            x: 0,
            y: 0,
            width: BIND_W,
            height: BIND_H,
        }],
    )
    .map_err(|e| e.to_string())?;

    let _ = conn.change_gc(gc, &ChangeGCAux::new().foreground(0xF0F0F0));
    draw_centered(
        conn,
        window,
        gc,
        Rectangle {
            x: 28,
            y: 20,
            width: BIND_W - 56,
            height: 32,
        },
        state.config.ui_lang().tr().bind_prompt,
    );
    let need_mod = state.config.ui_lang().bind_need_mod();
    draw_centered(
        conn,
        window,
        gc,
        Rectangle {
            x: 28,
            y: 52,
            width: BIND_W - 56,
            height: 32,
        },
        &need_mod,
    );
    let preview = if state.bind_preview.is_empty() {
        "..."
    } else {
        state.bind_preview.as_str()
    };
    let preview_color = if state.bind_draft.is_some() {
        0x3DDC97
    } else {
        0xA0A0A0
    };
    let _ = conn.change_gc(gc, &ChangeGCAux::new().foreground(preview_color));
    draw_centered(
        conn,
        window,
        gc,
        Rectangle {
            x: 28,
            y: 96,
            width: BIND_W - 56,
            height: 48,
        },
        preview,
    );
    let (save, cancel) = bind_button_rects();
    let label_color = if state.bind_draft.is_some() {
        0xF0F0F0
    } else {
        0xA0A0A0
    };
    let _ = conn.change_gc(gc, &ChangeGCAux::new().foreground(label_color));
    draw_centered(conn, window, gc, save, state.config.ui_lang().tr().save_enter);
    let _ = conn.change_gc(gc, &ChangeGCAux::new().foreground(0xF0F0F0));
    draw_centered(conn, window, gc, cancel, state.config.ui_lang().tr().cancel_esc);

    let (save, cancel) = bind_button_rects();
    let _ = conn.change_gc(gc, &ChangeGCAux::new().foreground(0x3DDC97));
    let mut borders = Vec::with_capacity(8);
    borders.extend_from_slice(&outline(save));
    borders.extend_from_slice(&outline(cancel));
    conn.poly_fill_rectangle(window, gc, &borders)
        .map_err(|e| e.to_string())?;
    let _ = conn.free_gc(gc);
    Ok(())
}
