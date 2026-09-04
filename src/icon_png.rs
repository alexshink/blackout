/// PNG frames from `build.rs`. Same monitor as the Windows ICO / tray draw.

pub const PNG_32: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/blackout-32.png"));
pub const PNG_48: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/blackout-48.png"));
pub const PNG_256: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/blackout-256.png"));
