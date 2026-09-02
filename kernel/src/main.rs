#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod anim;
mod apps;
mod arch;
mod datefmt;
mod display;
mod gfx;
mod input;
mod mem;
mod rng;
mod scene;
mod serial;
mod widgets;
mod wm;

use bootloader_api::config::{BootloaderConfig, Mapping};
use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use core::sync::atomic::Ordering;

use crate::arch::idt::TICKS;
use crate::input::Snapshot;
use crate::scene::Scene;

static BOOT_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 256 * 1024;
    config
};

entry_point!(kernel_main, config = &BOOT_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial::init();
    serial_println!("[lumen] serial up");

    match mem::init(boot_info) {
        Ok(()) => mem::selftest(),
        Err(err) => panic!("memory init failed: {}", err),
    }

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

    let boot_time = arch::rtc::read_at_edge();
    serial_println!(
        "[lumen] rtc {:02}:{:02}:{:02}",
        boot_time.hours,
        boot_time.minutes,
        boot_time.seconds
    );
    let mut day_base = day_seconds(&boot_time);
    let mut tick_origin = TICKS.load(Ordering::Relaxed);
    let mut last_sync = tick_origin;

    let boot_date = arch::rtc::read_date();
    let mut date_buf = [0u8; 48];
    let date_len = datefmt::format_german(&boot_date, &mut date_buf);
    serial_println!(
        "[lumen] date {:04}-{:02}-{:02} ({})",
        boot_date.year,
        boot_date.month,
        boot_date.day,
        core::str::from_utf8(&date_buf[..date_len]).unwrap_or("?")
    );

    let (width, height) = display::dimensions().unwrap_or((1280, 720));
    let mut scene = Scene::new(width, height);
    scene.set_clock(day_base);
    scene.set_date(&date_buf[..date_len]);
    serial_println!("[lumen] baking wallpaper and glass cache");
    display::bake_background(|fb| scene.draw_background(fb));
    scene.repaint_everything();
    let mut previous_damage = gfx::Damage::new(width as i32, height as i32);

    serial_println!("[lumen] entering main loop");

    const VERIFY_EVERY: u32 = 120;

    let dt = 1.0 / arch::TICK_HZ as f32;
    let mut last_seen: u64 = 0;
    let mut last_shown = day_base;
    let mut frames: u32 = 0;
    let mut fps_window = TICKS.load(Ordering::Relaxed);
    let mut busy_cycles: u64 = 0;
    let mut verify_cycles: u64 = 0;
    let mut window_start = arch::cycles();

    loop {
        let now = TICKS.load(Ordering::Relaxed);
        let mut steps = (now - last_seen).min(8);
        if steps == 0 {
            x86_64::instructions::interrupts::enable_and_hlt();
            continue;
        }

        if now - last_sync >= 300 * arch::TICK_HZ as u64 {
            last_sync = now;
            let t = arch::rtc::read();
            let rtc_day = day_seconds(&t);
            let shown = (day_base + ticks_to_secs(now - tick_origin) as u32) % 86_400;
            let diff = (rtc_day as i64 - shown as i64 + 43_200).rem_euclid(86_400) - 43_200;
            if diff.abs() >= 2 {
                serial_println!("[lumen] clock resync: {}s off, rebasing to rtc", diff);
                day_base = rtc_day;
                tick_origin = now;
            }
            refresh_date(&mut scene, &mut date_buf);
        }
        let shown = (day_base + ticks_to_secs(now - tick_origin) as u32) % 86_400;
        if shown < last_shown {
            refresh_date(&mut scene, &mut date_buf);
        }
        last_shown = shown;
        scene.set_clock(shown);

        let frame_start = arch::cycles();
        let real_input = input::snapshot();
        let carry = Snapshot {
            mouse_dx: 0,
            mouse_dy: 0,
            buttons: real_input.buttons,
            buttons_just_pressed: 0,
            keys: input::KeyBatch::EMPTY,
        };
        let mut applied = false;
        while steps > 0 {
            let snap = if !applied { applied = true; &real_input } else { &carry };
            scene.update(dt, snap);
            steps -= 1;
        }
        last_seen = now;

        let damage = scene.finish_frame();
        let mut repaint = damage;
        repaint.add_all(&previous_damage);
        previous_damage = damage;
        if !repaint.is_empty() {
            display::render(&repaint, |fb| scene.draw(fb));
        }
        busy_cycles += arch::cycles() - frame_start;

        if VERIFY_EVERY > 0 && frames % VERIFY_EVERY == VERIFY_EVERY - 1 {
            let started = arch::cycles();
            if let Some((x, y)) = display::verify(|fb| scene.draw(fb)) {
                serial_println!("[lumen] stale pixel at {},{}: damage missed a region", x, y);
            }
            verify_cycles += arch::cycles() - started;
        }

        frames += 1;
        if now - fps_window >= 10 * arch::TICK_HZ as u64 {
            let secs = ticks_to_secs_f32(now - fps_window);
            let elapsed = (arch::cycles() - window_start).saturating_sub(verify_cycles);
            serial_println!(
                "[lumen] {} fps, {}% busy",
                (frames as f32 / secs) as u32,
                busy_cycles * 100 / elapsed.max(1)
            );
            frames = 0;
            busy_cycles = 0;
            verify_cycles = 0;
            fps_window = now;
            window_start = arch::cycles();
        }

        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

fn refresh_date(scene: &mut Scene, buf: &mut [u8; 48]) {
    let d = arch::rtc::read_date();
    let n = datefmt::format_german(&d, buf);
    scene.set_date(&buf[..n]);
}

fn day_seconds(t: &arch::rtc::Time) -> u32 {
    t.hours as u32 * 3600 + t.minutes as u32 * 60 + t.seconds as u32
}

fn ticks_to_secs_f32(ticks: u64) -> f32 {
    ticks as f32 * arch::pit::divisor_for(arch::TICK_HZ) as f32 / arch::pit::PIT_BASE_FREQ as f32
}

fn ticks_to_secs(ticks: u64) -> u64 {
    ticks * arch::pit::divisor_for(arch::TICK_HZ) as u64 / arch::pit::PIT_BASE_FREQ as u64
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial::force_print(format_args!("\n[lumen] KERNEL PANIC: {}\n", info));
    loop {
        x86_64::instructions::hlt();
    }
}
