#![no_std]
#![no_main]

mod arch;
mod objects;
mod phases;
mod trace;

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(
    r#"
    .section .head.text, "ax"
    .globl _start
_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    la sp, boot_stack_top
    addi sp, sp, -256
    tail rust_entry

    .section .boot.stack, "aw", @nobits
    .align 12
    .globl boot_stack
boot_stack:
    .space 4096 * 4
    .globl boot_stack_top
boot_stack_top:
"#
);

#[unsafe(no_mangle)]
extern "C" fn rust_entry(hartid: usize, dtb_pa: usize) -> ! {
    let boot_args = objects::boot_args::BootArgs::new(hartid, dtb_pa);
    phases::boot::run(&boot_args);
    app_main();
    arch::riscv64::sbi::system_shutdown()
}

fn app_main() {
    arch::riscv64::sbi::putstr("Hello, world!\n");
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    arch::riscv64::sbi::putstr("arceos_ex panic\n");
    arch::riscv64::sbi::system_shutdown()
}
