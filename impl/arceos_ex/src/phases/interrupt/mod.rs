pub mod irq_open_prepare;
pub mod irq_time_init;
pub mod local_irq_enable;
pub mod process_prepare;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static INTERRUPT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    crate::checkpoint::checkpoint(Checkpoint::InterruptPhaseStarted);
    irq_time_init::preset(crate::context::context())
}

pub fn setup_after_children() -> ! {
    crate::phases::shutdown_on_error(
        interrupt_phase_ready(),
        "arceos_ex interrupt event failed\n",
    );
    handoff()
}

fn handoff() -> ! {
    crate::systems::kernel::setup_after_interrupt()
}

fn interrupt_phase_ready() -> EventResult {
    crate::phases::state::mark(
        &INTERRUPT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::InterruptPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&INTERRUPT_PHASE_STATE) == State::Ready
        && irq_time_init::is_ready()
        && local_irq_enable::is_ready()
        && irq_open_prepare::is_ready()
        && process_prepare::is_ready()
}
