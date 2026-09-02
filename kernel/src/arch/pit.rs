use x86_64::instructions::port::Port;

pub const PIT_BASE_FREQ: u32 = 1_193_182;

pub const fn divisor_for(hz: u32) -> u32 {
    let hz = if hz == 0 { 1 } else { hz };
    let d = PIT_BASE_FREQ / hz;
    if d < 1 {
        1
    } else if d > 0xFFFF {
        0xFFFF
    } else {
        d
    }
}

pub fn set_frequency(hz: u32) {
    let divisor = divisor_for(hz) as u16;
    let mut command: Port<u8> = Port::new(0x43);
    let mut data: Port<u8> = Port::new(0x40);
    unsafe {
        command.write(0x36);
        data.write((divisor & 0xFF) as u8);
        data.write((divisor >> 8) as u8);
    }
}
