use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::hotkey::Hotkey;
use crate::i18n::Language;

/// Layered / opacity alpha for Tint and Frost. ~84% opaque — window-tint, not light fog.
pub const FROST_ALPHA: u8 = 214;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Solid,
    Tint,
    Frost,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub style: Style,
    pub hotkey: Hotkey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            style: Style::Solid,
            hotkey: Hotkey::default(),
            language: None,
        }
    }
}

impl Config {
    pub fn ui_lang(&self) -> Language {
        self.language.unwrap_or_else(Language::from_os)
    }

    pub fn load() -> Self {
        let Ok(path) = strict_config_path() else {
            return Self::default();
        };
        let (cfg, persist) = match fs::read_to_string(&path) {
            Ok(raw) => {
                let cfg = toml::from_str(&raw).unwrap_or_default();
                (cfg, !raw.contains("hotkey"))
            }
            Err(_) => (Self::default(), true),
        };
        if persist {
            cfg.save();
        }
        cfg
    }

    pub fn save(&self) {
        let Ok(path) = strict_config_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(raw) = toml::to_string_pretty(self) {
            let _ = fs::write(path, raw);
        }
    }
}

#[allow(dead_code)] // X11 prints the path at startup.
pub fn config_path() -> PathBuf {
    strict_config_path().unwrap_or_default()
}

/// Absolute `…/blackout/config.toml` from the OS profile. Never cwd.
pub fn strict_config_path() -> Result<PathBuf, String> {
    let path = profile_config_file()?;
    if !path.is_absolute() {
        return Err("путь конфига не абсолютный".into());
    }
    if path.file_name().and_then(|n| n.to_str()) != Some("config.toml") {
        return Err("неожиданный путь конфига".into());
    }
    let dir = path.parent().ok_or("нет папки конфига")?;
    if dir.file_name().and_then(|n| n.to_str()) != Some("blackout") {
        return Err("неожиданная папка конфига".into());
    }
    Ok(path)
}

#[allow(dead_code)]
pub fn remove_our_files() -> Result<(), String> {
    let path = strict_config_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|_| "не удалось удалить config.toml")?;
    }
    if let Some(dir) = path.parent() {
        if dir.is_dir() {
            let _ = fs::remove_dir(dir);
        }
    }
    Ok(())
}

fn profile_config_file() -> Result<PathBuf, String> {
    Ok(profile_config_dir()?.join("config.toml"))
}

fn profile_config_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        Ok(roaming_appdata()?.join("blackout"))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(user_home()?.join("Library/Application Support/blackout"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Ok(xdg_config_home()?.join("blackout"))
    }
}

#[cfg(windows)]
fn roaming_appdata() -> Result<PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
    };

    let from_api =
        unsafe { SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None).ok() };
    if let Some(pwstr) = from_api {
        let path = unsafe {
            let path = PathBuf::from(std::ffi::OsString::from_wide(pwstr.as_wide()));
            CoTaskMemFree(Some(pwstr.0 as *const _));
            path
        };
        if path.is_absolute() {
            return Ok(path);
        }
    }
    env_absolute("APPDATA").ok_or_else(|| "нет профиля AppData".into())
}

#[cfg(unix)]
pub fn user_home() -> Result<PathBuf, String> {
    if let Some(home) = passwd_home() {
        if home.is_absolute() {
            return Ok(home);
        }
    }
    env_absolute("HOME").ok_or_else(|| "нет профиля HOME".into())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn xdg_config_home() -> Result<PathBuf, String> {
    if let Some(dir) = env_absolute("XDG_CONFIG_HOME") {
        return Ok(dir);
    }
    Ok(user_home()?.join(".config"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn xdg_data_home() -> Result<PathBuf, String> {
    if let Some(dir) = env_absolute("XDG_DATA_HOME") {
        return Ok(dir);
    }
    Ok(user_home()?.join(".local/share"))
}

#[cfg(unix)]
fn passwd_home() -> Option<PathBuf> {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() {
            return None;
        }
        let dir = std::ffi::CStr::from_ptr((*pw).pw_dir);
        if dir.is_empty() {
            return None;
        }
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        Some(PathBuf::from(OsStr::from_bytes(dir.to_bytes())))
    }
}

fn env_absolute(key: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(key)?);
    path.is_absolute().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::env_absolute;
    use std::path::Path;

    #[test]
    fn env_absolute_rejects_relative() {
        std::env::set_var("BLACKOUT_TEST_REL", "not/absolute");
        assert!(env_absolute("BLACKOUT_TEST_REL").is_none());
        std::env::remove_var("BLACKOUT_TEST_REL");
    }

    #[test]
    fn env_absolute_keeps_abs() {
        let abs = if cfg!(windows) {
            r"C:\Users\Alex"
        } else {
            "/home/alex"
        };
        std::env::set_var("BLACKOUT_TEST_ABS", abs);
        assert_eq!(
            env_absolute("BLACKOUT_TEST_ABS").as_deref(),
            Some(Path::new(abs))
        );
        std::env::remove_var("BLACKOUT_TEST_ABS");
    }
}
