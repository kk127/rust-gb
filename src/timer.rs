use log::debug;

// pub struct Timer {
//     div_counter: u16,
//     tima: u8,
//     tima_total_count: u16,
//     tma: u8,
//     tac: u8,
//     irq_timer: bool,
// }

// impl Timer {
//     pub fn new() -> Self {
//         Timer {
//             div_counter: 0,
//             tima: 0,
//             tima_total_count: 0,
//             tma: 0,
//             tac: 0,
//             irq_timer: false,
//         }
//     }

//     pub fn is_irq_timer(&self) -> bool {
//         self.irq_timer
//     }

//     pub fn set_irq_timer(&mut self, flag: bool) {
//         self.irq_timer = flag;
//     }

//     pub fn read(&self, addr: u16) -> u8 {
//         match addr {
//             0xff04 => (self.div_counter >> 8) as u8,
//             0xff05 => self.tima,
//             0xff06 => self.tma,
//             0xff07 => self.tac,
//             _ => panic!("Invalid address: 0x{:04x}", addr),
//         }
//     }

//     pub fn write(&mut self, addr: u16, value: u8) {
//         match addr {
//             0xff04 => self.div_counter = 0,
//             0xff05 => self.tima = value,
//             0xff06 => self.tma = value,
//             0xff07 => self.tac = value & 7,
//             _ => panic!("Invalid address: 0x{:04x}", addr),
//         }
//     }

//     pub fn update(&mut self, clock: u8) {
//         self.div_counter = self.div_counter.wrapping_add(clock as u16);

//         if self.tac & 4 > 0 {
//             self.tima_total_count = self.tima_total_count.wrapping_add(clock as u16);
//             let divider = match self.tac & 3 {
//                 0 => 1024,
//                 1 => 16,
//                 2 => 64,
//                 3 => 256,
//                 _ => panic!("Invalid tac: {}", self.tac & 3),
//             };

//             if self.tima_total_count >= divider {
//                 self.tima_total_count -= divider;
//                 let (res, overflow_flag) = self.tima.overflowing_add(1);

//                 if overflow_flag {
//                     self.tima = self.tma;
//                     self.irq_timer = true;
//                 } else {
//                     self.tima = res;
//                 }
//             }
//         }
//     }
// }

pub struct Timer {
    /// Timer counter
    tima: u8,
    /// Timer modulo
    tma: u8,
    /// Timer control
    tac: u8,
    /// Internal 16-bit counter
    counter: u16,
    /// Interrupt request
    pub irq_timer: bool,
}

impl Timer {
    /// Creates a new `Timer`.
    pub fn new() -> Self {
        Timer {
            tima: 0,
            tma: 0,
            tac: 0,
            counter: 0,
            irq_timer: false,
        }
    }
}

impl Timer {
    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // DIV
            0xff04 => self.counter = 0,
            // TIMA
            0xff05 => self.tima = val,
            // TMA
            0xff06 => self.tma = val,
            // TAC
            0xff07 => self.tac = val & 0x7,
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // DIV
            0xff04 => (self.counter >> 8) as u8,
            // TIMA
            0xff05 => self.tima,
            // TMA
            0xff06 => self.tma,
            // TAC
            0xff07 => self.tac,
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }
    pub fn is_irq_timer(&self) -> bool {
        self.irq_timer
    }

    pub fn set_irq_timer(&mut self, flag: bool) {
        self.irq_timer = flag;
    }

    pub fn update(&mut self, tick: u8) {
        debug!(
            "div: {}, tima: {}, tma: {}, tac: {}, irq_timer: {}",
            self.counter, self.tima, self.tma, self.tac, self.irq_timer
        );
        let counter_prev = self.counter;

        self.counter = self.counter.wrapping_add(tick as u16);

        if self.tac & 4 > 0 {
            let divider = match self.tac & 3 {
                0 => 10,
                1 => 4,
                2 => 6,
                3 | _ => 8,
            };

            let x = self.counter >> divider;
            let y = counter_prev >> divider;
            let mask = (1 << (16 - divider)) - 1;
            let diff = x.wrapping_sub(y) & mask;

            if diff > 0 {
                let (res, overflow) = self.tima.overflowing_add(diff as u8);

                if overflow {
                    self.tima = self.tma + (diff as u8 - 1);
                    self.irq_timer = true;
                } else {
                    self.tima = res;
                }
            }
        }
        debug!(
            "div: {}, tima: {}, tma: {}, tac: {}, irq_timer: {}",
            self.counter, self.tima, self.tma, self.tac, self.irq_timer
        );
    }
}
