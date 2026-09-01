use alloc::vec;
use alloc::vec::Vec;
use bootloader_api::info::{FrameBuffer as BootFb, FrameBufferInfo, PixelFormat as FbFormat};
use spin::Mutex;

use crate::gfx::{shapes, Canvas, Color, Damage, Framebuffer, Rect};

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
    /// Lays the blurred wallpaper into a rounded rectangle and tints it in the
    /// same pass. The rim light and border go on top afterwards.
    ///
    /// The inside of the shape is the hot loop of the whole compositor, so the
    /// tint is folded into three 256-entry tables first: every interior pixel
    /// is then three table lookups and three byte writes, with no blending
    /// arithmetic and no per-pixel format decision at all. Only the
    /// antialiased rim takes the general path.
    pub fn glass_fill(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        tint: Color,
        opacity: u8,
    ) {
        if opacity == 0 {
            return;
        }
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.max(0).min(w / 2).min(h / 2);
        let clip = self.target.clip();
        let x0 = x.max(clip.x0);
        let y0 = y.max(clip.y0);
        let x1 = (x + w).min(clip.x1);
        let y1 = (y + h).min(clip.y1);
        if x1 <= x0 || y1 <= y0 {
            return;
        }

        let lut = TintTable::new(tint, self.target.format());
        let bpp = self.target.bytes_per_pixel();

        for py in y0..y1 {
            // Inside the vertical straight section every pixel between the
            // corners is fully covered, so the run can be done wholesale.
            let ly = py - y;
            let (run_start, run_end) = if ly >= r && ly < h - r {
                (x0, x1)
            } else {
                let inset = shapes::corner_inset(ly, h, r);
                ((x + inset).max(x0), (x + w - inset).min(x1))
            };

            for px in x0..run_start {
                self.glass_pixel(px, py, x, y, w, h, r, tint, opacity);
            }
            if run_end > run_start {
                if opacity == 255 {
                    let src = self.blurred.row(py as usize);
                    let dst = self.target.row_mut(py as usize);
                    let from = run_start as usize * bpp;
                    let to = run_end as usize * bpp;
                    lut.apply(&src[from..to], &mut dst[from..to], bpp);
                } else {
                    // A panel that is still fading in has to blend against what
                    // is behind it, so the table path cannot be used.
                    for px in run_start..run_end {
                        self.glass_pixel(px, py, x, y, w, h, r, tint, opacity);
                    }
                }
            }
            for px in run_end.max(run_start)..x1 {
                self.glass_pixel(px, py, x, y, w, h, r, tint, opacity);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn glass_pixel(
        &mut self,
        px: i32,
        py: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        tint: Color,
        opacity: u8,
    ) {
        let cov = shapes::rounded_coverage(px - x, py - y, w, h, r);
        if cov == 0 {
            return;
        }
        let alpha = crate::gfx::color::mul255(cov, opacity);
        if alpha == 0 {
            return;
        }
        let sample = self.blurred.read_pixel(px as usize, py as usize);
        let glass = tint.over(sample);
        self.target.paint(px as usize, py as usize, glass.fade(alpha));
    }
}

/// Per-channel tables mapping a wallpaper byte to the tinted glass byte.
struct TintTable {
    table: [[u8; 256]; 3],
}

impl TintTable {
    fn new(tint: Color, format: FbFormat) -> Self {
        let channels = match format {
            FbFormat::Rgb => [tint.r, tint.g, tint.b],
            _ => [tint.b, tint.g, tint.r],
        };
        let mut table = [[0u8; 256]; 3];
        let a = tint.a;
        let inv = 255 - a;
        for (slot, channel) in table.iter_mut().zip(channels) {
            let base = crate::gfx::color::mul255(channel, a);
            for (v, out) in slot.iter_mut().enumerate() {
                *out = base + crate::gfx::color::mul255(v as u8, inv);
            }
        }
        Self { table }
    }

    #[inline(always)]
    fn apply(&self, src: &[u8], dst: &mut [u8], bpp: usize) {
        let n = src.len().min(dst.len()) / bpp;
        for i in 0..n {
            let o = i * bpp;
            dst[o] = self.table[0][src[o] as usize];
            dst[o + 1] = self.table[1][src[o + 1] as usize];
            dst[o + 2] = self.table[2][src[o + 2] as usize];
        }
    }
}

impl Canvas for Frame<'_> {
    fn width(&self) -> usize {
        self.target.width()
    }

    fn clip(&self) -> Rect {
        self.target.clip()
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

    fn blend_row(&mut self, x: usize, y: usize, src: &[Color], opacity: u8) {
        self.target.blend_row(x, y, src, opacity);
    }

    fn fill_row(&mut self, x: usize, y: usize, len: usize, c: Color) {
        self.target.fill_row(x, y, len, c);
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

/// Repaints only the damaged regions and shows only those.
///
/// The back buffer is never cleared between frames: whatever was presented last
/// frame is still in it, so a region that nobody damaged is already correct.
/// Each damaged region is restored from the wallpaper, redrawn with the clip
/// set to that region, and copied out to the screen.
pub fn render<F: Fn(&mut Frame)>(damage: &Damage, f: F) {
    let mut guard = DISPLAY.lock();
    let display = match guard.as_mut() {
        Some(d) => d,
        None => return,
    };
    let info = display.info;
    let stride = info.stride * info.bytes_per_pixel;
    let bpp = info.bytes_per_pixel;

    for region in damage.regions() {
        let region = *region;
        if region.is_empty() {
            continue;
        }
        let x_off = region.x0 as usize * bpp;
        let run = (region.x1 - region.x0) as usize * bpp;

        for py in region.y0 as usize..region.y1 as usize {
            let off = py * stride + x_off;
            if off + run > display.back.len() {
                break;
            }
            display.back[off..off + run].copy_from_slice(&display.background[off..off + run]);
        }

        {
            let mut target = Framebuffer::from_raw(&mut display.back, info);
            target.set_clip(region);
            let mut frame = Frame {
                target,
                blurred: Framebuffer::from_raw(&mut display.blurred, info),
            };
            f(&mut frame);
        }

        for py in region.y0 as usize..region.y1 as usize {
            let off = py * stride + x_off;
            if off + run > display.back.len() {
                break;
            }
            unsafe {
                core::ptr::copy_nonoverlapping(
                    display.back.as_ptr().add(off),
                    display.front.add(off),
                    run,
                );
            }
        }
    }
}
