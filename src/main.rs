#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(any(windows, target_os = "macos"))]
mod about;
mod app;
mod autostart;
mod config;
mod hotkey;
mod i18n;
#[cfg(any(windows, test))]
mod icon;
#[cfg(not(windows))]
mod icon_png;
#[cfg(test)]
mod icon_ico;
mod platform;
#[cfg(any(windows, target_os = "macos"))]
mod purge;

fn main() {
    if let Err(err) = platform::run() {
        eprintln!("blackout: {err}");
        std::process::exit(1);
    }
}
