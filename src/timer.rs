use log::debug;

pub struct Timer {
    div_counter: u16,
    tima: u8,
    tima_total_count: u16,
    tma: u8,
    tac: u8,
    irq_timer: bool,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div_counter: 0,
            tima: 0,
            tima_total_count: 0,
            tma: 0,
            tac: 0,
            irq_timer: false,
        }
    }

    pub fn is_irq_timer(&self) -> bool {
        self.irq_timer
    }

    pub fn set_irq_timer(&mut self, flag: bool) {
        self.irq_timer = flag;
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff04 => (self.div_counter >> 8) as u8,
            0xff05 => self.tima,
            0xff06 => self.tma,
            0xff07 => self.tac,
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xff04 => self.div_counter = 0,
            0xff05 => self.tima = value,
            0xff06 => self.tma = value,
            0xff07 => self.tac = value & 7,
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    pub fn update(&mut self, clock: u8) {
        self.div_counter = self.div_counter.wrapping_add(clock as u16);

        if self.tac & 4 > 0 {
            self.tima_total_count = self.tima_total_count.wrapping_add(clock as u16);
            let divider = match self.tac & 3 {
                0 => 1024,
                1 => 16,
                2 => 64,
                3 => 256,
                _ => panic!("Invalid tac: {}", self.tac & 3),
            };

            if self.tima_total_count >= divider {
                self.tima_total_count -= divider;
                let (res, overflow_flag) = self.tima.overflowing_add(1);

                if overflow_flag {
                    self.tima = self.tma;
                    self.irq_timer = true;
                } else {
                    self.tima = res;
                }
            }
        }
    }
}
