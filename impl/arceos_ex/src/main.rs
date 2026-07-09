#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

mod apps;
mod arch;
mod checkpoint;
mod context;
mod objects;
mod phases;
mod projects;
#[cfg(checkpoint_handler_stress_mem)]
mod stress_mem;
mod systems;

use core::panic::PanicInfo;

use objects::mm_core::KernelGlobalAllocAdapter;

#[global_allocator]
static KERNEL_GLOBAL_ALLOCATOR: KernelGlobalAllocAdapter = KernelGlobalAllocAdapter;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    arch::riscv64::sbi::putstr("arceos_ex panic\n");
    arch::riscv64::sbi::system_shutdown()
}

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    arch::riscv64::sbi::putstr("arceos_ex allocation error\n");
    arch::riscv64::sbi::system_shutdown()
}
