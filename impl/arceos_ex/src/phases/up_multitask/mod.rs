pub mod rest_init;

use crate::{
    objects::state::{EventResult, LifecycleEvent, State},
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static UP_MULTITASK_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    crate::trace::checkpoint(Checkpoint::UpMultitaskPhaseStarted);
    rest_init::setup(crate::context::context())
}

pub fn setup_after_children() -> ! {
    crate::phases::shutdown_on_error(
        up_multitask_phase_ready(),
        "arceos_ex up multitask event failed\n",
    );
    handoff()
}

fn handoff() -> ! {
    crate::startup_timeline_ready()
}

fn up_multitask_phase_ready() -> EventResult {
    crate::phases::state::mark(
        &UP_MULTITASK_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::UpMultitaskPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&UP_MULTITASK_PHASE_STATE) == State::Ready && rest_init::is_ready()
}
