pub mod idt;
pub mod pic;
pub mod pit;

pub fn init() {
    idt::init();
    unsafe { pic::PICS.lock().initialize(); }
    pit::set_frequency(60);
    crate::input::init();
    x86_64::instructions::interrupts::enable();
}
