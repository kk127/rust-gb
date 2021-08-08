use std::fs;
use std::path::Path;

use log::info;

pub struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mode_flag: bool,
}

impl Cartridge {
    fn new(cartridge_name: &str) -> Self {
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
            mode_flag: false, //TODO,
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => self.rom[addr as usize],
            _ => todo!(),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        todo!();
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
