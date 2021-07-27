use log::{debug, info};

use crate::mmu::Mmu;
use crate::register::Register;
use crate::utils::get_addr_from_registers;

pub struct Cpu {
    a: u8,
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

    mmu: Mmu,
    clock: u32,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
            zero_flag: false,
            subtraction_flag: false,
            half_carry_flag: false,
            carry_flag: false,

            mmu: Mmu::new(),
            clock: 0,
        }
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

        self.add_program_count(8);
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
        let addr = self.mmu.read_word(pc);
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
        let addr = self.mmu.read_word(pc);
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_a_imm addr: {}, value: {}", addr, value);

        self.add_program_count(2);
        self.add_clock(16);
    }

    /// Put value at address 0xFF00 + register C into A
    /// Opcode for E2
    fn load_a_c(&mut self) {
        let addr = 0xFF00 + self.c as u16;
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_a_c addr: {}, value: {}", addr, value);

        self.add_program_count(1);
        self.add_clock(8);
    }

    /// Put A into address 0xFF00 + register C
    /// Opcode for F2
    fn load_c_a(&mut self) {
        let addr = 0xFF00 + self.c as u16;
        let value = self.a;
        self.mmu.write_byte(addr, value);

        debug!("Instruction load_c_a addr: {}, value: {}", addr, value);

        self.add_program_count(1);
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
        self.mmu.write_byte(addr, value);

        self.l = self.l.wrapping_add(1);
        if self.l == 0 {
            self.h = self.h.wrapping_add(1);
        }

        debug!("Instruction load_hli_a addr: {}, value: {}", addr, value);

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
        self.mmu.write_byte(addr, value);

        debug!("Instruction load_n_a addr: {}, value: {}", addr, value);

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
        let value = self.mmu.read_byte(addr);
        self.a = value;

        debug!("Instruction load_a_n addr: {}, value: {}", addr, value);

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
            "Instruction load_n_nn high_value: {}, low_value: {}",
            high_value, low_value
        );

        self.add_program_count(2);
        self.add_clock(12);
    }

    /// Put HL into SP
    /// Opcode for F9
    fn load_sp_hl(&mut self) {
        self.sp = ((self.h as u16) << 8) + self.l as u16;
        debug!("Instruction load_sp_hl");

        self.add_program_count(8);
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
        let addr = self.mmu.read_word(pc);

        self.mmu.write_word(addr, sp);

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
        let value = (high_value as u16) << 8 + low_value as u16;

        self.mmu.write_word(addr, value);

        self.add_clock(16);
    }

    /// Pop tow bytes off stack into register pair nn.
    /// Increment SP twice.
    /// nn = AF, BC, DE, HL
    /// Opcode for F1, C1, D1, E1
    fn pop_nn(&mut self, reg1: Register, reg2: Register) {
        debug!("Instruction Pop {}{}", reg1, reg2);

        let low_value = self.mmu.read_byte(self.sp);
        self.sp += 1;
        let high_value = self.mmu.read_byte(self.sp);
        self.sp += 1;

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

    fn add_program_count(&mut self, count: u16) {
        self.pc = self.pc.wrapping_add(count)
    }

    fn add_clock(&mut self, count: u32) {
        self.clock = self.clock.wrapping_add(count)
    }

    fn set_zero_flag(&mut self, flag: bool) {
        self.zero_flag = flag;
    }

    fn set_subtraction_flag(&mut self, flag: bool) {
        self.subtraction_flag = flag;
    }

    fn set_half_carry_flag(&mut self, flag: bool) {
        self.half_carry_flag = flag;
    }

    fn set_carry_flag(&mut self, flag: bool) {
        self.carry_flag = flag;
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
        }
        if (value & 0b0100_0000) > 0 {
            self.set_subtraction_flag(true);
        }
        if (value & 0b0010_0000) > 0 {
            self.set_half_carry_flag(true);
        }
        if (value & 0b0001_0000) > 0 {
            self.set_carry_flag(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_byte_flags_zero() {
        let mut cpu = Cpu::new();
        cpu.set_zero_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b1000_0000, res);
    }

    #[test]
    fn test_get_byte_flags_sub() {
        let mut cpu = Cpu::new();
        cpu.set_subtraction_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b0100_0000, res);
    }

    #[test]
    fn test_get_byte_flags_half() {
        let mut cpu = Cpu::new();
        cpu.set_half_carry_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b0010_0000, res);
    }

    #[test]
    fn test_get_byte_flags_carry() {
        let mut cpu = Cpu::new();
        cpu.set_carry_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b0001_0000, res);
    }

    #[test]
    fn test_get_byte_flags_all() {
        let mut cpu = Cpu::new();
        cpu.set_zero_flag(true);
        cpu.set_subtraction_flag(true);
        cpu.set_half_carry_flag(true);
        cpu.set_carry_flag(true);
        let res = cpu.get_byte_from_flags();
        assert_eq!(0b1111_0000, res);
    }
    #[test]
    fn test_set_flags_from_bytes_zero() {
        let mut cpu = Cpu::new();
        cpu.set_flags_from_byte(128);
        assert_eq!(true, cpu.zero_flag);
        assert_eq!(false, cpu.subtraction_flag);
        assert_eq!(false, cpu.half_carry_flag);
        assert_eq!(false, cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_sub() {
        let mut cpu = Cpu::new();
        cpu.set_flags_from_byte(64);
        assert_eq!(false, cpu.zero_flag);
        assert_eq!(true, cpu.subtraction_flag);
        assert_eq!(false, cpu.half_carry_flag);
        assert_eq!(false, cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_half() {
        let mut cpu = Cpu::new();
        cpu.set_flags_from_byte(32);
        assert_eq!(false, cpu.zero_flag);
        assert_eq!(false, cpu.subtraction_flag);
        assert_eq!(true, cpu.half_carry_flag);
        assert_eq!(false, cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_carry() {
        let mut cpu = Cpu::new();
        cpu.set_flags_from_byte(16);
        assert_eq!(false, cpu.zero_flag);
        assert_eq!(false, cpu.subtraction_flag);
        assert_eq!(false, cpu.half_carry_flag);
        assert_eq!(true, cpu.carry_flag);
    }

    #[test]
    fn test_set_flags_from_bytes_all() {
        let mut cpu = Cpu::new();
        cpu.set_flags_from_byte(248);
        assert_eq!(true, cpu.zero_flag);
        assert_eq!(true, cpu.subtraction_flag);
        assert_eq!(true, cpu.half_carry_flag);
        assert_eq!(true, cpu.carry_flag);
    }
}
