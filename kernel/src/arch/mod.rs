pub mod gdt;
pub mod idt;
pub mod pic;
pub mod pit;
pub mod rtc;

pub const TICK_HZ: u32 = 60;

#[inline(always)]
pub fn cycles() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn init() {
    gdt::init();
    idt::init();
    unsafe {
        let mut pics = pic::PICS.lock();
        pics.initialize();
        pics.write_masks(0b1111_1000, 0b1110_1111);
    }
    pit::set_frequency(TICK_HZ);
    crate::input::init();
    x86_64::instructions::interrupts::enable();
}
