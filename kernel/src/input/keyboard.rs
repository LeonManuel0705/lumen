use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use spin::{Lazy, Mutex};

#[derive(Default, Copy, Clone)]
pub struct Keys {
    pub space: bool,
    pub r: bool,
}

struct State {
    pending: Keys,
    kbd: Keyboard<layouts::Us104Key, ScancodeSet1>,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    Mutex::new(State {
        pending: Keys::default(),
        kbd: Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore),
    })
});

// Force the Lazy before interrupts are enabled, so its one-time init can never
// race an IRQ landing mid-initialization.
pub fn init() {
    Lazy::force(&STATE);
}

pub fn process_byte(scancode: u8) {
    let mut s = STATE.lock();
    if let Ok(Some(event)) = s.kbd.add_byte(scancode) {
        if let Some(key) = s.kbd.process_keyevent(event) {
            match key {
                DecodedKey::Unicode(' ') => s.pending.space = true,
                DecodedKey::Unicode('r') | DecodedKey::Unicode('R') => s.pending.r = true,
                DecodedKey::RawKey(KeyCode::Spacebar) => s.pending.space = true,
                _ => {}
            }
        }
    }
}

pub fn take_keys() -> Keys {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut s = STATE.lock();
        let k = s.pending;
        s.pending = Keys::default();
        k
    })
}
