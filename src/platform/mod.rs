#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::run;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::run;

#[cfg(all(unix, not(target_os = "macos")))]
mod x11;
#[cfg(all(unix, not(target_os = "macos")))]
pub use x11::run;
