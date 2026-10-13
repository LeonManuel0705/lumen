#[inline(always)]
pub fn mul255(a: u8, b: u8) -> u8 {
    let t = a as u32 * b as u32 + 128;
    ((t + (t >> 8)) >> 8) as u8
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    #[allow(dead_code)]
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    #[allow(dead_code)]
    pub const BLACK:       Color = Color::rgb(0, 0, 0);
    pub const WHITE:       Color = Color::rgb(255, 255, 255);

    pub const LUMEN_BG_TOP:    Color = Color::rgb(58, 130, 200);
    pub const LUMEN_BG_MID:    Color = Color::rgb(140, 195, 235);
    pub const LUMEN_BG_BOTTOM: Color = Color::rgb(225, 240, 250);
    pub const LUMEN_ACCENT:    Color = Color::rgb(255, 220, 130);
    pub const LUMEN_GLOW:      Color = Color::rgb(255, 245, 205);
    pub const LUMEN_CARD:      Color = Color::rgba(255, 255, 255, 130);
    pub const LUMEN_INK:       Color = Color::rgb(20, 50, 90);
    pub const LUMEN_CLOUD:     Color = Color::rgba(255, 255, 255, 230);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    #[inline(always)]
    pub fn fade(self, factor: u8) -> Self {
        Self { a: mul255(self.a, factor), ..self }
    }

    pub fn lerp(a: Color, b: Color, t: u8) -> Color {
        let mix = |x: u8, y: u8| -> u8 {
            let xi = x as u16;
            let yi = y as u16;
            let ti = t as u16;
            ((xi * (255 - ti) + yi * ti) / 255) as u8
        };
        Color {
            r: mix(a.r, b.r),
            g: mix(a.g, b.g),
            b: mix(a.b, b.b),
            a: mix(a.a, b.a),
        }
    }

    pub fn over(self, dst: Color) -> Color {
        if self.a == 255 { return self; }
        if self.a == 0   { return dst; }
        let sa = self.a;
        let inv = 255 - sa;
        let blend = |s: u8, d: u8| -> u8 { mul255(s, sa) + mul255(d, inv) };
        Color {
            r: blend(self.r, dst.r),
            g: blend(self.g, dst.g),
            b: blend(self.b, dst.b),
            a: 255,
        }
    }

    #[allow(dead_code)]
    pub fn over_straight(self, dst: Color) -> Color {
        if self.a == 255 || dst.a == 0 { return self; }
        if self.a == 0 { return dst; }
        let sa = self.a as u32;
        let da = dst.a as u32;
        let keep = da * (255 - sa);
        let total = sa * 255 + keep;
        let mix = |s: u8, d: u8| -> u8 {
            ((s as u32 * sa * 255 + d as u32 * keep) / total) as u8
        };
        Color {
            r: mix(self.r, dst.r),
            g: mix(self.g, dst.g),
            b: mix(self.b, dst.b),
            a: (total / 255) as u8,
        }
    }
}
