use log::{info, debug};

use crate::mmu::Mmu;
use crate::utils::{self, get_addr_from_registers};
use crate::register::Register;

pub struct  Cpu {
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
        let value = self.mmu.read_byte(self.pc);
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

        debug!("Instruction load_r1_hl r1: {}, memory8: {}, addr: {}", reg1, value, addr);

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
        let addr = get_addr_from_registers(self.h, self.l);

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
        let addr = get_addr_from_registers(self.h, self.l);
        let value = self.mmu.read_byte(self.pc);
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

    // fn nop(&mut self) {
    //     debug!("Instruction NOP");
    //     self.clock += 4;
    // }

    // fn ld_bc_nn(&mut self) {
    //     debug!("Instruction Load BC nn");
    //     let pc = self.pc;
    //     self.c = self.mmu.read_byte(pc);
    //     self.b = self.mmu.read_byte(pc+1);

    //     self.add_program_count(2);
    //     self.add_clock(12);
    // }

    // fn ld_n_a(&mut self) {
    //     debug!("Instruction Load (BC) A");
    //     let addr =  utils::get_addr_from_registers(self.b, self.c);
    //     self.mmu.write_byte(addr, self.a);

    //     self.add_clock(8);
    // }

    // fn inc_bc(&mut self) {
    //     debug!("Instruction Inc BC");
    //     self.c = self.c.wrapping_add(1);
    //     if self.c == 0 {
    //         self.b = self.b.wrapping_add(1);
    //     }

    //     self.add_clock(8);
    // }

    // fn inc_b(&mut self) {
    //     self.b = self.b.wrapping_add(1);
        
    //     if self.b == 0 {
    //         self.set_zero_flag(true);
    //     }
    //     if self.b == 0x10 {
    //         self.set_half_carry_flag(true);
    //     }
    //     self.set_subtraction_flag(false);

    // }


    fn add_program_count(&mut self, count: u16) {
        self.pc = self.pc.wrapping_add(count)
    }

    fn add_clock(&mut self, count: u32) {
        self.clock += self.clock.wrapping_add(count)
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
}