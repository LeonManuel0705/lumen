use spin::Mutex;
use x86_64::instructions::port::Port;

#[derive(Default, Copy, Clone)]
pub struct Delta {
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
    pub just_pressed: u8,
}

struct State {
    phase: u8,
    bytes: [u8; 3],
    accum_dx: i32,
    accum_dy: i32,
    last_buttons: u8,
    just_pressed: u8,
    cur_buttons: u8,
}

static STATE: Mutex<State> = Mutex::new(State {
    phase: 0,
    bytes: [0; 3],
    accum_dx: 0,
    accum_dy: 0,
    last_buttons: 0,
    just_pressed: 0,
    cur_buttons: 0,
});

pub fn handle_irq() {
    let byte = unsafe {
        let mut p: Port<u8> = Port::new(0x60);
        p.read()
    };
    let mut s = STATE.lock();

    if s.phase == 0 && (byte & 0x08) == 0 {
        return;
    }

    let phase = s.phase as usize;
    s.bytes[phase] = byte;
    s.phase += 1;
    if s.phase < 3 { return; }

    let b0 = s.bytes[0];
    let b1 = s.bytes[1];
    let b2 = s.bytes[2];

    let dx = if b0 & 0x10 != 0 { b1 as i32 - 256 } else { b1 as i32 };
    let dy = if b0 & 0x20 != 0 { b2 as i32 - 256 } else { b2 as i32 };

    s.accum_dx += dx;
    s.accum_dy -= dy;

    let buttons = b0 & 0x07;
    let pressed_now = buttons & !s.cur_buttons;
    s.just_pressed |= pressed_now;
    s.cur_buttons = buttons;

    s.phase = 0;
}

pub fn take_delta() -> Delta {
    let mut s = STATE.lock();
    let out = Delta {
        dx: s.accum_dx,
        dy: s.accum_dy,
        buttons: s.cur_buttons,
        just_pressed: s.just_pressed,
    };
    s.accum_dx = 0;
    s.accum_dy = 0;
    s.last_buttons = s.cur_buttons;
    s.just_pressed = 0;
    out
}
