pub struct Wram {
    bank_index: u8,
    wram: [u8; 0x8000],
}

impl Wram {
    pub fn new() -> Self {
        Self {
            bank_index: 0,
            wram: [0; 0x8000],
        }
    }

    pub fn set_bank_index(&mut self, index: u8) {
        if index >= 8 {
            panic!("Invalid wram bank index: {}", index);
        }
        self.bank_index = index;
    }

    pub fn get_bank_idnex(&self) -> u8 {
        self.bank_index
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x0fff => self.wram[addr as usize],
            0x1000..=0x1fff => {
                if self.bank_index == 0 || self.bank_index == 1 {
                    self.wram[addr as usize]
                } else {
                    let wram_addr = (addr as usize) + ((self.bank_index - 1) as usize) * 0x1000;
                    self.wram[wram_addr]
                }
            }
            _ => panic!("Invalid wram access: addr 0x{:0x}", addr),
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x0fff => self.wram[addr as usize] = value,
            0x1000..=0x1fff => {
                if self.bank_index == 0 || self.bank_index == 1 {
                    self.wram[addr as usize] = value;
                } else {
                    let wram_addr = (addr as usize) + ((self.bank_index - 1) as usize) * 0x1000;
                    self.wram[wram_addr] = value;
                }
            }
            _ => panic!(
                "Invalid wram access: addr 0x{:0x}, value 0x{:0x}",
                addr, value
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_bank0() {
        let mut wram = Wram::new();
        for i in 0x0000..0x1000 {
            wram.write_byte(i, 100);
        }
        for bank in 1..8 {
            wram.set_bank_index(bank);
            let value = wram.read_byte(0x0500);
            assert_eq!(value, 100);
        }
    }

    #[test]
    fn read_write_bank1_to_bank7() {
        let mut wram = Wram::new();
        for bank in 1..8 {
            wram.set_bank_index(bank);
            wram.write_byte(0x1000, bank);
        }

        for bank in 0..8 {
            if bank == 0 || bank == 1 {
                wram.set_bank_index(bank);
                let value = wram.read_byte(0x1000);
                assert_eq!(value, 1);
            } else {
                wram.set_bank_index(bank);
                let value = wram.read_byte(0x1000);
                assert_eq!(value, bank);
            }
        }
    }
}
