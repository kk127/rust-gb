pub mod cpu;
pub mod mmu;
pub mod utils;
pub mod register;
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
