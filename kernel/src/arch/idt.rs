use core::sync::atomic::{AtomicU64, Ordering};
use spin::Lazy;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use super::pic::{KEYBOARD_IRQ, MOUSE_IRQ, PICS, TIMER_IRQ};
use crate::input;

pub static TICKS: AtomicU64 = AtomicU64::new(0);

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.double_fault.set_handler_fn(double_fault_handler);
    idt[TIMER_IRQ].set_handler_fn(timer_handler);
    idt[KEYBOARD_IRQ].set_handler_fn(keyboard_handler);
    idt[MOUSE_IRQ].set_handler_fn(mouse_handler);
    idt
});

pub fn init() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(_stack: InterruptStackFrame) {}

extern "x86-interrupt" fn double_fault_handler(_stack: InterruptStackFrame, _err: u64) -> ! {
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn timer_handler(_stack: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { PICS.lock().notify_end_of_interrupt(TIMER_IRQ); }
}

extern "x86-interrupt" fn keyboard_handler(_stack: InterruptStackFrame) {
    input::keyboard::handle_irq();
    unsafe { PICS.lock().notify_end_of_interrupt(KEYBOARD_IRQ); }
}

extern "x86-interrupt" fn mouse_handler(_stack: InterruptStackFrame) {
    input::mouse::handle_irq();
    unsafe { PICS.lock().notify_end_of_interrupt(MOUSE_IRQ); }
}
