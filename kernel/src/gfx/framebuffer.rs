use bootloader_api::info::{FrameBuffer as BootFb, FrameBufferInfo, PixelFormat};
use super::color::mul255;
use super::{Canvas, Color, Rect};

pub struct Framebuffer<'a> {
    buf: &'a mut [u8],
    width: usize,
    height: usize,
    clip: Rect,
    stride_bytes: usize,
    bpp: usize,
    format: PixelFormat,
}

impl<'a> Framebuffer<'a> {
    #[allow(dead_code)]
    pub fn from_boot(fb: &'a mut BootFb) -> Self {
        let info = fb.info();
        let buf = fb.buffer_mut();
        Self::from_raw(buf, info)
    }

    pub fn from_raw(buf: &'a mut [u8], info: FrameBufferInfo) -> Self {
        let stride_bytes = info.stride * info.bytes_per_pixel;
        // The buffer may be shorter than the mode (display.rs clamps to MAX_BYTES);
        // never expose rows the slice can't back.
        let max_rows = if stride_bytes == 0 { 0 } else { buf.len() / stride_bytes };
        let width = info.width;
        let height = info.height.min(max_rows);
        Self {
            buf,
            width,
            height,
            clip: Rect::new(0, 0, width as i32, height as i32),
            stride_bytes,
            bpp: info.bytes_per_pixel,
            format: info.pixel_format,
        }
    }

    /// Confines every later write to `rect`, intersected with the buffer.
    pub fn set_clip(&mut self, rect: Rect) {
        self.clip = rect.intersect(&Rect::new(0, 0, self.width as i32, self.height as i32));
    }

    /// Trims a horizontal run to the clip. Returns the surviving start, its
    /// length, and how many pixels were dropped off the front, which callers
    /// with a source buffer need in order to stay in step.
    #[inline(always)]
    fn clip_run(&self, x: usize, y: usize, len: usize) -> Option<(usize, usize, usize)> {
        let row = y as i64;
        if row < self.clip.y0 as i64 || row >= self.clip.y1 as i64 {
            return None;
        }
        let start = (x as i64).max(self.clip.x0 as i64);
        let end = ((x + len) as i64).min(self.clip.x1 as i64);
        if end <= start {
            return None;
        }
        Some((start as usize, (end - start) as usize, (start - x as i64) as usize))
    }

    #[inline(always)]
    pub fn row(&self, y: usize) -> &[u8] {
        let off = y * self.stride_bytes;
        &self.buf[off..off + self.width * self.bpp]
    }

    #[inline(always)]
    pub fn row_mut(&mut self, y: usize) -> &mut [u8] {
        let off = y * self.stride_bytes;
        let end = off + self.width * self.bpp;
        &mut self.buf[off..end]
    }

    #[inline(always)]
    pub fn bytes_per_pixel(&self) -> usize {
        self.bpp
    }

    #[inline(always)]
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    #[allow(dead_code)]
    pub fn clear(&mut self, c: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, c);
            }
        }
    }
}

impl Canvas for Framebuffer<'_> {
    #[inline(always)]
    fn width(&self) -> usize {
        self.width
    }

    #[inline(always)]
    fn clip(&self) -> Rect {
        self.clip
    }

    #[inline(always)]
    fn height(&self) -> usize {
        self.height
    }

    #[inline(always)]
    fn put_pixel(&mut self, x: usize, y: usize, c: Color) {
        if !self.clip.contains(x, y) { return; }
        let off = y * self.stride_bytes + x * self.bpp;
        encode(&mut self.buf[off..off + self.bpp], self.format, c);
    }

    fn fill_row(&mut self, x: usize, y: usize, len: usize, c: Color) {
        if c.a == 0 {
            return;
        }
        let Some((x, n, _)) = self.clip_run(x, y, len) else { return };
        let bpp = self.bpp;
        let start = y * self.stride_bytes + x * bpp;
        let row = &mut self.buf[start..start + n * bpp];
        let (i_r, i_g, i_b) = match self.format {
            PixelFormat::Rgb => (0, 1, 2),
            PixelFormat::Bgr => (2, 1, 0),
            _ => {
                for i in 0..n {
                    let slot = &mut row[i * bpp..i * bpp + bpp];
                    encode(slot, self.format, c.over(decode(slot, self.format)));
                }
                return;
            }
        };
        if c.a == 255 {
            for i in 0..n {
                let o = i * bpp;
                row[o + i_r] = c.r;
                row[o + i_g] = c.g;
                row[o + i_b] = c.b;
            }
            return;
        }
        let inv = 255 - c.a;
        let (pre_r, pre_g, pre_b) = (mul255(c.r, c.a), mul255(c.g, c.a), mul255(c.b, c.a));
        for i in 0..n {
            let o = i * bpp;
            row[o + i_r] = pre_r + mul255(row[o + i_r], inv);
            row[o + i_g] = pre_g + mul255(row[o + i_g], inv);
            row[o + i_b] = pre_b + mul255(row[o + i_b], inv);
        }
    }

    fn blend_row(&mut self, x: usize, y: usize, src: &[Color], opacity: u8) {
        let Some((x, n, skipped)) = self.clip_run(x, y, src.len()) else { return };
        let src = &src[skipped..];
        let bpp = self.bpp;
        let start = y * self.stride_bytes + x * bpp;
        let row = &mut self.buf[start..start + n * bpp];
        let (i_r, i_g, i_b) = match self.format {
            PixelFormat::Rgb => (0, 1, 2),
            PixelFormat::Bgr => (2, 1, 0),
            _ => {
                for (i, c) in src[..n].iter().enumerate() {
                    let a = mul255(c.a, opacity);
                    if a == 0 { continue; }
                    let slot = &mut row[i * bpp..i * bpp + bpp];
                    encode(slot, self.format, c.with_alpha(a).over(decode(slot, self.format)));
                }
                return;
            }
        };
        for (i, c) in src[..n].iter().enumerate() {
            let a = mul255(c.a, opacity);
            if a == 0 {
                continue;
            }
            let o = i * bpp;
            if a == 255 {
                row[o + i_r] = c.r;
                row[o + i_g] = c.g;
                row[o + i_b] = c.b;
            } else {
                let inv = 255 - a;
                row[o + i_r] = mul255(c.r, a) + mul255(row[o + i_r], inv);
                row[o + i_g] = mul255(c.g, a) + mul255(row[o + i_g], inv);
                row[o + i_b] = mul255(c.b, a) + mul255(row[o + i_b], inv);
            }
        }
    }

    fn read_pixel(&self, x: usize, y: usize) -> Color {
        if x >= self.width || y >= self.height { return Color::rgb(0, 0, 0); }
        let off = y * self.stride_bytes + x * self.bpp;
        decode(&self.buf[off..off + self.bpp], self.format)
    }

    fn blend_pixel(&mut self, x: usize, y: usize, c: Color) {
        if c.a == 0 || !self.clip.contains(x, y) { return; }
        if c.a == 255 { return self.put_pixel(x, y, c); }
        let off = y * self.stride_bytes + x * self.bpp;
        let dst = decode(&self.buf[off..off + self.bpp], self.format);
        let mixed = c.over(dst);
        encode(&mut self.buf[off..off + self.bpp], self.format, mixed);
    }
}

#[inline(always)]
fn encode(slot: &mut [u8], fmt: PixelFormat, c: Color) {
    match fmt {
        PixelFormat::Rgb => { slot[0] = c.r; slot[1] = c.g; slot[2] = c.b; }
        PixelFormat::Bgr => { slot[0] = c.b; slot[1] = c.g; slot[2] = c.r; }
        PixelFormat::U8  => {
            slot[0] = ((c.r as u16 * 30 + c.g as u16 * 59 + c.b as u16 * 11) / 100) as u8;
        }
        // Unknown formats are guessed at as BGR, which is what every device
        // this kernel has met uses. Never past the end of the slot, though.
        _ if slot.len() >= 3 => { slot[0] = c.b; slot[1] = c.g; slot[2] = c.r; }
        _ => {}
    }
}

#[inline(always)]
fn decode(slot: &[u8], fmt: PixelFormat) -> Color {
    match fmt {
        PixelFormat::Rgb => Color::rgb(slot[0], slot[1], slot[2]),
        PixelFormat::Bgr => Color::rgb(slot[2], slot[1], slot[0]),
        PixelFormat::U8  => Color::rgb(slot[0], slot[0], slot[0]),
        _ if slot.len() >= 3 => Color::rgb(slot[2], slot[1], slot[0]),
        _ => Color::rgb(0, 0, 0),
    }
}
