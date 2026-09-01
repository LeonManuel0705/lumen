use crate::anim::easing;
use crate::gfx::text::{self, Font};
use crate::gfx::{Canvas, Color, Rect};

const ROLL_SECONDS: f32 = 0.28;

pub struct RollingClock {
    font: &'static Font,
    show_seconds: bool,
    chars: [u8; 8],
    prev: [u8; 8],
    anim: [f32; 8],
    /// Cells that moved at any point this frame. A roll that finishes still
    /// changes the pixels on the frame it finishes, and the cell is no longer
    /// animating by the time anyone asks, so it has to be remembered.
    moved_this_frame: [bool; 8],
}

impl RollingClock {
    pub const fn new(font: &'static Font, show_seconds: bool) -> Self {
        Self {
            font,
            show_seconds,
            chars: [0; 8],
            prev: [0; 8],
            anim: [1.0; 8],
            moved_this_frame: [false; 8],
        }
    }

    fn len(&self) -> usize {
        if self.show_seconds { 8 } else { 5 }
    }

    /// Called once per frame by the shell, which is what makes it the right
    /// place to open a new frame's worth of movement bookkeeping.
    pub fn set(&mut self, day_seconds: u32) {
        self.moved_this_frame = [false; 8];
        let h = (day_seconds / 3600) % 24;
        let m = (day_seconds / 60) % 60;
        let s = day_seconds % 60;
        let new = [
            b'0' + (h / 10) as u8, b'0' + (h % 10) as u8, b':',
            b'0' + (m / 10) as u8, b'0' + (m % 10) as u8, b':',
            b'0' + (s / 10) as u8, b'0' + (s % 10) as u8,
        ];
        if self.chars[0] == 0 {
            self.chars = new;
            self.prev = new;
            return;
        }
        for i in 0..self.len() {
            if new[i] != self.chars[i] {
                self.prev[i] = self.chars[i];
                self.anim[i] = 0.0;
            }
        }
        self.chars = new;
    }

    pub fn update(&mut self, dt: f32) {
        for (a, moved) in self.anim.iter_mut().zip(self.moved_this_frame.iter_mut()) {
            if *a < 1.0 {
                *moved = true;
            }
            *a = (*a + dt / ROLL_SECONDS).min(1.0);
        }
    }

    pub fn width(&self) -> i32 {
        let cell = self.font.digit_advance as i32;
        let colon_adv = self.font.glyph(':').map(|g| g.advance as i32).unwrap_or(8);
        let colons = if self.show_seconds { 2 } else { 1 };
        (self.len() as i32 - colons) * cell + colons * colon_adv
    }

    pub fn digit_height(&self) -> i32 {
        self.font.glyph('0').map(|g| g.h as i32).unwrap_or(20)
    }

    /// The pixels that differ from the last frame: the colons, which breathe
    /// every frame, plus any digit still rolling. Walks the same pen the
    /// drawing does, so the two cannot drift apart.
    pub fn changed_rect(&self, center_x: i32, baseline: i32) -> Rect {
        if self.chars[0] == 0 {
            return Rect::EMPTY;
        }
        let font = self.font;
        let cell = font.digit_advance as i32;
        let colon_adv = font.glyph(':').map(|g| g.advance as i32).unwrap_or(8);
        let digit_h = self.digit_height();
        let lift = (digit_h as f32 * 0.65) as i32;
        let mut pen = center_x - self.width() / 2;
        let mut changed = Rect::EMPTY;

        for i in 0..self.len() {
            let colon = self.chars[i] as char == ':';
            let advance = if colon { colon_adv } else { cell };
            if colon || self.anim[i] < 1.0 || self.moved_this_frame[i] {
                changed = changed.union(&Rect::new(
                    pen - 4,
                    baseline - digit_h - lift - 4,
                    pen + advance + 4,
                    baseline + lift + 6,
                ));
            }
            pen += advance;
        }
        changed
    }

    pub fn draw<C: Canvas>(&self, fb: &mut C, center_x: i32, baseline: i32, master: f32, pulse: f32) {
        if master <= 0.01 || self.chars[0] == 0 {
            return;
        }
        let font = self.font;
        let cell = font.digit_advance as i32;
        let colon_adv = font.glyph(':').map(|g| g.advance as i32).unwrap_or(8);
        let digit_h = self.digit_height();
        let lift = (digit_h as f32 * 0.65) as i32;
        let mut pen = center_x - self.width() / 2;

        for i in 0..self.len() {
            let c = self.chars[i] as char;
            if c == ':' {
                let alpha = (master * (140.0 + 90.0 * pulse)) as u8;
                glyph_embossed(fb, font, ':', pen, baseline, alpha);
                pen += colon_adv;
                continue;
            }
            let centered = |ch: char| pen + (cell - font.glyph(ch).map(|g| g.advance as i32).unwrap_or(cell)) / 2;
            let p = self.anim[i];
            if p >= 1.0 {
                glyph_embossed(fb, font, c, centered(c), baseline, (master * 235.0) as u8);
            } else {
                let pe = easing::ease_out_quad(p);
                let prev = self.prev[i] as char;
                let off_old = (-pe * lift as f32) as i32;
                let off_new = ((1.0 - pe) * lift as f32) as i32;
                glyph_embossed(fb, font, prev, centered(prev), baseline + off_old, (master * (1.0 - pe) * 235.0) as u8);
                glyph_embossed(fb, font, c, centered(c), baseline + off_new, (master * pe * 235.0) as u8);
            }
            pen += cell;
        }
    }
}

pub fn glyph_embossed<C: Canvas>(fb: &mut C, font: &Font, c: char, x: i32, baseline: i32, alpha: u8) {
    if alpha < 4 {
        return;
    }
    text::draw_char(fb, font, c, x, baseline + 1, Color::WHITE.with_alpha((alpha as u32 * 70 / 255) as u8));
    text::draw_char(fb, font, c, x, baseline, Color::LUMEN_INK.with_alpha(alpha));
}

pub fn text_embossed<C: Canvas>(fb: &mut C, font: &'static Font, s: &str, center_x: i32, baseline: i32, alpha: u8) {
    if alpha < 4 {
        return;
    }
    let x = center_x - text::measure(font, s) / 2;
    text::draw_text(fb, font, s, x, baseline + 1, Color::WHITE.with_alpha((alpha as u32 * 70 / 255) as u8));
    text::draw_text(fb, font, s, x, baseline, Color::LUMEN_INK.with_alpha(alpha));
}
