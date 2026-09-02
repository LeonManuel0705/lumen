use core::fmt::{self, Write};
use spin::Mutex;
use x86_64::instructions::port::Port;

const COM1: u16 = 0x3F8;

pub struct SerialPort;

impl SerialPort {
    fn write_byte(&mut self, byte: u8) {
        unsafe {
            let mut lsr: Port<u8> = Port::new(COM1 + 5);
            let mut thr: Port<u8> = Port::new(COM1);
            for _ in 0..100_000 {
                if lsr.read() & 0x20 != 0 {
                    break;
                }
            }
            thr.write(byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(b);
        }
        Ok(())
    }
}

static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort);

pub fn init() {
    unsafe {
        let mut int_enable: Port<u8> = Port::new(COM1 + 1);
        let mut line_ctrl: Port<u8> = Port::new(COM1 + 3);
        let mut data: Port<u8> = Port::new(COM1);
        let mut fifo_ctrl: Port<u8> = Port::new(COM1 + 2);
        let mut modem_ctrl: Port<u8> = Port::new(COM1 + 4);

        int_enable.write(0x00);
        line_ctrl.write(0x80);
        data.write(0x03);
        int_enable.write(0x00);
        line_ctrl.write(0x03);
        fifo_ctrl.write(0xC7);
        modem_ctrl.write(0x0B);
    }
}

pub fn print(args: fmt::Arguments) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let _ = SERIAL.lock().write_fmt(args);
    });
}

pub fn force_print(args: fmt::Arguments) {
    unsafe {
        SERIAL.force_unlock();
    }
    let _ = SERIAL.lock().write_fmt(args);
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial::print(format_args!("\n")));
    ($($arg:tt)*) => ($crate::serial::print(format_args!("{}\n", format_args!($($arg)*))));
}
