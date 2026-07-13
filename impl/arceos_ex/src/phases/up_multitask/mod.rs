pub mod rest_init;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static UP_MULTITASK_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    crate::checkpoint::checkpoint(Checkpoint::UpMultitaskPhaseStarted);
    rest_init::preset(crate::context::context())
}

pub fn setup_after_children() -> ! {
    crate::phases::shutdown_on_error(
        up_multitask_phase_ready(),
        "arceos_ex up multitask event failed\n",
    );
    handoff()
}

fn handoff() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        ctx.scheduler
            .handoff_boot_idle_to_kernel_init(&ctx.kernel_init_task, &ctx.boot_cpu_current_task),
        "arceos_ex kernel_init task handoff failed\n",
    );

    loop {
        let ctx = crate::context::context();
        let result = ctx.scheduler.schedule_idle(
            &ctx.cpu_group,
            &mut ctx.kernel_init_task,
            &ctx.kthreadd_task,
            &mut ctx.boot_cpu_local_interrupt,
            &mut ctx.boot_cpu_current_task,
        );
        crate::phases::shutdown_on_error(result, "boot idle schedule loop failed\n");
    }
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
