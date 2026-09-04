pub const BORDER_R: u8 = 0x3D;
pub const BORDER_G: u8 = 0xDC;
pub const BORDER_B: u8 = 0x97;

/// BGRA, premultiplied — same layout as a Windows 32-bit DIB.
pub fn pixel(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let r = (u16::from(r) * u16::from(a) / 255) as u8;
    let g = (u16::from(g) * u16::from(a) / 255) as u8;
    let b = (u16::from(b) * u16::from(a) / 255) as u8;
    u32::from(b) | (u32::from(g) << 8) | (u32::from(r) << 16) | (u32::from(a) << 24)
}

pub fn draw_monitor_icon(pixels: &mut [u32], size: i32) {
    let green = pixel(BORDER_R, BORDER_G, BORDER_B, 255);
    let clear = pixel(0, 0, 0, 0);
    pixels.fill(clear);

    let stroke = if size >= 24 { 2 } else { 1 };
    let pad = if size >= 20 { 2 } else { 1 };
    let stand_h = (size / 4).max(4);
    let mut body_w = size - pad * 2;
    if body_w % 2 != size % 2 {
        body_w -= 1;
    }
    let body_h = ((body_w * 3) / 4).min(size - pad - stand_h);
    let x0 = (size - body_w) / 2;
    let x1 = x0 + body_w;
    let y0 = ((size - stand_h - body_h) / 2).max(0);
    let y1 = y0 + body_h;

    let plot = |pixels: &mut [u32], x: i32, y: i32| {
        if x >= 0 && y >= 0 && x < size && y < size {
            pixels[(y * size + x) as usize] = green;
        }
    };

    for x in x0 + 1..x1 - 1 {
        for t in 0..stroke {
            plot(pixels, x, y0 + t);
            plot(pixels, x, y1 - 1 - t);
        }
    }
    for y in y0 + 1..y1 - 1 {
        for t in 0..stroke {
            plot(pixels, x0 + t, y);
            plot(pixels, x1 - 1 - t, y);
        }
    }

    let left = x0;
    let right = x1 - 1;
    let mid_l = (left + right) / 2;
    let mid_r = (left + right + 1) / 2;
    let neck_h = (stand_h / 2).max(1);
    let half = ((body_w / 5).max(2)).max(stroke);
    let base_h = stroke;
    let neck_y0 = y1;
    let neck_y1 = (neck_y0 + neck_h).min(size - base_h);
    for y in neck_y0..neck_y1 {
        plot(pixels, mid_l, y);
        plot(pixels, mid_r, y);
    }
    let base_y0 = neck_y1.min(size - base_h);
    for y in base_y0..(base_y0 + base_h).min(size) {
        for x in (mid_l - half)..=(mid_r + half) {
            plot(pixels, x, y);
        }
    }
}
