use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Portable combo stored in config.toml, e.g. `Ctrl+Alt+B`.
/// Alt = Option on macOS, Super = Win / Cmd.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: Key,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    F(u8),
    Space,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
            key: Key::Char('B'),
        }
    }
}

#[allow(dead_code)]
impl Hotkey {
    pub fn new(ctrl: bool, alt: bool, shift: bool, meta: bool, key: Key) -> Option<Self> {
        if !ctrl && !alt && !shift && !meta {
            return None;
        }
        Some(Self {
            ctrl,
            alt,
            shift,
            meta,
            key,
        })
    }

    pub fn has_modifier(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.meta
    }

    pub fn win_mods(&self) -> u32 {
        let mut mods = 0x4000; // MOD_NOREPEAT
        if self.ctrl {
            mods |= 0x0002; // MOD_CONTROL
        }
        if self.alt {
            mods |= 0x0001; // MOD_ALT
        }
        if self.shift {
            mods |= 0x0004; // MOD_SHIFT
        }
        if self.meta {
            mods |= 0x0008; // MOD_WIN
        }
        mods
    }

    pub fn win_vk(&self) -> u32 {
        match self.key {
            Key::Char(c) => (c.to_ascii_uppercase() as u32) & 0xFF,
            Key::Space => 0x20,
            Key::F(n) => 0x70 + u32::from(n.saturating_sub(1)),
        }
    }

    /// Carbon / kVK_ANSI_* (US). Letters and digits only; F-keys use the Mac F-key codes.
    pub fn mac_vk(&self) -> Option<u32> {
        Some(match self.key {
            Key::Space => 0x31,
            Key::F(1) => 0x7A,
            Key::F(2) => 0x78,
            Key::F(3) => 0x63,
            Key::F(4) => 0x76,
            Key::F(5) => 0x60,
            Key::F(6) => 0x61,
            Key::F(7) => 0x62,
            Key::F(8) => 0x64,
            Key::F(9) => 0x65,
            Key::F(10) => 0x6D,
            Key::F(11) => 0x67,
            Key::F(12) => 0x6F,
            Key::F(_) => return None,
            Key::Char(c) => match c.to_ascii_uppercase() {
                'A' => 0x00,
                'S' => 0x01,
                'D' => 0x02,
                'F' => 0x03,
                'H' => 0x04,
                'G' => 0x05,
                'Z' => 0x06,
                'X' => 0x07,
                'C' => 0x08,
                'V' => 0x09,
                'B' => 0x0B,
                'Q' => 0x0C,
                'W' => 0x0D,
                'E' => 0x0E,
                'R' => 0x0F,
                'Y' => 0x10,
                'T' => 0x11,
                '1' => 0x12,
                '2' => 0x13,
                '3' => 0x14,
                '4' => 0x15,
                '6' => 0x16,
                '5' => 0x17,
                '9' => 0x19,
                '7' => 0x1A,
                '8' => 0x1C,
                '0' => 0x1D,
                'O' => 0x1F,
                'U' => 0x20,
                'I' => 0x22,
                'P' => 0x23,
                'L' => 0x25,
                'J' => 0x26,
                'K' => 0x28,
                'N' => 0x2D,
                'M' => 0x2E,
                _ => return None,
            },
        })
    }

    pub fn mac_mods(&self) -> u32 {
        let mut mods = 0;
        if self.meta {
            mods |= 0x0100; // cmdKey
        }
        if self.shift {
            mods |= 0x0200; // shiftKey
        }
        if self.alt {
            mods |= 0x0800; // optionKey
        }
        if self.ctrl {
            mods |= 0x1000; // controlKey
        }
        mods
    }

    /// X11 keysym (latin-1 letter / XK_Fn / XK_space).
    pub fn x11_keysym(&self) -> u32 {
        match self.key {
            Key::Char(c) => c.to_ascii_lowercase() as u32,
            Key::Space => 0x0020,
            Key::F(n) => 0xFFBE + u32::from(n.saturating_sub(1)),
        }
    }

    /// X11 modifier mask without Lock / NumLock.
    pub fn x11_mod_mask(&self) -> u16 {
        let mut mask = 0u16;
        if self.shift {
            mask |= 1; // Shift
        }
        if self.ctrl {
            mask |= 4; // Control
        }
        if self.alt {
            mask |= 8; // Mod1
        }
        if self.meta {
            mask |= 64; // Mod4
        }
        mask
    }

    pub fn x11_mod_variants(&self) -> [u16; 4] {
        let base = self.x11_mod_mask();
        [base, base | 2, base | 16, base | 18] // ± Lock ± NumLock
    }
}

impl Display for Hotkey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format_combo(
            self.ctrl,
            self.alt,
            self.shift,
            self.meta,
            Some(self.key),
        ))
    }
}

pub fn format_combo(ctrl: bool, alt: bool, shift: bool, meta: bool, key: Option<Key>) -> String {
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl".into());
    }
    if alt {
        parts.push("Alt".into());
    }
    if shift {
        parts.push("Shift".into());
    }
    if meta {
        parts.push("Super".into());
    }
    if let Some(key) = key {
        parts.push(match key {
            Key::Char(c) => c.to_ascii_uppercase().to_string(),
            Key::F(n) => format!("F{n}"),
            Key::Space => "Space".into(),
        });
    }
    parts.join("+")
}

impl FromStr for Hotkey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut meta = false;
        let mut key = None;
        for raw in s.split('+') {
            let part = raw.trim();
            if part.is_empty() {
                continue;
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "controlkey" => ctrl = true,
                "alt" | "option" | "opt" => alt = true,
                "shift" => shift = true,
                "super" | "win" | "windows" | "cmd" | "command" | "meta" => meta = true,
                "space" => key = Some(Key::Space),
                other if other.len() >= 2 && other.as_bytes()[0].eq_ignore_ascii_case(&b'f') => {
                    let n: u8 = other[1..]
                        .parse()
                        .map_err(|_| format!("unknown key {part}"))?;
                    if !(1..=12).contains(&n) {
                        return Err(format!("unsupported key {part}"));
                    }
                    key = Some(Key::F(n));
                }
                other if other.chars().count() == 1 => {
                    let c = other.chars().next().unwrap();
                    if c.is_ascii_alphanumeric() {
                        key = Some(Key::Char(c.to_ascii_uppercase()));
                    } else {
                        return Err(format!("unsupported key {part}"));
                    }
                }
                _ => return Err(format!("unknown token {part}")),
            }
        }
        let key = key.ok_or_else(|| "hotkey needs a key".to_string())?;
        Self::new(ctrl, alt, shift, meta, key)
            .ok_or_else(|| "hotkey needs a modifier (Ctrl, Alt, Shift or Super)".to_string())
    }
}

impl Serialize for Hotkey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Hotkey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(windows)]
pub fn key_from_win_vk(vk: u32) -> Option<Key> {
    match vk {
        0x20 => Some(Key::Space),
        0x30..=0x39 | 0x41..=0x5A => Some(Key::Char(char::from_u32(vk)?)),
        0x70..=0x7B => Some(Key::F((vk - 0x6F) as u8)),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn key_from_mac_vk(code: u16) -> Option<Key> {
    Some(match code {
        0x31 => Key::Space,
        0x7A => Key::F(1),
        0x78 => Key::F(2),
        0x63 => Key::F(3),
        0x76 => Key::F(4),
        0x60 => Key::F(5),
        0x61 => Key::F(6),
        0x62 => Key::F(7),
        0x64 => Key::F(8),
        0x65 => Key::F(9),
        0x6D => Key::F(10),
        0x67 => Key::F(11),
        0x6F => Key::F(12),
        0x00 => Key::Char('A'),
        0x01 => Key::Char('S'),
        0x02 => Key::Char('D'),
        0x03 => Key::Char('F'),
        0x04 => Key::Char('H'),
        0x05 => Key::Char('G'),
        0x06 => Key::Char('Z'),
        0x07 => Key::Char('X'),
        0x08 => Key::Char('C'),
        0x09 => Key::Char('V'),
        0x0B => Key::Char('B'),
        0x0C => Key::Char('Q'),
        0x0D => Key::Char('W'),
        0x0E => Key::Char('E'),
        0x0F => Key::Char('R'),
        0x10 => Key::Char('Y'),
        0x11 => Key::Char('T'),
        0x12 => Key::Char('1'),
        0x13 => Key::Char('2'),
        0x14 => Key::Char('3'),
        0x15 => Key::Char('4'),
        0x16 => Key::Char('6'),
        0x17 => Key::Char('5'),
        0x19 => Key::Char('9'),
        0x1A => Key::Char('7'),
        0x1C => Key::Char('8'),
        0x1D => Key::Char('0'),
        0x1F => Key::Char('O'),
        0x20 => Key::Char('U'),
        0x22 => Key::Char('I'),
        0x23 => Key::Char('P'),
        0x25 => Key::Char('L'),
        0x26 => Key::Char('J'),
        0x28 => Key::Char('K'),
        0x2D => Key::Char('N'),
        0x2E => Key::Char('M'),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrip() {
        let hk = Hotkey::default();
        assert_eq!(hk.to_string(), "Ctrl+Alt+B");
        assert_eq!(hk.to_string().parse::<Hotkey>().unwrap(), hk);
    }

    #[test]
    fn parse_aliases() {
        let hk: Hotkey = "Control+Option+Shift+F9".parse().unwrap();
        assert!(hk.ctrl && hk.alt && hk.shift && !hk.meta);
        assert_eq!(hk.key, Key::F(9));
        let hk: Hotkey = "cmd+space".parse().unwrap();
        assert!(hk.meta && matches!(hk.key, Key::Space));
    }

    #[test]
    fn reject_bare_key() {
        assert!("B".parse::<Hotkey>().is_err());
    }
}
