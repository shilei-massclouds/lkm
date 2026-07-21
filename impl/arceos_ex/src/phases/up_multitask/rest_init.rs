use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        rest_init::{SystemStateValue, TaskSpawnInputs, runtime_services_still_deferred},
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::{TaskEntry, TaskKind},
    },
};
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_REST_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_SCHEDULE_HANDOFF_DISPATCH_READY: AtomicBool = AtomicBool::new(false);
#[unsafe(link_section = ".data.phase")]
static BOOT_IDLE_ENTRY_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        require_boot_init_rest_init_preset(),
        "arceos_ex rest init preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootInitRestInitPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_boot_init_rest_init(ctx).and_then(|()| mark_boot_init_rest_init_prepared(ctx)),
        "arceos_ex rest init preset failed\n",
    );
    crate::phases::shutdown_on_error(
        mark_boot_init_rest_init_ready(ctx),
        "arceos_ex rest init setup failed\n",
    );
    crate::phases::shutdown_on_error(
        mark_boot_init_rest_init_online(ctx),
        "arceos_ex rest init enable failed\n",
    );
    crate::phases::up_multitask::preset_after_boot_init_rest_init()
}

pub fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        require_boot_init_schedule_handoff_preset(),
        "arceos_ex schedule handoff preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootInitScheduleHandoffPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_boot_init_schedule_handoff(ctx)
            .and_then(|()| mark_boot_init_schedule_handoff_prepared(ctx)),
        "arceos_ex schedule handoff preset failed\n",
    );
    crate::phases::shutdown_on_error(
        mark_boot_init_schedule_handoff_ready(ctx),
        "arceos_ex schedule handoff setup failed\n",
    );
    crate::phases::shutdown_on_error(
        mark_boot_init_schedule_handoff_online(ctx),
        "arceos_ex schedule handoff enable failed\n",
    );
    crate::phases::up_multitask::setup_after_boot_init_schedule_handoff()
}

pub fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        require_boot_idle_entry_preset(),
        "arceos_ex boot idle entry preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootIdleEntryPhaseStarted);
    crate::phases::shutdown_on_error(
        enter_boot_idle_startup_context(ctx)
            .and_then(|()| setup_boot_idle_flow(ctx))
            .and_then(|()| prepare_boot_idle_entry(ctx))
            .and_then(|()| run_boot_idle_loop(ctx))
            .and_then(|()| mark_boot_idle_entry_prepared(ctx)),
        "arceos_ex boot idle entry preset failed\n",
    );
    crate::phases::shutdown_on_error(
        mark_boot_idle_entry_ready(ctx),
        "arceos_ex boot idle entry setup failed\n",
    );
    crate::phases::shutdown_on_error(
        mark_boot_idle_entry_online(ctx),
        "arceos_ex boot idle entry enable failed\n",
    );
    crate::phases::up_multitask::enable_after_boot_idle_entry()
}

fn require_boot_init_rest_init_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE);
    if state != State::Base || !crate::phases::interrupt::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn require_boot_init_schedule_handoff_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE);
    if state != State::Base || !boot_init_rest_init_is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn require_boot_idle_entry_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE);
    if state != State::Base || !boot_init_schedule_handoff_is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
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
        boot_task: &ctx.boot_task,
    })?;
    ctx.kernel_init_task.setup(
        &mut ctx.task_creation_core,
        &ctx.root_pid_namespace,
        &ctx.credential_core,
        &ctx.signal_core,
        &ctx.task_file_context,
        &ctx.security_core,
        &ctx.boot_task,
        &ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &mut ctx.page_table_caches,
        &ctx.config,
    )?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskReady, ctx);
    ctx.kernel_init_flow
        .bind_initial(&mut ctx.kernel_init_task)?;
    ctx.kernel_init_task_pi_lock.setup()?;
    ctx.kernel_init_task.enable(
        &mut ctx.scheduler,
        &ctx.cpu_group,
        &ctx.boot_current_cpu,
        &mut ctx.boot_cpu_local_interrupt,
        &ctx.boot_cpu_current_task,
        &mut ctx.kernel_init_task_pi_lock,
    )?;
    ctx.kernel_init_flow.enable_initial(&ctx.kernel_init_task)?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskOnline, ctx);
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE),
            State::Base,
            State::Prepared,
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
        boot_task: &ctx.boot_task,
    })?;
    ctx.kthreadd_task.setup(
        &mut ctx.task_creation_core,
        &ctx.root_pid_namespace,
        &ctx.credential_core,
        &ctx.signal_core,
        &ctx.task_file_context,
        &ctx.security_core,
        &ctx.boot_task,
        &ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &mut ctx.page_table_caches,
        &ctx.config,
    )?;
    ctx.kthreadd_flow.bind_initial(&mut ctx.kthreadd_task)?;
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
    ctx.kthreadd_flow.enable_initial(&ctx.kthreadd_task)?;
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

fn setup_boot_idle_flow(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_flow.setup(
        &mut ctx.boot_task,
        &mut ctx.root_stream,
        &ctx.scheduler,
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.kthreadd_ready_gate,
        &ctx.cpu_group,
    )
}

fn prepare_boot_idle_entry(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_flow
        .prepare_idle_entry(&ctx.scheduler, &ctx.cpu_group)
}

fn run_boot_idle_loop(ctx: &mut Context) -> EventResult {
    // The object model executes both task branches linearly. Re-enter the
    // boot-idle continuation before modeling cpu_startup_entry()/do_idle();
    // schedule_idle() then commits the selected runnable task as this CPU's
    // CurrentTaskRef. The real future return to the idle-loop continuation is
    // left to the later continuation/task-stack model.
    ctx.boot_cpu_current_task.set_current_boot_task()?;
    ctx.boot_idle_flow.run_idle_loop(
        &mut ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    )
}

fn setup_boot_init_schedule_handoff(ctx: &mut Context) -> EventResult {
    if ctx.scheduler.state() != State::Online
        || !boot_init_rest_init_is_online()
        || !boot_init_rest_init_phase_ready_after_handoff(ctx)
        || ctx.kernel_init_task.state() != State::Online
        || ctx.kthreadd_task.state() != State::Online
        || ctx.system_state.state() != State::Ready
        || ctx.system_state.value() != SystemStateValue::Scheduling
        || ctx.kthreadd_ready_gate.state() != State::Online
        || !ctx.kthreadd_ready_gate.completion().complete_committed()
        || !ctx.kthreadd_ready_gate.completion().token_available()
    {
        return failed_schedule_handoff_preset();
    }

    if ctx
        .scheduler
        .boot_idle_preemption_mut()
        .enable_no_resched()
        .is_err()
    {
        return failed_schedule_handoff_preset();
    }

    let schedule_result = ctx.scheduler.schedule(
        &ctx.cpu_group,
        &mut ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    );
    if schedule_result.is_err() {
        return failed_schedule_handoff_preset();
    }
    crate::checkpoint::dispatch(Checkpoint::SchedulerPickNextTaskExit, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerSwitchToEntry, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerSwitchToExit, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerScheduleExit, ctx);
    Ok(())
}

fn enter_boot_idle_startup_context(ctx: &mut Context) -> EventResult {
    if !boot_init_schedule_handoff_is_online()
        || !boot_init_schedule_handoff_phase_ready(ctx)
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
    {
        return failed_boot_idle_entry_preset();
    }

    // schedule_preempt_disabled() disables preemption again before entering
    // cpu_startup_entry(). The formal BootIdleStartupContext exits by Never,
    // so the simulation keeps the preemption control disabled.
    ctx.scheduler.boot_idle_preemption_mut().disable()
}

fn failed_schedule_handoff_preset() -> EventResult {
    failed_condition(
        LifecycleEvent::Preset,
        crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE),
        State::Base,
        State::Prepared,
    )
}

fn failed_boot_idle_entry_preset() -> EventResult {
    failed_condition(
        LifecycleEvent::Preset,
        crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE),
        State::Base,
        State::Prepared,
    )
}

fn mark_boot_init_rest_init_prepared(ctx: &Context) -> EventResult {
    if !boot_init_rest_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    }

    crate::phases::state::mark_checked(
        &BOOT_INIT_REST_INIT_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::BootInitRestInitPhasePrepared,
    )
}

fn mark_boot_init_rest_init_ready(ctx: &Context) -> EventResult {
    if !boot_init_rest_init_phase_ready(ctx) {
        return phase_failure(
            &BOOT_INIT_REST_INIT_PHASE_STATE,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
        );
    }

    crate::phases::state::mark_checked(
        &BOOT_INIT_REST_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::BootInitRestInitPhaseReady,
    )
}

fn mark_boot_init_rest_init_online(ctx: &Context) -> EventResult {
    if !boot_init_rest_init_phase_ready(ctx) {
        return phase_failure(
            &BOOT_INIT_REST_INIT_PHASE_STATE,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
        );
    }

    crate::phases::state::mark_checked(
        &BOOT_INIT_REST_INIT_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::BootInitRestInitPhaseOnline,
    )
}

fn mark_boot_init_schedule_handoff_prepared(ctx: &Context) -> EventResult {
    if !boot_init_schedule_handoff_phase_ready(ctx) {
        return failed_schedule_handoff_preset();
    }

    crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::BootInitScheduleHandoffPhasePrepared,
    )
}

fn mark_boot_init_schedule_handoff_ready(ctx: &Context) -> EventResult {
    if !boot_init_schedule_handoff_phase_ready(ctx) {
        return phase_failure(
            &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
        );
    }

    crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::BootInitScheduleHandoffPhaseReady,
    )
}

fn mark_boot_init_schedule_handoff_online(ctx: &Context) -> EventResult {
    if !boot_init_schedule_handoff_phase_ready(ctx) {
        return phase_failure(
            &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
        );
    }

    BOOT_INIT_SCHEDULE_HANDOFF_DISPATCH_READY.store(true, Ordering::Release);
    let result = crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::BootInitScheduleHandoffPhaseOnline,
    );
    if result.is_err() {
        BOOT_INIT_SCHEDULE_HANDOFF_DISPATCH_READY.store(false, Ordering::Release);
    }
    result
}

fn mark_boot_idle_entry_prepared(ctx: &Context) -> EventResult {
    if !boot_idle_entry_phase_ready(ctx) {
        return failed_boot_idle_entry_preset();
    }

    crate::phases::state::mark_checked(
        &BOOT_IDLE_ENTRY_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::BootIdleEntryPhasePrepared,
    )
}

fn mark_boot_idle_entry_ready(ctx: &Context) -> EventResult {
    if !boot_idle_entry_phase_ready(ctx) {
        return phase_failure(
            &BOOT_IDLE_ENTRY_PHASE_STATE,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
        );
    }

    crate::phases::state::mark_checked(
        &BOOT_IDLE_ENTRY_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::BootIdleEntryPhaseReady,
    )
}

fn mark_boot_idle_entry_online(ctx: &Context) -> EventResult {
    if !boot_idle_entry_phase_ready(ctx) {
        return phase_failure(
            &BOOT_IDLE_ENTRY_PHASE_STATE,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
        );
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
pub fn is_online() -> bool {
    boot_init_rest_init_is_online()
        && boot_init_schedule_handoff_is_online()
        && boot_idle_entry_is_online()
}

pub fn boot_init_rest_init_is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE) == State::Online
}

pub fn boot_init_schedule_handoff_is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE) == State::Online
}

pub fn boot_idle_entry_is_online() -> bool {
    crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE) == State::Online
}

pub fn dispatch_ready() -> bool {
    boot_init_schedule_handoff_is_online()
        && BOOT_INIT_SCHEDULE_HANDOFF_DISPATCH_READY.load(Ordering::Acquire)
}

fn phase_failure(
    phase_state: &AtomicU8,
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(phase_state),
        expected,
        target,
    )
}

fn boot_init_rest_init_phase_ready(ctx: &Context) -> bool {
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };

    crate::phases::interrupt::process_prepare::is_online()
        && ctx.rcu_core.scheduler_starting_ready()
        && ctx.rcu_core.scheduler_active_init()
        && ctx.rcu_core.scheduler_start_single_online_cpu()
        && ctx.rcu_core.gp_seq_baseline_synced()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.boot_cpu_current_task.state() == State::Ready
        && ctx.boot_cpu_current_task.current_is_boot_task()
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

fn boot_init_rest_init_phase_ready_after_handoff(ctx: &Context) -> bool {
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };

    crate::phases::interrupt::process_prepare::is_online()
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
