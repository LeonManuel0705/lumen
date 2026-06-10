use super::{Color, Framebuffer};

pub struct GlyphMeta {
    pub xmin: i16,
    pub ymin: i16,
    pub w: u16,
    pub h: u16,
    pub advance: i16,
    pub offset: u32,
}

pub struct Font {
    pub first_char: u32,
    pub glyphs: &'static [GlyphMeta],
    pub bitmap: &'static [u8],
    #[allow(dead_code)]
    pub ascent: i16,
    #[allow(dead_code)]
    pub descent: i16,
    #[allow(dead_code)]
    pub line_height: i16,
    pub digit_advance: i16,
}

impl Font {
    pub fn glyph(&self, c: char) -> Option<&GlyphMeta> {
        self.glyphs.get((c as usize).checked_sub(self.first_char as usize)?)
    }
}

pub fn draw_char(fb: &mut Framebuffer, font: &Font, c: char, x: i32, baseline_y: i32, color: Color) -> i32 {
    let Some(g) = font.glyph(c) else { return 0 };
    if g.w > 0 && g.h > 0 && color.a > 0 {
        let x0 = x + g.xmin as i32;
        let y0 = baseline_y - g.ymin as i32 - g.h as i32;
        let mut i = g.offset as usize;
        for row in 0..g.h as i32 {
            for col in 0..g.w as i32 {
                let cov = font.bitmap[i];
                i += 1;
                if cov == 0 {
                    continue;
                }
                let px = x0 + col;
                let py = y0 + row;
                if px < 0 || py < 0 {
                    continue;
                }
                let a = ((color.a as u32 * cov as u32) / 255) as u8;
                fb.blend_pixel(px as usize, py as usize, color.with_alpha(a));
            }
        }
    }
    g.advance as i32
}

#[allow(dead_code)]
pub fn draw_text(fb: &mut Framebuffer, font: &Font, s: &str, x: i32, baseline_y: i32, color: Color) -> i32 {
    let mut pen = x;
    for c in s.chars() {
        pen += draw_char(fb, font, c, pen, baseline_y, color);
    }
    pen - x
}

#[allow(dead_code)]
pub fn measure(font: &Font, s: &str) -> i32 {
    s.chars().filter_map(|c| font.glyph(c)).map(|g| g.advance as i32).sum()
}
