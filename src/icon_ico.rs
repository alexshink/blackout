//! Build-time ICO writer. Not linked into the app.
//!
//! Microsoft: 16/24/32/48/256 minimum; 256 as PNG (Vista+), smaller sizes as
//! 32-bit DIB. Extra 20/40/64 so HiDPI shell scales down, not up.

use std::io::{self, Write};
use std::path::Path;

use crate::icon::draw_monitor_icon;

/// DIB frames — cheap, crisp at title-bar / tray / Explorer sizes.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
const DIB_SIZES: [i32; 7] = [16, 20, 24, 32, 40, 48, 64];
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
const PNG_SIZE: i32 = 256;

#[allow(dead_code)] // build.rs
pub fn write_png(path: &Path, size: i32) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, png_image(size)?)
}

/// Modern ICNS: PNG payloads (10.8+). Finder/Dock see this only inside an `.app`.
pub fn write_icns(path: &Path) -> io::Result<()> {
    let chunks: [(&[u8; 4], i32); 6] = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"icp6", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
    ];
    let mut body = Vec::new();
    for (tag, size) in chunks {
        let png = png_image(size)?;
        let chunk_len = 8 + png.len();
        body.extend_from_slice(tag.as_slice());
        body.extend_from_slice(&(chunk_len as u32).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let total = 8 + body.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(total as u32).to_be_bytes());
    out.extend_from_slice(&body);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, out)
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub fn write_ico(path: &Path) -> io::Result<()> {
    let mut images: Vec<(i32, Vec<u8>)> = DIB_SIZES
        .iter()
        .copied()
        .map(|size| (size, dib_image(size)))
        .collect();
    images.push((PNG_SIZE, png_image(PNG_SIZE)?));

    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = 6 + 16 * images.len();
    for (size, data) in &images {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        out.push(dim);
        out.push(dim);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in &images {
        out.extend_from_slice(data);
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(&out)
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn dib_image(size: i32) -> Vec<u8> {
    let mut pixels = vec![0u32; (size * size) as usize];
    draw_monitor_icon(&mut pixels, size);

    let and_row = ((size + 31) / 32) * 4;
    let xor_size = (size * size * 4) as usize;
    let and_size = (and_row * size) as usize;
    let mut data = Vec::with_capacity(40 + xor_size + and_size);

    data.extend_from_slice(&40u32.to_le_bytes());
    data.extend_from_slice(&size.to_le_bytes());
    data.extend_from_slice(&(size * 2).to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(xor_size as u32).to_le_bytes());
    data.extend_from_slice(&[0u8; 16]);

    for y in (0..size).rev() {
        let start = (y * size) as usize;
        for px in &pixels[start..start + size as usize] {
            data.extend_from_slice(&px.to_le_bytes());
        }
    }
    data.resize(data.len() + and_size, 0);
    data
}

fn png_image(size: i32) -> io::Result<Vec<u8>> {
    let mut bgra = vec![0u32; (size * size) as usize];
    draw_monitor_icon(&mut bgra, size);

    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for px in bgra {
        let b = (px & 0xFF) as u8;
        let g = ((px >> 8) & 0xFF) as u8;
        let r = ((px >> 16) & 0xFF) as u8;
        let a = ((px >> 24) & 0xFF) as u8;
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, size as u32, size as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Best);
        let mut writer = encoder
            .write_header()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    }
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ico_uses_png_for_256() {
        let dir = std::env::temp_dir().join("blackout-icon-ico-test");
        let path = dir.join("blackout.ico");
        write_ico(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], &[0, 0, 1, 0]);
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        assert_eq!(count, DIB_SIZES.len() + 1);

        let mut found_png = false;
        for i in 0..count {
            let e = 6 + i * 16;
            let size = u32::from_le_bytes(bytes[e + 8..e + 12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(bytes[e + 12..e + 16].try_into().unwrap()) as usize;
            let w = bytes[e];
            if w == 0 {
                assert!(bytes[offset..].starts_with(b"\x89PNG\r\n\x1a\n"));
                assert!(
                    size < 16 * 1024,
                    "256 PNG should stay small, got {size} bytes"
                );
                found_png = true;
            } else {
                assert_eq!(&bytes[offset..offset + 4], &[40, 0, 0, 0]);
            }
        }
        assert!(found_png);
        assert!(
            bytes.len() < 80 * 1024,
            "ICO grew too much: {} bytes",
            bytes.len()
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn icns_header_and_png_chunk() {
        let dir = std::env::temp_dir().join("blackout-icon-icns-test");
        let path = dir.join("blackout.icns");
        write_icns(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"icns");
        let total = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(total, bytes.len());
        assert_eq!(&bytes[8..12], b"icp4");
        let first = u32::from_be_bytes(bytes[12..16].try_into().unwrap()) as usize;
        assert!(bytes[16..].starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(first > 16);
        let _ = std::fs::remove_file(path);
    }
}
