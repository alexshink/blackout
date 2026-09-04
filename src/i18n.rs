use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Ru,
    En,
}

#[allow(dead_code)]
impl Language {
    pub const ALL: &[Language] = &[Language::Ru, Language::En];

    pub fn id(self) -> &'static str {
        match self {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }

    pub fn from_id(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ru" => Some(Language::Ru),
            "en" => Some(Language::En),
            _ => None,
        }
    }

    pub fn endonym(self) -> &'static str {
        match self {
            Language::Ru => "Русский",
            Language::En => "English",
        }
    }

    pub fn from_os() -> Self {
        detect_os().unwrap_or(Language::Ru)
    }

    pub fn tr(self) -> &'static Tr {
        match self {
            Language::Ru => &RU,
            Language::En => &EN,
        }
    }

    pub fn toggle(self, idle: bool) -> &'static str {
        let t = self.tr();
        if idle {
            t.enable
        } else {
            t.disable
        }
    }

    pub fn toggle_tab(self, idle: bool, hk: &str) -> String {
        format!("{}\t{hk}", self.toggle(idle))
    }

    pub fn hotkey_item(self, hk: &str, ok: bool) -> String {
        let t = self.tr();
        let tpl = if ok {
            t.hotkey_ok_fmt
        } else {
            t.hotkey_busy_fmt
        };
        tpl.replace("{hk}", hk)
    }

    pub fn bind_need_mod(self) -> String {
        self.tr().bind_need_mod.replace("{meta}", meta_token())
    }

    pub fn bind_hint(self) -> String {
        format!("{}\n{}", self.tr().bind_prompt, self.bind_need_mod())
    }

    pub fn version_line(self, n: &str) -> String {
        format!("{} {n}", self.tr().version)
    }

    pub fn author_line(self, name: &str) -> String {
        self.tr().author_fmt.replace("{name}", name)
    }

    pub fn credit_lines(self) -> [&'static str; 2] {
        let t = self.tr();
        [t.credit_via, t.credit_minds]
    }

    pub fn license_line(self, name: &str) -> String {
        format!("{} {name}", self.tr().license)
    }

    pub fn hotkey_busy_startup(self, hk: &str) -> String {
        self.tr().hotkey_busy_startup.replace("{hk}", hk)
    }

    pub fn hotkey_busy_retry(self, hk: &str) -> String {
        self.tr().hotkey_busy_retry.replace("{hk}", hk)
    }

    pub fn hotkey_saved(self, hk: &str) -> String {
        self.tr().hotkey_saved.replace("{hk}", hk)
    }

    pub fn x11_busy_startup(self, hk: &str) -> String {
        self.tr().x11_busy_startup.replace("{hk}", hk)
    }

    pub fn x11_ready(self, hk: &str, path: &str) -> String {
        self.tr()
            .x11_ready
            .replace("{hk}", hk)
            .replace("{path}", path)
    }

    pub fn x11_saved(self, hk: &str) -> String {
        self.tr().x11_saved.replace("{hk}", hk)
    }

    pub fn x11_busy_keep(self, hk: &str, old: &str) -> String {
        self.tr()
            .x11_busy_keep
            .replace("{hk}", hk)
            .replace("{old}", old)
    }

    pub fn x11_bind_hint(self, hk: &str) -> String {
        self.tr().x11_bind_hint.replace("{hk}", hk)
    }
}

fn meta_token() -> &'static str {
    #[cfg(windows)]
    {
        "Win"
    }
    #[cfg(target_os = "macos")]
    {
        "Cmd"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "Super"
    }
}

#[allow(dead_code)]
pub struct Tr {
    pub enable: &'static str,
    pub disable: &'static str,
    pub style: &'static str,
    pub solid: &'static str,
    pub tint: &'static str,
    pub frost: &'static str,
    pub settings: &'static str,
    pub language: &'static str,
    pub hotkey_ok_fmt: &'static str,
    pub hotkey_busy_fmt: &'static str,
    pub autostart: &'static str,
    pub wipe: &'static str,
    pub about: &'static str,
    pub quit: &'static str,
    pub bind_prompt: &'static str,
    pub bind_need_mod: &'static str,
    pub bind_title: &'static str,
    pub save_enter: &'static str,
    pub cancel_esc: &'static str,
    pub version: &'static str,
    pub author_fmt: &'static str,
    pub credit_via: &'static str,
    pub credit_minds: &'static str,
    pub license: &'static str,
    pub close_esc: &'static str,
    pub wipe_title: &'static str,
    pub wipe_body: &'static str,
    pub wipe_config_fmt: &'static str,
    pub wipe_config_unavailable: &'static str,
    pub wipe_confirm: &'static str,
    pub wipe_note_title: &'static str,
    pub wipe_failed: &'static str,
    pub hotkey_title: &'static str,
    pub hotkey_busy_title: &'static str,
    pub hotkey_busy_startup: &'static str,
    pub hotkey_busy_retry: &'static str,
    pub hotkey_saved: &'static str,
    pub autostart_failed: &'static str,
    pub wayland: &'static str,
    pub x11_busy_startup: &'static str,
    pub x11_ready: &'static str,
    pub x11_saved: &'static str,
    pub x11_busy_keep: &'static str,
    pub x11_bind_hint: &'static str,
}

const RU: Tr = Tr {
    enable: "Включить",
    disable: "Выключить",
    style: "Стиль",
    solid: "Чёрный",
    tint: "Тонировка",
    frost: "Матовое стекло",
    settings: "Настройки",
    language: "Язык",
    hotkey_ok_fmt: "Хоткей: {hk}…",
    hotkey_busy_fmt: "Хоткей: {hk} (занят)…",
    autostart: "Автозагрузка",
    wipe: "Удалить данные…",
    about: "О программе",
    quit: "Выход",
    bind_prompt: "Нажмите новую комбинацию",
    bind_need_mod: "Нужен Ctrl / Alt / Shift / {meta}",
    bind_title: "Хоткей Blackout",
    save_enter: "Сохранить (Enter)",
    cancel_esc: "Отмена (Esc)",
    version: "Версия",
    author_fmt: "Автор {name}",
    credit_via: "сделано с помощью",
    credit_minds: "естественного и искусственного интеллекта",
    license: "Лицензия",
    close_esc: "Закрыть (Esc)",
    wipe_title: "Удалить данные?",
    wipe_body: "\
Blackout будет исключена из автозагрузки, также\n\
будет удален конфиг и пустая папка blackout.\n\
Программа будет закрыта.\n\n\
{config}\n\n\
После этого просто удалите файл программы.",
    wipe_config_fmt: "Конфиг: {path}",
    wipe_config_unavailable: "Конфиг: профиль ОС недоступен.",
    wipe_confirm: "Удалить (Enter)",
    wipe_note_title: "Удаление данных",
    wipe_failed: "Не удалось удалить данные",
    hotkey_title: "Хоткей",
    hotkey_busy_title: "Хоткей занят",
    hotkey_busy_startup:
        "{hk} уже используется. Назначьте другую комбинацию через Трей → Настройки → Хоткей",
    hotkey_busy_retry: "{hk} уже используется. Попробуйте другую.",
    hotkey_saved: "Комбинация сохранена: {hk}",
    autostart_failed: "Не удалось изменить автозагрузку",
    wayland: "Wayland без X11 не поддерживается. Войдите в сессию X11 или используйте XWayland.",
    x11_busy_startup: "blackout: {hk} занята. Нажмите новую комбинацию в появившемся окне.",
    x11_ready: "Blackout (X11): {hk}, overlays ready. Смена: ПКМ по пелене или hotkey в {path}",
    x11_saved: "blackout: комбинация сохранена: {hk}",
    x11_busy_keep: "blackout: {hk} занята, остаётся {old}",
    x11_bind_hint: "blackout: {hk}  — Enter или клик «Сохранить»",
};

const EN: Tr = Tr {
    enable: "Enable",
    disable: "Disable",
    style: "Style",
    solid: "Black",
    tint: "Tint",
    frost: "Frosted glass",
    settings: "Settings",
    language: "Language",
    hotkey_ok_fmt: "Hotkey: {hk}…",
    hotkey_busy_fmt: "Hotkey: {hk} (in use)…",
    autostart: "Start at login",
    wipe: "Delete data…",
    about: "About",
    quit: "Quit",
    bind_prompt: "Press a new shortcut",
    bind_need_mod: "Needs Ctrl / Alt / Shift / {meta}",
    bind_title: "Blackout hotkey",
    save_enter: "Save (Enter)",
    cancel_esc: "Cancel (Esc)",
    version: "Version",
    author_fmt: "Author {name}",
    credit_via: "made with",
    credit_minds: "natural and artificial intelligence",
    license: "License",
    close_esc: "Close (Esc)",
    wipe_title: "Delete data?",
    wipe_body: "\
Blackout will be removed from startup. The config\n\
and the empty blackout folder will also be deleted.\n\
The program will close.\n\n\
{config}\n\n\
After that, just delete the program file.",
    wipe_config_fmt: "Config: {path}",
    wipe_config_unavailable: "Config: OS profile is unavailable.",
    wipe_confirm: "Delete (Enter)",
    wipe_note_title: "Delete data",
    wipe_failed: "Could not delete data",
    hotkey_title: "Hotkey",
    hotkey_busy_title: "Hotkey in use",
    hotkey_busy_startup:
        "{hk} is already in use. Assign another shortcut via Tray → Settings → Hotkey",
    hotkey_busy_retry: "{hk} is already in use. Try another one.",
    hotkey_saved: "Shortcut saved: {hk}",
    autostart_failed: "Could not change startup",
    wayland: "Wayland without X11 is not supported. Sign in to an X11 session or use XWayland.",
    x11_busy_startup: "blackout: {hk} is taken. Press a new shortcut in the window that opened.",
    x11_ready:
        "Blackout (X11): {hk}, overlays ready. Change: right-click the veil or the hotkey in {path}",
    x11_saved: "blackout: shortcut saved: {hk}",
    x11_busy_keep: "blackout: {hk} is taken, keeping {old}",
    x11_bind_hint: "blackout: {hk}  — Enter or click “Save”",
};

fn detect_os() -> Option<Language> {
    #[cfg(windows)]
    {
        if let Some(lang) = from_windows_ui() {
            return Some(lang);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(lang) = from_apple_languages() {
            return Some(lang);
        }
    }
    from_env()
}

fn parse_tag(tag: &str) -> Option<Language> {
    let tag = tag.trim().trim_matches(|c| c == '"' || c == '\'');
    if tag.is_empty() || tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        return None;
    }
    let primary = tag.split(['_', '-', '.', '@']).next()?.to_ascii_lowercase();
    match primary.as_str() {
        "ru" => Some(Language::Ru),
        "en" => Some(Language::En),
        _ => None,
    }
}

fn from_env() -> Option<Language> {
    for key in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(val) = std::env::var(key) else {
            continue;
        };
        for part in val.split(':') {
            if let Some(lang) = parse_tag(part) {
                return Some(lang);
            }
        }
    }
    None
}

#[cfg(windows)]
fn from_windows_ui() -> Option<Language> {
    use windows::core::PWSTR;
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};

    unsafe {
        let mut num = 0u32;
        let mut len = 0u32;
        let _ = GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut num, None, &mut len);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize];
        if GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut num,
            Some(PWSTR(buf.as_mut_ptr())),
            &mut len,
        )
        .is_err()
        {
            return None;
        }
        let mut start = 0usize;
        while start < buf.len() {
            if buf[start] == 0 {
                break;
            }
            let end = start + buf[start..].iter().position(|&c| c == 0)?;
            let tag = String::from_utf16_lossy(&buf[start..end]);
            if let Some(lang) = parse_tag(&tag) {
                return Some(lang);
            }
            start = end + 1;
        }
        None
    }
}

#[cfg(target_os = "macos")]
fn from_apple_languages() -> Option<Language> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyClass;
    use objc2::{class, msg_send};
    use objc2_foundation::{NSArray, NSString};

    let cls: &AnyClass = class!(NSLocale);
    let langs: Option<Retained<NSArray<NSString>>> = unsafe { msg_send![cls, preferredLanguages] };
    let langs = langs?;
    for item in langs.iter() {
        if let Some(lang) = parse_tag(&item.to_string()) {
            return Some(lang);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{parse_tag, Language};

    #[test]
    fn parse_ru_en_only() {
        assert_eq!(parse_tag("ru_RU.UTF-8"), Some(Language::Ru));
        assert_eq!(parse_tag("en-US"), Some(Language::En));
        assert_eq!(parse_tag("en"), Some(Language::En));
        assert_eq!(parse_tag("de_DE"), None);
        assert_eq!(parse_tag("C"), None);
        assert_eq!(Language::from_id("EN"), Some(Language::En));
        assert_eq!(Language::from_id("de"), None);
    }
}
