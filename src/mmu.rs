use crate::cartridge::Cartridge;
use crate::ppu::Ppu;

pub struct Mmu {
    cartridge: Cartridge,
    ppu: Ppu,
    ram: [u8; 0x2000],
}

impl Mmu {
    pub fn new(cartridge_name: &str) -> Self {
        Mmu {
            cartridge: Cartridge::new(cartridge_name),
            ppu: Ppu::new(),
            ram: [0; 0x2000],
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => self.cartridge.read(addr),
            0x8000..=0x9fff => self.ppu.read(addr),
            0xa000..=0xbfff => self.cartridge.read(addr),
            0xc000..=0xdfff => self.ram[(addr & 0x01ff) as usize],
            0xe000..=0xfdff => self.ram[((addr - 0x2000) & 0x01ff) as usize],
            0xfe00..=0xfe9f => self.ppu.read(addr),
            0xff00..=0xff7f => todo!(), // Memory map I/O
            0xff80..=0xffff => todo!(), // zero page ram
            _ => todo!(),
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7fff => self.cartridge.write(addr, value),
            0x8000..=0x9fff => self.ppu.write(addr, value),
            0xa000..=0xbfff => self.cartridge.write(addr, value),
            0xc000..=0xdfff => self.ram[(addr & 0x01ff) as usize] = value,
            0xe000..=0xfdff => self.ram[((addr - 0x2000) & 0x01ff) as usize] = value,
            0xfe00..=0xfe9f => self.ppu.write(addr, value),
            0xff00..=0xff7f => todo!(), // Memory map I/O
            0xff80..=0xffff => todo!(), // zero page ram
            _ => todo!(),
        }
    }
}
