#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
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
        let sa = self.a as u16;
        let inv = 255 - sa;
        let blend = |s: u8, d: u8| -> u8 {
            ((s as u16 * sa + d as u16 * inv) / 255) as u8
        };
        Color {
            r: blend(self.r, dst.r),
            g: blend(self.g, dst.g),
            b: blend(self.b, dst.b),
            a: 255,
        }
    }
}
