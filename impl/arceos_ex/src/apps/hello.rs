use crate::{arch::riscv64::sbi, objects::printk};

pub fn run() -> ! {
    printk::write_str("Hello, world!\n");
    sbi::system_shutdown()
}
