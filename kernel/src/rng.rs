use core::sync::atomic::{AtomicU32, Ordering};

static STATE: AtomicU32 = AtomicU32::new(0x4C55_4D45);

pub fn next() -> u32 {
    let mut x = STATE.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    STATE.store(x, Ordering::Relaxed);
    x
}

pub fn sign() -> f32 {
    if next() & 1 == 0 {
        1.0
    } else {
        -1.0
    }
}
