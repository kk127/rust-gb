
pub fn get_addr_from_registers(high_register: u8, low_register:u8) -> u16 {
    ((high_register as u16) << 8) + low_register as u16
}