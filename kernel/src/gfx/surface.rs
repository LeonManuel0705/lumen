use alloc::vec;
use alloc::vec::Vec;

use super::{Canvas, Color, Rect};

/// An off-screen RGBA buffer on the heap. Unlike the screen it keeps alpha, so
/// a window can be drawn once and composited translucent, faded, or scaled.
pub struct Surface {
    width: usize,
    height: usize,
    clip: Rect,
    pixels: Vec<Color>,
}

impl Surface {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            clip: Rect::new(0, 0, width as i32, height as i32),
            pixels: vec![Color::TRANSPARENT; width * height],
        }
    }

    pub fn full_rect(&self) -> Rect {
        Rect::new(0, 0, self.width as i32, self.height as i32)
    }

    pub fn set_clip(&mut self, rect: Rect) {
        self.clip = rect.intersect(&self.full_rect());
    }

    /// Fades out the alpha in the corners so window content follows the shape
    /// of the frame it sits in instead of poking out of it.
    ///
    /// This multiplies alpha in place, so it must run exactly once over any
    /// pixel per redraw. It therefore honours the clip: a pixel outside the
    /// region being redrawn was already rounded when it was last drawn, and
    /// rounding it again would fade the corners away over a few seconds.
    pub fn round_corners(&mut self, radius: i32, top: bool, bottom: bool) {
        if radius <= 0 {
            return;
        }
        let w = self.width as i32;
        let h = self.height as i32;
        let radius = radius.min(w / 2).min(h / 2);
        for y in self.clip.y0.max(0)..self.clip.y1.min(h) {
            let in_top = y < radius;
            let in_bottom = y >= h - radius;
            if !(in_top && top) && !(in_bottom && bottom) {
                continue;
            }
            for x in self.clip.x0.max(0)..self.clip.x1.min(w) {
                if x >= radius && x < w - radius {
                    continue;
                }
                let cov = super::shapes::rounded_coverage(x, y, w, h, radius);
                if cov == 255 {
                    continue;
                }
                let idx = y as usize * self.width + x as usize;
                let a = self.pixels[idx].a as u32 * cov as u32 / 255;
                self.pixels[idx].a = a as u8;
            }
        }
    }

    /// Hard-overwrites a region. Blending would fold this frame into the last
    /// one, so a surface has to be cleared, not painted over. Deliberately
    /// ignores the clip: this is what makes the clean slate the clip then
    /// confines the redraw to.
    pub fn clear_rect(&mut self, rect: Rect, c: Color) {
        let rect = rect.intersect(&self.full_rect());
        if rect.is_empty() {
            return;
        }
        for y in rect.y0 as usize..rect.y1 as usize {
            let start = y * self.width + rect.x0 as usize;
            let end = y * self.width + rect.x1 as usize;
            self.pixels[start..end].fill(c);
        }
    }

    #[inline(always)]
    fn at(&self, x: usize, y: usize) -> Color {
        self.pixels[y * self.width + x]
    }

    /// Composites the surface onto `dst` with its top-left at (`x`, `y`),
    /// scaling every pixel's alpha by `opacity`.
    pub fn blit<C: Canvas>(&self, dst: &mut C, x: i32, y: i32, opacity: u8) {
        if opacity == 0 {
            return;
        }
        let (cx0, cy0, cx1, cy1) = dst.bounds();
        let x0 = x.max(cx0);
        let y0 = y.max(cy0);
        let x1 = (x + self.width as i32).min(cx1);
        let y1 = (y + self.height as i32).min(cy1);
        // A clip can miss the surface in one axis while overlapping in the
        // other, which leaves a negative run. That has to be caught before the
        // row slice is cut, not after.
        if x1 <= x0 || y1 <= y0 {
            return;
        }

        let sx = (x0 - x) as usize;
        let run = (x1 - x0) as usize;
        for py in y0..y1 {
            let start = (py - y) as usize * self.width + sx;
            dst.blend_row(x0 as usize, py as usize, &self.pixels[start..start + run], opacity);
        }
    }

    /// Composites the surface scaled around its own centre, which is how
    /// windows grow into place and fold away again. Nearest sampling in 16.16
    /// fixed point: it only runs while something is in motion.
    pub fn blit_scaled<C: Canvas>(
        &self,
        dst: &mut C,
        center_x: f32,
        center_y: f32,
        scale: f32,
        opacity: u8,
    ) {
        if opacity == 0 || scale <= 0.01 {
            return;
        }
        if (scale - 1.0).abs() < 0.002 {
            let x = (center_x - self.width as f32 * 0.5) as i32;
            let y = (center_y - self.height as f32 * 0.5) as i32;
            return self.blit(dst, x, y, opacity);
        }

        let out_w = (self.width as f32 * scale) as i32;
        let out_h = (self.height as f32 * scale) as i32;
        if out_w <= 0 || out_h <= 0 {
            return;
        }
        let origin_x = (center_x - out_w as f32 * 0.5) as i32;
        let origin_y = (center_y - out_h as f32 * 0.5) as i32;

        let step = ((1.0 / scale) * 65536.0) as i64;
        let (cx0, cy0, cx1, cy1) = dst.bounds();
        let x0 = origin_x.max(cx0);
        let y0 = origin_y.max(cy0);
        let x1 = (origin_x + out_w).min(cx1);
        let y1 = (origin_y + out_h).min(cy1);
        if x1 <= x0 || y1 <= y0 {
            return;
        }

        for py in y0..y1 {
            let sy = (((py - origin_y) as i64 * step) >> 16) as usize;
            if sy >= self.height {
                continue;
            }
            for px in x0..x1 {
                let sx = (((px - origin_x) as i64 * step) >> 16) as usize;
                if sx >= self.width {
                    continue;
                }
                let src = self.at(sx, sy);
                if src.a == 0 {
                    continue;
                }
                dst.paint(px as usize, py as usize, src.fade(opacity));
            }
        }
    }
}

impl Canvas for Surface {
    #[inline(always)]
    fn width(&self) -> usize {
        self.width
    }

    #[inline(always)]
    fn height(&self) -> usize {
        self.height
    }

    #[inline(always)]
    fn clip(&self) -> Rect {
        self.clip
    }

    #[inline(always)]
    fn put_pixel(&mut self, x: usize, y: usize, c: Color) {
        if !self.clip.contains(x, y) {
            return;
        }
        self.pixels[y * self.width + x] = c;
    }

    fn fill_row(&mut self, x: usize, y: usize, len: usize, c: Color) {
        if c.a == 0 || (y as i32) < self.clip.y0 || y as i32 >= self.clip.y1 {
            return;
        }
        let x0 = (x as i32).max(self.clip.x0);
        let x1 = ((x + len) as i32).min(self.clip.x1);
        if x1 <= x0 {
            return;
        }
        let (x, n) = (x0 as usize, (x1 - x0) as usize);
        let start = y * self.width + x;
        if c.a == 255 {
            self.pixels[start..start + n].fill(c);
            return;
        }
        for slot in self.pixels[start..start + n].iter_mut() {
            *slot = c.over_straight(*slot);
        }
    }

    fn blend_pixel(&mut self, x: usize, y: usize, c: Color) {
        if c.a == 0 || !self.clip.contains(x, y) {
            return;
        }
        let idx = y * self.width + x;
        self.pixels[idx] = c.over_straight(self.pixels[idx]);
    }

    fn read_pixel(&self, x: usize, y: usize) -> Color {
        if x >= self.width || y >= self.height {
            return Color::TRANSPARENT;
        }
        self.at(x, y)
    }
}
