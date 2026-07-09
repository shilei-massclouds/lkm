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
use core::sync::atomic::AtomicU8;

use checkpoint::Checkpoint;
use objects::mm_core::KernelGlobalAllocAdapter;
use objects::state::{EventResult, LifecycleEvent, State};

#[global_allocator]
static KERNEL_GLOBAL_ALLOCATOR: KernelGlobalAllocAdapter = KernelGlobalAllocAdapter;

#[unsafe(link_section = ".data.phase")]
static STARTUP_TIMELINE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn startup_timeline_ready() -> ! {
    if !phases::prepare::is_online()
        || !phases::boot::is_ready()
        || !phases::interrupt::is_ready()
        || !phases::up_multitask::is_ready()
        || !phases::smp_runtime::is_ready()
    {
        arch::riscv64::sbi::putstr("arceos_ex startup invariant failed\n");
        arch::riscv64::sbi::system_shutdown()
    }

    phases::payload::setup_then_enable()
}

fn startup_timeline_event() -> EventResult {
    crate::phases::state::mark(
        &STARTUP_TIMELINE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::StartupTimelineReady,
    )
}

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
