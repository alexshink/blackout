pub const NAME: &str = "Blackout";
pub const AUTHOR: &str = "Alex Shink";
pub const LICENSE: &str = "MIT";
pub const GITHUB: &str = "https://github.com/alexshink/blackout";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn open_github() {
    #[cfg(windows)]
    {
        open_windows(GITHUB);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(GITHUB).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(GITHUB).spawn();
    }
}

#[cfg(windows)]
fn open_windows(url: &str) {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = std::ffi::OsStr::new(url)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR(wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}
