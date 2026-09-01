use alloc::vec;
use alloc::vec::Vec;

use super::{Canvas, Color};

/// An off-screen RGBA buffer on the heap. Unlike the screen it keeps alpha, so
/// a window can be drawn once and composited translucent, faded, or scaled.
pub struct Surface {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl Surface {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::TRANSPARENT; width * height],
        }
    }

    /// Fades out the alpha in the corners so window content follows the shape
    /// of the frame it sits in instead of poking out of it.
    pub fn round_corners(&mut self, radius: i32, top: bool, bottom: bool) {
        if radius <= 0 {
            return;
        }
        let w = self.width as i32;
        let h = self.height as i32;
        let radius = radius.min(w / 2).min(h / 2);
        for y in 0..h {
            let in_top = y < radius;
            let in_bottom = y >= h - radius;
            if !(in_top && top) && !(in_bottom && bottom) {
                continue;
            }
            for x in 0..w {
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

    /// Hard-overwrites every pixel. Blending would fold this frame into the
    /// last one, so a surface has to be cleared, not painted over.
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(c);
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
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + self.width as i32).min(dst.width() as i32);
        let y1 = (y + self.height as i32).min(dst.height() as i32);

        for py in y0..y1 {
            let sy = (py - y) as usize;
            let sx = (x0 - x) as usize;
            let run = (x1 - x0) as usize;
            let start = sy * self.width + sx;
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
        let x0 = origin_x.max(0);
        let y0 = origin_y.max(0);
        let x1 = (origin_x + out_w).min(dst.width() as i32);
        let y1 = (origin_y + out_h).min(dst.height() as i32);

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
    fn put_pixel(&mut self, x: usize, y: usize, c: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.pixels[y * self.width + x] = c;
    }

    fn fill_row(&mut self, x: usize, y: usize, len: usize, c: Color) {
        if c.a == 0 || y >= self.height || x >= self.width {
            return;
        }
        let n = len.min(self.width - x);
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
        if c.a == 0 || x >= self.width || y >= self.height {
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
