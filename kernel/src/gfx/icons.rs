use super::{shapes, Canvas, Color};

/// The colours one dock plate is built from.
#[derive(Copy, Clone)]
pub struct Plate {
    pub top: Color,
    pub bottom: Color,
    pub rim: Color,
    pub indicator: Color,
}

/// One plate per slot. The lightness rhythm across the row was fixed before
/// the hues were: bright, deep, paper, ink, slate, sky. Two warms, one paper,
/// three cools, and amber somewhere inside every emblem, which is the thread
/// that makes six different marks one family instead of six stickers.
pub const PLATES: [Plate; 6] = [
    Plate { top: Color::rgb(255, 209, 116), bottom: Color::rgb(243, 152, 52), rim: Color::rgba(168, 96, 24, 165), indicator: Color::rgb(255, 186, 92) },
    Plate { top: Color::rgb(104, 158, 240), bottom: Color::rgb(48, 88, 188), rim: Color::rgba(28, 52, 124, 165), indicator: Color::rgb(96, 158, 236) },
    Plate { top: Color::rgb(250, 244, 232), bottom: Color::rgb(226, 214, 194), rim: Color::rgba(156, 142, 120, 165), indicator: Color::rgb(214, 196, 166) },
    Plate { top: Color::rgb(52, 86, 140), bottom: Color::rgb(24, 50, 92), rim: Color::rgba(12, 30, 60, 165), indicator: Color::rgb(92, 132, 194) },
    Plate { top: Color::rgb(214, 224, 238), bottom: Color::rgb(168, 186, 210), rim: Color::rgba(116, 134, 162, 165), indicator: Color::rgb(176, 196, 224) },
    Plate { top: Color::rgb(150, 212, 248), bottom: Color::rgb(74, 158, 226), rim: Color::rgba(36, 104, 168, 165), indicator: Color::rgb(96, 180, 240) },
];

/// Maps the 52-unit design cell every emblem is authored in onto the tile as
/// it is actually drawn, which may be larger, squashed, or fading. Keeping the
/// recipes in one fixed space is what lets a tile be 52 or 80 pixels without
/// anyone redrawing the artwork.
struct Cell {
    x: f32,
    y: f32,
    ux: f32,
    uy: f32,
    alpha: u8,
}

impl Cell {
    fn px(&self, v: f32) -> i32 { (self.x + v * self.ux + 0.5) as i32 }
    fn py(&self, v: f32) -> i32 { (self.y + v * self.uy + 0.5) as i32 }
    fn sx(&self, v: f32) -> i32 { (v * self.ux + 0.5) as i32 }
    fn sy(&self, v: f32) -> i32 { (v * self.uy + 0.5) as i32 }
    fn sm(&self, v: f32) -> i32 { (v * (self.ux + self.uy) * 0.5 + 0.5) as i32 }

    fn disc<C: Canvas>(&self, fb: &mut C, cx: f32, cy: f32, r: f32, c: Color) {
        let c = c.fade(self.alpha);
        if (self.ux - self.uy).abs() < 0.02 {
            shapes::fill_circle(fb, self.px(cx), self.py(cy), self.sx(r).max(1), c);
        } else {
            shapes::fill_ellipse(fb, self.px(cx), self.py(cy), self.sx(r).max(1), self.sy(r).max(1), c);
        }
    }

    fn oval<C: Canvas>(&self, fb: &mut C, cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
        shapes::fill_ellipse(fb, self.px(cx), self.py(cy), self.sx(rx).max(1), self.sy(ry).max(1), c.fade(self.alpha));
    }

    fn bar<C: Canvas>(&self, fb: &mut C, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
        shapes::fill_rounded_rect(fb, self.px(x), self.py(y), self.sx(w).max(1), self.sy(h).max(1), self.sx(r).max(1), c.fade(self.alpha));
    }

    fn ring<C: Canvas>(&self, fb: &mut C, cx: f32, cy: f32, r: f32, t: f32, c: Color) {
        shapes::stroke_circle(fb, self.px(cx), self.py(cy), self.sm(r), self.sm(t).max(1), c.fade(self.alpha));
    }

    fn glow<C: Canvas>(&self, fb: &mut C, cx: f32, cy: f32, r: f32, c: Color) {
        shapes::radial_glow(fb, self.px(cx), self.py(cy), self.sm(r).max(1), c.fade(self.alpha), 0);
    }
}

const WHITE: Color = Color::WHITE;
const INK: Color = Color::rgb(22, 52, 94);
const AMBER: Color = Color::rgb(243, 168, 58);

/// Draws one slot's plate and emblem into the box (`x`, `y`, `w`, `h`).
/// `emb` is the emblem's own phase, 0 at rest and 255 fully awake, which every
/// live detail (a climbing ball, a sliding knob, a drifting cloud) reads from.
#[allow(clippy::too_many_arguments)]
pub fn draw_slot<C: Canvas>(fb: &mut C, slot: usize, x: i32, y: i32, w: i32, h: i32, alpha: u8, emb: u8) {
    if w <= 0 || h <= 0 || alpha == 0 {
        return;
    }
    let plate = &PLATES[slot.min(PLATES.len() - 1)];
    let r = (w.min(h) * 31 + 50) / 100;

    shapes::fill_rounded_rect_vgrad(fb, x, y, w, h, r, plate.top.fade(alpha), plate.bottom.fade(alpha));
    shapes::stroke_rounded_rect(fb, x, y, w, h, r, plate.rim.fade(alpha));
    shapes::fill_rect(fb, x + r, y + 1, w - 2 * r, 1, Color::rgba(255, 255, 255, 120).fade(alpha));
    shapes::fill_rect(fb, x + r, y + 2, w - 2 * r, 1, Color::rgba(255, 255, 255, 58).fade(alpha));
    shapes::fill_rect(fb, x + r, y + h - 2, w - 2 * r, 1, Color::rgba(255, 255, 255, 26).fade(alpha));

    let cell = Cell { x: x as f32, y: y as f32, ux: w as f32 / 52.0, uy: h as f32 / 52.0, alpha };
    let e = emb as f32 / 255.0;
    match slot {
        0 => ball(fb, &cell, e),
        1 => clock(fb, &cell, e),
        2 => colours(fb, &cell, e),
        3 => sketch(fb, &cell, e),
        4 => settings(fb, &cell, e),
        _ => weather(fb, &cell, e),
    }
}

/// The mascot mid-bounce: one big white disc over one white bar, and nothing
/// else in the set has a bar at the bottom edge. On hover it climbs out of
/// its own trail. It stays a perfect circle; squash is motion's job.
fn ball<C: Canvas>(fb: &mut C, c: &Cell, e: f32) {
    c.bar(fb, 11.0, 38.5, 30.0, 3.0, 1.5, WHITE.with_alpha(232));
    c.oval(fb, 26.0, 37.5, 10.0 - 3.0 * e, 2.6, Color::rgba(150, 72, 8, 110));
    c.disc(fb, 26.0, 30.5, 7.0, WHITE.with_alpha((40.0 * e) as u8));
    c.disc(fb, 26.0, 27.0, 9.0, WHITE.with_alpha((70.0 * e) as u8));
    c.disc(fb, 26.0, 21.0 - 4.0 * e, 13.5, WHITE.with_alpha(54));
    c.disc(fb, 26.0, 21.0 - 4.0 * e, 11.0, WHITE.with_alpha(252));
    c.disc(fb, 26.0, 25.5 - 4.0 * e, 5.5, Color::rgba(246, 166, 66, 86));
}

/// A dial at 12:15. Hands are axis-aligned because there is no rotation, and
/// the minute hand is longer than the hour so it cannot read as a plus sign.
fn clock<C: Canvas>(fb: &mut C, c: &Cell, e: f32) {
    c.disc(fb, 26.0, 25.5, 11.5, WHITE.with_alpha(30));
    c.ring(fb, 26.0, 25.5, 13.0, 3.0, WHITE.with_alpha(238));
    c.disc(fb, 26.0, 16.5, 1.6, Color::rgb(255, 206, 110));
    c.bar(fb, 24.5, 19.5, 3.0, 7.0, 1.5, WHITE.with_alpha(242));
    c.bar(fb, 26.0, 24.0, 10.0, 3.0, 1.5, WHITE.with_alpha(242));
    c.disc(fb, 26.0, 25.5, 2.6, WHITE);
    c.disc(fb, 26.0, 25.5, 1.2, Color::rgb(56, 96, 196));
    if e > 0.02 {
        c.ring(fb, 26.0, 25.5, 13.0 + 4.0 * e, 1.0, WHITE.with_alpha((120.0 * e) as u8));
    }
}

/// Three gels mixing on warm paper. Alpha 180 over a base lighter than the
/// discs means the overlaps come out saturated, never muddy: the icon performs
/// its own function. The only polychrome emblem, because colour is its subject.
fn colours<C: Canvas>(fb: &mut C, c: &Cell, e: f32) {
    c.disc(fb, 20.0 - 2.0 * e, 31.0 + 1.0 * e, 9.4, Color::rgba(255, 186, 60, 180));
    c.disc(fb, 32.0 + 2.0 * e, 31.0 + 1.0 * e, 9.4, Color::rgba(74, 168, 240, 180));
    c.disc(fb, 26.0, 20.5 - 2.0 * e, 9.4, Color::rgba(255, 92, 132, 180));
    c.disc(fb, 22.6, 16.6, 2.4, WHITE.with_alpha(150));
    c.oval(fb, 26.0, 41.5, 12.0, 2.2, Color::rgba(150, 132, 108, 80));
}

/// A pen stroke as a chain of overlapping discs with a growing radius, which
/// is the only honest curve this primitive set can draw, and it gives a
/// pressure-varying line for free. On hover the head extends and the nib
/// glow blooms.
fn sketch<C: Canvas>(fb: &mut C, c: &Cell, e: f32) {
    shapes::stroke_rounded_rect(fb, c.px(9.0), c.py(9.0), c.sx(34.0), c.sy(34.0), c.sm(7.0), WHITE.with_alpha(52).fade(c.alpha));

    let ghost = |t: f32| quad((14.0, 31.0), (20.0, 27.0), (28.0, 29.0), t);
    for k in 0..10 {
        let (gx, gy) = ghost(k as f32 / 9.0);
        c.disc(fb, gx, gy, 1.3, WHITE.with_alpha(46));
    }

    c.glow(fb, 35.0, 15.5, 6.2 + 2.0 * e, Color::rgba(255, 214, 140, (70.0 + 50.0 * e) as u8));

    let head = 21 + (4.0 * e) as i32;
    for k in 0..=head {
        let t = k as f32 / 25.0;
        let (sx, sy) = quad((13.5, 38.5), (20.0, 20.5), (35.0, 15.5), t);
        let tint = Color::lerp(Color::rgb(255, 196, 96), WHITE, (t * 255.0) as u8);
        c.disc(fb, sx, sy, 1.5 + 1.6 * t, tint.with_alpha((110.0 + 140.0 * t) as u8));
    }
    c.disc(fb, 35.0, 15.5, 3.6, WHITE);
}

fn quad(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (
        u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
        u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
    )
}

/// Two sliders. A gear needs rotation and a gear made of circles looks cheap;
/// a mixer is what settings are, and it is the one emblem that visibly
/// operates itself. The amber knob is the theme swatch.
fn settings<C: Canvas>(fb: &mut C, c: &Cell, e: f32) {
    let e1 = ((e - 0.18) / 0.82).clamp(0.0, 1.0);
    let ka = 32.0 - 10.0 * e;
    let kb = 19.5 + 12.0 * e1;
    c.bar(fb, 10.5, 18.5, 31.0, 6.0, 3.0, INK.with_alpha(60));
    c.bar(fb, 10.5, 18.5, ka - 10.5, 6.0, 3.0, INK.with_alpha(105));
    c.bar(fb, 10.5, 29.5, 31.0, 6.0, 3.0, INK.with_alpha(60));
    c.bar(fb, 10.5, 29.5, kb - 10.5, 6.0, 3.0, AMBER.with_alpha(190));
    c.oval(fb, ka, 27.9, 5.4, 1.6, INK.with_alpha(50));
    c.disc(fb, ka, 21.5, 6.2, WHITE.with_alpha(252));
    c.ring(fb, ka, 21.5, 6.2, 1.0, INK.with_alpha(70));
    c.oval(fb, kb, 38.9, 5.4, 1.6, INK.with_alpha(50));
    c.disc(fb, kb, 32.5, 6.2, AMBER);
    c.ring(fb, kb, 32.5, 6.2, 1.0, Color::rgba(140, 86, 10, 90));
}

/// Sun behind cloud: the wallpaper in miniature, built from its own two-ring
/// sun. The cloud is seven discs written here rather than `shapes::cloud`,
/// whose integer scale would pop four times while the icon grows.
fn weather<C: Canvas>(fb: &mut C, c: &Cell, e: f32) {
    c.glow(fb, 33.0, 18.5, 12.0 + 3.0 * e, Color::rgba(255, 226, 150, (110.0 + 40.0 * e) as u8));
    c.disc(fb, 33.0, 18.5, 7.6, Color::LUMEN_ACCENT);
    c.disc(fb, 33.0, 18.5, 5.4, Color::LUMEN_GLOW);
    let d = 2.0 * e;
    const PUFFS: [(f32, f32, f32, u8); 7] = [
        (23.5, 31.0, 8.0, 244),
        (17.5, 32.0, 6.0, 230),
        (29.5, 32.0, 6.0, 230),
        (12.5, 33.0, 4.5, 201),
        (34.5, 33.0, 4.5, 201),
        (20.5, 27.0, 5.5, 220),
        (27.0, 27.5, 6.0, 225),
    ];
    for (px, py, r, a) in PUFFS {
        c.disc(fb, px + d, py, r, WHITE.with_alpha(a));
    }
    c.oval(fb, 23.5 + d, 36.0, 11.5, 2.4, Color::rgba(120, 170, 214, 80));
}
