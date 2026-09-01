use alloc::vec;
use alloc::vec::Vec;
use bootloader_api::info::{FrameBuffer as BootFb, FrameBufferInfo};
use spin::Mutex;

use crate::gfx::{shapes, Canvas, Color, Framebuffer};

/// How far the wallpaper is smeared for the frosted-glass cache. Two passes of
/// a box blur at this radius is a close enough gaussian at this scale.
const GLASS_RADIUS: usize = 16;
const GLASS_PASSES: usize = 2;

struct Display {
    front: *mut u8,
    info: FrameBufferInfo,
    /// What the next frame is assembled in before it is shown.
    back: Vec<u8>,
    /// The wallpaper, drawn once, copied in at the start of every frame.
    background: Vec<u8>,
    /// The same wallpaper pre-blurred. Glass panels read from here instead of
    /// blurring the screen live, which is what keeps them cheap enough to move.
    blurred: Vec<u8>,
}

unsafe impl Send for Display {}

static DISPLAY: Mutex<Option<Display>> = Mutex::new(None);

/// The screen for one frame, plus the blurred wallpaper behind it.
pub struct Frame<'a> {
    target: Framebuffer<'a>,
    blurred: Framebuffer<'a>,
}

impl Frame<'_> {
    /// Lays the blurred wallpaper into a rounded rectangle. Everything a glass
    /// panel needs on top of that (tint, rim light, border) is drawn after.
    pub fn glass_backdrop(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.max(0).min(w / 2).min(h / 2);
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.target.width() as i32);
        let y1 = (y + h).min(self.target.height() as i32);

        for py in y0..y1 {
            for px in x0..x1 {
                let cov = shapes::rounded_coverage(px - x, py - y, w, h, r);
                if cov == 0 {
                    continue;
                }
                let sample = self.blurred.read_pixel(px as usize, py as usize);
                self.target
                    .blend_pixel(px as usize, py as usize, sample.with_alpha(cov));
            }
        }
    }
}

impl Canvas for Frame<'_> {
    fn width(&self) -> usize {
        self.target.width()
    }

    fn height(&self) -> usize {
        self.target.height()
    }

    fn put_pixel(&mut self, x: usize, y: usize, c: Color) {
        self.target.put_pixel(x, y, c);
    }

    fn blend_pixel(&mut self, x: usize, y: usize, c: Color) {
        self.target.blend_pixel(x, y, c);
    }

    fn read_pixel(&self, x: usize, y: usize) -> Color {
        self.target.read_pixel(x, y)
    }
}

pub fn init(fb: &'static mut BootFb) {
    let info = fb.info();
    let buf = fb.buffer_mut();
    let len = buf.len();
    let front = buf.as_mut_ptr();

    *DISPLAY.lock() = Some(Display {
        front,
        info,
        back: vec![0; len],
        background: vec![0; len],
        blurred: vec![0; len],
    });
    crate::serial_println!(
        "[display] {} KiB of buffers on the heap ({} bytes per frame)",
        len * 3 / 1024,
        len
    );
}

pub fn dimensions() -> Option<(usize, usize)> {
    DISPLAY.lock().as_ref().map(|d| (d.info.width, d.info.height))
}

/// Draws the wallpaper once and refreshes the blurred copy the glass reads
/// from. Expensive on purpose: it happens at boot, not per frame.
pub fn bake_background<F: FnOnce(&mut Framebuffer)>(f: F) {
    let mut guard = DISPLAY.lock();
    let display = match guard.as_mut() {
        Some(d) => d,
        None => return,
    };
    let info = display.info;
    {
        let mut fb = Framebuffer::from_raw(&mut display.background, info);
        f(&mut fb);
    }
    display.blurred.copy_from_slice(&display.background);
    let mut blurred = Framebuffer::from_raw(&mut display.blurred, info);
    let (w, h) = (blurred.width() as i32, blurred.height() as i32);
    crate::gfx::blur::box_blur_region(&mut blurred, 0, 0, w, h, GLASS_RADIUS, GLASS_PASSES);
}

pub fn render<F: FnOnce(&mut Frame)>(f: F) {
    let mut guard = DISPLAY.lock();
    let display = match guard.as_mut() {
        Some(d) => d,
        None => return,
    };
    let info = display.info;

    display.back.copy_from_slice(&display.background);

    {
        let mut frame = Frame {
            target: Framebuffer::from_raw(&mut display.back, info),
            blurred: Framebuffer::from_raw(&mut display.blurred, info),
        };
        f(&mut frame);
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            display.back.as_ptr(),
            display.front,
            display.back.len(),
        );
    }
}
