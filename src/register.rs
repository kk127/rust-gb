use std::{fmt};
#[derive(Clone, Copy)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    BC,
    DE,
    HL,
    SP,
    PC,
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Register::A => write!(f, "A"),
            Register::B => write!(f, "B"),
            Register::C => write!(f, "C"),
            Register::D => write!(f, "D"),
            Register::E => write!(f, "E"),
            Register::F => write!(f, "F"),
            Register::H => write!(f, "H"),
            Register::L => write!(f, "L"),
            Register::BC => write!(f, "BC"),
            Register::DE => write!(f, "DE"),
            Register::HL => write!(f, "HL"),
            Register::SP => write!(f, "SP"),
            Register::PC => write!(f, "PC"),
        }
    }
}