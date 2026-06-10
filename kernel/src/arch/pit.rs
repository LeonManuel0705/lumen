use x86_64::instructions::port::Port;

const PIT_BASE_FREQ: u32 = 1_193_182;

pub fn set_frequency(hz: u32) {
    let divisor = (PIT_BASE_FREQ / hz.max(1)).clamp(1, 0xFFFF) as u16;
    let mut command: Port<u8> = Port::new(0x43);
    let mut data: Port<u8> = Port::new(0x40);
    unsafe {
        command.write(0x36);
        data.write((divisor & 0xFF) as u8);
        data.write((divisor >> 8) as u8);
    }
}
