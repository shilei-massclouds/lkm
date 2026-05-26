pub mod entry_prelude;
pub mod entry_successor;

use crate::{
    objects::state::{EventOutcome, LifecycleEvent, State},
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static BOOT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup_after_children() -> ! {
    crate::phases::shutdown_on_error(boot_phase_ready(), "arceos_ex boot event failed\n");
    handoff()
}

fn handoff() -> ! {
    crate::startup_timeline_ready()
}

fn boot_phase_ready() -> EventOutcome {
    crate::phases::state::mark(
        &BOOT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootPhaseReady,
    )
    .into_result()
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&BOOT_PHASE_STATE) == State::Ready
}
