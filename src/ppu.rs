pub struct Ppu {}

impl Ppu {
    pub(crate) fn new() -> Self {
        Ppu {}
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        todo!();
    }

    pub(crate) fn write(&mut self, addr: u16, value: u8) {
        todo!();
    }
}
