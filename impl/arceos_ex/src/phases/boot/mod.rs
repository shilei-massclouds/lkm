pub mod entry_prelude;
pub mod entry_successor;

use crate::{
    objects::state::{EventResult, LifecycleEvent, State},
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static BOOT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup_after_children() -> ! {
    require(crate::phases::state::mark(
        &BOOT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootPhaseReady,
    ));
    handoff()
}

fn handoff() -> ! {
    crate::startup_timeline_ready()
}

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex boot event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&BOOT_PHASE_STATE) == State::Ready
}
