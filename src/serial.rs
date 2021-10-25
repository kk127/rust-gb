pub struct Serial {
    data: u8,
    control: u8,
}

impl Serial {
    pub fn new() -> Self {
        Self {
            data: 0x00,
            control: 0x00,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        // println!("Serial read address: 0x{:04x}", addr);
        match addr {
            0xff01 => self.data,
            0xff02 => self.control,
            _ => panic!("Invalid serial address 0x{:04x}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        // println!(
        //     "Serial write address: 0x{:04x}, value: 0x{:02x}",
        //     addr, value
        // );
        match addr {
            0xff01 => self.data = value,
            0xff02 => self.control = value,
            _ => panic!("Ivalid serial address 0x{:04x}", addr),
        };
    }
}
