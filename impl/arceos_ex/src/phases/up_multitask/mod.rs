pub mod rest_init;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{failed_condition, EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static UP_MULTITASK_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    crate::phases::shutdown_on_error(
        require_state(LifecycleEvent::Preset, State::Base, State::Prepared),
        "arceos_ex up multitask preset start failed\n",
    );
    if !crate::phases::interrupt::is_online() {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared),
            "arceos_ex up multitask preset dependency failed\n",
        );
    }
    crate::checkpoint::checkpoint(Checkpoint::UpMultitaskPhaseStarted);
    rest_init::preset(crate::context::context())
}

pub fn preset_after_boot_init_rest_init() -> ! {
    let result = if rest_init::boot_init_rest_init_is_online() {
        commit_state(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::UpMultitaskPhasePrepared,
        )
    } else {
        phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex up multitask preset failed\n");
    rest_init::setup(crate::context::context())
}

pub fn setup_after_boot_init_schedule_handoff() -> ! {
    let result = if rest_init::boot_init_rest_init_is_online()
        && rest_init::boot_init_schedule_handoff_is_online()
    {
        commit_state(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::UpMultitaskPhaseReady,
        )
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex up multitask setup failed\n");
    rest_init::enable(crate::context::context())
}

pub fn enable_after_boot_idle_entry() -> ! {
    let result = if children_online() {
        commit_state(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::UpMultitaskPhaseOnline,
        )
    } else {
        phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex up multitask enable failed\n");
    handoff_boot_idle_to_kernel_init()
}

fn handoff_boot_idle_to_kernel_init() -> ! {
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

pub fn is_online() -> bool {
    crate::phases::state::load(&UP_MULTITASK_PHASE_STATE) == State::Online && children_online()
}

fn children_online() -> bool {
    rest_init::boot_init_rest_init_is_online()
        && rest_init::boot_init_schedule_handoff_is_online()
        && rest_init::boot_idle_entry_is_online()
}

fn require_state(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    let actual = crate::phases::state::load(&UP_MULTITASK_PHASE_STATE);
    if actual != expected {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn commit_state(
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    crate::phases::state::mark_checked(
        &UP_MULTITASK_PHASE_STATE,
        event,
        expected,
        target,
        checkpoint,
    )
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&UP_MULTITASK_PHASE_STATE),
        expected,
        target,
    )
}
