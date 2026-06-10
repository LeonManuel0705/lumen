#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod anim;
mod arch;
mod display;
mod gfx;
mod input;
mod rng;
mod scene;
mod serial;

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use core::sync::atomic::Ordering;

use crate::arch::idt::TICKS;
use crate::input::Snapshot;
use crate::scene::Scene;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial::init();
    serial_println!("[lumen] serial up");

    if let Some(boot_fb) = boot_info.framebuffer.as_mut() {
        let info = boot_fb.info();
        serial_println!(
            "[lumen] framebuffer {}x{} ({:?}, {} bpp)",
            info.width,
            info.height,
            info.pixel_format,
            info.bytes_per_pixel
        );
        display::init(boot_fb);
    } else {
        serial_println!("[lumen] no framebuffer provided by bootloader");
    }

    arch::init();
    serial_println!("[lumen] arch ready (gdt/idt/pic/pit/ps2)");

    let (width, height) = display::dimensions().unwrap_or((1280, 720));
    let mut scene = Scene::new(width, height);
    display::cache_background(|fb| scene.draw_background(fb));

    serial_println!("[lumen] entering main loop");

    let dt = 1.0 / arch::TICK_HZ as f32;
    let mut last_seen: u64 = 0;

    let empty = Snapshot {
        mouse_dx: 0,
        mouse_dy: 0,
        buttons: 0,
        buttons_just_pressed: 0,
        key_pressed_space: false,
        key_pressed_r: false,
    };

    loop {
        let now = TICKS.load(Ordering::Relaxed);
        let mut steps = (now - last_seen).min(8);
        if steps == 0 {
            x86_64::instructions::interrupts::enable_and_hlt();
            continue;
        }

        let real_input = input::snapshot();
        let mut applied = false;
        while steps > 0 {
            let snap = if !applied { applied = true; &real_input } else { &empty };
            scene.update(dt, snap);
            steps -= 1;
        }
        last_seen = now;

        display::render(|fb| scene.draw(fb));

        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial::force_print(format_args!("\n[lumen] KERNEL PANIC: {}\n", info));
    loop {
        x86_64::instructions::hlt();
    }
}
