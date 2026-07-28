//! BootIdleEntryPhase lowering owned by BootIdleFlow.

use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        rest_init::runtime_services_still_deferred,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};

#[unsafe(link_section = ".data.phase")]
static BOOT_IDLE_ENTRY_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub(super) fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        require_preset(),
        "arceos_ex boot idle entry preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootIdleEntryPhaseStarted);
    crate::phases::shutdown_on_error(
        enter_startup_context(ctx)
            .and_then(|()| prepare_entry(ctx))
            .and_then(|()| run_idle_loop(ctx))
            .and_then(|()| mark_prepared(ctx)),
        "arceos_ex boot idle entry preset failed\n",
    );
    crate::phases::shutdown_on_error(mark_ready(ctx), "arceos_ex boot idle entry setup failed\n");
    crate::phases::shutdown_on_error(
        mark_online(ctx),
        "arceos_ex boot idle entry enable failed\n",
    );
    idle_continuation(ctx)
}

fn require_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE);
    if state != State::Base || !crate::flows::boot_init_flow::schedule_handoff_is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn enter_startup_context(ctx: &mut Context) -> EventResult {
    if !crate::flows::boot_init_flow::schedule_handoff_is_online()
        || !restore_ready(ctx)
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
    {
        return failed_preset();
    }

    // schedule_preempt_disabled() disables preemption again before entering
    // cpu_startup_entry(). BootIdleStartupContext exits by Never.
    ctx.scheduler.boot_idle_preemption_mut().disable()
}

fn prepare_entry(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_flow
        .prepare_idle_entry(&ctx.boot_task, &ctx.scheduler, &ctx.cpu_group)
}

fn run_idle_loop(ctx: &mut Context) -> EventResult {
    let Some(current_cpu) = ctx.current_cpu() else {
        return failed_preset();
    };
    let Context {
        boot_idle_flow,
        scheduler,
        cpu_group,
        kernel_init_task,
        kernel_init_flow,
        user_app_flow,
        kthreadd_task,
        kthreadd_flow,
        user_task_set,
        boot_task,
        ..
    } = ctx;
    let Some((local_interrupt, current_task_slot)) = cpu_group.boot_cpu_controls_mut() else {
        return failed_preset();
    };
    boot_idle_flow.run_idle_loop(
        scheduler,
        current_cpu,
        kernel_init_task,
        kernel_init_flow,
        user_app_flow,
        kthreadd_task,
        kthreadd_flow,
        user_task_set,
        boot_task,
        local_interrupt,
        current_task_slot,
    )
}

fn idle_continuation(ctx: &mut Context) -> ! {
    loop {
        let Some(current_cpu) = ctx.current_cpu() else {
            crate::arch::riscv64::sbi::system_shutdown()
        };
        let Context {
            scheduler,
            cpu_group,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            ..
        } = ctx;
        let Some((local_interrupt, current_task_slot)) = cpu_group.boot_cpu_controls_mut() else {
            crate::arch::riscv64::sbi::system_shutdown()
        };
        let result = scheduler.schedule_idle(
            current_cpu,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            local_interrupt,
            current_task_slot,
        );
        crate::phases::shutdown_on_error(result, "boot idle schedule loop failed\n");
    }
}

fn failed_preset() -> EventResult {
    failed_condition(
        LifecycleEvent::Preset,
        crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE),
        State::Base,
        State::Prepared,
    )
}

fn mark_prepared(ctx: &Context) -> EventResult {
    if !phase_ready(ctx) {
        return failed_preset();
    }
    crate::phases::state::mark_checked(
        &BOOT_IDLE_ENTRY_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::BootIdleEntryPhasePrepared,
    )
}

fn mark_ready(ctx: &Context) -> EventResult {
    if !phase_ready(ctx) {
        return phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready);
    }
    crate::phases::state::mark_checked(
        &BOOT_IDLE_ENTRY_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::BootIdleEntryPhaseReady,
    )
}

fn mark_online(ctx: &Context) -> EventResult {
    if !phase_ready(ctx) {
        return phase_failure(LifecycleEvent::Enable, State::Ready, State::Online);
    }
    crate::phases::state::mark_checked(
        &BOOT_IDLE_ENTRY_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::BootIdleEntryPhaseOnline,
    )
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(super) fn is_online() -> bool {
    crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE) == State::Online
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE),
        expected,
        target,
    )
}

fn phase_ready(ctx: &Context) -> bool {
    restore_ready(ctx)
        && ctx.scheduler.boot_idle_preemption().state() == State::Ready
        && ctx.scheduler.boot_idle_preemption().disabled()
        && ctx.boot_idle_flow.state() == State::Ready
        && ctx.boot_idle_flow.first_schedule_committed()
        && ctx.boot_idle_flow.idle_entry_prepared()
        && ctx.boot_idle_flow.cpu_startup_entry_ready()
        && ctx.boot_idle_flow.idle_loop_entered()
        && ctx.boot_idle_flow.idle_cycle_committed()
        && ctx
            .boot_idle_flow
            .representative_need_resched_cycle_committed()
        && ctx.boot_idle_flow.nohz_run_idle_balance_done()
        && ctx.boot_idle_flow.local_irq_disabled_for_sleep()
        && ctx.boot_idle_flow.arch_cpu_idle_enter_done()
        && ctx.boot_idle_flow.arch_cpu_idle_exit_done()
        && ctx.boot_idle_flow.smp_call_function_queue_flushed()
        && ctx.scheduler.idle_schedule_passes() != 0
        && ctx.scheduler.idle_schedule_returned_passes() != 0
        && ctx.boot_idle_flow.boot_init_handoff_complete()
        && ctx.boot_idle_flow.boot_cpu_hotplug_online()
        && ctx.boot_idle_flow.secondary_cpus_not_started()
        && ctx.boot_idle_flow.kernel_init_task_switch_handoff_ready()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn restore_ready(ctx: &Context) -> bool {
    crate::flows::boot_init_flow::rest_init_facts_stable(ctx)
        && crate::flows::boot_init_flow::is_online()
        && ctx.scheduler.schedule_passes() != 0
        && ctx.scheduler.current_runqueue_resolve_passes() != 0
        && ctx.scheduler.pick_next_task_passes() != 0
        && ctx.scheduler.switch_to_passes() != 0
        && ctx.scheduler.identity_switch_passes() == 0
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
        && ctx.scheduler.kernel_init_stack_switch_returned_count() == 1
        && ctx.boot_cpu_current_task().current_is_boot_task()
        && ctx.boot_idle_flow.state() == State::Ready
        && ctx.boot_idle_flow.active()
        && ctx.boot_idle_flow.owner() == ctx.boot_task.task_ref()
        && ctx.boot_task.task().active_flow() == ctx.boot_idle_flow.flow_ref()
}
