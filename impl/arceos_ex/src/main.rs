#![no_std]
#![no_main]

mod apps;
mod arch;
mod checkpoint;
mod context;
mod objects;
mod phases;
mod trace;

use core::panic::PanicInfo;
use core::sync::atomic::AtomicU8;

use objects::state::{EventResult, LifecycleEvent, State};
use trace::Checkpoint;

#[unsafe(link_section = ".data.phase")]
static STARTUP_TIMELINE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn startup_timeline_ready() -> ! {
    if !phases::prepare::is_online()
        || !phases::boot::is_ready()
        || !phases::interrupt::is_ready()
        || !phases::up_multitask::is_ready()
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
