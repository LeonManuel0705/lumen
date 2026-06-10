use core::sync::atomic::{AtomicU64, Ordering};
use spin::Lazy;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use super::gdt;
use super::pic::{KEYBOARD_IRQ, MOUSE_IRQ, PICS, TIMER_IRQ};
use crate::input;
use crate::serial;

pub static TICKS: AtomicU64 = AtomicU64::new(0);

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.divide_error.set_handler_fn(divide_error_handler);
    idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
    idt.general_protection_fault.set_handler_fn(general_protection_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt[TIMER_IRQ].set_handler_fn(timer_handler);
    idt[KEYBOARD_IRQ].set_handler_fn(keyboard_handler);
    idt[MOUSE_IRQ].set_handler_fn(mouse_handler);
    idt
});

pub fn init() {
    IDT.load();
}

fn halt_forever() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack: InterruptStackFrame) {
    serial::force_print(format_args!(
        "[lumen] BREAKPOINT at {:?}\n",
        stack.instruction_pointer
    ));
}

extern "x86-interrupt" fn divide_error_handler(stack: InterruptStackFrame) {
    serial::force_print(format_args!("[lumen] DIVIDE ERROR\n{:#?}\n", stack));
    halt_forever();
}

extern "x86-interrupt" fn invalid_opcode_handler(stack: InterruptStackFrame) {
    serial::force_print(format_args!("[lumen] INVALID OPCODE\n{:#?}\n", stack));
    halt_forever();
}

extern "x86-interrupt" fn general_protection_handler(stack: InterruptStackFrame, err: u64) {
    serial::force_print(format_args!(
        "[lumen] GENERAL PROTECTION FAULT (error {:#x})\n{:#?}\n",
        err, stack
    ));
    halt_forever();
}

extern "x86-interrupt" fn page_fault_handler(stack: InterruptStackFrame, err: PageFaultErrorCode) {
    serial::force_print(format_args!(
        "[lumen] PAGE FAULT accessing {:#x} ({:?})\n{:#?}\n",
        Cr2::read_raw(),
        err,
        stack
    ));
    halt_forever();
}

extern "x86-interrupt" fn double_fault_handler(stack: InterruptStackFrame, _err: u64) -> ! {
    serial::force_print(format_args!("[lumen] DOUBLE FAULT\n{:#?}\n", stack));
    halt_forever();
}

extern "x86-interrupt" fn timer_handler(_stack: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { PICS.lock().notify_end_of_interrupt(TIMER_IRQ); }
}

extern "x86-interrupt" fn keyboard_handler(_stack: InterruptStackFrame) {
    input::dispatch_irq();
    unsafe { PICS.lock().notify_end_of_interrupt(KEYBOARD_IRQ); }
}

extern "x86-interrupt" fn mouse_handler(_stack: InterruptStackFrame) {
    input::dispatch_irq();
    unsafe { PICS.lock().notify_end_of_interrupt(MOUSE_IRQ); }
}
