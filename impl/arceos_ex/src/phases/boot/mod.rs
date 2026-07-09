pub mod core_prepare;
pub mod entry_prelude;
pub mod entry_successor;
pub mod mm_core_init;
pub mod sched_init;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static BOOT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup_after_children() -> ! {
    crate::phases::shutdown_on_error(boot_phase_ready(), "arceos_ex boot event failed\n");
    handoff()
}

fn handoff() -> ! {
    crate::systems::kernel::preset_after_boot()
}

fn boot_phase_ready() -> EventResult {
    crate::phases::state::mark(
        &BOOT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&BOOT_PHASE_STATE) == State::Ready
        && core_prepare::is_ready()
        && mm_core_init::is_ready()
        && sched_init::is_ready()
}
