#[cfg(checkpoint_sbi_char)]
pub fn checkpoint(byte: u8) {
    crate::arch::riscv64::sbi::putchar(byte);
}

#[cfg(not(checkpoint_sbi_char))]
pub fn checkpoint(_byte: u8) {}
