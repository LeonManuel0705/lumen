pub mod keyboard;
pub mod mouse;
mod ps2;

pub fn init() {
    ps2::init();
    keyboard::init();
}

pub fn dispatch_irq() {
    for _ in 0..16 {
        let status = ps2::read_status();
        if status & 0x01 == 0 {
            return;
        }
        let byte = ps2::read_data();
        if status & 0x20 != 0 {
            mouse::process_byte(byte);
        } else {
            keyboard::process_byte(byte);
        }
    }
}

pub use keyboard::{Key, KeyBatch};

pub struct Snapshot {
    pub mouse_dx: i32,
    pub mouse_dy: i32,
    pub buttons: u8,
    pub buttons_just_pressed: u8,
    pub keys: KeyBatch,
}

pub fn snapshot() -> Snapshot {
    let m = mouse::take_delta();
    Snapshot {
        mouse_dx: m.dx,
        mouse_dy: m.dy,
        buttons: m.buttons,
        buttons_just_pressed: m.just_pressed,
        keys: keyboard::take_events(),
    }
}
