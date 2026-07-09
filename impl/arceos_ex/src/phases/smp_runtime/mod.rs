pub mod finalize;
pub mod initcall;
pub mod pre_smp_init;
pub mod rootfs;
pub mod runtime_core;
pub mod smp_bringup;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static SMP_RUNTIME_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    crate::checkpoint::checkpoint(Checkpoint::SmpRuntimePhaseStarted);
    pre_smp_init::setup(crate::context::context())
}

pub fn setup_after_children() -> ! {
    crate::phases::shutdown_on_error(
        smp_runtime_phase_ready(),
        "arceos_ex smp runtime event failed\n",
    );
    handoff()
}

fn handoff() -> ! {
    crate::systems::kernel::ready()
}

fn smp_runtime_phase_ready() -> EventResult {
    crate::phases::state::mark(
        &SMP_RUNTIME_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::SmpRuntimePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE) == State::Ready
        && pre_smp_init::is_ready()
        && smp_bringup::is_ready()
        && runtime_core::is_ready()
        && initcall::is_ready()
        && rootfs::is_ready()
        && finalize::is_ready()
}
