//! Runtime lifecycle boundary for the running `Kernel` system instance.

use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State},
};

#[unsafe(link_section = ".data.phase")]
static KERNEL_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn ready() -> ! {
    if !crate::phases::prepare::is_online()
        || !crate::phases::boot::is_ready()
        || !crate::phases::interrupt::is_ready()
        || !crate::phases::up_multitask::is_ready()
        || !crate::phases::smp_runtime::is_ready()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex kernel invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }

    crate::phases::shutdown_on_error(mark_ready(), "arceos_ex kernel ready event failed\n");
    crate::phases::payload::setup_then_enable()
}

fn mark_ready() -> EventResult {
    crate::phases::state::adopt(
        &KERNEL_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
    )
}

pub fn mark_online() -> EventResult {
    crate::phases::state::mark(
        &KERNEL_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::KernelOnline,
    )
}
