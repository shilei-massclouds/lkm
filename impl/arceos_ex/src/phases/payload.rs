use core::sync::atomic::AtomicU8;

use crate::{
    objects::{
        printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};

#[unsafe(link_section = ".data.phase")]
static PAYLOAD_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup_then_enable() -> ! {
    crate::phases::shutdown_on_error(setup(), "arceos_ex payload setup failed\n");
    crate::phases::shutdown_on_error(enable(), "arceos_ex payload enable failed\n");
    crate::checkpoint::dispatch_mut(Checkpoint::PayloadPhaseOnline, crate::context::context());
    crate::apps::run()
}

fn setup() -> EventResult {
    if !payload_phase_dependencies_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&PAYLOAD_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &PAYLOAD_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::PayloadPhaseReady,
    )
}

fn enable() -> EventResult {
    crate::phases::state::mark(
        &PAYLOAD_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::PayloadPhaseOnline,
    )
    .and_then(|()| crate::startup_timeline_event())
}

fn payload_phase_dependencies_ready() -> bool {
    crate::phases::prepare::is_online()
        && crate::phases::boot::is_ready()
        && crate::phases::interrupt::is_ready()
        && crate::phases::up_multitask::is_ready()
        && crate::phases::smp_runtime::is_ready()
        && crate::phases::boot::core_prepare::is_ready()
        && crate::phases::boot::mm_core_init::is_ready()
        && printk::is_ready()
        && (printk::boot_console_online() || printk::console_handoff_complete())
}
