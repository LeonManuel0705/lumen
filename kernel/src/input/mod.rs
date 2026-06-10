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

pub struct Snapshot {
    pub mouse_dx: i32,
    pub mouse_dy: i32,
    #[allow(dead_code)]
    pub buttons: u8,
    pub buttons_just_pressed: u8,
    pub key_pressed_space: bool,
    pub key_pressed_r: bool,
}

pub fn snapshot() -> Snapshot {
    let m = mouse::take_delta();
    let k = keyboard::take_keys();
    Snapshot {
        mouse_dx: m.dx,
        mouse_dy: m.dy,
        buttons: m.buttons,
        buttons_just_pressed: m.just_pressed,
        key_pressed_space: k.space,
        key_pressed_r: k.r,
    }
}
