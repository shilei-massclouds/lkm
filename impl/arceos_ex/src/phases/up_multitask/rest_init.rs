use crate::{
    context::Context,
    objects::{
        rest_init::{
            runtime_services_still_deferred, SystemStateValue, TaskEntry, TaskKind, TaskSpawnInputs,
        },
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static REST_INIT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::RestInitPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_dispatch_objects(ctx).and_then(|()| checkpoint_dispatch_ready(ctx)),
        "arceos_ex rest init event failed\n",
    );
    crate::phases::up_multitask::pre_smp_init::setup(ctx)
}

pub fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        setup_boot_idle_tail(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex rest init tail event failed\n",
    );
    handoff()
}

fn setup_dispatch_objects(ctx: &mut Context) -> EventResult {
    ctx.rcu_core
        .scheduler_start(&ctx.scheduler, &ctx.cpu_group)?;
    ctx.kernel_init_task.preset(TaskSpawnInputs {
        task_creation_core: &ctx.task_creation_core,
        root_pid_namespace: &ctx.root_pid_namespace,
        credential_core: &ctx.credential_core,
        signal_core: &ctx.signal_core,
        task_file_context: &ctx.task_file_context,
        security_core: &ctx.security_core,
        init_task: &ctx.init_task,
    })?;
    ctx.kernel_init_task.setup(
        &ctx.task_creation_core,
        &ctx.root_pid_namespace,
        &ctx.scheduler,
    )?;
    ctx.kernel_init_task_pi_lock.setup()?;
    ctx.kernel_init_task.enable(
        &mut ctx.scheduler,
        &ctx.boot_current_cpu,
        &mut ctx.boot_cpu_local_interrupt,
        &ctx.boot_cpu_current_task,
        &mut ctx.kernel_init_task_pi_lock,
    )?;
    if !ctx
        .kernel_init_task
        .pin_to_boot_cpu(ctx.scheduler.boot_runqueue().cpu_id())
    {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&REST_INIT_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    }
    checkpoint_numa_default_policy_noop()?;
    ctx.kthreadd_task.preset(TaskSpawnInputs {
        task_creation_core: &ctx.task_creation_core,
        root_pid_namespace: &ctx.root_pid_namespace,
        credential_core: &ctx.credential_core,
        signal_core: &ctx.signal_core,
        task_file_context: &ctx.task_file_context,
        security_core: &ctx.security_core,
        init_task: &ctx.init_task,
    })?;
    ctx.kthreadd_task.setup(
        &ctx.task_creation_core,
        &ctx.root_pid_namespace,
        &ctx.scheduler,
    )?;
    ctx.kthreadd_task_pi_lock
        .setup_with_checkpoint(Checkpoint::KthreaddTaskPiLockReady)?;
    ctx.kthreadd_task.enable(
        &mut ctx.scheduler,
        &ctx.boot_current_cpu,
        &mut ctx.boot_cpu_local_interrupt,
        &ctx.boot_cpu_current_task,
        &mut ctx.kthreadd_task_pi_lock,
    )?;
    if !ctx.kthreadd_task.bind_global_ref(&ctx.root_pid_namespace) {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&REST_INIT_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    }
    ctx.system_state.preset()?;
    ctx.system_state
        .setup(&ctx.kernel_init_task, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate
        .setup(&ctx.kernel_init_task, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate
        .enable(&ctx.system_state, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate.complete(
        &ctx.system_state,
        &ctx.kthreadd_task,
        &mut ctx.kernel_init_task,
    )?;
    schedule_once_from_preempt_disabled_context(ctx)
}

fn setup_boot_idle_tail(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_runtime.setup(
        &ctx.scheduler,
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.kthreadd_ready_gate,
        &ctx.cpu_group,
    )?;
    ctx.boot_idle_runtime
        .prepare_idle_entry(&ctx.scheduler, &ctx.cpu_group)?;
    ctx.boot_idle_runtime.run_idle_loop(&ctx.scheduler)
}

fn schedule_once_from_preempt_disabled_context(ctx: &mut Context) -> EventResult {
    if ctx.scheduler.state() != State::Online
        || ctx.kernel_init_task.state() != State::Online
        || !ctx.kernel_init_task.released_for_pre_smp_init()
        || ctx.kthreadd_task.state() != State::Online
        || ctx.system_state.state() != State::Ready
        || ctx.system_state.value() != SystemStateValue::Scheduling
        || ctx.kthreadd_ready_gate.state() != State::Online
        || !ctx.kthreadd_ready_gate.release_committed()
    {
        return failed_dispatch_preset();
    }

    if ctx
        .scheduler
        .boot_idle_preemption_mut()
        .enable_no_resched()
        .is_err()
    {
        return failed_dispatch_preset();
    }

    let schedule_result = ctx.scheduler.schedule(
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    );
    let preempt_disable_result = ctx.scheduler.boot_idle_preemption_mut().disable();
    // arceos_ex runs these phases linearly in one execution context. Restore
    // the simulated CPU controls after recording the post-schedule boot-idle
    // atomic context so later smoke tests and object phases observe the normal
    // boot environment.
    let preempt_enable_result = ctx.scheduler.boot_idle_preemption_mut().enable();
    if schedule_result.is_err() || preempt_disable_result.is_err() || preempt_enable_result.is_err()
    {
        return failed_dispatch_preset();
    }
    Ok(())
}

fn failed_dispatch_preset() -> EventResult {
    failed_condition(
        LifecycleEvent::Preset,
        crate::phases::state::load(&REST_INIT_PHASE_STATE),
        State::Base,
        State::Prepared,
    )
}

fn handoff() -> ! {
    crate::phases::up_multitask::setup_after_children()
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !rest_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&REST_INIT_PHASE_STATE),
            State::Prepared,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &REST_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::RestInitPhaseReady,
    )
}

fn checkpoint_dispatch_ready(ctx: &Context) -> EventResult {
    if !rest_init_dispatch_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&REST_INIT_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    }

    crate::phases::state::mark(
        &REST_INIT_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::RestInitDispatchReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&REST_INIT_PHASE_STATE) == State::Ready
}

pub fn dispatch_ready() -> bool {
    let ctx = crate::context::context_ref();

    crate::phases::state::load(&REST_INIT_PHASE_STATE) != State::Base
        && ctx.scheduler.schedule_passes() != 0
        && ctx.boot_cpu_current_task.switch_committed_count() != 0
}

fn rest_init_phase_ready(ctx: &Context) -> bool {
    rest_init_dispatch_ready(ctx)
        && ctx.boot_idle_runtime.state() == State::Ready
        && ctx.boot_idle_runtime.first_schedule_committed()
        && ctx.boot_idle_runtime.idle_entry_prepared()
        && ctx.boot_idle_runtime.cpu_startup_entry_ready()
        && ctx.boot_idle_runtime.idle_loop_entered()
        && ctx.boot_idle_runtime.idle_cycle_committed()
        && ctx
            .boot_idle_runtime
            .representative_need_resched_cycle_committed()
        && ctx.boot_idle_runtime.boot_init_handoff_complete()
        && ctx.boot_idle_runtime.boot_cpu_hotplug_online()
        && ctx.boot_idle_runtime.secondary_cpus_not_started()
        && ctx.boot_idle_runtime.real_task_switch_deferred()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn rest_init_dispatch_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::process_prepare::is_ready()
        && ctx.rcu_core.scheduler_starting_ready()
        && ctx.rcu_core.scheduler_active_init()
        && ctx.rcu_core.scheduler_start_single_online_cpu()
        && ctx.rcu_core.gp_seq_baseline_synced()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.boot_cpu_current_task.state() == State::Ready
        && ctx.boot_cpu_current_task.current_is_boot_idle()
        && ctx.boot_cpu_current_task.switch_committed_count() != 0
        && ctx.kernel_init_task.state() == State::Online
        && ctx.kernel_init_task.pid() == 1
        && ctx.kernel_init_task.entry() == TaskEntry::KernelInit
        && ctx.kernel_init_task.kind() == TaskKind::UserModeThread
        && ctx.kernel_init_task.clone_fs()
        && !ctx.kernel_init_task.user_mm_created()
        && ctx.kernel_init_task.thread_context_ready()
        && ctx.kernel_init_task.sched_entity_ready()
        && ctx.kernel_init_task.running()
        && ctx.kernel_init_task.enqueued()
        && ctx
            .scheduler
            .boot_runqueue()
            .contains_task(ctx.kernel_init_task.pid())
        && ctx.kernel_init_task_pi_lock.state() == State::Ready
        && !ctx.kernel_init_task_pi_lock.locked()
        && ctx.kernel_init_task_pi_lock.irqsave_entered_count() != 0
        && ctx.kernel_init_task_pi_lock.irqrestore_exited_count() != 0
        && !ctx.kernel_init_task.waiting_for_kthreadd_done()
        && ctx.kernel_init_task.released_for_pre_smp_init()
        && ctx.kernel_init_task.pinned_to_boot_cpu()
        && ctx.kernel_init_task.pf_no_setaffinity()
        && ctx.kernel_init_task.cpu_id() == ctx.scheduler.boot_runqueue().cpu_id()
        && ctx.kthreadd_task.state() == State::Online
        && ctx.kthreadd_task.pid() == 2
        && ctx.kthreadd_task.entry() == TaskEntry::Kthreadd
        && ctx.kthreadd_task.kind() == TaskKind::KernelThread
        && ctx.kthreadd_task.clone_fs()
        && ctx.kthreadd_task.clone_files()
        && ctx.kthreadd_task.clone_vm()
        && ctx.kthreadd_task.clone_untraced()
        && ctx.kthreadd_task.kernel_thread_flag()
        && ctx.kthreadd_task.thread_context_ready()
        && ctx.kthreadd_task.sched_entity_ready()
        && ctx.kthreadd_task.running()
        && ctx.kthreadd_task.cpu_id() == ctx.scheduler.boot_runqueue().cpu_id()
        && ctx.scheduler.selected_runqueue_task_id() == ctx.kthreadd_task.pid()
        && ctx
            .scheduler
            .boot_runqueue()
            .contains_task(ctx.kthreadd_task.pid())
        && ctx.kthreadd_task_pi_lock.state() == State::Ready
        && !ctx.kthreadd_task_pi_lock.locked()
        && ctx.kthreadd_task_pi_lock.irqsave_entered_count() != 0
        && ctx.kthreadd_task_pi_lock.irqrestore_exited_count() != 0
        && ctx
            .kthreadd_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
        && ctx.kthreadd_task.global_ref_bound()
        && ctx.kthreadd_task.provider_ready()
        && ctx.kthreadd_task.enqueued()
        && ctx.system_state.state() == State::Ready
        && ctx.system_state.value() == SystemStateValue::Scheduling
        && ctx.kthreadd_ready_gate.state() == State::Online
        && !ctx.kthreadd_ready_gate.pending()
        && ctx.kthreadd_ready_gate.completed()
        && ctx.kthreadd_ready_gate.release_committed()
        && ctx.scheduler.schedule_passes() != 0
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn checkpoint_numa_default_policy_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::NumaDefaultPolicyNoop);
    Ok(())
}
