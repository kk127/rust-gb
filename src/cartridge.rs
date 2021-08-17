use std::fs;
use std::path::Path;

use log::info;

pub struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mode_flag: bool,
    ram_enable: bool,
    rom_bank_no: u8,
    ram_bank_no: u8,
    num_rom_banks: u8,
}

impl Cartridge {
    pub(crate) fn new(cartridge_name: &str) -> Self {
        info!("Reading {} file...", cartridge_name);

        let path = Path::new("cartridges").join(cartridge_name);
        let rom = fs::read(path).expect("Error while reading ROM file");

        info!("Finish reading {} file", cartridge_name);

        let title = rom[0x134..=0x143]
            .iter()
            .map(|&s| s as char)
            .collect::<String>();

        info!("ROM title: {}", title);

        let mbc_type = rom[0x147];

        let mbc_type_name = match mbc_type {
            0x00 => "ROM ONLY",
            0x01 => "MBC1",
            0x02 => "MBC1+RAM",
            0x03 => "MBC1+RAM+BATTERY",
            0x05 => "MBC2",
            0x06 => "MBC2+BATTERY",
            0x08 => "ROM+RAM",
            0x09 => "ROM+RAM+BATTERY",
            0x0B => "MMM01",
            0x0C => "MMM01+RAM",
            0x0D => "MMM01+RAM+BATTERY",
            0x0F => "MBC3+TIMER+BATTERY",
            0x10 => "MBC3+TIMER+RAM+BATTERY",
            0x11 => "MBC3",
            0x12 => "MBC3+RAM",
            0x13 => "MBC3+RAM+BATTERY",
            0x19 => "MBC5",
            0x1A => "MBC5+RAM",
            0x1B => "MBC5+RAM+BATTERY",
            0x1C => "MBC5+RUMBLE",
            0x1D => "MBC5+RUMBLE+RAM",
            0x1E => "MBC5+RUMBLE+RAM+BATTERY",
            0x20 => "MBC6",
            0x22 => "MBC7+SENSOR+RUMBLE+RAM+BATTERY",
            0xFC => "POCKET CAMERA",
            0xFD => "BANDAI TAMA5",
            0xFE => "HuC3",
            0xFF => "HuC1+RAM+BATTERY",
            _ => panic!("Invalid mbc type: {}", mbc_type),
        };

        let num_rom_banks = 2 << rom[0x148];

        let rom_size_kb = match rom[0x148] {
            n if (0x00..=0x08).contains(&n) => 32 << n,
            _ => panic!("Unknown ROM size, rom_code: {}", rom[0x148]),
        };

        let ram_size_kb = match rom[0x149] {
            0x00 => 0,
            0x01 => 2, // Listed in various unofficial docs as 2KB
            0x02 => 8,
            0x03 => 32,
            0x04 => 128,
            0x05 => 64,
            _ => panic!("Unknown RAM size, ram_code: {}", rom[0x149]),
        };

        let mut checksum: u8 = 0;
        for index in 0x134..=0x14c {
            checksum = checksum.wrapping_sub(rom[index]).wrapping_sub(1);
        }
        if checksum != rom[0x14d] {
            panic!("Error rom checksum");
        }

        info!("ROM size: {}KB", rom_size_kb);
        info!("RAM size: {}KB", ram_size_kb);
        info!("MBC type: {}", mbc_type_name);

        Cartridge {
            rom,
            ram: vec![0; ram_size_kb * 1024],
            mode_flag: false,
            ram_enable: false,
            rom_bank_no: 0,
            ram_bank_no: 0,
            num_rom_banks,
        }
    }

    fn rom_bank_no(&self) -> u8 {
        let bank_no = if self.mode_flag {
            self.rom_bank_no
        } else {
            self.ram_bank_no << 5 | self.rom_bank_no
        };

        let bank_no = match bank_no {
            0 | 0x20 | 0x40 | 0x60 => bank_no + 1,
            _ => bank_no,
        };

        bank_no & (self.num_rom_banks - 1)
    }

    fn ram_bank_no(&self) -> u8 {
        if self.mode_flag {
            self.ram_bank_no
        } else {
            0
        }
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        match addr {
            // ROM bank 00
            0x0000..=0x3fff => self.rom[addr as usize],
            // ROM bank 01-7f
            0x4000..=0x7fff => {
                let offset = (16 * 1024) * self.rom_bank_no() as usize;
                self.rom[(addr & 0x3fff) as usize + offset]
            }
            // RAM bank 00-03
            0xa000..=0xbfff => {
                if !self.ram_enable {
                    return 0xff;
                }
                let offset = (8 * 1024) * self.ram_bank_no() as usize;
                self.ram[(addr & 0x1fff) as usize + offset]
            }
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    pub(crate) fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.ram_enable = value & 0x0f == 0x0a,
            0x2000..=0x3fff => self.rom_bank_no = value & 0x1f,
            0x4000..=0x5fff => self.ram_bank_no = value & 0x03,
            0x6000..=0x7fff => self.mode_flag = value & 0x01 == 0x01,
            0xa000..=0xbfff => {
                if !self.ram_enable {
                    return;
                }
                let offset = (8 * 1024) * self.ram_bank_no() as usize;
                self.ram[(addr & 0x1fff) as usize + offset] = value
            }
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_read_cartridge() {
        init();
        let cartridge = Cartridge::new("hello.gb");

        assert!(true);
    }
}
