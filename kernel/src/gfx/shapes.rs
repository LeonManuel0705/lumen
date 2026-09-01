use super::color;
use super::{Canvas, Color};

pub fn fill_rect<C: Canvas>(fb: &mut C, x: i32, y: i32, w: i32, h: i32, c: Color) {
    if w <= 0 || h <= 0 { return; }
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = ((x + w).max(0) as usize).min(fb.width());
    let y1 = ((y + h).max(0) as usize).min(fb.height());
    if x1 <= x0 { return; }
    for py in y0..y1 {
        fb.fill_row(x0, py, x1 - x0, c);
    }
}

/// How far in from the edge a row of a rounded rect is still inside the corner
/// arc. Rows between the arcs are straight and can be filled in one run. The
/// answer is rounded outwards by a pixel so the run it describes is strictly
/// inside the shape, leaving the boundary column to the antialiased path.
pub fn corner_inset(ly: i32, h: i32, r: i32) -> i32 {
    if r <= 0 {
        return 0;
    }
    let dy = if ly < r {
        r - ly
    } else if ly >= h - r {
        ly - (h - r - 1)
    } else {
        return 0;
    };
    if dy >= r {
        return r;
    }
    let mut inset = 0;
    while inset < r {
        let dx = r - inset;
        if dx * dx + dy * dy <= r * r {
            break;
        }
        inset += 1;
    }
    (inset + 1).min(r)
}

pub fn vertical_gradient<C: Canvas>(fb: &mut C, top: Color, mid: Color, bottom: Color) {
    let h = fb.height().max(1);
    let w = fb.width();
    for y in 0..fb.height() {
        let p = (y * 510) / h;
        let row = if p <= 255 {
            Color::lerp(top, mid, p as u8)
        } else {
            Color::lerp(mid, bottom, (p - 255) as u8)
        };
        fb.fill_row(0, y, w, row);
    }
}

pub fn fill_rounded_rect<C: Canvas>(fb: &mut C, x: i32, y: i32, w: i32, h: i32, r: i32, c: Color) {
    if w <= 0 || h <= 0 { return; }
    let r = r.max(0).min(w / 2).min(h / 2);
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(fb.width() as i32);
    let y1 = (y + h).min(fb.height() as i32);

    for py in y0..y1 {
        let ly = py - y;
        let (run_start, run_end) = if ly >= r && ly < h - r {
            (x0, x1)
        } else {
            let inset = corner_inset(ly, h, r);
            ((x + inset).max(x0), (x + w - inset).min(x1))
        };
        for px in x0..run_start {
            let cov = rounded_coverage(px - x, py - y, w, h, r);
            if cov == 0 { continue; }
            fb.blend_pixel(px as usize, py as usize, c.fade(cov));
        }
        if run_end > run_start {
            fb.fill_row(run_start as usize, py as usize, (run_end - run_start) as usize, c);
        }
        for px in run_end.max(run_start)..x1 {
            let cov = rounded_coverage(px - x, py - y, w, h, r);
            if cov == 0 { continue; }
            fb.blend_pixel(px as usize, py as usize, c.fade(cov));
        }
    }
}

pub fn rounded_coverage(lx: i32, ly: i32, w: i32, h: i32, r: i32) -> u8 {
    let in_x_band = lx >= r && lx < w - r;
    let in_y_band = ly >= r && ly < h - r;
    if in_x_band || in_y_band { return 255; }

    let cx = if lx < r { r } else { w - 1 - r };
    let cy = if ly < r { r } else { h - 1 - r };
    circle_coverage(lx, ly, cx, cy, r)
}

pub fn stroke_circle<C: Canvas>(fb: &mut C, cx: i32, cy: i32, r: i32, thickness: i32, c: Color) {
    if r <= 0 || thickness <= 0 { return; }
    let inner = (r - thickness / 2).max(0) as i64;
    let outer = (r + (thickness + 1) / 2) as i64;
    let inner_sq = inner * inner;
    let outer_sq = outer * outer;
    let outer_i = outer as i32;
    for py in (cy - outer_i - 1).max(0)..(cy + outer_i + 1).min(fb.height() as i32) {
        for px in (cx - outer_i - 1).max(0)..(cx + outer_i + 1).min(fb.width() as i32) {
            let cov = ring_coverage(px, py, cx, cy, inner, outer, inner_sq, outer_sq);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.fade(cov) };
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

pub fn fill_ellipse<C: Canvas>(fb: &mut C, cx: i32, cy: i32, rx: i32, ry: i32, c: Color) {
    if rx <= 0 || ry <= 0 { return; }
    let rx2 = rx as i64;
    let ry2 = ry as i64;
    let denom = rx2 * rx2 * ry2 * ry2;
    for py in (cy - ry - 1).max(0)..(cy + ry + 1).min(fb.height() as i32) {
        for px in (cx - rx - 1).max(0)..(cx + rx + 1).min(fb.width() as i32) {
            let cov = ellipse_coverage(px, py, cx, cy, rx as i64, ry as i64, denom);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.fade(cov) };
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

pub fn fill_circle<C: Canvas>(fb: &mut C, cx: i32, cy: i32, r: i32, c: Color) {
    if r <= 0 { return; }
    let r2 = r + 1;
    for py in (cy - r2).max(0)..(cy + r2).min(fb.height() as i32) {
        for px in (cx - r2).max(0)..(cx + r2).min(fb.width() as i32) {
            let cov = circle_coverage(px, py, cx, cy, r);
            if cov == 0 { continue; }
            let pixel = if cov == 255 { c } else { c.fade(cov) };
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
pub fn draw_line<C: Canvas>(fb: &mut C, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        if x >= 0 && y >= 0 && (x as usize) < fb.width() && (y as usize) < fb.height() {
            fb.paint(x as usize, y as usize, c);
        }
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}

/// A soft shadow around a rounded rect, drawn only where the panel will not
/// cover it. The falloff is separable, one weight per column times one per row,
/// which means the band directly above and below the panel has a single alpha
/// for the whole run and can be filled without touching pixels one at a time.
pub fn drop_shadow<C: Canvas>(fb: &mut C, x: i32, y: i32, w: i32, h: i32, r: i32) {
    const SPREAD: i32 = 24;
    const DROP: i32 = 8;
    const PEAK: u8 = 62;
    if w <= 0 || h <= 0 {
        return;
    }

    let sy = y + DROP;
    let x0 = (x - SPREAD).max(0);
    let y0 = (sy - SPREAD).max(0);
    let x1 = (x + w + SPREAD).min(fb.width() as i32);
    let y1 = (sy + h + SPREAD).min(fb.height() as i32);
    let shade = |fx: u8, fy: u8| -> Color {
        let weight = color::mul255(fx, fy);
        Color::rgba(30, 60, 100, color::mul255(color::mul255(weight, weight), PEAK))
    };

    for py in y0..y1 {
        let fy = falloff(py, sy, h, SPREAD);
        if fy == 0 {
            continue;
        }
        // Rows above and below the panel borrow the width of the nearest row
        // of the shape, so the shadow keeps the panel's rounded ends.
        let ly = (py - y).clamp(0, h - 1);
        let inset = corner_inset(ly, h, r);
        let span_start = x + inset;
        let span_len = w - inset * 2;
        let mid_start = span_start.max(x0);
        let mid_end = (span_start + span_len).min(x1);

        for px in x0..mid_start {
            let c = shade(falloff(px, span_start, span_len, SPREAD), fy);
            if c.a > 0 {
                fb.blend_pixel(px as usize, py as usize, c);
            }
        }

        // Whatever the panel itself covers is not worth shading.
        if mid_end > mid_start && (py < y || py >= y + h) {
            let c = shade(255, fy);
            if c.a > 0 {
                fb.fill_row(mid_start as usize, py as usize, (mid_end - mid_start) as usize, c);
            }
        }

        for px in mid_end.max(mid_start)..x1 {
            let c = shade(falloff(px, span_start, span_len, SPREAD), fy);
            if c.a > 0 {
                fb.blend_pixel(px as usize, py as usize, c);
            }
        }
    }
}

/// 255 inside the span, easing to 0 over `spread` pixels outside it.
#[inline(always)]
fn falloff(v: i32, start: i32, len: i32, spread: i32) -> u8 {
    let d = if v < start {
        start - v
    } else if v >= start + len {
        v - (start + len) + 1
    } else {
        return 255;
    };
    if d >= spread {
        0
    } else {
        (255 - d * 255 / spread) as u8
    }
}

pub fn radial_glow<C: Canvas>(fb: &mut C, cx: i32, cy: i32, r: i32, inner: Color, outer_alpha: u8) {
    if r <= 0 { return; }
    let r64 = r as i64;
    let r_sq = r64 * r64;
    for py in (cy - r).max(0)..(cy + r).min(fb.height() as i32) {
        for px in (cx - r).max(0)..(cx + r).min(fb.width() as i32) {
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

pub fn cloud<C: Canvas>(fb: &mut C, cx: i32, cy: i32, scale: i32, color: Color) {
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

/// The rim light and border of a frosted panel. The tinted backdrop below it
/// comes from the frame's glass cache and the shadow from `drop_shadow`, so
/// this layer can go on top of anything.
pub fn glass_highlights<C: Canvas>(fb: &mut C, x: i32, y: i32, w: i32, h: i32, r: i32, strength: u8) {
    let inset = r.max(2);
    let scaled = |a: u32| ((a * strength as u32) / 255) as u8;
    for i in 0..3 {
        let alpha = scaled(130 - i as u32 * 35);
        fill_rect(fb, x + inset, y + 1 + i, w - inset * 2, 1, Color::rgba(255, 255, 255, alpha));
    }
    fill_rect(fb, x + inset, y + h - 2, w - inset * 2, 1, Color::rgba(255, 255, 255, scaled(35)));
    stroke_rounded_rect(fb, x, y, w, h, r, Color::rgba(255, 255, 255, scaled(70)));
}

pub fn stroke_rounded_rect<C: Canvas>(fb: &mut C, x: i32, y: i32, w: i32, h: i32, r: i32, c: Color) {
    if w <= 0 || h <= 0 { return; }
    let r = r.max(0).min(w / 2).min(h / 2);

    let edge_pixel = |fb: &mut C, px: i32, py: i32| {
        if px < 0 || py < 0 || px >= fb.width() as i32 || py >= fb.height() as i32 {
            return;
        }
        let cov_in = rounded_coverage_signed(px - x, py - y, w, h, r, 0);
        let cov_out = rounded_coverage_signed(px - x, py - y, w, h, r, 1);
        let edge = cov_out.saturating_sub(cov_in);
        if edge == 0 { return; }
        fb.blend_pixel(px as usize, py as usize, c.fade(edge));
    };

    // Only the outermost ring can be on the border. Straight rows need their
    // two edge columns; corner rows need the arc bands as well.
    for py in y..(y + h) {
        let ly = py - y;
        let side = if ly <= 1 || ly >= h - 2 {
            w
        } else if ly < r || ly >= h - r {
            r + 2
        } else {
            2
        };
        if side * 2 >= w {
            for px in x..(x + w) {
                edge_pixel(fb, px, py);
            }
        } else {
            for px in x..(x + side) {
                edge_pixel(fb, px, py);
            }
            for px in (x + w - side)..(x + w) {
                edge_pixel(fb, px, py);
            }
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
