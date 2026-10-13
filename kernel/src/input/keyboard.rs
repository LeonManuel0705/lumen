use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::{Lazy, Mutex};

const QUEUE: usize = 32;

pub const MAX_FRAME_KEYS: usize = 12;

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Space,
    Enter,
    Backspace,
    Escape,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Other,
}

#[derive(Copy, Clone)]
pub struct KeyEvent {
    pub key: Key,
    #[allow(dead_code)]
    pub text: Option<char>,
    pub pressed: bool,
    pub mods: Mods,
}

impl KeyEvent {
    const NONE: KeyEvent = KeyEvent {
        key: Key::Other,
        text: None,
        pressed: false,
        mods: Mods { shift: false, ctrl: false, alt: false },
    };
}

#[derive(Copy, Clone)]
pub struct KeyBatch {
    events: [KeyEvent; MAX_FRAME_KEYS],
    len: usize,
}

impl KeyBatch {
    pub const EMPTY: KeyBatch = KeyBatch {
        events: [KeyEvent::NONE; MAX_FRAME_KEYS],
        len: 0,
    };

    pub fn iter(&self) -> core::slice::Iter<'_, KeyEvent> {
        self.events[..self.len].iter()
    }

    pub fn pressed(&self, key: Key) -> bool {
        self.iter().any(|e| e.pressed && e.key == key)
    }

    pub fn push(&mut self, event: KeyEvent) {
        if self.len < MAX_FRAME_KEYS {
            self.events[self.len] = event;
            self.len += 1;
        }
    }
}

struct State {
    kbd: Keyboard<layouts::Us104Key, ScancodeSet1>,
    mods: Mods,
    queue: [KeyEvent; QUEUE],
    head: usize,
    len: usize,
    dropped: u32,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    Mutex::new(State {
        kbd: Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore),
        mods: Mods::default(),
        queue: [KeyEvent::NONE; QUEUE],
        head: 0,
        len: 0,
        dropped: 0,
    })
});

pub fn init() {
    Lazy::force(&STATE);
}

pub fn process_byte(scancode: u8) {
    let mut s = STATE.lock();
    let Ok(Some(event)) = s.kbd.add_byte(scancode) else {
        return;
    };
    let code = event.code;
    let pressed = !matches!(event.state, KeyState::Up);

    match code {
        KeyCode::LShift | KeyCode::RShift => s.mods.shift = pressed,
        KeyCode::LControl | KeyCode::RControl => s.mods.ctrl = pressed,
        KeyCode::LAlt | KeyCode::RAltGr => s.mods.alt = pressed,
        _ => {}
    }

    let text = match s.kbd.process_keyevent(event) {
        Some(DecodedKey::Unicode(c)) if !c.is_control() => Some(c),
        _ => None,
    };

    let mods = s.mods;
    s.push(KeyEvent { key: key_from_code(code), text, pressed, mods });
}

impl State {
    fn push(&mut self, event: KeyEvent) {
        if self.len == QUEUE {
            self.dropped += 1;
            return;
        }
        let slot = (self.head + self.len) % QUEUE;
        self.queue[slot] = event;
        self.len += 1;
    }
}

pub fn take_events() -> KeyBatch {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut s = STATE.lock();
        let mut batch = KeyBatch::EMPTY;
        while s.len > 0 && batch.len < MAX_FRAME_KEYS {
            let event = s.queue[s.head];
            s.head = (s.head + 1) % QUEUE;
            s.len -= 1;
            batch.push(event);
        }
        if s.dropped > 0 {
            crate::serial_println!("[input] dropped {} keystrokes, queue overran", s.dropped);
            s.dropped = 0;
        }
        batch
    })
}

fn key_from_code(code: KeyCode) -> Key {
    use KeyCode::*;
    match code {
        Escape => Key::Escape,
        Backspace => Key::Backspace,
        Tab => Key::Tab,
        Return | NumpadEnter => Key::Enter,
        Spacebar => Key::Space,
        ArrowLeft => Key::Left,
        ArrowRight => Key::Right,
        ArrowUp => Key::Up,
        ArrowDown => Key::Down,
        A => Key::Char('a'), B => Key::Char('b'), C => Key::Char('c'), D => Key::Char('d'),
        E => Key::Char('e'), F => Key::Char('f'), G => Key::Char('g'), H => Key::Char('h'),
        I => Key::Char('i'), J => Key::Char('j'), K => Key::Char('k'), L => Key::Char('l'),
        M => Key::Char('m'), N => Key::Char('n'), O => Key::Char('o'), P => Key::Char('p'),
        Q => Key::Char('q'), R => Key::Char('r'), S => Key::Char('s'), T => Key::Char('t'),
        U => Key::Char('u'), V => Key::Char('v'), W => Key::Char('w'), X => Key::Char('x'),
        Y => Key::Char('y'), Z => Key::Char('z'),
        Key1 | Numpad1 => Key::Char('1'), Key2 | Numpad2 => Key::Char('2'),
        Key3 | Numpad3 => Key::Char('3'), Key4 | Numpad4 => Key::Char('4'),
        Key5 | Numpad5 => Key::Char('5'), Key6 | Numpad6 => Key::Char('6'),
        Key7 | Numpad7 => Key::Char('7'), Key8 | Numpad8 => Key::Char('8'),
        Key9 | Numpad9 => Key::Char('9'), Key0 | Numpad0 => Key::Char('0'),
        _ => Key::Other,
    }
}
