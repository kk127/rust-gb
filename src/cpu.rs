use std::fmt;

use log::debug;

use crate::mmu::Mmu;
use crate::register::Register;
use crate::utils::get_addr_from_registers;

#[derive(Copy, Clone, Debug)]
pub enum Interrupt {
    VBlank,
    LCDStat,
    Timer,
    Serial,
    Joypad,
}

#[derive(Clone, Copy)]
enum CcFlag {
    NZ,
    Z,
    NC,
    C,
}
impl fmt::Display for CcFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CcFlag::NZ => write!(f, "NZ"),
            CcFlag::Z => write!(f, "Z"),
            CcFlag::NC => write!(f, "NC"),
            CcFlag::C => write!(f, "C"),
        }
    }
}

pub struct Cpu {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    zero_flag: bool,
    subtraction_flag: bool,
    half_carry_flag: bool,
    carry_flag: bool,

    pub mmu: Mmu,
    clock: u32,
    ime: bool,
    halt: bool,
    total_elapsed_clock: u32, // for debug
}

impl Cpu {
    pub fn new(cartridge_name: &str) -> Self {
        Cpu {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0x100,
            zero_flag: false,
            subtraction_flag: false,
            half_carry_flag: false,
            carry_flag: false,

            mmu: Mmu::new(cartridge_name),
            clock: 0,
            ime: false,
            halt: false,
            total_elapsed_clock: 0,
        }
    }

    fn get_f_num(&self) -> u8 {
        let mut res: u8 = 0;
        if self.zero_flag {
            res |= 1 << 7;
        }
        if self.subtraction_flag {
            res |= 1 << 6;
        }
        if self.half_carry_flag {
            res |= 1 << 5;
        }
        if self.carry_flag {
            res |= 1 << 4;
        }
        res
    }

    pub fn step(&mut self) -> u16 {
        let pc = self.pc;
        let opcode = self.mmu.read_byte(pc);
        debug!(
            "PC: 0x{:04x}, opcode: 0x{:04x}, sp: 0x{:04x}",
            pc, opcode, self.sp
        );
        debug!(
            "a: 0x{:02x}, f: {:02x}, b: 0x{:02x}, c: 0x{:02x}",
            self.a,
            self.get_f_num(),
            self.b,
            self.c
        );
        debug!(
            "d: 0x{:02x}, e: 0x{:02x}, h: 0x{:02x}, l: 0x{:02x}",
            self.d, self.e, self.h, self.l
        );
        debug!("halted: {}", self.halt);

        let mut elapse_clock = 0;
        if self.halt {
            elapse_clock += 4;
            self.add_clock(4);
        } else {
            self.add_program_count(1);
            let before_clock = self.clock;
            self.exec(opcode);
            let after_clock = self.clock;
            elapse_clock = after_clock.wrapping_sub(before_clock);
        }

        self.mmu.update(elapse_clock as u8);

        debug!(
            "ime: {}, interrupt_flag: 0b{:08b}, interrupt_enable: 0b{:08b}",
            self.ime, self.mmu.interrupt_flag, self.mmu.interrupt_enable
        );

        if self.ime {
            self.handle_interrupt();
            // self.mmu.update(8);
            // elapse_clock += 8;
        }

        self.total_elapsed_clock += elapse_clock as u32;
        debug!("total_elapsed_clock: {}", self.clock);
        elapse_clock as u16
    }

    fn handle_interrupt(&mut self) {
        let interrupt_source = self.mmu.interrupt_flag & self.mmu.interrupt_enable;
        for bit in 0..=4 {
            let interrupt_type = match interrupt_source & (1 << bit) {
                0x01 => Interrupt::VBlank,
                0x02 => Interrupt::LCDStat,
                0x04 => Interrupt::Timer,
                0x08 => Interrupt::Serial,
                0x10 => Interrupt::Joypad,
                _ => continue,
            };

            self.exec_interrupt(interrupt_type);
        }
    }

    fn exec_interrupt(&mut self, interrupt_type: Interrupt) {
        self.ime = false;
        self.halt = false;
        self.mmu.reset_interrupt(interrupt_type);

        let addr = match interrupt_type {
            Interrupt::VBlank => 0x40,
            Interrupt::LCDStat => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        };

        self.sp = self.sp.wrapping_sub(2);
        let sp = self.sp;
        let pc = self.pc;

        self.write_word(sp, pc);
        self.add_clock(20); // todo
        self.pc = addr;

        self.mmu.update(20);
        debug!("Interrupt {:?}, addr: 0x{:04x}", interrupt_type, self.pc);
    }

    /// Put value n into nn.
    ///
    /// nn = B,C,D,E,H,L
    /// n = 8 bit immediate value
    /// Opcode for 06, 0E, 16, 1E, 26, 2E
    fn load_nn_n(&mut self, reg: Register) {
        let pc = self.pc;
        let value = self.mmu.read_byte(pc);
        debug!("Instruction load_nn_n reg: {}, value: {}", reg, value);

        match reg {
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
            _ => panic!("Invalid Register: {}", reg),
        }

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Put value r2 into r1.
    ///
    /// r1,r2 = A,B,C,D,E,H,L
    /// Opcode for
    ///  B,  C,  D,  E,  H,  L,  A      r2
    ///                                 r1
    /// 78, 79, 7A, 7B, 7C, 7D, 7F       A
    /// 40, 41, 42, 43, 44, 45, 47       B
    /// 48, 49, 4A, 4B, 4C, 4D, 4F       C
    /// 50, 51, 52, 53, 54, 55, 57       D
    /// 58, 59, 5A, 5B, 5C, 5D, 5F       E
    /// 60, 61, 62, 63, 64, 65, 67       H
    /// 68, 69, 6A, 6B, 6C, 6D, 6F       L
    fn load_r1_r2(&mut self, reg1: Register, reg2: Register) {
        debug!("Instruction load_r1_r2 r1: {}, r2: {}", reg1, reg2);

        let value = match reg2 {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register2 {}", reg2),
        };

        match reg1 {
            Register::A => self.a = value,
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
            _ => panic!("Invalid register1 {}", reg1),
        }

        self.add_clock(4);
    }

    /// Put value memory8 into r1.
    ///
    /// r1 = A,B,C,D,E,H,L
    /// memory8 = (HL)
    ///
    /// Opcode for
    /// 7E, 46, 4E, 56, 5E, 66, 6E
    fn load_r1_hl(&mut self, reg1: Register) {
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        debug!(
            "Instruction load_r1_hl r1: {}, memory8: {}, addr: {}",
            reg1, value, addr
        );

        match reg1 {
            Register::A => self.a = value,
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
            _ => panic!("Invalid register1 {}", reg1),
        }

        self.add_clock(8);
    }

    /// Put value r1 into memory8.
    ///
    /// r1 = A,B,C,D,E,H,L
    /// memory8 = (HL)
    ///
    /// Opcode for
    /// 70, 71, 72, 73, 74, 75, 77
    fn load_hl_r1(&mut self, reg1: Register) {
        let high_register = self.h;
        let low_register = self.l;
        let addr = get_addr_from_registers(high_register, low_register);

        let value = match reg1 {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register1 {}", reg1),
        };
        self.mmu.write_byte(addr, value);

        debug!("Instruction load_hl_r1 addr: {}, r1: {}", addr, reg1);

        self.add_clock(8);
    }

    /// Put immediate 8bit into memory8.
    ///
    /// r1 = A,B,C,D,E,H,L
    /// memory8 = (HL)
    ///
    /// Opcode for 36
    fn load_hl_imm(&mut self) {
        let high_register = self.h;
        let low_register = self.l;
        let pc = self.pc;

        let addr = get_addr_from_registers(high_register, low_register);
        let value = self.mmu.read_byte(pc);
        self.mmu.write_byte(addr, value);
        debug!("Instruction load_hl_imm hl: {}, value: {}", addr, value);

        self.add_program_count(1);
        self.add_clock(12);
    }

    /// Put value a into nn
    ///
    /// nn = (BC), (DE)
    /// Opcode for 02, 12
    fn load_nn_a(&mut self, reg: Register) {
        let addr = match reg {
            Register::BC => get_addr_from_registers(self.b, self.c),
            Register::DE => get_addr_from_registers(self.d, self.e),
            _ => panic!("Invalid register {}", reg),
        };
        let value = self.a;
        self.mmu.write_byte(addr, value);
        debug!("Instruction load_nn_a addr: {}, value: {}", addr, value);

        self.add_clock(8);
    }

    /// Put value nn into a
    ///
    /// nn = (BC), (DE)
    /// Opcode for 0A, 1A
    fn load_a_nn(&mut self, reg: Register) {
        let addr = match reg {
            Register::BC => get_addr_from_registers(self.b, self.c),
            Register::DE => get_addr_from_registers(self.d, self.e),
            _ => panic!("Invalid register {}", reg),
        };
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_nn_a addr: {}, value: {}", addr, value);

        self.add_clock(8);
    }

    /// Put value a into n
    ///
    /// n == (a16)
    /// Opcode for EA
    fn load_imm_a(&mut self) {
        let pc = self.pc;
        let addr = self.read_word(pc);
        let value = self.a;
        self.mmu.write_byte(addr, value);

        debug!("Instruction load_imm_a addr: {}, value: {}", addr, value);

        self.add_program_count(2);
        self.add_clock(16);
    }

    /// put value d8 into a
    ///
    /// Opcode for 3E
    fn load_a_d8(&mut self) {
        let addr = self.pc;
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_a_d8 addr: {}, value: {}", addr, value);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Put value n into a
    ///
    /// n == (a16)
    /// Opcode for FA
    fn load_a_imm(&mut self) {
        let pc = self.pc;
        let addr = self.read_word(pc);
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_a_imm addr: {}, value: {}", addr, value);

        self.add_program_count(2);
        self.add_clock(16);
    }

    /// Put value at address 0xFF00 + register C into A
    /// Opcode for F2
    fn load_a_c(&mut self) {
        let addr = 0xFF00 + self.c as u16;
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_a_c addr: {}, value: {}", addr, value);

        // self.add_program_count(1);
        self.add_clock(8);
    }

    /// Put A into address 0xFF00 + register C
    /// Opcode for E2
    fn load_c_a(&mut self) {
        let addr = 0xFF00 + self.c as u16;
        let value = self.a;
        self.mmu.write_byte(addr, value);

        debug!("Instruction load_c_a addr: {}, value: {}", addr, value);

        // self.add_program_count(1);
        self.add_clock(8);
    }

    /// Put value a into address HL.
    /// Then, Increment HL
    /// Opcode for 22
    fn load_hli_a(&mut self) {
        let high_register = self.h;
        let low_register = self.l;
        let addr = get_addr_from_registers(high_register, low_register);
        let value = self.a;
        debug!(
            "Instruction load_hli_a addr: 0x{:04x}, value: 0x{:04x}",
            addr, value
        );
        self.mmu.write_byte(addr, value);

        self.l = self.l.wrapping_add(1);
        if self.l == 0 {
            self.h = self.h.wrapping_add(1);
        }

        self.add_clock(8);
    }

    /// Put value a into address HL
    /// Then, Decrement HL
    /// Opcode for 32
    fn load_hld_a(&mut self) {
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.a;
        self.mmu.write_byte(addr, value);

        self.l = self.l.wrapping_sub(1);
        if self.l == 255 {
            self.h = self.h.wrapping_sub(1);
        }

        debug!("Instruction load_hld_a addr: {}, value: {}", addr, value);

        self.add_clock(8);
    }

    /// Put value at address HL into a
    /// Then, Increment HL
    /// Opcode for 2A
    fn load_a_hli(&mut self) {
        let high_register = self.h;
        let low_register = self.l;
        let addr = get_addr_from_registers(high_register, low_register);
        self.a = self.mmu.read_byte(addr);

        self.l = self.l.wrapping_add(1);
        if self.l == 0 {
            self.h = self.h.wrapping_add(1);
        }

        debug!("Instruction load_a_hli addr: {}", addr);

        self.add_clock(8);
    }

    /// Put value at address HL into a
    /// Then, Decrement HL
    /// Opcode for 3A
    fn load_a_hld(&mut self) {
        let high_register = self.h;
        let low_register = self.l;
        let addr = get_addr_from_registers(high_register, low_register);
        self.a = self.mmu.read_byte(addr);

        self.l = self.l.wrapping_sub(1);
        if self.l == 255 {
            self.h = self.h.wrapping_sub(1);
        }

        debug!("Instruction load_a_hli addr: {}", addr);

        self.add_clock(8);
    }

    /// Put A into memory address $FF00 + n
    /// n = one byte immediate value
    /// Opcode for E0
    fn load_n_a(&mut self) {
        let pc = self.pc;
        let n = self.mmu.read_byte(pc);
        let addr = 0xFF00 + n as u16;
        let value = self.a;
        debug!("Instruction load_n_a addr: {:0x}, value: {}", addr, value);
        self.mmu.write_byte(addr, value);

        self.add_program_count(1);
        self.add_clock(12);
    }

    /// Put memory address $FF00 + n into A
    /// n = one byte immediate value
    /// Opcode for F0
    fn load_a_n(&mut self) {
        let pc = self.pc;
        let n = self.mmu.read_byte(pc);
        let addr = 0xFF00 + n as u16;
        debug!("Instruction load_a_n addr: 0x{:0x}", addr);
        let value = self.mmu.read_byte(addr);
        self.a = value;

        self.add_program_count(1);
        self.add_clock(12);
    }

    /// Put value nn into n.
    /// nn = 16 bit immediate value
    /// n = BC, DE, HL, SP
    /// Opcode for 01, 11, 21, 31
    fn load_n_nn(&mut self, reg: Register) {
        let pc = self.pc;
        let low_value = self.mmu.read_byte(pc);
        let high_value = self.mmu.read_byte(pc + 1);

        match reg {
            Register::BC => {
                self.b = high_value;
                self.c = low_value;
            }
            Register::DE => {
                self.d = high_value;
                self.e = low_value;
            }
            Register::HL => {
                self.h = high_value;
                self.l = low_value;
            }
            Register::SP => {
                self.sp = ((high_value as u16) << 8) + low_value as u16;
            }
            _ => panic!("Invalid register: {}", reg),
        }

        debug!(
            "Instruction load_n_nn high_value: 0x{:0x}, low_value: 0x{:0x}, register: {}",
            high_value, low_value, reg
        );

        self.add_program_count(2);
        self.add_clock(12);
    }

    /// Put HL into SP
    /// Opcode for F9
    fn load_sp_hl(&mut self) {
        self.sp = ((self.h as u16) << 8) + self.l as u16;
        debug!("Instruction load_sp_hl");

        self.add_clock(8);
    }

    /// Put SP + n effective address into HL.
    /// n = one byte signed immediate value.
    /// Opcode for F8
    ///
    /// Affected Flag:
    /// Z reset
    /// N reset
    /// H Set or reset according to operation.
    /// C Set or reset according to operation.
    fn load_sp_n(&mut self) {
        // Add a signed integer to an unsigned integer reference URL
        // https://stackoverflow.com/questions/53453628/how-do-i-add-a-signed-integer-to-an-unsigned-integer-in-rust
        let sp = self.sp;
        let pc = self.pc;
        let n = self.mmu.read_byte(pc) as i8 as u16;

        let value = sp.wrapping_add(n);

        self.h = (value >> 8) as u8;
        self.l = value as u8;

        debug!("Instruction load_sp_n sp: {}, n: {}", sp, n as i8);

        let half_carry_flag = (sp & 0x0f) + (n & 0x0f) > 0x0f;
        let carry_flag = (sp & 0xff) + (n & 0xff) > 0xff;

        self.set_zero_flag(false);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(12);
    }

    /// Put SP at address nn
    /// nn = two byte immediate address
    /// Opcode for 08
    fn load_nn_sp(&mut self) {
        let pc = self.pc;
        let sp = self.sp;
        let addr = self.read_word(pc);

        self.write_word(addr, sp);

        debug!("Instruction load_nn_sp addr: {}, sp: {}", addr, sp);

        self.add_program_count(2);
        self.add_clock(20);
    }

    /// Push register pair nn onto stack.
    /// Decrement SP twice.
    /// nn = AF, BC, DE, HL
    /// Opcode for F5, C5, D5, E5
    fn push_nn(&mut self, reg1: Register, reg2: Register) {
        debug!("Instruction Push {}{}", reg1, reg2);

        self.sp = self.sp.wrapping_sub(2);

        let (high_value, low_value) = match (reg1, reg2) {
            (Register::A, Register::F) => (self.a, self.get_byte_from_flags()),
            (Register::B, Register::C) => (self.b, self.c),
            (Register::D, Register::E) => (self.d, self.e),
            (Register::H, Register::L) => (self.h, self.l),
            _ => panic!("Invalid register: {},{}", reg1, reg2),
        };

        let addr = self.sp;
        let value = ((high_value as u16) << 8) | low_value as u16;

        self.write_word(addr, value);

        self.add_clock(16);
    }

    /// Pop tow bytes off stack into register pair nn.
    /// Increment SP twice.
    /// nn = AF, BC, DE, HL
    /// Opcode for F1, C1, D1, E1
    fn pop_nn(&mut self, reg1: Register, reg2: Register) {
        let low_value = self.mmu.read_byte(self.sp);
        self.sp += 1;
        let high_value = self.mmu.read_byte(self.sp);
        self.sp += 1;

        debug!(
            "Instruction Pop {}{}, high_value: 0x{:04x}, low_value: 0x{:04x}",
            reg1, reg2, high_value, low_value
        );

        match (reg1, reg2) {
            (Register::A, Register::F) => {
                self.a = high_value;
                self.set_flags_from_byte(low_value)
            }
            (Register::B, Register::C) => {
                self.b = high_value;
                self.c = low_value;
            }
            (Register::D, Register::E) => {
                self.d = high_value;
                self.e = low_value;
            }
            (Register::H, Register::L) => {
                self.h = high_value;
                self.l = low_value;
            }
            _ => panic!("Invalid register {} {}", reg1, reg2),
        }

        self.add_clock(12);
    }

    /// Add register n value to A.
    /// n = A, B,C,D,E,H,L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N reset
    /// H Set if carry from bit 3
    /// C Set if carry from bit 7
    ///
    /// Opcode for 87, 80, 81, 82, 83, 84, 85
    fn add_a_r(&mut self, reg: Register) {
        debug!("Instruction add_a_r reg: {}", reg);

        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        let half_carry_flag = (self.a & 0x0f) + (value & 0x0f) > 0x0f;
        let (res, carry_flag) = self.a.overflowing_add(value);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Add HL value to A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N reset
    /// H Set if carry from bit 3
    /// C Set if carry from bit 7
    ///
    /// Opcode for 86
    fn add_a_hl(&mut self) {
        debug!("Instruction add_a_hl");

        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        let half_carry_flag = (self.a & 0x0f) + (value & 0x0f) > 0x0f;
        let (res, carry_flag) = self.a.overflowing_add(value);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(8);
    }

    /// Add d8 to A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N reset
    /// H Set if carry from bit 3
    /// C Set if carry from bit 7
    ///
    /// Opcode for C6
    fn add_a_d8(&mut self) {
        debug!("Instruction add_a_d8");

        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        let half_carry_flag = (self.a & 0x0f) + (value & 0x0f) > 0x0f;
        let (res, carry_flag) = self.a.overflowing_add(value);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Add n + Carry flag to A
    /// n = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N reset
    /// H Set if carry from bit 3
    /// C Set if carry from bit 7
    ///
    /// Opcode for 8F, 88, 89, 8A, 8B, 8C, 8D
    fn adc_a_n(&mut self, reg: Register) {
        debug!("Instruction adc_a_n reg: {}", reg);

        let c = if self.carry_flag { 1 } else { 0 };

        let register_value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        let res = self.a.wrapping_add(register_value).wrapping_add(c);
        let half_carry_flag = (self.a & 0x0f) + (register_value & 0x0f) + c > 0x0f;
        let carry_flag = (self.a as u16) + (register_value as u16) + (c as u16) > 0xff;

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Add HL value + Carry flag to A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N reset
    /// H Set if carry from bit 3
    /// C Set if carry from bit 7
    ///
    /// Opcode for 8E
    fn adc_a_hl(&mut self) {
        debug!("Instruction adc_a_hl");

        let c = if self.carry_flag { 1 } else { 0 };

        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        let res = self.a.wrapping_add(value).wrapping_add(c);
        let half_carry_flag = (self.a & 0x0f) + (value & 0x0f) + c > 0x0f;
        let carry_flag = (self.a as u16) + (value as u16) + (c as u16) > 0xff;

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(8);
    }

    /// Add d8 + Carry flag to A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N reset
    /// H Set if carry from bit 3
    /// C Set if carry from bit 7
    ///
    /// Opcode for CE
    fn adc_a_d8(&mut self) {
        debug!("Instruction adc_a_d8");

        let c = if self.carry_flag { 1 } else { 0 };

        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        let res = self.a.wrapping_add(value).wrapping_add(c);
        let half_carry_flag = (self.a & 0x0f) + (value & 0x0f) + c > 0x0f;
        let carry_flag = (self.a as u16) + (value as u16) + (c as u16) > 0xff;

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Subtract n from A
    /// n = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set if no borrow
    ///
    /// Opcode for 97, 90, 91, 92, 93, 94, 95
    fn sub_a_n(&mut self, reg: Register) {
        debug!("Instruction sub_a_n reg: {}", reg);

        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f);
        let (res, carry_flag) = self.a.overflowing_sub(value);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Subtract (HL) from A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set if no borrow
    ///
    /// Opcode for 96
    fn sub_a_hl(&mut self) {
        debug!("Instruction sub_a_hl");

        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f);
        let (res, carry_flag) = self.a.overflowing_sub(value);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(8);
    }

    /// Subtract d8 from A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set if no borrow
    ///
    /// Opcode for D6
    fn sub_a_d8(&mut self) {
        debug!("Instruction sub_a_d8");

        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f);
        let (res, carry_flag) = self.a.overflowing_sub(value);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Subtract n + Carry flag from A
    /// n = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set if no borrow
    ///
    /// Opcode for 9F, 98, 99, 9A, 9B, 9C, 9D
    fn sbc_a_n(&mut self, reg: Register) {
        debug!("Instruction sbc_a_n reg: {}", reg);

        let c = if self.carry_flag { 1 } else { 0 };

        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        let res = self.a.wrapping_sub(value).wrapping_sub(c);
        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f) + c;
        let carry_flag = (self.a as u16) < (value as u16) + (c as u16);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Subtract (HL) + carry flag from A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set if no borrow
    ///
    /// Opcode for 9E
    fn sbc_a_hl(&mut self) {
        debug!("Instruction sbc_a_hl");

        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        let c = if self.carry_flag { 1 } else { 0 };

        let res = self.a.wrapping_sub(value).wrapping_sub(c);
        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f) + c;
        let carry_flag = (self.a as u16) < (value as u16) + (c as u16);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(8);
    }

    /// Subtract d8 + Carry flag from A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set if no borrow
    ///
    /// Opcode for DE
    fn sbc_a_d8(&mut self) {
        debug!("Instruction sbc_a_d8");

        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        let c = if self.carry_flag { 1 } else { 0 };

        let res = self.a.wrapping_sub(value).wrapping_sub(c);
        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f) + c;
        let carry_flag = (self.a as u16) < (value as u16) + (c as u16);

        self.a = res;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// And n with A, result in A
    /// n = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H Set
    /// C Reset
    ///
    /// Opcode for A7, A0, A1, A2, A3, A4, A5
    fn and_r8(&mut self, reg: Register) {
        debug!("Instruction and_r8 reg: {}", reg);

        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        self.a &= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(true);
        self.set_carry_flag(false);

        self.add_clock(4);
    }

    /// And (HL) with A, result in A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H Set
    /// C Reset
    ///
    /// Opcode for A6
    fn and_hl(&mut self) {
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        self.a &= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(true);
        self.set_carry_flag(false);

        self.add_clock(8);
    }

    /// And d8 with A, result in A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H Set
    /// C Reset
    ///
    /// Opcode for E6
    fn and_d8(&mut self) {
        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        self.a &= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(true);
        self.set_carry_flag(false);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Or r8 with register A, result in Affected
    /// r8 = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H ReSet
    /// C Reset
    ///
    /// Opcode for B7, B0, B1, B2, B3, B4, B5
    fn or_r8(&mut self, reg: Register) {
        debug!("Instruction or_r8 reg: {}", reg);
        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        self.a |= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_clock(4);
    }

    /// Or (HL) with register A, result in Affected
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H ReSet
    /// C Reset
    ///
    /// Opcode for B6
    fn or_hl(&mut self) {
        debug!("Instruction or_hl");
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        self.a |= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_clock(8);
    }

    /// Or d8 with register A, result in Affected
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H ReSet
    /// C Reset
    ///
    /// Opcode for F6
    fn or_d8(&mut self) {
        debug!("Instruction or_d8");
        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        self.a |= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Xor r8 with register A, result in A
    /// r8 = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H ReSet
    /// C Reset
    ///
    /// Opcode for AF, A8, A9, AA, AB, AC, AD
    fn xor_r8(&mut self, reg: Register) {
        debug!("Instruction xor_r8 reg: {}", reg);
        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        self.a ^= value;
        debug!("xor A self.a: 0x{:02x}, value: {:0b}", self.a, value);

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_clock(4);
    }

    /// Xor (HL) with register A, result in A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H ReSet
    /// C Reset
    ///
    /// Opcode for AE
    fn xor_hl(&mut self) {
        debug!("Instruction xor_hl");
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        self.a ^= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_clock(8);
    }

    /// Xor d8 with register A, result in A
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H ReSet
    /// C Reset
    ///
    /// Opcode for EE
    fn xor_d8(&mut self) {
        debug!("Instruction xor_d8");
        let addr = self.pc;
        let value = self.mmu.read_byte(addr);

        self.a ^= value;

        self.set_zero_flag(self.a == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// compare A with r8. Result are thrown away
    /// r8 = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero (A == r8)
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set for no borrow (set if A < r8)
    ///
    /// Opcode for BF, B8, B9, BA, BB, BC, BD
    fn cp_r8(&mut self, reg: Register) {
        debug!("Instruction cp_r8 reg: {}", reg);
        let value = match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register {}", reg),
        };

        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f);
        let carry_flag = self.a < value;

        self.set_zero_flag(self.a == value);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// compare A with (HL). Result are thrown away
    ///
    /// Affected Flag:
    /// Z Set if result is zero (A == r8)
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set for no borrow (set if A < r8)
    ///
    /// Opcode for BE
    fn cp_hl(&mut self) {
        debug!("Instruction cp_hl");
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(addr);

        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f);
        let carry_flag = self.a < value;

        self.set_zero_flag(self.a == value);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(8);
    }

    /// compare A with d8. Result are thrown away
    ///
    /// Affected Flag:
    /// Z Set if result is zero (A == r8)
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Set for no borrow (set if A < r8)
    ///
    /// Opcode for FE
    fn cp_d8(&mut self) {
        let addr = self.pc;
        let value = self.mmu.read_byte(addr);
        debug!(
            "Instruction cp_d8 addr: 0x{:04x}, value: 0x{:04x}",
            addr, value
        );

        let half_carry_flag = (self.a & 0x0f) < (value & 0x0f);
        let carry_flag = self.a < value;

        self.set_zero_flag(self.a == value);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// INcrement register n
    /// n = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H Set if carry form bit 3
    /// C Not affected
    ///
    /// Opcode for 3C, 04, 0C, 14, 1C, 24, 2C
    fn inc_r8(&mut self, reg: Register) {
        debug!("Instruction inc_r8 reg: {}", reg);
        let value = match reg {
            Register::A => self.a.wrapping_add(1),
            Register::B => self.b.wrapping_add(1),
            Register::C => self.c.wrapping_add(1),
            Register::D => self.d.wrapping_add(1),
            Register::E => self.e.wrapping_add(1),
            Register::H => self.h.wrapping_add(1),
            Register::L => self.l.wrapping_add(1),
            _ => panic!("Invalid register {}", reg),
        };

        let half_carry_flag = (value.wrapping_sub(1) & 0x0f) == 0x0f;

        match reg {
            Register::A => self.a = value,
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
            _ => panic!("Invalid register {}", reg),
        }

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);

        self.add_clock(4);
    }

    /// INcrement (HL)
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H Set if carry form bit 3
    /// C Not affected
    ///
    /// Opcode for 34
    fn inc_hl(&mut self) {
        debug!("Instruction inc_hl");
        let addr = get_addr_from_registers(self.h, self.l);
        let mut value = self.mmu.read_byte(addr);

        value = value.wrapping_add(1);
        self.mmu.write_byte(addr, value);

        let half_carry_flag = (value.wrapping_sub(1) & 0x0f) == 0x0f;

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);

        self.add_clock(12);
    }

    /// Decrement register n
    /// n = A, B, C, D, E, H, L
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Not affected
    ///
    /// Opcode for 3D, 05, 0D, 15, 1D, 25, 2D
    fn dec_r8(&mut self, reg: Register) {
        debug!("dec_r8 reg {}", reg);
        let value = match reg {
            Register::A => {
                self.a = self.a.wrapping_sub(1);
                self.a
            }
            Register::B => {
                self.b = self.b.wrapping_sub(1);
                self.b
            }
            Register::C => {
                self.c = self.c.wrapping_sub(1);
                self.c
            }
            Register::D => {
                self.d = self.d.wrapping_sub(1);
                self.d
            }
            Register::E => {
                self.e = self.e.wrapping_sub(1);
                self.e
            }
            Register::H => {
                self.h = self.h.wrapping_sub(1);
                self.h
            }
            Register::L => {
                self.l = self.l.wrapping_sub(1);
                self.l
            }
            _ => panic!("Invalid register {}", reg),
        };

        let half_carry_flag = (value.wrapping_add(1) & 0x0f) == 0x00;

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);

        self.add_clock(4);
    }

    /// Decrement (HL)
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Set
    /// H Set if no borrow from bit 4
    /// C Not affected
    ///
    /// Opcode for 35
    fn dec_hl(&mut self) {
        debug!("Instruction dec_hl");
        let addr = get_addr_from_registers(self.h, self.l);
        let mut value = self.mmu.read_byte(addr);

        value = value.wrapping_sub(1);
        self.mmu.write_byte(addr, value);

        let half_carry_flag = (value & 0x0f) == 0x0f;

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(true);
        self.set_half_carry_flag(half_carry_flag);

        self.add_clock(12);
    }

    /// add n to HL
    /// n = BC, DE, HL, SP
    ///
    /// Affected Flag:
    /// Z Not affected
    /// N Reset
    /// H Set if carry from bit 11
    /// C Set if carry from bit 15
    ///
    /// Opcode for 09, 19, 29, 39
    fn add_hl_n(&mut self, reg: Register) {
        debug!("Instruction add_hl_n reg: {}", reg);
        let value = match reg {
            Register::BC => ((self.b as u16) << 8) + (self.c as u16),
            Register::DE => ((self.d as u16) << 8) + (self.e as u16),
            Register::HL => ((self.h as u16) << 8) + (self.l as u16),
            Register::SP => self.sp,
            _ => panic!("Invalid register {}", reg),
        };
        let hl = ((self.h as u16) << 8) + (self.l as u16);

        let half_carry_flag = (hl & 0x0fff) + (value & 0x0fff) > 0x0fff;
        let (res, carry_flag) = hl.overflowing_add(value);

        self.h = (res >> 8) as u8;
        self.l = res as u8;

        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_clock(8);
    }

    /// Add n to SP
    /// n = one byte signed immediate value
    ///
    /// Affected Flag:
    /// Z Reset
    /// N Reset
    /// H Set or reset according to operation.
    /// C Set or reset according to operation.
    ///
    /// Opcode for E8
    fn add_sp_d8(&mut self) {
        let addr = self.pc;
        let value = self.mmu.read_byte(addr) as i8 as u16;

        let half_carry_flag = (self.sp & 0x0f) + (value & 0x0f) > 0x0f;
        let carry_flag = (self.sp & 0x00ff) + (value & 0x00ff) > 0x00ff;

        self.sp = self.sp.wrapping_add(value);

        self.set_zero_flag(false);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(half_carry_flag);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        self.add_clock(16);
    }

    /// Increment register nn
    /// nn = BC, DE, HL, SP
    ///
    /// Affected Flag
    /// None
    ///
    /// Opcode for 03, 13, 23, 33
    fn inc_r16(&mut self, reg: Register) {
        debug!("Instruction inc_r16 reg: {}", reg);
        let (mut high_value, mut low_value) = match reg {
            Register::BC => (self.b, self.c),
            Register::DE => (self.d, self.e),
            Register::HL => (self.h, self.l),
            Register::SP => ((self.sp >> 8) as u8, self.sp as u8),
            _ => panic!("Invalid register {}", reg),
        };

        low_value = low_value.wrapping_add(1);
        if low_value == 0 {
            high_value = high_value.wrapping_add(1);
        }

        match reg {
            Register::BC => {
                self.b = high_value;
                self.c = low_value;
            }
            Register::DE => {
                self.d = high_value;
                self.e = low_value;
            }
            Register::HL => {
                self.h = high_value;
                self.l = low_value;
            }
            Register::SP => self.sp = ((high_value as u16) << 8) + low_value as u16,
            _ => panic!("Invalid register {}", reg),
        }

        self.add_clock(8);
    }

    /// Decrement register nn
    /// nn = BC, DE, HL, SP
    ///
    /// Affected Flag
    /// None
    ///
    /// Opcode for 0B, 1B, 2B, 3B
    fn dec_r16(&mut self, reg: Register) {
        debug!("Instruction dec_r16 reg: {}", reg);
        let (mut high_value, mut low_value) = match reg {
            Register::BC => (self.b, self.c),
            Register::DE => (self.d, self.e),
            Register::HL => (self.h, self.l),
            Register::SP => ((self.sp >> 8) as u8, self.sp as u8),
            _ => panic!("Invalid register {}", reg),
        };

        low_value = low_value.wrapping_sub(1);
        if low_value == 0xff {
            high_value = high_value.wrapping_sub(1);
        }

        match reg {
            Register::BC => {
                self.b = high_value;
                self.c = low_value;
            }
            Register::DE => {
                self.d = high_value;
                self.e = low_value;
            }
            Register::HL => {
                self.h = high_value;
                self.l = low_value;
            }
            Register::SP => self.sp = ((high_value as u16) << 8) + low_value as u16,
            _ => panic!("Invalid register {}", reg),
        }

        self.add_clock(8);
    }

    /// Decimal adjust register A
    ///
    /// Flag Affected
    /// Z Set if register A is zero
    /// N Not affected
    /// H Reset
    /// C Set or reset according to operation.
    ///
    /// Opcode for 27
    fn daa(&mut self) {
        debug!("Instruction daa");

        let mut a = self.a;

        if !self.subtraction_flag {
            if self.carry_flag || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.set_carry_flag(true);
            }
            if self.half_carry_flag || a & 0x0f > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if self.carry_flag {
                a = a.wrapping_sub(0x60);
            }
            if self.half_carry_flag {
                a = a.wrapping_sub(0x06)
            }
        }

        self.a = a;

        self.set_zero_flag(a == 0);
        self.set_half_carry_flag(false);

        self.add_clock(4);
    }

    /// Complement A register. (Flip all bits)
    ///
    /// Flag Affected
    /// Z Not affected
    /// N Set
    /// H Set
    /// C Not Affected
    ///
    /// Opcode for 2F
    fn cpl(&mut self) {
        debug!("Instruction cpl");

        self.a = !self.a;

        self.set_subtraction_flag(true);
        self.set_half_carry_flag(true);

        self.add_clock(4);
    }

    /// Complement carry falg
    ///
    /// Flag Affected
    /// Z Not affected
    /// N Reset
    /// H Reset
    /// C Complemented
    ///
    /// Opcode for 3F
    fn ccf(&mut self) {
        debug!("Instruction ccf");

        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(!self.carry_flag);

        self.add_clock(4);
    }

    /// Set carry flag
    ///
    /// Flag Affected
    /// Z Not affected
    /// N Reset
    /// H Reset
    /// C Set
    ///
    /// Opcode for 37
    fn scf(&mut self) {
        debug!("Instruction scf");

        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(true);

        self.add_clock(4);
    }

    /// No operation
    ///
    /// Opcode for 00
    fn nop(&mut self) {
        debug!("Instruction nop");

        self.add_clock(4);
    }

    /// Halt instruction
    /// Opcode for 76
    fn halt(&mut self) {
        debug!("Instruction halt");

        if self.ime {
            self.halt = true;
        }

        self.add_clock(4);
    }

    /// Stop instruction
    /// Opcode for 10
    fn stop(&mut self) {
        debug!("Instruction stop");

        self.add_clock(4);
    }

    /// DI
    ///
    /// Flag Affected
    /// None
    ///
    /// Opcode for F3
    fn di(&mut self) {
        debug!("Instruction DI");

        self.ime = false;

        self.add_clock(4);
    }

    // EI
    ///
    /// Flag Affected
    /// None
    ///
    /// Opcode for FB
    fn ei(&mut self) {
        debug!("Instruction ei");

        self.ime = true;

        self.add_clock(4);
    }

    /// Rotate A left. Old bit 7 to carry flag
    ///
    /// Affected Flag:
    /// Z Reset
    /// N Reset
    /// H Reset
    /// C set if carry flag
    ///
    /// Opcode for 07
    fn rlca(&mut self) {
        debug!("Instruction rlca");

        let a = self.a;
        let carry_flag = ((a >> 7) & 0x01) == 0x01;

        self.a = a.rotate_left(1);

        self.set_zero_flag(false);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Rotate A left through carry flag.
    ///
    /// Affected Flag:
    /// Z Reset
    /// N Reset
    /// H Reset
    /// C set if carry flag
    ///
    /// Opcode for 17
    fn rla(&mut self) {
        debug!("Instruction rla");
        let c = if self.carry_flag { 1 } else { 0 };
        let carry_flag = (self.a >> 7) & 1 == 1;
        self.a = (self.a << 1) | c;

        self.set_zero_flag(false);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Rotate A right. Old bit 0 to Carry flag.
    ///
    /// Affected Flag:
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 0 data
    ///
    /// Opcode for 0F
    fn rrca(&mut self) {
        debug!("Instruction rrca");
        let carry_flag = self.a & 1 == 1;
        self.a = self.a.rotate_right(1);

        self.set_zero_flag(false);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Rotate A ritght through Carry flag
    ///
    /// Affected Flag
    /// Z Reset
    /// N Reset
    /// H Reset
    /// C Reset
    ///
    /// Opcode for 1F
    fn rra(&mut self) {
        debug!("Instruction rra");
        let carry_flag = self.a & 1 == 1;
        let c = if self.carry_flag { 1 } else { 0 };
        self.a = (self.a >> 1) | (c << 7);

        self.set_zero_flag(false);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_clock(4);
    }

    /// Rotate n left. Old 7 to Carry flag
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old 7 data
    ///
    /// Opcode for CB (07, 00, 01, 02, 03, 04, 05, 06)
    fn rlc_n(&mut self, reg: Register) {
        debug!("Instruction rlc_n reg: {}", reg);
        let value = self.read_r8(reg);

        let carry_flag = (value >> 7) & 1 == 1;
        let value = value.rotate_left(1);

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Rotate n right. Old bit 0 to Carry flag
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 0 data
    ///
    /// Opcode for CB (0F, 08, 09, 0A, 0B, 0C, 0D, 0E)
    fn rrc_n(&mut self, reg: Register) {
        debug!("Instruction rrc_n reg: {}", reg);
        let value = self.read_r8(reg);

        let carry_flag = (value & 1) == 1;
        let value = value.rotate_right(1);

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Rotate n left through Carry flag
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 7 data
    ///
    /// Opcode for CB (17, 10, 11, 12, 13, 14, 15, 16)
    fn rl_n(&mut self, reg: Register) {
        debug!("Instruction rl_n reg: {}", reg);
        let value = self.read_r8(reg);
        let c = if self.carry_flag { 1 } else { 0 };

        let carry_flag = ((value >> 7) & 1) == 1;

        let value = (value << 1) | c;

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Rotate n right through Carry flag
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 7 data
    ///
    /// Opcode for CB (1F, 18, 19, 1A, 1B, 1C, 1D, 1E)
    fn rr_n(&mut self, reg: Register) {
        debug!("Instruction rr_n reg: {}", reg);
        let value = self.read_r8(reg);
        let c = if self.carry_flag { 1 } else { 0 };

        let carry_flag = (value & 1) == 1;

        let value = (value >> 1) | (c << 7);

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Shift n left into Carry. LSB of n set ot 0
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 7 data
    ///
    /// Opcode for CB (27, 20, 21, 22, 23, 24, 25, 26)
    fn sla_n(&mut self, reg: Register) {
        debug!("Instruction sla_n reg: {}", reg);
        let value = self.read_r8(reg);

        let carry_flag = (value & 0x80) > 0;
        let value = value << 1;

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Shift n right into Carry. MSB doesn't change
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 0 data
    ///
    /// Opcode for CB (2F, 28, 29, 2A, 2B, 2C, 2D, 2E)
    fn sra_n(&mut self, reg: Register) {
        debug!("Instruction sra_n reg: {}", reg);
        let value = self.read_r8(reg);

        let carry_flag = (value & 1) == 1;
        let value = (value >> 1) | (value & 0x80);

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Swap upper & lower nibles of n.
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Reset
    ///
    /// Opcode for CB (37, 30, 31, 32, 33, 34, 35, 36)
    fn swap(&mut self, reg: Register) {
        debug!("Instruction Swap reg: {}", reg);
        let value = self.read_r8(reg);
        let value = ((value & 0xf0) >> 4) | ((value & 0x0f) << 4);

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(false);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Shift n right into carry. MSB set to 0.
    /// n = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Contains old bit 0 data
    ///
    /// Opcode for CB (3F, 38, 39, 3A, 3B, 3C, 3D, 3E)
    fn srl_n(&mut self, reg: Register) {
        debug!("Instruction srl_n reg: {}", reg);
        let value = self.read_r8(reg);

        let carry_flag = (value & 1) == 1;

        let value = value >> 1;

        self.write_r8(reg, value);

        self.set_zero_flag(value == 0);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(false);
        self.set_carry_flag(carry_flag);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Test bit b in register r.
    /// b = 0 - 7
    /// r = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// Z Set if result is zero
    /// N Reset
    /// H Reset
    /// C Not affected
    ///
    /// Opcode for CB (40 - 7F)
    fn bit(&mut self, reg: Register, b: u8) {
        debug!("Instruction bit reg: {}, bit: {}", reg, b);

        let value = self.read_r8(reg);

        let zero_flag = ((value >> b) & 1) == 0;

        self.set_zero_flag(zero_flag);
        self.set_subtraction_flag(false);
        self.set_half_carry_flag(true);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(12);
        } else {
            self.add_clock(8);
        }
    }

    /// Reset bit b in register r
    /// b = 0 - 7
    /// r = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// None
    ///
    /// Opcode for CB (80 - BF)
    fn res(&mut self, reg: Register, b: u8) {
        debug!("Instruction res reg: {}, bit b: {}", reg, b);

        let value = self.read_r8(reg);
        let value = value & !(1 << b);
        self.write_r8(reg, value);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Set bit b in register r
    /// b = 0 - 7
    /// r = A, B, C, D, E, H, L, (HL)
    ///
    /// Affected Flag
    /// None
    ///
    /// Opcode for CB (C0 - FF)
    fn set(&mut self, reg: Register, b: u8) {
        debug!("Instruction set reg: {}, bit: {}", reg, b);

        let value = self.read_r8(reg);
        let value = value | (1 << b);

        self.write_r8(reg, value);

        self.add_program_count(1);
        if reg == Register::HL {
            self.add_clock(16);
        } else {
            self.add_clock(8);
        }
    }

    /// Prefix CB
    fn prefix_cb(&mut self) {
        debug!("Instruction prefix_cb");
        let pc = self.pc;
        let opcode = self.mmu.read_byte(pc);
        let b = (opcode >> 3) & 0x07;

        let reg = match opcode & 0x07 {
            0x00 => Register::B,
            0x01 => Register::C,
            0x02 => Register::D,
            0x03 => Register::E,
            0x04 => Register::H,
            0x05 => Register::L,
            0x06 => Register::HL,
            0x07 => Register::A,
            _ => panic!("Invalid opcode: {}", opcode),
        };

        match opcode {
            0x00..=0x07 => self.rlc_n(reg),
            0x08..=0x0f => self.rrc_n(reg),
            0x10..=0x17 => self.rl_n(reg),
            0x18..=0x1f => self.rr_n(reg),
            0x20..=0x27 => self.sla_n(reg),
            0x28..=0x2f => self.sra_n(reg),
            0x30..=0x37 => self.swap(reg),
            0x38..=0x3f => self.srl_n(reg),
            0x40..=0x7f => self.bit(reg, b),
            0x80..=0xbf => self.res(reg, b),
            0xc0..=0xff => self.set(reg, b),
        }
    }

    /// Junm to adress nn
    /// nn = two byte immediate value
    ///
    /// Opcode for C3
    fn jp_nn(&mut self) {
        debug!("Instruction jp_nn");
        let addr = self.pc;
        let value = self.read_word(addr);
        self.pc = value;

        // self.add_program_count(2);
        self.add_clock(16);
    }

    /// Jump to address nn if following condition is true:
    /// cc = NZ, Jump if Z flag is reset.
    /// cc =  Z, Jump if Z flag is set.
    /// cc = NC, Jump if C flag is reset.
    /// cc =  C, Jump if C flag is set.
    ///
    /// Opcode for C2, CA, D2, DA
    fn jump_cc_nn(&mut self, cc: CcFlag) {
        debug!("Instruction jump_cc_nn {}", cc);
        let flag = match cc {
            CcFlag::NZ => !self.zero_flag,
            CcFlag::Z => self.zero_flag,
            CcFlag::NC => !self.carry_flag,
            CcFlag::C => self.carry_flag,
        };

        if flag {
            let addr = self.pc;
            let value = self.read_word(addr);
            self.pc = value;

            self.add_clock(16);
        } else {
            self.add_program_count(2);
            self.add_clock(12)
        }
    }

    /// Jump to address contained in HL.
    ///
    /// Opcode for E9
    fn jump_hl(&mut self) {
        debug!("Instruction jump_hl");
        let addr = get_addr_from_registers(self.h, self.l);
        self.pc = addr;

        self.add_clock(4);
    }

    /// Add n to current address and jump to it.
    /// n = one byte signed immediate value
    ///
    /// Opcode for 18
    fn jr_n(&mut self) {
        debug!("Instruction jr_n");
        let addr = self.pc;
        let value = self.mmu.read_byte(addr) as i8;
        self.pc = self.pc.wrapping_add(value as u16);

        self.add_program_count(1);
        self.add_clock(12);
    }

    /// If following condition is true then add n to current
    /// address and jump to it.
    /// n = one byte signed immediate value
    /// cc = NZ, Jump if Z flag is reset.
    /// cc =  Z, Jump if Z flag is set.
    /// cc = NC, Jump if C flag is reset.
    /// cc =  C, Jump if C flag is set.
    ///
    /// Opcode for 20, 28, 30, 38
    fn jr_cc_n(&mut self, cc: CcFlag) {
        debug!("Instruction jr_cc_n {}", cc);
        let flag = match cc {
            CcFlag::NZ => !self.zero_flag,
            CcFlag::Z => self.zero_flag,
            CcFlag::NC => !self.carry_flag,
            CcFlag::C => self.carry_flag,
        };

        if flag {
            let addr = self.pc;
            let value = self.mmu.read_byte(addr) as i8;
            self.pc = self.pc.wrapping_add(value as u16).wrapping_add(1);
            self.add_clock(12);
        } else {
            self.add_program_count(1);
            self.add_clock(8);
        }
    }

    /// Push address of next instruction onto stack and then
    /// jump to address nn.
    /// nn = two byte immediate value
    ///
    /// Opcode for CD
    fn call_nn(&mut self) {
        let addr = self.read_word(self.pc);
        debug!("Instruction call_nn 0x{:04x}", addr);

        self.add_program_count(2);
        self.sp = self.sp.wrapping_sub(2);

        let sp = self.sp;
        let pc = self.pc;
        debug!("call_nn sp: 0x{:04x}, pc: 0x{:04x}", sp, pc);
        self.write_word(sp, pc);

        // self.add_program_count(value);
        self.pc = addr;
        self.add_clock(24);
    }

    /// Call address nn if following condition is true.
    /// nn = two byte signed immediate value
    /// cc = NZ, Jump if Z flag is reset.
    /// cc =  Z, Jump if Z flag is set.
    /// cc = NC, Jump if C flag is reset.
    /// cc =  C, Jump if C flag is set.
    ///
    /// Opcode for C4, CC, D4, DC
    fn call_cc_nn(&mut self, cc: CcFlag) {
        debug!("Instruction call_cc_nn {}", cc);
        let flag = match cc {
            CcFlag::NZ => !self.zero_flag,
            CcFlag::Z => self.zero_flag,
            CcFlag::NC => !self.carry_flag,
            CcFlag::C => self.carry_flag,
        };
        if flag {
            let addr = self.read_word(self.pc);
            self.add_program_count(2);

            self.sp = self.sp.wrapping_sub(2);

            let sp = self.sp;
            let pc = self.pc;
            self.write_word(sp, pc);

            self.pc = addr;
            self.add_clock(24);
        } else {
            self.add_program_count(2);
            self.add_clock(12);
        }
    }

    /// Push present address onto stack.
    /// Jump to address $0000 + n.
    /// n = $00, $08, $10, $18, $20, $28, $30, $38
    ///
    /// Opcode for C7, CF, D7, DF, E7, EF, F7, FF
    fn rst_n(&mut self, n: u16) {
        debug!("Instruction rst_n {}", n);
        self.sp = self.sp.wrapping_sub(2);
        let sp = self.sp;
        let pc = self.pc;
        self.write_word(sp, pc);

        self.pc = n;
        self.add_clock(16);
    }

    /// Pop two bytes from stack & jump to that address
    /// Opcode for C9
    fn ret(&mut self) {
        debug!("Instruction ret ");
        let sp = self.sp;
        let addr = self.read_word(sp);
        self.pc = addr;
        self.sp = self.sp.wrapping_add(2);

        self.add_clock(16);
    }

    /// Pop two bytes from stack & jump to that address
    /// Return if following condition is true
    /// cc = NZ, Jump if Z flag is reset.
    /// cc =  Z, Jump if Z flag is set.
    /// cc = NC, Jump if C flag is reset.
    /// cc =  C, Jump if C flag is set.
    ///
    /// Opcode for C0, C8, D0, D8
    fn ret_cc(&mut self, cc: CcFlag) {
        debug!("Instruction ret_cc {}", cc);
        let flag = match cc {
            CcFlag::NZ => !self.zero_flag,
            CcFlag::Z => self.zero_flag,
            CcFlag::NC => !self.carry_flag,
            CcFlag::C => self.carry_flag,
        };
        if flag {
            let sp = self.sp;
            let addr = self.read_word(sp);
            self.pc = addr;
            self.sp = self.sp.wrapping_add(2);

            self.add_clock(20);
        } else {
            self.add_clock(8);
        }
    }

    /// Pop two bytes from stack & jump to that address then
    /// enable interrupts.
    /// Opcode for D9
    fn reti(&mut self) {
        debug!("Instruction reti");
        let sp = self.sp;
        let addr = self.read_word(sp);
        self.pc = addr;
        self.sp = self.sp.wrapping_add(2);

        self.ime = true;

        self.add_clock(16);
    }

    pub fn exec(&mut self, opcode: u8) {
        match opcode {
            // 00
            0x00 => self.nop(),
            0x01 => self.load_n_nn(Register::BC),
            0x02 => self.load_nn_a(Register::BC),
            0x03 => self.inc_r16(Register::BC),
            0x04 => self.inc_r8(Register::B),
            0x05 => self.dec_r8(Register::B),
            0x06 => self.load_nn_n(Register::B),
            0x07 => self.rlca(),
            0x08 => self.load_nn_sp(),
            0x09 => self.add_hl_n(Register::BC),
            0x0A => self.load_a_nn(Register::BC),
            0x0B => self.dec_r16(Register::BC),
            0x0C => self.inc_r8(Register::C),
            0x0D => self.dec_r8(Register::C),
            0x0E => self.load_nn_n(Register::C),
            0x0F => self.rrca(),
            // 10
            0x10 => self.stop(),
            0x11 => self.load_n_nn(Register::DE),
            0x12 => self.load_nn_a(Register::DE),
            0x13 => self.inc_r16(Register::DE),
            0x14 => self.inc_r8(Register::D),
            0x15 => self.dec_r8(Register::D),
            0x16 => self.load_nn_n(Register::D),
            0x17 => self.rla(),
            0x18 => self.jr_n(),
            0x19 => self.add_hl_n(Register::DE),
            0x1A => self.load_a_nn(Register::DE),
            0x1B => self.dec_r16(Register::DE),
            0x1C => self.inc_r8(Register::E),
            0x1D => self.dec_r8(Register::E),
            0x1E => self.load_nn_n(Register::E),
            0x1F => self.rra(),
            // 20
            0x20 => self.jr_cc_n(CcFlag::NZ),
            0x21 => self.load_n_nn(Register::HL),
            0x22 => self.load_hli_a(),
            0x23 => self.inc_r16(Register::HL),
            0x24 => self.inc_r8(Register::H),
            0x25 => self.dec_r8(Register::H),
            0x26 => self.load_nn_n(Register::H),
            0x27 => self.daa(),
            0x28 => self.jr_cc_n(CcFlag::Z),
            0x29 => self.add_hl_n(Register::HL),
            0x2A => self.load_a_hli(),
            0x2B => self.dec_r16(Register::HL),
            0x2C => self.inc_r8(Register::L),
            0x2D => self.dec_r8(Register::L),
            0x2E => self.load_nn_n(Register::L),
            0x2F => self.cpl(),
            // 30
            0x30 => self.jr_cc_n(CcFlag::NC),
            0x31 => self.load_n_nn(Register::SP),
            0x32 => self.load_hld_a(),
            0x33 => self.inc_r16(Register::SP),
            0x34 => self.inc_hl(),
            0x35 => self.dec_hl(),
            0x36 => self.load_hl_imm(),
            0x37 => self.scf(),
            0x38 => self.jr_cc_n(CcFlag::C),
            0x39 => self.add_hl_n(Register::SP),
            0x3A => self.load_a_hld(),
            0x3B => self.dec_r16(Register::SP),
            0x3C => self.inc_r8(Register::A),
            0x3D => self.dec_r8(Register::A),
            0x3E => self.load_a_d8(),
            0x3F => self.ccf(),
            // 40
            0x40 => self.load_r1_r2(Register::B, Register::B),
            0x41 => self.load_r1_r2(Register::B, Register::C),
            0x42 => self.load_r1_r2(Register::B, Register::D),
            0x43 => self.load_r1_r2(Register::B, Register::E),
            0x44 => self.load_r1_r2(Register::B, Register::H),
            0x45 => self.load_r1_r2(Register::B, Register::L),
            0x46 => self.load_r1_hl(Register::B),
            0x47 => self.load_r1_r2(Register::B, Register::A),
            0x48 => self.load_r1_r2(Register::C, Register::B),
            0x49 => self.load_r1_r2(Register::C, Register::C),
            0x4A => self.load_r1_r2(Register::C, Register::D),
            0x4B => self.load_r1_r2(Register::C, Register::E),
            0x4C => self.load_r1_r2(Register::C, Register::H),
            0x4D => self.load_r1_r2(Register::C, Register::L),
            0x4E => self.load_r1_hl(Register::C),
            0x4F => self.load_r1_r2(Register::C, Register::A),
            // 50
            0x50 => self.load_r1_r2(Register::D, Register::B),
            0x51 => self.load_r1_r2(Register::D, Register::C),
            0x52 => self.load_r1_r2(Register::D, Register::D),
            0x53 => self.load_r1_r2(Register::D, Register::E),
            0x54 => self.load_r1_r2(Register::D, Register::H),
            0x55 => self.load_r1_r2(Register::D, Register::L),
            0x56 => self.load_r1_hl(Register::D),
            0x57 => self.load_r1_r2(Register::D, Register::A),
            0x58 => self.load_r1_r2(Register::E, Register::B),
            0x59 => self.load_r1_r2(Register::E, Register::C),
            0x5A => self.load_r1_r2(Register::E, Register::D),
            0x5B => self.load_r1_r2(Register::E, Register::E),
            0x5C => self.load_r1_r2(Register::E, Register::H),
            0x5D => self.load_r1_r2(Register::E, Register::L),
            0x5E => self.load_r1_hl(Register::E),
            0x5F => self.load_r1_r2(Register::E, Register::A),
            // 60
            0x60 => self.load_r1_r2(Register::H, Register::B),
            0x61 => self.load_r1_r2(Register::H, Register::C),
            0x62 => self.load_r1_r2(Register::H, Register::D),
            0x63 => self.load_r1_r2(Register::H, Register::E),
            0x64 => self.load_r1_r2(Register::H, Register::H),
            0x65 => self.load_r1_r2(Register::H, Register::L),
            0x66 => self.load_r1_hl(Register::H),
            0x67 => self.load_r1_r2(Register::H, Register::A),
            0x68 => self.load_r1_r2(Register::L, Register::B),
            0x69 => self.load_r1_r2(Register::L, Register::C),
            0x6A => self.load_r1_r2(Register::L, Register::D),
            0x6B => self.load_r1_r2(Register::L, Register::E),
            0x6C => self.load_r1_r2(Register::L, Register::H),
            0x6D => self.load_r1_r2(Register::L, Register::L),
            0x6E => self.load_r1_hl(Register::L),
            0x6F => self.load_r1_r2(Register::L, Register::A),
            // 70
            0x70 => self.load_hl_r1(Register::B),
            0x71 => self.load_hl_r1(Register::C),
            0x72 => self.load_hl_r1(Register::D),
            0x73 => self.load_hl_r1(Register::E),
            0x74 => self.load_hl_r1(Register::H),
            0x75 => self.load_hl_r1(Register::L),
            0x76 => self.halt(),
            0x77 => self.load_hl_r1(Register::A),
            0x78 => self.load_r1_r2(Register::A, Register::B),
            0x79 => self.load_r1_r2(Register::A, Register::C),
            0x7A => self.load_r1_r2(Register::A, Register::D),
            0x7B => self.load_r1_r2(Register::A, Register::E),
            0x7C => self.load_r1_r2(Register::A, Register::H),
            0x7D => self.load_r1_r2(Register::A, Register::L),
            0x7E => self.load_r1_hl(Register::A),
            0x7F => self.load_r1_r2(Register::A, Register::A),
            // 80
            0x80 => self.add_a_r(Register::B),
            0x81 => self.add_a_r(Register::C),
            0x82 => self.add_a_r(Register::D),
            0x83 => self.add_a_r(Register::E),
            0x84 => self.add_a_r(Register::H),
            0x85 => self.add_a_r(Register::L),
            0x86 => self.add_a_hl(),
            0x87 => self.add_a_r(Register::A),
            0x88 => self.adc_a_n(Register::B),
            0x89 => self.adc_a_n(Register::C),
            0x8A => self.adc_a_n(Register::D),
            0x8B => self.adc_a_n(Register::E),
            0x8C => self.adc_a_n(Register::H),
            0x8D => self.adc_a_n(Register::L),
            0x8E => self.adc_a_hl(),
            0x8F => self.adc_a_n(Register::A),
            // 90
            0x90 => self.sub_a_n(Register::B),
            0x91 => self.sub_a_n(Register::C),
            0x92 => self.sub_a_n(Register::D),
            0x93 => self.sub_a_n(Register::E),
            0x94 => self.sub_a_n(Register::H),
            0x95 => self.sub_a_n(Register::L),
            0x96 => self.sub_a_hl(),
            0x97 => self.sub_a_n(Register::A),
            0x98 => self.sbc_a_n(Register::B),
            0x99 => self.sbc_a_n(Register::C),
            0x9A => self.sbc_a_n(Register::D),
            0x9B => self.sbc_a_n(Register::E),
            0x9C => self.sbc_a_n(Register::H),
            0x9D => self.sbc_a_n(Register::L),
            0x9E => self.sbc_a_hl(),
            0x9F => self.sbc_a_n(Register::A),
            // A0
            0xA0 => self.and_r8(Register::B),
            0xA1 => self.and_r8(Register::C),
            0xA2 => self.and_r8(Register::D),
            0xA3 => self.and_r8(Register::E),
            0xA4 => self.and_r8(Register::H),
            0xA5 => self.and_r8(Register::L),
            0xA6 => self.and_hl(),
            0xA7 => self.and_r8(Register::A),
            0xA8 => self.xor_r8(Register::B),
            0xA9 => self.xor_r8(Register::C),
            0xAA => self.xor_r8(Register::D),
            0xAB => self.xor_r8(Register::E),
            0xAC => self.xor_r8(Register::H),
            0xAD => self.xor_r8(Register::L),
            0xAE => self.xor_hl(),
            0xAF => self.xor_r8(Register::A),
            // B0
            0xB0 => self.or_r8(Register::B),
            0xB1 => self.or_r8(Register::C),
            0xB2 => self.or_r8(Register::D),
            0xB3 => self.or_r8(Register::E),
            0xB4 => self.or_r8(Register::H),
            0xB5 => self.or_r8(Register::L),
            0xB6 => self.or_hl(),
            0xB7 => self.or_r8(Register::A),
            0xB8 => self.cp_r8(Register::B),
            0xB9 => self.cp_r8(Register::C),
            0xBA => self.cp_r8(Register::D),
            0xBB => self.cp_r8(Register::E),
            0xBC => self.cp_r8(Register::H),
            0xBD => self.cp_r8(Register::L),
            0xBE => self.cp_hl(),
            0xBF => self.cp_r8(Register::A),
            // C0
            0xC0 => self.ret_cc(CcFlag::NZ),
            0xC1 => self.pop_nn(Register::B, Register::C),
            0xC2 => self.jump_cc_nn(CcFlag::NZ),
            0xC3 => self.jp_nn(),
            0xC4 => self.call_cc_nn(CcFlag::NZ),
            0xC5 => self.push_nn(Register::B, Register::C),
            0xC6 => self.add_a_d8(),
            0xC7 => self.rst_n(0x00),
            0xC8 => self.ret_cc(CcFlag::Z),
            0xC9 => self.ret(),
            0xCA => self.jump_cc_nn(CcFlag::Z),
            0xCB => self.prefix_cb(),
            0xCC => self.call_cc_nn(CcFlag::Z),
            0xCD => self.call_nn(),
            0xCE => self.adc_a_d8(),
            0xCF => self.rst_n(0x08),
            // D0
            0xD0 => self.ret_cc(CcFlag::NC),
            0xD1 => self.pop_nn(Register::D, Register::E),
            0xD2 => self.jump_cc_nn(CcFlag::NC),
            0xD3 => panic!("Invalid opcode {}", opcode),
            0xD4 => self.call_cc_nn(CcFlag::NC),
            0xD5 => self.push_nn(Register::D, Register::E),
            0xD6 => self.sub_a_d8(),
            0xD7 => self.rst_n(0x10),
            0xD8 => self.ret_cc(CcFlag::C),
            0xD9 => self.reti(),
            0xDA => self.jump_cc_nn(CcFlag::C),
            0xDB => panic!("Invalid opcode {}", opcode),
            0xDC => self.call_cc_nn(CcFlag::C),
            0xDD => panic!("Invalid opcode {}", opcode),
            0xDE => self.sbc_a_d8(),
            0xDF => self.rst_n(0x18),
            // E0
            0xE0 => self.load_n_a(),
            0xE1 => self.pop_nn(Register::H, Register::L),
            0xE2 => self.load_c_a(),
            0xE3 => panic!("Invalid opcode {}", opcode),
            0xE4 => panic!("Invalid opcode {}", opcode),
            0xE5 => self.push_nn(Register::H, Register::L),
            0xE6 => self.and_d8(),
            0xE7 => self.rst_n(0x20),
            0xE8 => self.add_sp_d8(),
            0xE9 => self.jump_hl(),
            0xEA => self.load_imm_a(),
            0xEB => panic!("Invalid opcode {}", opcode),
            0xEC => panic!("Invalid opcode {}", opcode),
            0xED => panic!("Invalid opcode {}", opcode),
            0xEE => self.xor_d8(),
            0xEF => self.rst_n(0x28),
            // F0
            0xF0 => self.load_a_n(),
            0xF1 => self.pop_nn(Register::A, Register::F),
            0xF2 => self.load_a_c(),
            0xF3 => self.di(),
            0xF4 => panic!("Invalid opcode {}", opcode),
            0xF5 => self.push_nn(Register::A, Register::F),
            0xF6 => self.or_d8(),
            0xF7 => self.rst_n(0x30),
            0xF8 => self.load_sp_n(),
            0xF9 => self.load_sp_hl(),
            0xFA => self.load_a_imm(),
            0xFB => self.ei(),
            0xFC => panic!("Invalid opcode {}", opcode),
            0xFD => panic!("Invalid opcode {}", opcode),
            0xFE => self.cp_d8(),
            0xFF => self.rst_n(0x38),
        }
    }

    fn add_program_count(&mut self, count: u16) {
        self.pc = self.pc.wrapping_add(count)
    }

    fn add_clock(&mut self, count: u32) {
        self.clock = self.clock.wrapping_add(count)
    }

    fn set_zero_flag(&mut self, flag: bool) {
        self.zero_flag = flag;
        self.f = (self.f & !(1 << 7)) | (u8::from(flag) << 7);
    }

    fn set_subtraction_flag(&mut self, flag: bool) {
        self.subtraction_flag = flag;
        self.f = (self.f & !(1 << 6)) | (u8::from(flag) << 6);
    }

    fn set_half_carry_flag(&mut self, flag: bool) {
        self.half_carry_flag = flag;
        self.f = (self.f & !(1 << 5)) | (u8::from(flag) << 5);
    }

    fn set_carry_flag(&mut self, flag: bool) {
        self.carry_flag = flag;
        self.f = (self.f & !(1 << 4)) | (u8::from(flag) << 4);
    }

    /// Get byte from F flags
    fn get_byte_from_flags(&self) -> u8 {
        let mut res = 0;
        if self.zero_flag {
            res |= 0b1000_0000;
        }
        if self.subtraction_flag {
            res |= 0b0100_0000;
        }
        if self.half_carry_flag {
            res |= 0b0010_0000;
        }
        if self.carry_flag {
            res |= 0b0001_0000;
        }
        res
    }

    /// set flags from 8bit value
    fn set_flags_from_byte(&mut self, value: u8) {
        if (value & 0b1000_0000) > 0 {
            self.set_zero_flag(true);
        } else {
            self.set_zero_flag(false);
        }

        if (value & 0b0100_0000) > 0 {
            self.set_subtraction_flag(true);
        } else {
            self.set_subtraction_flag(false);
        }

        if (value & 0b0010_0000) > 0 {
            self.set_half_carry_flag(true);
        } else {
            self.set_half_carry_flag(false);
        }
        if (value & 0b0001_0000) > 0 {
            self.set_carry_flag(true);
        } else {
            self.set_carry_flag(false);
        }
    }

    /// Read 8 byte value from register
    /// Regisger for A, B, C, D, E, H, L, (HL)
    fn read_r8(&mut self, reg: Register) -> u8 {
        debug!("read_r8");
        match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            Register::HL => {
                let addr = get_addr_from_registers(self.h, self.l);
                self.mmu.read_byte(addr)
            }
            _ => panic!("Invalid register {}", reg),
        }
    }

    /// Write 8 byte value to register
    /// Regisger for A, B, C, D, E, H, L, (HL)
    fn write_r8(&mut self, reg: Register, value: u8) {
        debug!("write_r8");
        match reg {
            Register::A => self.a = value,
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
            Register::HL => {
                let addr = get_addr_from_registers(self.h, self.l);
                self.mmu.write_byte(addr, value);
            }
            _ => panic!("Invalid register {}", reg),
        }
    }

    fn read_word(&mut self, addr: u16) -> u16 {
        let low_value = self.mmu.read_byte(addr);
        let high_value = self.mmu.read_byte(addr.wrapping_add(1));

        ((high_value as u16) << 8) + (low_value as u16)
    }

    fn write_word(&mut self, addr: u16, value: u16) {
        let low_value = (value & 0xff) as u8;
        let high_value = (value >> 8) as u8;

        debug!(
            "write_word low_value: 0x{:0x}, high_value: {:0x}",
            low_value, high_value
        );
        self.mmu.write_byte(addr, low_value);
        self.mmu.write_byte(addr.wrapping_add(1), high_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_byte_from_flags_zero() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_zero_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b1000_0000, res);
    }

    #[test]
    fn test_get_byte_from_flags_sub() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_subtraction_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b0100_0000, res);
    }

    #[test]
    fn test_get_byte_from_flags_half() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_half_carry_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b0010_0000, res);
    }

    #[test]
    fn test_get_byte_from_flags_carry() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_carry_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b0001_0000, res);
    }

    #[test]
    fn test_get_byte_from_flags_all() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_zero_flag(true);
        cpu.set_subtraction_flag(true);
        cpu.set_half_carry_flag(true);
        cpu.set_carry_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b1111_0000, res);
    }
    #[test]
    fn test_set_flags_from_bytes_zero() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_flags_from_byte(128);
        assert!(cpu.zero_flag);
        assert!(!cpu.subtraction_flag);
        assert!(!cpu.half_carry_flag);
        assert!(!cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_sub() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_flags_from_byte(64);
        assert!(!cpu.zero_flag);
        assert!(cpu.subtraction_flag);
        assert!(!cpu.half_carry_flag);
        assert!(!cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_half() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_flags_from_byte(32);
        assert!(!cpu.zero_flag);
        assert!(!cpu.subtraction_flag);
        assert!(cpu.half_carry_flag);
        assert!(!cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_carry() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_flags_from_byte(16);
        assert!(!cpu.zero_flag);
        assert!(!cpu.subtraction_flag);
        assert!(!cpu.half_carry_flag);
        assert!(cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_all() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.set_flags_from_byte(248);
        assert!(cpu.zero_flag);
        assert!(cpu.subtraction_flag);
        assert!(cpu.half_carry_flag);
        assert!(cpu.carry_flag);
    }

    #[test]
    fn test_read_r8_all() {
        let mut cpu = Cpu::new("cartridges/hello.gb");
        cpu.write_r8(Register::A, 1);
        cpu.write_r8(Register::B, 2);
        cpu.write_r8(Register::C, 3);
        cpu.write_r8(Register::D, 4);
        cpu.write_r8(Register::E, 5);
        cpu.write_r8(Register::H, 6);
        cpu.write_r8(Register::L, 7);
        // cpu.write_r8(Register::HL, 8); TODO

        assert_eq!(cpu.read_r8(Register::A), 1);
        assert_eq!(cpu.read_r8(Register::B), 2);
        assert_eq!(cpu.read_r8(Register::C), 3);
        assert_eq!(cpu.read_r8(Register::D), 4);
        assert_eq!(cpu.read_r8(Register::E), 5);
        assert_eq!(cpu.read_r8(Register::H), 6);
        assert_eq!(cpu.read_r8(Register::L), 7);
        // assert_eq!(cpu.read_r8(Register::HL), 8);TODO
    }
}
