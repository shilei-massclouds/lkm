//! BootInitScheduleHandoffPhase lowering owned by BootInitFlow.

use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        rest_init::{SystemStateValue, runtime_services_still_deferred},
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};

#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub(super) fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        require_preset(),
        "arceos_ex schedule handoff preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootInitScheduleHandoffPhaseStarted);
    crate::phases::shutdown_on_error(
        setup(ctx).and_then(|()| mark_prepared(ctx)),
        "arceos_ex schedule handoff preset failed\n",
    );
    crate::phases::shutdown_on_error(mark_ready(ctx), "arceos_ex schedule handoff setup failed\n");
    crate::phases::shutdown_on_error(
        mark_online(ctx),
        "arceos_ex schedule handoff enable failed\n",
    );
    super::enable_after_boot_init_schedule_handoff()
}

fn require_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE);
    if state != State::Base || !super::rest_init::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn setup(ctx: &mut Context) -> EventResult {
    if ctx.scheduler.state() != State::Online
        || !super::rest_init::is_online()
        || !super::rest_init::facts_stable(ctx)
        || ctx.kernel_init_task.state() != State::Online
        || ctx.kthreadd_task.state() != State::Online
        || ctx.system_state.state() != State::Ready
        || ctx.system_state.value() != SystemStateValue::Scheduling
        || ctx.kthreadd_ready_gate.state() != State::Online
        || !ctx.kthreadd_ready_gate.completion().complete_committed()
        || !ctx.kthreadd_ready_gate.completion().token_available()
    {
        return failed_preset();
    }

    if ctx
        .scheduler
        .boot_idle_preemption_mut()
        .enable_no_resched()
        .is_err()
    {
        return failed_preset();
    }

    ctx.boot_idle_flow.setup(
        &mut ctx.boot_task,
        ctx.boot_init_flow.core_mut(),
        &ctx.scheduler,
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.kthreadd_ready_gate,
        &ctx.cpu_group,
    )
}

fn failed_preset() -> EventResult {
    failed_condition(
        LifecycleEvent::Preset,
        crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE),
        State::Base,
        State::Prepared,
    )
}

fn mark_prepared(ctx: &Context) -> EventResult {
    if !phase_ready(ctx) {
        return failed_preset();
    }
    crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::BootInitScheduleHandoffPhasePrepared,
    )
}

fn mark_ready(ctx: &Context) -> EventResult {
    if !phase_ready(ctx) {
        return phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready);
    }
    crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::BootInitScheduleHandoffPhaseReady,
    )
}

fn mark_online(ctx: &Context) -> EventResult {
    if !phase_ready(ctx) {
        return phase_failure(LifecycleEvent::Enable, State::Ready, State::Online);
    }
    crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::BootInitScheduleHandoffPhaseOnline,
    )
}

pub(super) fn is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE) == State::Online
}

pub(super) fn precommit_ready() -> bool {
    phase_ready(crate::context::context_ref())
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE),
        expected,
        target,
    )
}

fn phase_ready(ctx: &Context) -> bool {
    super::rest_init::facts_stable(ctx)
        && ctx.scheduler.schedule_passes() == 0
        && ctx.scheduler.current_runqueue_resolve_passes() == 0
        && ctx.scheduler.pick_next_task_passes() == 0
        && ctx.scheduler.switch_to_passes() == 0
        && ctx.boot_cpu_current_task().switch_committed_count() == 0
        && ctx.boot_cpu_current_task().current_is_boot_task()
        && ctx.boot_idle_flow.state() == State::Ready
        && ctx.boot_idle_flow.active()
        && ctx.boot_idle_flow.owner() == ctx.boot_task.task_ref()
        && ctx
            .boot_task
            .task()
            .owns_flow(ctx.boot_idle_flow.flow_ref())
        && ctx.boot_task.task().active_flow() == ctx.boot_idle_flow.flow_ref()
        && ctx.kthreadd_ready_gate.completion().complete_committed()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}
