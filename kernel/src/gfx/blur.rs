#![allow(static_mut_refs)]

use super::{Color, Framebuffer};

const MAX_LINE: usize = 2048;

static mut SCRATCH_R: [u32; MAX_LINE] = [0; MAX_LINE];
static mut SCRATCH_G: [u32; MAX_LINE] = [0; MAX_LINE];
static mut SCRATCH_B: [u32; MAX_LINE] = [0; MAX_LINE];

pub fn box_blur_region(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, radius: usize, passes: usize) {
    if radius == 0 || passes == 0 || w <= 0 || h <= 0 { return; }
    let x_start = x.max(0) as usize;
    let y_start = y.max(0) as usize;
    let x_end = ((x + w).max(0) as usize).min(fb.width);
    let y_end = ((y + h).max(0) as usize).min(fb.height);
    if x_end <= x_start || y_end <= y_start { return; }

    let region_w = x_end - x_start;
    let region_h = y_end - y_start;
    if region_w > MAX_LINE || region_h > MAX_LINE {
        crate::serial_println!("[blur] region {}x{} exceeds MAX_LINE {}, skipping", region_w, region_h, MAX_LINE);
        return;
    }

    for _ in 0..passes {
        for ry in y_start..y_end {
            horizontal_pass(fb, x_start, ry, region_w, radius);
        }
        for rx in x_start..x_end {
            vertical_pass(fb, rx, y_start, region_h, radius);
        }
    }
}

fn horizontal_pass(fb: &mut Framebuffer, x_start: usize, y: usize, n: usize, radius: usize) {
    unsafe {
        for i in 0..n {
            let c = fb.read_pixel(x_start + i, y);
            SCRATCH_R[i] = c.r as u32;
            SCRATCH_G[i] = c.g as u32;
            SCRATCH_B[i] = c.b as u32;
        }
        run_blur(n, radius, |i, r, g, b| {
            fb.put_pixel(x_start + i, y, Color::rgb(r, g, b));
        });
    }
}

fn vertical_pass(fb: &mut Framebuffer, x: usize, y_start: usize, n: usize, radius: usize) {
    unsafe {
        for i in 0..n {
            let c = fb.read_pixel(x, y_start + i);
            SCRATCH_R[i] = c.r as u32;
            SCRATCH_G[i] = c.g as u32;
            SCRATCH_B[i] = c.b as u32;
        }
        run_blur(n, radius, |i, r, g, b| {
            fb.put_pixel(x, y_start + i, Color::rgb(r, g, b));
        });
    }
}

unsafe fn run_blur<F: FnMut(usize, u8, u8, u8)>(n: usize, radius: usize, mut write: F) {
    let r = radius;
    let mut sum_r = 0u32;
    let mut sum_g = 0u32;
    let mut sum_b = 0u32;
    let mut count = 0u32;
    let prefill = r.min(n.saturating_sub(1));
    for i in 0..=prefill {
        sum_r += SCRATCH_R[i];
        sum_g += SCRATCH_G[i];
        sum_b += SCRATCH_B[i];
        count += 1;
    }
    for i in 0..n {
        let avg_r = (sum_r / count) as u8;
        let avg_g = (sum_g / count) as u8;
        let avg_b = (sum_b / count) as u8;
        write(i, avg_r, avg_g, avg_b);
        let add_idx = i + r + 1;
        if add_idx < n {
            sum_r += SCRATCH_R[add_idx];
            sum_g += SCRATCH_G[add_idx];
            sum_b += SCRATCH_B[add_idx];
            count += 1;
        }
        if i >= r {
            let rem_idx = i - r;
            sum_r -= SCRATCH_R[rem_idx];
            sum_g -= SCRATCH_G[rem_idx];
            sum_b -= SCRATCH_B[rem_idx];
            count -= 1;
        }
    }
}
