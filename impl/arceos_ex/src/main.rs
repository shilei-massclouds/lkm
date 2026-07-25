#![no_std]
#![no_main]

extern crate alloc;

mod apps;
mod arch;
mod checkpoint;
mod context;
mod flows;
mod objects;
mod phases;
#[cfg(checkpoint_handler_stress_mem)]
mod stress_mem;
mod systems;

use core::{
    fmt::{self, Write},
    panic::PanicInfo,
};

use objects::mm_core::KernelGlobalAllocAdapter;

#[global_allocator]
static KERNEL_GLOBAL_ALLOCATOR: KernelGlobalAllocAdapter = KernelGlobalAllocAdapter;

struct SbiPanicWriter;

impl Write for SbiPanicWriter {
    fn write_str(&mut self, message: &str) -> fmt::Result {
        arch::riscv64::sbi::putstr(message);
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    arch::riscv64::sbi::putstr("arceos_ex panic\n");
    let _ = writeln!(SbiPanicWriter, "panic message: {}", info.message());
    arch::riscv64::sbi::system_shutdown()
}
