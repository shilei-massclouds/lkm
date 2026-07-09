use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    objects::{
        printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
};

#[unsafe(link_section = ".data.phase")]
static PAYLOAD_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup_then_enable() -> ! {
    crate::phases::shutdown_on_error(setup(), "arceos_ex payload setup failed\n");
    crate::phases::shutdown_on_error(enable(), "arceos_ex payload enable failed\n");
    crate::checkpoint::dispatch_after_trace(
        Checkpoint::PayloadPhaseOnline,
        crate::context::context_ref(),
    );
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

    let ctx = crate::context::context();
    ctx.payload_exec_sync_boundaries
        .setup(&ctx.kernel_init_task, &ctx.system_state)?;
    ctx.user_clone_deferred_boundaries
        .setup(&ctx.payload_exec_sync_boundaries)?;
    ctx.user_child_process.preset()?;

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
    .and_then(|()| crate::systems::kernel::mark_online())
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
