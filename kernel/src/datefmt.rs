use crate::arch::rtc::Date;

const WEEKDAYS: [&str; 7] = [
    "Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag",
];

const MONTHS: [&str; 12] = [
    "Januar", "Februar", "M\u{e4}rz", "April", "Mai", "Juni",
    "Juli", "August", "September", "Oktober", "November", "Dezember",
];

// Sakamoto's algorithm; returns 0 = Sunday.
fn weekday(y: u32, m: u32, d: u32) -> usize {
    const T: [u32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    ((y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d) % 7) as usize
}

pub fn format_german(date: &Date, buf: &mut [u8]) -> usize {
    let mut w = Writer { buf, len: 0 };
    w.push_str(WEEKDAYS[weekday(date.year, date.month as u32, date.day as u32)]);
    w.push_str(", ");
    if date.day >= 10 {
        w.push_byte(b'0' + date.day / 10);
    }
    w.push_byte(b'0' + date.day % 10);
    w.push_str(". ");
    w.push_str(MONTHS[(date.month as usize - 1).min(11)]);
    w.len
}

struct Writer<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl Writer<'_> {
    fn push_str(&mut self, s: &str) {
        for &b in s.as_bytes() {
            self.push_byte(b);
        }
    }

    fn push_byte(&mut self, b: u8) {
        if self.len < self.buf.len() {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }
}
