#![allow(static_mut_refs)]

use bootloader_api::info::{FrameBuffer as BootFb, FrameBufferInfo};
use spin::Mutex;

use crate::gfx::Framebuffer;

const MAX_BYTES: usize = 1920 * 1200 * 4;

#[repr(align(64))]
struct AlignedBuffer([u8; MAX_BYTES]);

static mut BACK_BUFFER: AlignedBuffer = AlignedBuffer([0; MAX_BYTES]);
static mut BG_CACHE:    AlignedBuffer = AlignedBuffer([0; MAX_BYTES]);

struct Display {
    front: *mut u8,
    front_len: usize,
    info: FrameBufferInfo,
}

unsafe impl Send for Display {}

static DISPLAY: Mutex<Option<Display>> = Mutex::new(None);

pub fn init(fb: &'static mut BootFb) {
    let info = fb.info();
    let buf = fb.buffer_mut();
    let front = buf.as_mut_ptr();
    let front_len = buf.len();
    if front_len > MAX_BYTES {
        crate::serial_println!(
            "[display] framebuffer is {} bytes but MAX_BYTES is {}, bottom rows will not render",
            front_len,
            MAX_BYTES
        );
    }
    *DISPLAY.lock() = Some(Display { front, front_len, info });
}

pub fn dimensions() -> Option<(usize, usize)> {
    DISPLAY.lock().as_ref().map(|d| (d.info.width, d.info.height))
}

pub fn cache_background<F: FnOnce(&mut Framebuffer)>(f: F) {
    let guard = DISPLAY.lock();
    let display = match guard.as_ref() { Some(d) => d, None => return };
    let bytes = display.front_len.min(MAX_BYTES);
    let bg_slice = unsafe { core::slice::from_raw_parts_mut(BG_CACHE.0.as_mut_ptr(), bytes) };
    let mut fb = Framebuffer::from_raw(bg_slice, display.info);
    f(&mut fb);
}

pub fn render<F: FnOnce(&mut Framebuffer)>(f: F) {
    let guard = DISPLAY.lock();
    let display = match guard.as_ref() { Some(d) => d, None => return };
    let bytes = display.front_len.min(MAX_BYTES);

    unsafe {
        core::ptr::copy_nonoverlapping(BG_CACHE.0.as_ptr(), BACK_BUFFER.0.as_mut_ptr(), bytes);
    }

    let back_slice = unsafe { core::slice::from_raw_parts_mut(BACK_BUFFER.0.as_mut_ptr(), bytes) };
    {
        let mut fb = Framebuffer::from_raw(back_slice, display.info);
        f(&mut fb);
    }

    unsafe {
        core::ptr::copy_nonoverlapping(BACK_BUFFER.0.as_ptr(), display.front, bytes);
    }
}
