use bootloader_api::info::{FrameBuffer as BootFb, FrameBufferInfo, PixelFormat};
use super::Color;

pub struct Framebuffer<'a> {
    buf: &'a mut [u8],
    pub width: usize,
    pub height: usize,
    stride_bytes: usize,
    bpp: usize,
    format: PixelFormat,
}

impl<'a> Framebuffer<'a> {
    pub fn from_boot(fb: &'a mut BootFb) -> Self {
        let info = fb.info();
        let buf = fb.buffer_mut();
        Self::from_raw(buf, info)
    }

    pub fn from_raw(buf: &'a mut [u8], info: FrameBufferInfo) -> Self {
        Self {
            buf,
            width: info.width,
            height: info.height,
            stride_bytes: info.stride * info.bytes_per_pixel,
            bpp: info.bytes_per_pixel,
            format: info.pixel_format,
        }
    }

    #[inline(always)]
    pub fn put_pixel(&mut self, x: usize, y: usize, c: Color) {
        if x >= self.width || y >= self.height { return; }
        let off = y * self.stride_bytes + x * self.bpp;
        encode(&mut self.buf[off..off + self.bpp], self.format, c);
    }

    pub fn read_pixel(&self, x: usize, y: usize) -> Color {
        if x >= self.width || y >= self.height { return Color::rgb(0, 0, 0); }
        let off = y * self.stride_bytes + x * self.bpp;
        decode(&self.buf[off..off + self.bpp], self.format)
    }

    pub fn blend_pixel(&mut self, x: usize, y: usize, c: Color) {
        if c.a == 0 || x >= self.width || y >= self.height { return; }
        if c.a == 255 { return self.put_pixel(x, y, c); }
        let off = y * self.stride_bytes + x * self.bpp;
        let dst = decode(&self.buf[off..off + self.bpp], self.format);
        let mixed = c.over(dst);
        encode(&mut self.buf[off..off + self.bpp], self.format, mixed);
    }

    pub fn clear(&mut self, c: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, c);
            }
        }
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
        _ => { slot[0] = c.b; slot[1] = c.g; slot[2] = c.r; }
    }
}

#[inline(always)]
fn decode(slot: &[u8], fmt: PixelFormat) -> Color {
    match fmt {
        PixelFormat::Rgb => Color::rgb(slot[0], slot[1], slot[2]),
        PixelFormat::Bgr => Color::rgb(slot[2], slot[1], slot[0]),
        PixelFormat::U8  => Color::rgb(slot[0], slot[0], slot[0]),
        _ => Color::rgb(slot[2], slot[1], slot[0]),
    }
}
