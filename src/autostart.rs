use std::path::{Path, PathBuf};

pub fn is_enabled() -> bool {
    platform::is_enabled()
}

pub fn set_enabled(on: bool) -> Result<(), String> {
    platform::set_enabled(on)
}

/// Remove autostart only if the stored command is this binary. Foreign entries stay.
#[allow(dead_code)]
pub fn remove_if_ours() -> Result<(), String> {
    platform::remove_if_ours()
}

/// If autostart is on, rewrite the command to the current binary (moved / rebuilt).
pub fn refresh_path() {
    if is_enabled() {
        let _ = set_enabled(true);
    }
}

fn exe_path() -> Result<PathBuf, String> {
    let path = std::env::current_exe().map_err(|_| "не удалось определить путь программы")?;
    Ok(strip_verbatim(path))
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn unquote(raw: &str) -> &str {
    let s = raw.trim();
    s.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(s)
}

#[cfg_attr(not(test), allow(dead_code))]
fn same_program(stored: &str, ours: &Path) -> bool {
    let a = strip_verbatim(PathBuf::from(unquote(stored)));
    let b = strip_verbatim(ours.to_path_buf());
    #[cfg(windows)]
    {
        let a = a.to_string_lossy().replace('/', "\\");
        let b = b.to_string_lossy().replace('/', "\\");
        a.eq_ignore_ascii_case(&b)
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::{same_program, unquote};
    use std::path::Path;

    #[test]
    fn unquote_strips_quotes() {
        assert_eq!(
            unquote(r#""C:\Apps\blackout.exe""#),
            r"C:\Apps\blackout.exe"
        );
        assert_eq!(unquote("  /opt/blackout  "), "/opt/blackout");
    }

    #[test]
    fn same_program_quoted() {
        let ours = Path::new("/opt/blackout");
        assert!(same_program("\"/opt/blackout\"", ours));
        assert!(!same_program("/opt/other", ours));
    }

    #[cfg(windows)]
    #[test]
    fn same_program_windows_slash_case() {
        let ours = Path::new(r"C:\Apps\blackout.exe");
        assert!(same_program(r#""C:\Apps\blackout.exe""#, ours));
        assert!(same_program(r"C:/Apps/blackout.exe", ours));
        assert!(!same_program(r#""C:\Other\blackout.exe""#, ours));
    }
}

#[cfg(windows)]
mod platform {
    use super::exe_path;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_BINARY,
        REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
    };

    const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    const APPROVED: PCWSTR =
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");
    const VALUE: PCWSTR = w!("Blackout");

    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }

    fn open(subkey: PCWSTR, write: bool) -> Result<Key, String> {
        let mut hkey = HKEY(std::ptr::null_mut());
        let access = if write {
            KEY_QUERY_VALUE | KEY_SET_VALUE
        } else {
            KEY_QUERY_VALUE
        };
        let err = unsafe {
            if write {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    subkey,
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    access,
                    None,
                    &mut hkey,
                    None,
                )
            } else {
                RegOpenKeyExW(HKEY_CURRENT_USER, subkey, None, access, &mut hkey)
            }
        };
        if err.is_ok() {
            Ok(Key(hkey))
        } else if !write && (err == ERROR_FILE_NOT_FOUND || err == ERROR_PATH_NOT_FOUND) {
            Err(String::new())
        } else {
            Err("не удалось открыть ключ автозагрузки".into())
        }
    }

    fn command() -> Result<Vec<u16>, String> {
        let path = exe_path()?;
        let s = path.to_string_lossy();
        let quoted = format!("\"{s}\"");
        Ok(quoted.encode_utf16().chain(std::iter::once(0)).collect())
    }

    fn has_run_value(key: HKEY) -> bool {
        let mut kind = REG_VALUE_TYPE::default();
        let mut size = 0u32;
        unsafe { RegQueryValueExW(key, VALUE, None, Some(&mut kind), None, Some(&mut size)) }
            .is_ok()
    }

    fn approved_allows(key: HKEY) -> bool {
        let mut kind = REG_VALUE_TYPE::default();
        let mut buf = [0u8; 12];
        let mut size = buf.len() as u32;
        let err = unsafe {
            RegQueryValueExW(
                key,
                VALUE,
                None,
                Some(&mut kind),
                Some(buf.as_mut_ptr()),
                Some(&mut size),
            )
        };
        if !err.is_ok() {
            return true;
        }
        buf[0] & 1 == 0
    }

    pub fn is_enabled() -> bool {
        let Ok(run) = open(RUN, false) else {
            return false;
        };
        if !has_run_value(run.0) {
            return false;
        }
        match open(APPROVED, false) {
            Ok(approved) => approved_allows(approved.0),
            Err(_) => true,
        }
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        if on {
            enable()
        } else {
            disable()
        }
    }

    fn enable() -> Result<(), String> {
        let run = open(RUN, true)?;
        let cmd = command()?;
        let bytes = unsafe { std::slice::from_raw_parts(cmd.as_ptr() as *const u8, cmd.len() * 2) };
        if unsafe { RegSetValueExW(run.0, VALUE, None, REG_SZ, Some(bytes)) }.is_err() {
            return Err("не удалось записать автозагрузку".into());
        }
        let approved = open(APPROVED, true)?;
        let enabled = [2u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let _ = unsafe { RegSetValueExW(approved.0, VALUE, None, REG_BINARY, Some(&enabled)) };
        Ok(())
    }

    fn disable() -> Result<(), String> {
        delete_value(RUN)?;
        let _ = delete_value(APPROVED);
        Ok(())
    }

    fn delete_value(subkey: PCWSTR) -> Result<(), String> {
        let Ok(key) = open(subkey, true) else {
            return Ok(());
        };
        let err = unsafe { RegDeleteValueW(key.0, VALUE) };
        if err.is_ok() || err == ERROR_FILE_NOT_FOUND || err == ERROR_PATH_NOT_FOUND {
            Ok(())
        } else {
            Err("не удалось снять автозагрузку".into())
        }
    }

    fn read_run_command() -> Option<String> {
        let key = open(RUN, false).ok()?;
        let mut kind = REG_VALUE_TYPE::default();
        let mut size = 0u32;
        if unsafe { RegQueryValueExW(key.0, VALUE, None, Some(&mut kind), None, Some(&mut size)) }
            .is_err()
            || size == 0
        {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if unsafe {
            RegQueryValueExW(
                key.0,
                VALUE,
                None,
                Some(&mut kind),
                Some(buf.as_mut_ptr()),
                Some(&mut size),
            )
        }
        .is_err()
        {
            return None;
        }
        let words: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let end = words.iter().position(|&c| c == 0).unwrap_or(words.len());
        Some(String::from_utf16_lossy(&words[..end]))
    }

    pub fn remove_if_ours() -> Result<(), String> {
        let ours = exe_path()?;
        match read_run_command() {
            None => Ok(()),
            Some(stored) if super::same_program(&stored, &ours) => disable(),
            Some(_) => Ok(()),
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{exe_path, xml_escape};
    use std::fs;
    use std::path::PathBuf;

    fn plist_path() -> Result<PathBuf, String> {
        Ok(crate::config::user_home()?.join("Library/LaunchAgents/local.blackout.plist"))
    }

    pub fn is_enabled() -> bool {
        plist_path().map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        let path = plist_path()?;
        if on {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|_| "не удалось создать LaunchAgents")?;
            }
            let exe = exe_path()?;
            let exe = xml_escape(&exe.to_string_lossy());
            let body = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>local.blackout</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
</dict>
</plist>
"#
            );
            fs::write(&path, body).map_err(|_| "не удалось записать LaunchAgent")?;
        } else if path.exists() {
            fs::remove_file(&path).map_err(|_| "не удалось удалить LaunchAgent")?;
        }
        Ok(())
    }

    pub fn remove_if_ours() -> Result<(), String> {
        let path = plist_path()?;
        if !path.is_absolute() {
            return Err("путь LaunchAgent не абсолютный".into());
        }
        if !path.is_file() {
            return Ok(());
        }
        let ours = exe_path()?;
        let body = fs::read_to_string(&path).map_err(|_| "не удалось прочитать LaunchAgent")?;
        if !plist_points_to(&body, &ours) {
            return Ok(());
        }
        fs::remove_file(&path).map_err(|_| "не удалось удалить LaunchAgent")
    }

    fn plist_points_to(body: &str, ours: &std::path::Path) -> bool {
        let Some(start) = body.find("<key>ProgramArguments</key>") else {
            return false;
        };
        let rest = &body[start..];
        let Some(array) = rest.find("<array>") else {
            return false;
        };
        let rest = &rest[array..];
        let Some(end) = rest.find("</array>") else {
            return false;
        };
        let block = &rest[..end];
        let mut search = block;
        while let Some(open) = search.find("<string>") {
            let rest = &search[open + 8..];
            let Some(close) = rest.find("</string>") else {
                break;
            };
            let value = super::xml_unescape(&rest[..close]);
            if super::same_program(&value, ours) {
                return true;
            }
            search = &rest[close + 9..];
        }
        false
    }
}

#[cfg(target_os = "macos")]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(target_os = "macos")]
fn xml_unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use super::exe_path;
    use std::fs;
    use std::path::PathBuf;

    fn desktop_path() -> Result<PathBuf, String> {
        Ok(crate::config::xdg_config_home()?.join("autostart/blackout.desktop"))
    }

    pub fn is_enabled() -> bool {
        desktop_path().map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        let path = desktop_path()?;
        if on {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|_| "не удалось создать autostart")?;
            }
            let exe = exe_path()?;
            let exec = desktop_exec(&exe.to_string_lossy());
            let icon = install_icons()?;
            let body = format!(
                "\
[Desktop Entry]
Type=Application
Name=Blackout
Comment=Cinema blackout for other monitors
Exec={exec}
Icon={icon}
Terminal=false
X-GNOME-Autostart-enabled=true
"
            );
            fs::write(&path, body).map_err(|_| "не удалось записать .desktop")?;
        } else if path.exists() {
            fs::remove_file(&path).map_err(|_| "не удалось удалить .desktop")?;
            remove_icons();
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_if_ours() -> Result<(), String> {
        remove_icons();
        let path = desktop_path()?;
        if !path.is_absolute() {
            return Err("путь autostart не абсолютный".into());
        }
        if !path.is_file() {
            return Ok(());
        }
        let ours = exe_path()?;
        let body = fs::read_to_string(&path).map_err(|_| "не удалось прочитать .desktop")?;
        let Some(exec) = exec_from_desktop(&body) else {
            return Ok(());
        };
        if !super::same_program(&exec, &ours) {
            return Ok(());
        }
        fs::remove_file(&path).map_err(|_| "не удалось удалить .desktop".to_string())
    }

    fn icon_files() -> Result<Vec<(PathBuf, &'static [u8])>, String> {
        let root = crate::config::xdg_data_home()?.join("icons/hicolor");
        Ok(vec![
            (root.join("32x32/apps/blackout.png"), crate::icon_png::PNG_32),
            (root.join("48x48/apps/blackout.png"), crate::icon_png::PNG_48),
            (
                root.join("256x256/apps/blackout.png"),
                crate::icon_png::PNG_256,
            ),
        ])
    }

    fn install_icons() -> Result<String, String> {
        let files = icon_files()?;
        for (path, bytes) in &files {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|_| "не удалось создать папку иконок")?;
            }
            fs::write(path, bytes).map_err(|_| "не удалось записать иконку")?;
        }
        files
            .last()
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .ok_or_else(|| "нет иконки".into())
    }

    fn remove_icons() {
        if let Ok(files) = icon_files() {
            for (path, _) in files {
                let _ = fs::remove_file(path);
            }
        }
    }

    #[allow(dead_code)]
    fn exec_from_desktop(body: &str) -> Option<String> {
        for line in body.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("Exec=") {
                return Some(unquote_exec(rest));
            }
        }
        None
    }

    #[allow(dead_code)]
    fn unquote_exec(raw: &str) -> String {
        let s = raw.trim();
        if let Some(inner) = s.strip_prefix('"') {
            let mut out = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                } else if c == '"' {
                    break;
                } else {
                    out.push(c);
                }
            }
            out
        } else {
            s.split_whitespace().next().unwrap_or(s).to_string()
        }
    }

    fn desktop_exec(path: &str) -> String {
        if path
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '"' | '\\'))
        {
            format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""))
        } else {
            path.to_string()
        }
    }
}
