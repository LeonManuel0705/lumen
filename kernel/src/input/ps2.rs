use x86_64::instructions::port::Port;

const DATA: u16 = 0x60;
const STATUS_CMD: u16 = 0x64;

pub fn read_status() -> u8 {
    unsafe {
        let mut status: Port<u8> = Port::new(STATUS_CMD);
        status.read()
    }
}

pub fn read_data() -> u8 {
    unsafe {
        let mut data: Port<u8> = Port::new(DATA);
        data.read()
    }
}

pub fn init() {
    unsafe {
        let mut cmd: Port<u8> = Port::new(STATUS_CMD);
        let mut data: Port<u8> = Port::new(DATA);

        wait_write();
        cmd.write(0xAD);
        wait_write();
        cmd.write(0xA7);

        let mut drain = Port::<u8>::new(DATA);
        let mut status_in = Port::<u8>::new(STATUS_CMD);
        for _ in 0..32 {
            if status_in.read() & 0x01 == 0 { break; }
            let _ = drain.read();
        }

        wait_write();
        cmd.write(0x20);
        wait_read();
        let mut config = data.read();
        config |= 0b0100_0011;
        wait_write();
        cmd.write(0x60);
        wait_write();
        data.write(config);

        wait_write();
        cmd.write(0xAE);
        wait_write();
        cmd.write(0xA8);

        write_to_mouse(0xFF);
        for _ in 0..2 { let _ = read_aux_with_timeout(); }

        write_to_mouse(0xF6);
        write_to_mouse(0xF3);
        write_to_mouse(200);
        write_to_mouse(0xF4);
    }
}

unsafe fn wait_write() {
    let mut status: Port<u8> = Port::new(STATUS_CMD);
    for _ in 0..100_000 {
        if status.read() & 0x02 == 0 { return; }
    }
    crate::serial_println!("[ps2] timeout waiting for controller write-ready");
}

unsafe fn wait_read() {
    let mut status: Port<u8> = Port::new(STATUS_CMD);
    for _ in 0..100_000 {
        if status.read() & 0x01 != 0 { return; }
    }
    crate::serial_println!("[ps2] timeout waiting for controller data");
}

// Reads only bytes coming from the mouse (AUX status bit set), discarding any
// keyboard scancodes queued during boot so they can't desync the handshake.
unsafe fn read_aux_with_timeout() -> Option<u8> {
    let mut status: Port<u8> = Port::new(STATUS_CMD);
    let mut data: Port<u8> = Port::new(DATA);
    for _ in 0..100_000 {
        let st = status.read();
        if st & 0x01 != 0 {
            let byte = data.read();
            if st & 0x20 != 0 {
                return Some(byte);
            }
        }
    }
    None
}

unsafe fn write_to_mouse(byte: u8) {
    let mut cmd: Port<u8> = Port::new(STATUS_CMD);
    let mut data: Port<u8> = Port::new(DATA);
    wait_write();
    cmd.write(0xD4);
    wait_write();
    data.write(byte);
    let _ = read_aux_with_timeout();
}
