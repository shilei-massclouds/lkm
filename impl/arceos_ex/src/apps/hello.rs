use crate::{
    arch::riscv64::sbi,
    objects::{earlycon, printk},
};

pub fn run() -> ! {
    printk::write_str("Hello, world!\n");
    earlycon::drain_printk();
    sbi::system_shutdown()
}
