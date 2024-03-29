use crate::cartridge::{self, Cartridge};
use crate::cpu::Interrupt;
use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::serial::Serial;
use crate::timer::Timer;

pub struct Mmu {
    pub cartridge: Box<dyn Cartridge>,
    pub ppu: Ppu,
    pub joypad: Joypad,
    serial: Serial,
    timer: Timer,
    ram: [u8; 0x2000],
    pub interrupt_flag: u8,
    pub interrupt_enable: u8,
    hram: [u8; 0x7f],
}

impl Mmu {
    pub fn new(cartridge_name: &str) -> Self {
        Mmu {
            cartridge: cartridge::new(cartridge_name),
            ppu: Ppu::new(),
            joypad: Joypad::new(),
            serial: Serial::new(),
            timer: Timer::new(),
            ram: [0; 0x2000],
            interrupt_flag: 0,
            interrupt_enable: 0,
            hram: [0; 0x7f],
        }
    }

    #[rustfmt::skip]
    pub fn reset_interrupt(&mut self, interrupt_type: Interrupt) {
        match interrupt_type {
            Interrupt::VBlank  => self.interrupt_flag &= 0b1111_1110,
            Interrupt::LCDStat => self.interrupt_flag &= 0b1111_1101,
            Interrupt::Timer   => self.interrupt_flag &= 0b1111_1011,
            Interrupt::Serial  => self.interrupt_flag &= 0b1111_0111,
            Interrupt::Joypad  => self.interrupt_flag &= 0b1110_1111,
        }
    }

    fn do_dma(&mut self, val: u8) {
        // if val < 0x80 || 0xdf < val {
        //     panic!("Invalid DMA source address: 0x{:04x}", val)
        // }
        assert!(val <= 0xf1);
        let src_base = (val as u16) << 8;
        let dst_base = 0xfe00;

        for i in 0..0xa0 {
            let tmp = self.read_byte(src_base | i);
            self.write_byte(dst_base | i, tmp);
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
            0xfea0..=0xfeff => 0x00, // Not usable
            0xff00 => self.joypad.read_byte(addr),
            0xff01..=0xff02 => self.serial.read(addr),
            0xff0f => self.interrupt_flag,
            0xff04..=0xff07 => self.timer.read(addr),
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.read(addr),
            0xff80..=0xfffe => self.hram[(addr & 0x7f) as usize],
            0xffff => self.interrupt_enable,
            _ => 0x00,
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
            0xff00 => self.joypad.write_byte(addr, value),
            0xff0f => self.interrupt_flag = value,
            0xff01..=0xff02 => self.serial.write(addr, value),
            0xff04..=0xff07 => self.timer.write(addr, value),
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.write(addr, value),
            0xff46 => self.do_dma(value),
            0xff80..=0xfffe => self.hram[(addr & 0x7f) as usize] = value,
            0xffff => self.interrupt_enable = value,
            _ => (),
        }
    }

    pub fn update(&mut self, clock: u8) {
        self.ppu.update(clock);
        self.timer.update(clock);

        if self.ppu.is_irq_vblank() {
            self.interrupt_flag |= 0x1;
            self.ppu.set_irq_vblank(false);
        }

        if self.ppu.is_irq_lcdc() {
            self.interrupt_flag |= 0x2;
            self.ppu.set_irq_lcdc(false);
        }

        if self.timer.is_irq_timer() {
            self.interrupt_flag |= 0x4;
            self.timer.set_irq_timer(false);
        }

        if self.joypad.irq {
            self.interrupt_flag |= 0x10;
            self.joypad.irq = false;
        }
    }
}
