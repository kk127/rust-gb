pub struct Mmu {

}

impl Mmu {
    pub fn new() -> Self {
        Mmu {

        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        todo!();
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        todo!();
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) -> u8 {
        todo!();
    }

    pub fn write_word(&mut self, addr: u16, value: u16) -> u8 {
        todo!();
    }
}