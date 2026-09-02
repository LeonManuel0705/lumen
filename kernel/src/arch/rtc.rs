use x86_64::instructions::port::Port;

pub struct Time {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

pub struct Date {
    pub year: u32,
    pub month: u8,
    pub day: u8,
}

pub fn read_date() -> Date {
    let mut prev = read_date_raw();
    for _ in 0..16 {
        let cur = read_date_raw();
        if cur == prev {
            return decode_date(cur);
        }
        prev = cur;
    }
    decode_date(prev)
}

fn read_date_raw() -> (u8, u8, u8, u8) {
    for _ in 0..1_000_000 {
        if reg(0x0A) & 0x80 == 0 {
            break;
        }
    }
    (reg(0x07), reg(0x08), reg(0x09), reg(0x0B))
}

fn decode_date((d, m, y, status_b): (u8, u8, u8, u8)) -> Date {
    let bcd = status_b & 0x04 == 0;
    let day = if bcd { from_bcd(d) } else { d };
    let month = if bcd { from_bcd(m) } else { m };
    let year = if bcd { from_bcd(y) } else { y };
    Date {
        year: 2000 + year as u32,
        month: month.clamp(1, 12),
        day: day.clamp(1, 31),
    }
}

pub fn read_at_edge() -> Time {
    let start = reg(0x00);
    for _ in 0..2_000_000 {
        if reg(0x00) != start {
            break;
        }
    }
    read()
}

pub fn read() -> Time {
    let mut prev = read_raw();
    for _ in 0..16 {
        let cur = read_raw();
        if cur == prev {
            return decode(cur);
        }
        prev = cur;
    }
    decode(prev)
}

fn read_raw() -> (u8, u8, u8, u8) {
    for _ in 0..1_000_000 {
        if reg(0x0A) & 0x80 == 0 {
            break;
        }
    }
    (reg(0x00), reg(0x02), reg(0x04), reg(0x0B))
}

fn reg(idx: u8) -> u8 {
    unsafe {
        let mut sel: Port<u8> = Port::new(0x70);
        let mut data: Port<u8> = Port::new(0x71);
        sel.write(idx);
        data.read()
    }
}

fn decode((s, m, h, status_b): (u8, u8, u8, u8)) -> Time {
    let bcd = status_b & 0x04 == 0;
    let pm = h & 0x80 != 0;
    let raw_hours = h & 0x7F;

    let seconds = if bcd { from_bcd(s) } else { s };
    let minutes = if bcd { from_bcd(m) } else { m };
    let mut hours = if bcd { from_bcd(raw_hours) } else { raw_hours };

    if status_b & 0x02 == 0 {
        hours = match (hours, pm) {
            (12, false) => 0,
            (12, true) => 12,
            (hh, true) => hh + 12,
            (hh, false) => hh,
        };
    }

    Time { hours: hours % 24, minutes: minutes % 60, seconds: seconds % 60 }
}

fn from_bcd(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}
