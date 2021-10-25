use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::rtc;
use log::info;

pub trait Cartridge {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn write_save_data(&self);
}

struct RomOnly {
    rom: Vec<u8>,
}

struct MBC1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mode_flag: bool,
    is_ram_enable: bool,
    rom_bank_no: u8,
    ram_bank_no: u8,
    num_rom_banks: u8,
    title: String,
}
pub struct MBC2 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_no: usize,
    ram_enable: bool,
    title: String,
}
struct MBC3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_no: u8,
    ram_bank_no: u8,
    rtc: rtc::RTC,
    ram_enable: bool,
    title: String,
}

struct MBC5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_no: usize,
    ram_bank_no: usize,
    ram_enable: bool,
    title: String,
}

pub fn new(cartridge_name: &str) -> Box<dyn Cartridge> {
    info!("Reading {} file...", cartridge_name);
    let path = Path::new("cartridges").join(cartridge_name);
    let rom = fs::read(path).expect("Error while reading ROM file");
    info!("Finish reading {} file", cartridge_name);

    let title = get_title(&rom[0x134..=0x143]);
    info!("ROM title: {}", title);

    let mbc_type = rom[0x147];
    let mbc_type_name = get_mbc_type_name(mbc_type);

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

    match mbc_type {
        0x00 => Box::new(RomOnly::new(rom)),
        0x01..=0x03 => Box::new(MBC1::new(rom, &title)),
        0x05 | 0x06 => Box::new(MBC2::new(rom, &title)),
        0x0f..=0x13 => Box::new(MBC3::new(rom, &title)),
        0x19..=0x1e => Box::new(MBC5::new(rom, &title)),
        _ => panic!("Invalid mbc type not implemented"),
    }
}

fn get_title(rom: &[u8]) -> String {
    rom.iter()
        .filter(|&s| (*s != 0) & (*s != 128))
        .map(|&s| s as char)
        .collect::<String>()
}

fn get_mbc_type_name(mbc_type: u8) -> &'static str {
    match mbc_type {
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
        // _ => panic!("Invalid mbc type: {}", mbc_type),
        _ => "Invalid mbc type",
    }
}

impl Cartridge for RomOnly {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => self.rom[addr as usize],
            _ => panic!("Invalid address: {}", addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            _ => {}
        }
    }
    fn write_save_data(&self) {}
}

impl RomOnly {
    fn new(rom: Vec<u8>) -> Self {
        RomOnly { rom }
    }
}

impl Cartridge for MBC1 {
    fn read(&self, addr: u16) -> u8 {
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
                if !self.is_ram_enable {
                    return 0xff;
                }
                let offset = (8 * 1024) * self.ram_bank_no() as usize;
                self.ram[(addr & 0x1fff) as usize + offset]
            }
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.is_ram_enable = value & 0x0f == 0x0a,
            0x2000..=0x3fff => self.rom_bank_no = value & 0x1f,
            0x4000..=0x5fff => self.ram_bank_no = value & 0x03,
            0x6000..=0x7fff => self.mode_flag = value & 0x01 == 0x01,
            0xa000..=0xbfff => {
                if !self.is_ram_enable {
                    return;
                }
                let offset = (8 * 1024) * self.ram_bank_no() as usize;
                self.ram[(addr & 0x1fff) as usize + offset] = value
            }
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    fn write_save_data(&self) {
        let save_file_path = Path::new("save_data").join(&self.title);
        info!("Writing save file to: {:?}", &save_file_path);
        fs::write(&save_file_path, &self.ram).unwrap();
    }
}

impl MBC1 {
    fn new(rom: Vec<u8>, title: &str) -> Self {
        let num_rom_banks = 2 << rom[0x148];
        let ram_size_kb = match rom[0x149] {
            0x00 => 0,
            0x01 => 2, // Listed in various unofficial docs as 2KB
            0x02 => 8,
            0x03 => 32,
            0x04 => 128,
            0x05 => 64,
            _ => panic!("Unknown RAM size, ram_code: {}", rom[0x149]),
        };

        let ram = get_ram(title, ram_size_kb);

        info!("MBC1 created");
        MBC1 {
            rom,
            ram,
            mode_flag: false,
            is_ram_enable: false,
            rom_bank_no: 0,
            ram_bank_no: 0,
            num_rom_banks,
            title: title.to_string(),
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
}

impl Cartridge for MBC2 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3fff => self.rom[addr as usize],
            0x4000..=0x7fff => {
                let i = self.rom_bank_no * 0x4000 + (addr as usize) - 0x4000;
                self.rom[i]
            }
            0xa000..=0xa1ff => {
                if self.ram_enable {
                    self.ram[(addr - 0xa000) as usize]
                } else {
                    0x00
                }
            }
            _ => 0x00,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        let value = value & 0x0f;
        match addr {
            0xa000..=0xa1ff => {
                if self.ram_enable {
                    self.ram[(addr - 0xa000) as usize] = value
                }
            }
            0x0000..=0x1fff => {
                if addr & 0x0100 == 0 {
                    self.ram_enable = value == 0x0a;
                }
            }
            0x2000..=0x3fff => {
                if addr & 0x0100 != 0 {
                    self.rom_bank_no = value as usize;
                }
            }
            _ => {}
        }
    }
    fn write_save_data(&self) {
        let save_file_path = Path::new("save_data").join(&self.title);
        info!("Writing save file to: {:?}", &save_file_path);
        fs::write(&save_file_path, &self.ram).unwrap();
    }
}

impl MBC2 {
    fn new(rom: Vec<u8>, title: &str) -> Self {
        let num_rom_banks = 2 << rom[0x148];

        info!("MBC2 created");
        MBC2 {
            rom,
            ram: vec![0; 512],
            rom_bank_no: 0,
            ram_enable: false,
            title: title.to_string(),
        }
    }
}

impl Cartridge for MBC3 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3fff => self.rom[addr as usize],
            0x4000..=0x7fff => {
                let rom_addr = (self.rom_bank_no as usize * 0x4000) + (addr as usize) - 0x4000;
                self.rom[rom_addr]
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    match self.ram_bank_no {
                        0x00..=0x03 => {
                            let ram_addr =
                                (self.ram_bank_no as usize) * 0x2000 + (addr as usize) - 0xa000;
                            self.ram[ram_addr]
                        }
                        n if (0x08..=0x0c).contains(&n) => self.rtc.read(n as u16),
                        _ => panic!("Invalid addr 0x{:04x}, MBC3 read", addr),
                    }
                } else {
                    0x00
                }
            }
            _ => panic!("Invalid addr 0x{:04x}, MBC3 read", addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.ram_enable = (value & 0x0f) == 0x0a,
            0x2000..=0x3fff => {
                let rom_bank = value & 0x7f;
                let rom_bank = match rom_bank {
                    0 => 1,
                    _ => rom_bank,
                };
                self.rom_bank_no = rom_bank;
            }
            0x4000..=0x5fff => {
                self.ram_bank_no = value & 0x0f;
            }
            0x6000..=0x7fff => {
                if value & 0x01 != 0 {
                    self.rtc.tic();
                }
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    match self.ram_bank_no {
                        0x00..=0x03 => {
                            let ram_addr =
                                (self.ram_bank_no as usize) * 0x2000 + (addr as usize) - 0xa000;
                            self.ram[ram_addr] = value;
                        }
                        0x08..=0x0c => self.rtc.write(self.ram_bank_no as u16, value),
                        _ => panic!("Invalid address: 0x{:04x}", addr),
                    }
                }
            }
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }
    fn write_save_data(&self) {
        let save_file_path = Path::new("save_data").join(&self.title);
        info!("Writing save file to: {:?}", &save_file_path);
        fs::write(&save_file_path, &self.ram).unwrap();
    }
}

impl MBC3 {
    fn new(rom: Vec<u8>, title: &str) -> Self {
        let num_rom_banks = 2 << rom[0x148];

        let ram_size_kb = match rom[0x149] {
            0x00 => 0,
            0x01 => 2, // Listed in various unofficial docs as 2KB
            0x02 => 8,
            0x03 => 32,
            0x04 => 128,
            0x05 => 64,
            _ => panic!("Unknown RAM size, ram_code: {}", rom[0x149]),
        };

        let ram = get_ram(title, ram_size_kb);

        info!("MBC3 created");
        MBC3 {
            rom,
            ram,
            rom_bank_no: 0,
            ram_bank_no: 0,
            rtc: rtc::RTC::new(),
            ram_enable: false,
            title: title.to_string(),
        }
    }
}

impl Cartridge for MBC5 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3fff => self.rom[addr as usize],
            0x4000..=0x7fff => {
                let rom_addr = self.rom_bank_no * 0x4000 + (addr as usize) - 0x4000;
                self.rom[rom_addr]
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let ram_addr = self.ram_bank_no * 0x2000 + (addr as usize) - 0xa000;
                    self.ram[ram_addr]
                } else {
                    0x00
                }
            }
            _ => 0x00,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => {
                self.ram_enable = (value & 0x0f) == 0x0a;
            }
            0x2000..=0x2fff => self.rom_bank_no = (self.rom_bank_no & 0x100) | (value as usize),
            0x3000..=0x3fff => {
                self.rom_bank_no = (self.rom_bank_no & 0x0ff) | (((value & 0x01) as usize) << 8)
            }
            0x4000..=0x5fff => self.ram_bank_no = (value & 0x0f) as usize,
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let i = self.ram_bank_no * 0x2000 + (addr as usize) - 0xa000;
                    self.ram[i] = value;
                }
            }
            _ => {}
        }
    }
    fn write_save_data(&self) {
        let save_file_path = Path::new("save_data").join(&self.title);
        info!("Writing save file to: {:?}", &save_file_path);
        fs::write(&save_file_path, &self.ram).unwrap();
    }
}

impl MBC5 {
    fn new(rom: Vec<u8>, title: &str) -> Self {
        let ram_size_kb = match rom[0x149] {
            0x00 => 0,
            0x01 => 2, // Listed in various unofficial docs as 2KB
            0x02 => 8,
            0x03 => 32,
            0x04 => 128,
            0x05 => 64,
            _ => panic!("Unknown RAM size, ram_code: {}", rom[0x149]),
        };

        let ram = get_ram(title, ram_size_kb);

        info!("MBC5 created");
        MBC5 {
            rom,
            ram,
            rom_bank_no: 0,
            ram_bank_no: 0,
            ram_enable: false,
            title: title.to_string(),
        }
    }
}

fn get_ram(title: &str, ram_size_kb: usize) -> Vec<u8> {
    let save_file_path = Path::new("save_data").join(title);
    let mut ram = Vec::new();
    if let Ok(mut file) = File::open(&save_file_path) {
        file.read_to_end(&mut ram).unwrap();
        info!("Read save data, path: {:?}", &save_file_path);
    } else {
        info!("No save data, checked path: {:?}", &save_file_path);
        ram = vec![0; ram_size_kb * 1024];
    }
    ram
}
