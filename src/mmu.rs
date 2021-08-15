use crate::cartridge::Cartridge;
use crate::ppu::Ppu;

pub struct Mmu {
    cartridge: Cartridge,
    pub ppu: Ppu,
    ram: [u8; 0x2000],
    interrupt_flag: u8,
    interrupt_enable: u8,
}

impl Mmu {
    pub fn new(cartridge_name: &str) -> Self {
        Mmu {
            cartridge: Cartridge::new(cartridge_name),
            ppu: Ppu::new(),
            ram: [0; 0x2000],
            interrupt_flag: 0,
            interrupt_enable: 0,
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => self.cartridge.read(addr),
            0x8000..=0x9fff => self.ppu.read(addr),
            0xa000..=0xbfff => self.cartridge.read(addr),
            0xc000..=0xdfff => self.ram[(addr & 0x1fff) as usize],
            0xe000..=0xfdff => self.ram[((addr - 0x2000) & 0x1fff) as usize],
            0xfe00..=0xfe9f => self.ppu.read(addr),
            0xfea0..=0xfeff => 0xff, // Not usable
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.read(addr),
            _ => todo!(),
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7fff => self.cartridge.write(addr, value),
            0x8000..=0x9fff => self.ppu.write(addr, value),
            0xa000..=0xbfff => self.cartridge.write(addr, value),
            0xc000..=0xdfff => self.ram[(addr & 0x1fff) as usize] = value,
            0xe000..=0xfdff => self.ram[((addr - 0x2000) & 0x1fff) as usize] = value,
            0xfe00..=0xfe9f => self.ppu.write(addr, value),
            0xfea0..=0xfeff => (), // Not usable
            0xff0f => self.interrupt_flag = value,
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.write(addr, value),
            0xffff => self.interrupt_enable = value,
            _ => (),
        }
    }

    pub fn update(&mut self, clock: u8) {
        self.ppu.update(clock);
    }
}
