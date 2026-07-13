use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        rest_init::{runtime_services_still_deferred, SystemStateValue, TaskSpawnInputs},
        state::{failed_condition, EventResult, LifecycleEvent, State},
        task::{TaskEntry, TaskKind},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_REST_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
#[unsafe(link_section = ".data.phase")]
static BOOT_IDLE_ENTRY_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        setup_boot_init_rest_init(ctx).and_then(|()| checkpoint_boot_init_rest_init_ready(ctx)),
        "arceos_ex rest init event failed\n",
    );
    setup(ctx)
}

pub fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        setup_boot_init_schedule_handoff(ctx)
            .and_then(|()| checkpoint_boot_init_schedule_handoff_ready(ctx))
            .and_then(|()| enter_boot_idle_startup_context(ctx))
            .and_then(|()| setup_boot_idle_runtime(ctx))
            .and_then(|()| prepare_boot_idle_entry(ctx))
            .and_then(|()| run_boot_idle_loop(ctx))
            .and_then(|()| checkpoint_boot_idle_entry_ready(ctx)),
        "arceos_ex rest init tail event failed\n",
    );
    handoff()
}

fn setup_boot_init_rest_init(ctx: &mut Context) -> EventResult {
    ctx.rcu_core.scheduler_start(
        &ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.boot_cpu_local_interrupt,
    )?;
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
        &mut ctx.task_creation_core,
        &ctx.root_pid_namespace,
        &ctx.credential_core,
        &ctx.signal_core,
        &ctx.task_file_context,
        &ctx.security_core,
        &ctx.init_task,
        &ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &mut ctx.page_table_caches,
        &ctx.config,
    )?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskReady, ctx);
    ctx.kernel_init_task_pi_lock.setup()?;
    ctx.kernel_init_task.enable(
        &mut ctx.scheduler,
        &ctx.cpu_group,
        &ctx.boot_current_cpu,
        &mut ctx.boot_cpu_local_interrupt,
        &ctx.boot_cpu_current_task,
        &mut ctx.kernel_init_task_pi_lock,
    )?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskOnline, ctx);
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    };
    ctx.kernel_init_task.pin_to_boot_cpu(
        boot_cpu.logical_id(),
        &ctx.root_pid_namespace,
        ctx.scheduler.boot_idle_rcu_read_side_mut(),
    )?;
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
        &mut ctx.task_creation_core,
        &ctx.root_pid_namespace,
        &ctx.credential_core,
        &ctx.signal_core,
        &ctx.task_file_context,
        &ctx.security_core,
        &ctx.init_task,
        &ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &mut ctx.page_table_caches,
        &ctx.config,
    )?;
    ctx.kthreadd_task_pi_lock
        .setup_with_checkpoint(Checkpoint::KthreaddTaskPiLockReady)?;
    ctx.kthreadd_task.enable(
        &mut ctx.scheduler,
        &ctx.cpu_group,
        &ctx.boot_current_cpu,
        &mut ctx.boot_cpu_local_interrupt,
        &ctx.boot_cpu_current_task,
        &mut ctx.kthreadd_task_pi_lock,
    )?;
    ctx.kthreadd_task.bind_global_ref(
        &ctx.root_pid_namespace,
        ctx.scheduler.boot_idle_rcu_read_side_mut(),
    )?;
    ctx.system_state.preset()?;
    ctx.system_state
        .setup(&ctx.kernel_init_task, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate
        .setup(&ctx.kernel_init_task, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate_wait_lock
        .setup_with_checkpoint(Checkpoint::KthreaddReadyGateWaitLockReady)?;
    ctx.kthreadd_ready_gate
        .enable(&ctx.system_state, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate.complete(
        &ctx.system_state,
        &ctx.kthreadd_task,
        &mut ctx.kthreadd_ready_gate_wait_lock,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.scheduler,
    )
}

fn setup_boot_idle_runtime(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_runtime.setup(
        &ctx.scheduler,
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.kthreadd_ready_gate,
        &ctx.cpu_group,
    )
}

fn prepare_boot_idle_entry(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_runtime
        .prepare_idle_entry(&ctx.scheduler, &ctx.cpu_group)
}

fn run_boot_idle_loop(ctx: &mut Context) -> EventResult {
    // The object model executes both task branches linearly. Re-enter the
    // boot-idle continuation before modeling cpu_startup_entry()/do_idle();
    // schedule_idle() then commits the selected runnable task as this CPU's
    // CurrentTaskRef. The real future return to the idle-loop continuation is
    // left to the later continuation/task-stack model.
    ctx.boot_cpu_current_task.set_current_boot_idle()?;
    let result = ctx.boot_idle_runtime.run_idle_loop(
        &mut ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    );
    result
}

fn setup_boot_init_schedule_handoff(ctx: &mut Context) -> EventResult {
    if ctx.scheduler.state() != State::Online
        || !boot_init_rest_init_phase_ready(ctx)
        || ctx.kernel_init_task.state() != State::Online
        || ctx.kthreadd_task.state() != State::Online
        || ctx.system_state.state() != State::Ready
        || ctx.system_state.value() != SystemStateValue::Scheduling
        || ctx.kthreadd_ready_gate.state() != State::Online
        || !ctx.kthreadd_ready_gate.completion().complete_committed()
        || !ctx.kthreadd_ready_gate.completion().token_available()
    {
        return failed_schedule_handoff_setup();
    }

    if ctx
        .scheduler
        .boot_idle_preemption_mut()
        .enable_no_resched()
        .is_err()
    {
        return failed_schedule_handoff_setup();
    }

    let schedule_result = ctx.scheduler.schedule(
        &ctx.cpu_group,
        &mut ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    );
    if schedule_result.is_err() {
        return failed_schedule_handoff_setup();
    }
    crate::checkpoint::dispatch(Checkpoint::SchedulerPickNextTaskExit, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerSwitchToEntry, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerSwitchToExit, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerScheduleExit, ctx);
    Ok(())
}

fn enter_boot_idle_startup_context(ctx: &mut Context) -> EventResult {
    if !boot_init_schedule_handoff_phase_ready(ctx)
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
    {
        return failed_boot_idle_entry_setup();
    }

    // schedule_preempt_disabled() disables preemption again before entering
    // cpu_startup_entry(). The formal BootIdleStartupContext exits by Never,
    // so the simulation keeps the preemption control disabled.
    ctx.scheduler.boot_idle_preemption_mut().disable()
}

fn failed_schedule_handoff_setup() -> EventResult {
    failed_condition(
        LifecycleEvent::Setup,
        crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE),
        State::Base,
        State::Ready,
    )
}

fn failed_boot_idle_entry_setup() -> EventResult {
    failed_condition(
        LifecycleEvent::Setup,
        crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE),
        State::Base,
        State::Ready,
    )
}

fn handoff() -> ! {
    crate::phases::up_multitask::setup_after_children()
}

fn checkpoint_boot_init_rest_init_ready(ctx: &Context) -> EventResult {
    if !boot_init_rest_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &BOOT_INIT_REST_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootInitRestInitPhaseReady,
    )
}

fn checkpoint_boot_init_schedule_handoff_ready(ctx: &Context) -> EventResult {
    if !boot_init_schedule_handoff_phase_ready(ctx) {
        return failed_schedule_handoff_setup();
    }

    crate::phases::state::mark(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootInitScheduleHandoffPhaseReady,
    )
}

fn checkpoint_boot_idle_entry_ready(ctx: &Context) -> EventResult {
    if !boot_idle_entry_phase_ready(ctx) {
        return failed_boot_idle_entry_setup();
    }

    crate::phases::state::mark(
        &BOOT_IDLE_ENTRY_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootIdleEntryPhaseReady,
    )
}

pub fn is_ready() -> bool {
    boot_init_rest_init_ready() && boot_init_schedule_handoff_ready() && boot_idle_entry_ready()
}

pub fn boot_init_rest_init_ready() -> bool {
    crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE) == State::Ready
}

pub fn boot_init_schedule_handoff_ready() -> bool {
    crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE) == State::Ready
}

pub fn boot_idle_entry_ready() -> bool {
    crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE) == State::Ready
}

pub fn dispatch_ready() -> bool {
    boot_init_schedule_handoff_ready()
}

fn boot_init_rest_init_phase_ready(ctx: &Context) -> bool {
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };

    crate::phases::interrupt::process_prepare::is_ready()
        && ctx.rcu_core.scheduler_starting_ready()
        && ctx.rcu_core.scheduler_active_init()
        && ctx.rcu_core.scheduler_start_single_online_cpu()
        && ctx.rcu_core.gp_seq_baseline_synced()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.boot_cpu_current_task.state() == State::Ready
        && ctx.boot_cpu_current_task.current_is_boot_idle()
        && ctx.task_creation_core.entry_contract_ready()
        && ctx.task_creation_core.kernel_init_created()
        && ctx.task_creation_core.kthreadd_created()
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
        && boot_scheduler_view.runqueue_contains_task_id(ctx.kernel_init_task.pid())
        && ctx.kernel_init_task_pi_lock.state() == State::Ready
        && !ctx.kernel_init_task_pi_lock.locked()
        && ctx.kernel_init_task_pi_lock.irqsave_entered_count() != 0
        && ctx.kernel_init_task_pi_lock.irqrestore_exited_count() != 0
        && ctx.kernel_init_task.waiting_for_kthreadd_done()
        && !ctx.kernel_init_task.observed_kthreadd_done_release()
        && !ctx.kernel_init_task.released_for_pre_smp_init()
        && ctx.kernel_init_task.pinned_to_boot_cpu()
        && ctx.kernel_init_task.pf_no_setaffinity()
        && ctx.kernel_init_task.cpu_id() == boot_cpu.logical_id()
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
        && ctx.kthreadd_task.cpu_id() == boot_cpu.logical_id()
        && ctx.scheduler.selected_runqueue_task_id() == ctx.kthreadd_task.pid()
        && boot_scheduler_view.runqueue_contains_task_id(ctx.kthreadd_task.pid())
        && ctx.kthreadd_task_pi_lock.state() == State::Ready
        && !ctx.kthreadd_task_pi_lock.locked()
        && ctx.kthreadd_task_pi_lock.irqsave_entered_count() != 0
        && ctx.kthreadd_task_pi_lock.irqrestore_exited_count() != 0
        && ctx
            .kthreadd_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
        && ctx.kthreadd_task.global_ref_bound()
        && ctx.kthreadd_task.provider_ready()
        && ctx.kthreadd_task.schedule_loop_active()
        && ctx.kthreadd_task.enqueued()
        && ctx.system_state.state() == State::Ready
        && ctx.system_state.value() == SystemStateValue::Scheduling
        && ctx.kthreadd_ready_gate.state() == State::Online
        && ctx.kthreadd_ready_gate.completion().complete_committed()
        && ctx.kthreadd_ready_gate.completion().token_available()
        && ctx.scheduler.schedule_passes() == 0
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn boot_init_schedule_handoff_phase_ready(ctx: &Context) -> bool {
    boot_init_rest_init_phase_ready_after_handoff(ctx)
        && ctx.scheduler.schedule_passes() != 0
        && ctx.scheduler.current_runqueue_resolve_passes() != 0
        && ctx.scheduler.pick_next_task_passes() != 0
        && ctx.scheduler.switch_to_passes() != 0
        && ctx.scheduler.identity_switch_passes() == 0
        && ctx.boot_cpu_current_task.switch_committed_count() != 0
        && ctx.boot_cpu_current_task.current_is_kernel_init()
        && ctx.kthreadd_ready_gate.completion().complete_committed()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn boot_idle_entry_phase_ready(ctx: &Context) -> bool {
    boot_init_schedule_handoff_phase_ready(ctx)
        && ctx.scheduler.boot_idle_preemption().state() == State::Ready
        && ctx.scheduler.boot_idle_preemption().disabled()
        && ctx.boot_idle_runtime.state() == State::Ready
        && ctx.boot_idle_runtime.first_schedule_committed()
        && ctx.boot_idle_runtime.idle_entry_prepared()
        && ctx.boot_idle_runtime.cpu_startup_entry_ready()
        && ctx.boot_idle_runtime.idle_loop_entered()
        && ctx.boot_idle_runtime.idle_cycle_committed()
        && ctx
            .boot_idle_runtime
            .representative_need_resched_cycle_committed()
        && ctx.boot_idle_runtime.nohz_run_idle_balance_done()
        && ctx.boot_idle_runtime.local_irq_disabled_for_sleep()
        && ctx.boot_idle_runtime.arch_cpu_idle_enter_done()
        && ctx.boot_idle_runtime.arch_cpu_idle_exit_done()
        && ctx.boot_idle_runtime.smp_call_function_queue_flushed()
        && ctx.scheduler.idle_schedule_passes() != 0
        && ctx.scheduler.idle_schedule_returned_passes() != 0
        && ctx.boot_idle_runtime.boot_init_handoff_complete()
        && ctx.boot_idle_runtime.boot_cpu_hotplug_online()
        && ctx.boot_idle_runtime.secondary_cpus_not_started()
        && ctx
            .boot_idle_runtime
            .kernel_init_task_switch_handoff_ready()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn boot_init_rest_init_phase_ready_after_handoff(ctx: &Context) -> bool {
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };

    crate::phases::interrupt::process_prepare::is_ready()
        && ctx.rcu_core.scheduler_starting_ready()
        && ctx.rcu_core.scheduler_active_init()
        && ctx.rcu_core.scheduler_start_single_online_cpu()
        && ctx.rcu_core.gp_seq_baseline_synced()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.boot_cpu_current_task.state() == State::Ready
        && ctx.task_creation_core.entry_contract_ready()
        && ctx.task_creation_core.kernel_init_created()
        && ctx.task_creation_core.kthreadd_created()
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
        && boot_scheduler_view.runqueue_contains_task_id(ctx.kernel_init_task.pid())
        && ctx.kernel_init_task_pi_lock.state() == State::Ready
        && !ctx.kernel_init_task_pi_lock.locked()
        && ctx.kernel_init_task_pi_lock.irqsave_entered_count() != 0
        && ctx.kernel_init_task_pi_lock.irqrestore_exited_count() != 0
        && ctx.kernel_init_task.pinned_to_boot_cpu()
        && ctx.kernel_init_task.pf_no_setaffinity()
        && ctx.kernel_init_task.cpu_id() == boot_cpu.logical_id()
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
        && ctx.kthreadd_task.cpu_id() == boot_cpu.logical_id()
        && boot_scheduler_view.runqueue_contains_task_id(ctx.kthreadd_task.pid())
        && ctx.kthreadd_task_pi_lock.state() == State::Ready
        && !ctx.kthreadd_task_pi_lock.locked()
        && ctx.kthreadd_task_pi_lock.irqsave_entered_count() != 0
        && ctx.kthreadd_task_pi_lock.irqrestore_exited_count() != 0
        && ctx
            .kthreadd_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
        && ctx.kthreadd_task.global_ref_bound()
        && ctx.kthreadd_task.provider_ready()
        && ctx.kthreadd_task.schedule_loop_active()
        && ctx.kthreadd_task.enqueued()
        && ctx.system_state.state() == State::Ready
        && ctx.system_state.value() == SystemStateValue::Scheduling
        && ctx.kthreadd_ready_gate.state() == State::Online
        && ctx.kthreadd_ready_gate.completion().complete_committed()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}
