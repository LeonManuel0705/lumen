use pic8259::ChainedPics;
use spin::Mutex;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe {
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
});

pub const TIMER_IRQ: u8 = PIC_1_OFFSET;
pub const KEYBOARD_IRQ: u8 = PIC_1_OFFSET + 1;
pub const MOUSE_IRQ: u8 = PIC_2_OFFSET + 4;
