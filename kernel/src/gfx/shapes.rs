use super::{Color, Framebuffer};

pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, c: Color) {
    if w <= 0 || h <= 0 { return; }
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = ((x + w).max(0) as usize).min(fb.width);
    let y1 = ((y + h).max(0) as usize).min(fb.height);
    for py in y0..y1 {
        for px in x0..x1 {
            if c.a == 255 { fb.put_pixel(px, py, c); }
            else { fb.blend_pixel(px, py, c); }
        }
    }
}

pub fn vertical_gradient(fb: &mut Framebuffer, top: Color, mid: Color, bottom: Color) {
    let h = fb.height.max(1);
    for y in 0..fb.height {
        let p = (y * 510) / h;
        let row = if p <= 255 {
            Color::lerp(top, mid, p as u8)
        } else {
            Color::lerp(mid, bottom, (p - 255) as u8)
        };
        for x in 0..fb.width {
            fb.put_pixel(x, y, row);
        }
    }
}

pub fn fill_rounded_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: i32, c: Color) {
    if w <= 0 || h <= 0 { return; }
    let r = r.max(0).min(w / 2).min(h / 2);
    let x0 = x.max(0) as i32;
    let y0 = y.max(0) as i32;
    let x1 = (x + w).min(fb.width as i32);
    let y1 = (y + h).min(fb.height as i32);

    for py in y0..y1 {
        for px in x0..x1 {
            let cov = rounded_coverage(px - x, py - y, w, h, r);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.with_alpha(((c.a as u16 * cov as u16) / 255) as u8) };
            fb.blend_pixel(px as usize, py as usize, pixel);
        }
    }
}

fn rounded_coverage(lx: i32, ly: i32, w: i32, h: i32, r: i32) -> u8 {
    let in_x_band = lx >= r && lx < w - r;
    let in_y_band = ly >= r && ly < h - r;
    if in_x_band || in_y_band { return 255; }

    let cx = if lx < r { r } else { w - 1 - r };
    let cy = if ly < r { r } else { h - 1 - r };
    circle_coverage(lx, ly, cx, cy, r)
}

pub fn stroke_circle(fb: &mut Framebuffer, cx: i32, cy: i32, r: i32, thickness: i32, c: Color) {
    if r <= 0 || thickness <= 0 { return; }
    let inner = (r - thickness / 2).max(0) as i64;
    let outer = (r + (thickness + 1) / 2) as i64;
    let inner_sq = inner * inner;
    let outer_sq = outer * outer;
    let outer_i = outer as i32;
    for py in (cy - outer_i - 1).max(0)..(cy + outer_i + 1).min(fb.height as i32) {
        for px in (cx - outer_i - 1).max(0)..(cx + outer_i + 1).min(fb.width as i32) {
            let cov = ring_coverage(px, py, cx, cy, inner, outer, inner_sq, outer_sq);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.with_alpha(((c.a as u16 * cov as u16) / 255) as u8) };
            fb.blend_pixel(px as usize, py as usize, pixel);
        }
    }
}

fn ring_coverage(px: i32, py: i32, cx: i32, cy: i32, inner: i64, outer: i64, inner_sq: i64, outer_sq: i64) -> u8 {
    const SAMPLES: i32 = 4;
    const STEP: i32 = 256 / SAMPLES;
    let dx_c = (px - cx) as i64;
    let dy_c = (py - cy) as i64;
    let coarse = dx_c * dx_c + dy_c * dy_c;
    if coarse > outer_sq + 2 * outer + 1 { return 0; }
    if inner > 1 && coarse + 2 * inner + 1 < inner_sq { return 0; }

    let mut hit = 0u32;
    let inner_s = inner_sq * 65536;
    let outer_s = outer_sq * 65536;
    for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
            let dx = dx_c * 256 + sx as i64 * STEP as i64 + STEP as i64 / 2 - 128;
            let dy = dy_c * 256 + sy as i64 * STEP as i64 + STEP as i64 / 2 - 128;
            let d_sq = dx * dx + dy * dy;
            if d_sq >= inner_s && d_sq < outer_s { hit += 1; }
        }
    }
    ((hit * 255) / (SAMPLES * SAMPLES) as u32) as u8
}

pub fn fill_ellipse(fb: &mut Framebuffer, cx: i32, cy: i32, rx: i32, ry: i32, c: Color) {
    if rx <= 0 || ry <= 0 { return; }
    let rx2 = rx as i64;
    let ry2 = ry as i64;
    let denom = rx2 * rx2 * ry2 * ry2;
    for py in (cy - ry - 1).max(0)..(cy + ry + 1).min(fb.height as i32) {
        for px in (cx - rx - 1).max(0)..(cx + rx + 1).min(fb.width as i32) {
            let cov = ellipse_coverage(px, py, cx, cy, rx as i64, ry as i64, denom);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.with_alpha(((c.a as u16 * cov as u16) / 255) as u8) };
            fb.blend_pixel(px as usize, py as usize, pixel);
        }
    }
}

fn ellipse_coverage(px: i32, py: i32, cx: i32, cy: i32, rx: i64, ry: i64, denom: i64) -> u8 {
    const SAMPLES: i32 = 4;
    const STEP: i32 = 256 / SAMPLES;
    let dx_c = (px - cx) as i64;
    let dy_c = (py - cy) as i64;
    let dxc2 = dx_c * dx_c * ry * ry;
    let dyc2 = dy_c * dy_c * rx * rx;
    let coarse = dxc2 + dyc2;

    let outer_lim = denom + 2 * rx * ry * (rx + ry);
    if coarse > outer_lim { return 0; }
    let inner_lim = denom.saturating_sub(2 * rx * ry * (rx + ry));
    if coarse < inner_lim { return 255; }

    let mut hit = 0u32;
    let total = (SAMPLES * SAMPLES) as u32;
    for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
            let dx = (px - cx) as i64 * 256 + sx as i64 * STEP as i64 + STEP as i64 / 2 - 128;
            let dy = (py - cy) as i64 * 256 + sy as i64 * STEP as i64 + STEP as i64 / 2 - 128;
            let lhs = dx * dx * ry * ry + dy * dy * rx * rx;
            let rhs = denom * 256 * 256;
            if lhs < rhs { hit += 1; }
        }
    }
    ((hit * 255) / total) as u8
}

pub fn fill_circle(fb: &mut Framebuffer, cx: i32, cy: i32, r: i32, c: Color) {
    if r <= 0 { return; }
    let r2 = r + 1;
    for py in (cy - r2).max(0)..(cy + r2).min(fb.height as i32) {
        for px in (cx - r2).max(0)..(cx + r2).min(fb.width as i32) {
            let cov = circle_coverage(px, py, cx, cy, r);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.with_alpha(((c.a as u16 * cov as u16) / 255) as u8) };
            fb.blend_pixel(px as usize, py as usize, pixel);
        }
    }
}

fn circle_coverage(px: i32, py: i32, cx: i32, cy: i32, r: i32) -> u8 {
    const SAMPLES: i32 = 4;
    const STEP: i32 = 256 / SAMPLES;
    let mut hit = 0u32;
    let total = (SAMPLES * SAMPLES) as u32;
    let r2_lo = ((r as i64 - 1) * (r as i64 - 1)).max(0);
    let r2_hi = ((r as i64 + 1) * (r as i64 + 1)) as i64;

    let dx_c = (px - cx) as i64;
    let dy_c = (py - cy) as i64;
    let coarse = dx_c * dx_c + dy_c * dy_c;
    if coarse >= r2_hi { return 0; }
    if coarse <= r2_lo { return 255; }

    let r256 = (r as i64) * 256;
    let r2_256 = r256 * r256;
    for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
            let dx = (px - cx) as i64 * 256 + sx as i64 * STEP as i64 + STEP as i64 / 2 - 128;
            let dy = (py - cy) as i64 * 256 + sy as i64 * STEP as i64 + STEP as i64 / 2 - 128;
            if dx * dx + dy * dy < r2_256 { hit += 1; }
        }
    }
    ((hit * 255) / total) as u8
}

#[allow(dead_code)]
pub fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        if x >= 0 && y >= 0 && (x as usize) < fb.width && (y as usize) < fb.height {
            if c.a == 255 { fb.put_pixel(x as usize, y as usize, c); }
            else { fb.blend_pixel(x as usize, y as usize, c); }
        }
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}

pub fn drop_shadow_stub(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: i32) {
    let layers = [
        (14, Color::rgba(20, 40, 80, 14)),
        (9,  Color::rgba(20, 40, 80, 22)),
        (5,  Color::rgba(20, 40, 80, 32)),
        (2,  Color::rgba(20, 40, 80, 40)),
    ];
    for (offset, color) in layers {
        fill_rounded_rect(fb, x - offset, y - offset + offset * 2, w + offset * 2, h + offset * 2, r + offset, color);
    }
}

pub fn radial_glow(fb: &mut Framebuffer, cx: i32, cy: i32, r: i32, inner: Color, outer_alpha: u8) {
    if r <= 0 { return; }
    let r64 = r as i64;
    let r_sq = r64 * r64;
    for py in (cy - r).max(0)..(cy + r).min(fb.height as i32) {
        for px in (cx - r).max(0)..(cx + r).min(fb.width as i32) {
            let dx = (px - cx) as i64;
            let dy = (py - cy) as i64;
            let d_sq = dx * dx + dy * dy;
            if d_sq >= r_sq { continue; }
            let t = ((d_sq * 255) / r_sq) as u32;
            let alpha = (inner.a as u32 * (255 - t) / 255 + outer_alpha as u32 * t / 255) as u8;
            fb.blend_pixel(px as usize, py as usize, inner.with_alpha(alpha));
        }
    }
}

pub fn cloud(fb: &mut Framebuffer, cx: i32, cy: i32, scale: i32, color: Color) {
    let bumps = [
        ( 0,  0, 16, 255),
        (-12, 2, 12, 240),
        ( 12, 2, 12, 240),
        (-22, 4, 9,  210),
        ( 22, 4, 9,  210),
        (-6, -8, 11, 230),
        ( 7, -7, 12, 235),
    ];
    for (dx, dy, r, a) in bumps {
        let alpha = ((color.a as u32 * a as u32) / 255) as u8;
        let radius = (r * scale + 5) / 10;
        fill_circle(fb, cx + dx * scale / 10, cy + dy * scale / 10, radius, color.with_alpha(alpha));
    }
}

pub fn glass_panel(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: i32, fill: Color) {
    drop_shadow_stub(fb, x, y, w, h, r);
    fill_rounded_rect(fb, x, y, w, h, r, fill);

    let inset = r.max(2);
    for i in 0..3 {
        let alpha = (130 - i * 35) as u8;
        fill_rect(fb, x + inset, y + 1 + i, w - inset * 2, 1, Color::rgba(255, 255, 255, alpha));
    }

    fill_rect(fb, x + inset, y + h - 2, w - inset * 2, 1, Color::rgba(255, 255, 255, 35));
    stroke_rounded_rect(fb, x, y, w, h, r, Color::rgba(255, 255, 255, 70));
}

pub fn stroke_rounded_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: i32, c: Color) {
    if w <= 0 || h <= 0 { return; }
    let r = r.max(0).min(w / 2).min(h / 2);
    let x0 = x.max(0) as i32;
    let y0 = y.max(0) as i32;
    let x1 = (x + w).min(fb.width as i32);
    let y1 = (y + h).min(fb.height as i32);

    for py in y0..y1 {
        for px in x0..x1 {
            let cov_in = rounded_coverage_signed(px - x, py - y, w, h, r, 0);
            let cov_out = rounded_coverage_signed(px - x, py - y, w, h, r, 1);
            let edge = cov_out.saturating_sub(cov_in);
            if edge == 0 { continue; }
            let alpha = ((c.a as u32 * edge as u32) / 255) as u8;
            fb.blend_pixel(px as usize, py as usize, c.with_alpha(alpha));
        }
    }
}

fn rounded_coverage_signed(lx: i32, ly: i32, w: i32, h: i32, r: i32, shrink: i32) -> u8 {
    let inner_w = w - shrink * 2;
    let inner_h = h - shrink * 2;
    let inner_r = (r - shrink).max(0);
    let llx = lx - shrink;
    let lly = ly - shrink;
    if llx < 0 || lly < 0 || llx >= inner_w || lly >= inner_h { return 0; }
    rounded_coverage(llx, lly, inner_w, inner_h, inner_r)
}
