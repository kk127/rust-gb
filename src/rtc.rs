use chrono::{DateTime, Local};
pub struct Rtc {
    s: u8,
    m: u8,
    h: u8,
    dl: u8,
    dh: u8,
    initialization_time: DateTime<Local>,
}

impl Rtc {
    pub fn new() -> Self {
        Rtc {
            s: 0,
            m: 0,
            h: 0,
            dl: 0,
            dh: 0,
            initialization_time: Local::now(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0008 => self.s,
            0x0009 => self.m,
            0x000a => self.h,
            0x000b => self.dl,
            0x000c => self.dh,
            _ => panic!("Invalid address: 0x{:04x}, RTC read", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0008 => self.s = value,
            0x0009 => self.m = value,
            0x000a => self.h = value,
            0x000b => self.dl = value,
            0x000c => self.dh = value,
            _ => panic!("Invalid address: 0x{:04x}, RTC write", addr),
        }
    }

    pub fn tic(&mut self) {
        let date_diff = Local::now() - self.initialization_time;

        self.s = date_diff.num_seconds() as u8;
        self.m = date_diff.num_minutes() as u8;
        self.h = date_diff.num_hours() as u8;
        let days_diff = date_diff.num_days() as u16;
        self.dl = (days_diff % 256) as u8;
        match days_diff {
            0x0000..=0x00ff => {}
            0x0100..=0x01ff => {
                self.dh |= 0x01;
            }
            _ => {
                self.dh |= 0x01;
                self.dh |= 0x80;
            }
        }
    }
}
