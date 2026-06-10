pub mod keyboard;
pub mod mouse;
mod ps2;

pub fn init() {
    ps2::init();
}

pub struct Snapshot {
    pub mouse_dx: i32,
    pub mouse_dy: i32,
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
