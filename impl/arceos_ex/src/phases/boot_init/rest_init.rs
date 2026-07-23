use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        process_prepare::TaskCopyProcessInputs,
        rest_init::{
            KERNEL_INIT_PID, KTHREADD_PID, SystemStateValue, allocate_kernel_stack,
            kernel_init_entry, kthreadd_entry, runtime_services_still_deferred,
        },
        state::{EventResult, FailureDiagnostic, LifecycleEvent, State, failed_condition},
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
    crate::phases::boot_init::setup_after_boot_init_rest_init()
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
    crate::phases::boot_init::enable_after_boot_init_schedule_handoff()
}

pub fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        require_boot_idle_entry_preset(),
        "arceos_ex boot idle entry preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootIdleEntryPhaseStarted);
    crate::phases::shutdown_on_error(
        enter_boot_idle_startup_context(ctx)
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
    boot_idle_continuation(ctx)
}

fn require_boot_init_rest_init_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE);
    if state != State::Base || !crate::phases::interrupt::process_prepare::is_online() {
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
    preset_kernel_init_task(ctx)?;
    setup_kernel_init_task(ctx)?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskReady, ctx);
    ctx.kernel_init_flow
        .bind_initial(&mut ctx.kernel_init_task)?;
    ctx.kernel_init_task_pi_lock.setup()?;
    wake_and_enable_kernel_init_task(ctx)?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskOnline, ctx);
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    };
    pin_kernel_init_to_boot_cpu(ctx, boot_cpu.logical_id())?;
    preset_kthreadd_task(ctx)?;
    setup_kthreadd_task(ctx)?;
    ctx.kthreadd_flow.bind_initial(&mut ctx.kthreadd_task)?;
    ctx.kthreadd_task_pi_lock
        .setup_with_checkpoint(Checkpoint::KthreaddTaskPiLockReady)?;
    wake_and_enable_kthreadd_task(ctx)?;
    publish_kthreadd_global_ref(ctx)?;
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

fn preset_kernel_init_task(ctx: &mut Context) -> EventResult {
    if ctx.kernel_init_task.state() != State::Base
        || ctx.task_creation_core.state() != State::Ready
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.credential_core.state() != State::Prepared
        || ctx.signal_core.state() != State::Prepared
        || ctx.task_file_context.state() != State::Prepared
        || ctx.security_core.state() != State::Ready
        || ctx.boot_task.state() != State::OnCpu
    {
        return failed_condition(
            LifecycleEvent::Preset,
            ctx.kernel_init_task.state(),
            State::Base,
            State::Prepared,
        );
    }

    ctx.kernel_init_task.commit_preset_metadata()?;
    ctx.kernel_init_task
        .task_mut()
        .preset(Checkpoint::KernelInitTaskPrepared)?;
    copy_kernel_init_task(ctx)
}

fn copy_kernel_init_task(ctx: &mut Context) -> EventResult {
    if ctx.kernel_init_task.state() != State::Prepared
        || ctx.task_creation_core.state() != State::Ready
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.scheduler.state() != State::Online
        || ctx
            .scheduler
            .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
            .is_none()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kernel_init_task.state(),
            State::Prepared,
            State::Ready,
        );
    }

    let copy_result = ctx
        .task_creation_core
        .copy_process(
            TaskCopyProcessInputs {
                src_task: &ctx.boot_task,
                root_pid_namespace: &ctx.root_pid_namespace,
                credential_core: &ctx.credential_core,
                signal_core: &ctx.signal_core,
                task_file_context: &ctx.task_file_context,
                security_core: &ctx.security_core,
                scheduler: &ctx.scheduler,
                cpu_group: &ctx.cpu_group,
                entry: TaskEntry::KernelInit,
            },
            ctx.kernel_init_task.state(),
            ctx.kernel_init_task.entry(),
        )
        .map_err(|error| {
            error.with_diagnostic_if_absent(FailureDiagnostic::new(
                "BootInitRestInitPhase",
                "copy_kernel_init_task",
                "KernelInitTask",
                "copy_process",
                "TaskCreationCore.copy_process",
            ))
        })?;
    if copy_result.entry() != TaskEntry::KernelInit
        || !copy_result.task_struct_allocated()
        || !copy_result.thread_context_ready()
        || !copy_result.sched_entity_ready()
        || !copy_result.task_state_new()
        || !copy_result.task_not_enqueued()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kernel_init_task.state(),
            State::Prepared,
            State::Ready,
        );
    }

    let stack_top = allocate_kernel_stack(
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &mut ctx.page_table_caches,
        &ctx.config,
    )
    .map_err(|error| {
        error.with_diagnostic_if_absent(FailureDiagnostic::new(
            "BootInitRestInitPhase",
            "copy_kernel_init_task",
            "KernelInitTask",
            "kernel_stack",
            "allocate_kernel_stack",
        ))
    })?;
    ctx.kernel_init_task.commit_copy_process_metadata(
        copy_result.thread_context_ready(),
        copy_result.sched_entity_ready(),
        stack_top,
    )?;
    ctx.kernel_init_task
        .task_mut()
        .init_switch_context(kernel_init_entry, stack_top);
    Ok(())
}

fn setup_kernel_init_task(ctx: &mut Context) -> EventResult {
    if ctx.kernel_init_task.state() != State::Prepared
        || !ctx.kernel_init_task.thread_context_ready()
        || !ctx.kernel_init_task.sched_entity_ready()
        || ctx.kernel_init_task.kernel_stack_top() == 0
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kernel_init_task.state(),
            State::Prepared,
            State::Ready,
        );
    }
    ctx.kernel_init_task
        .task_mut()
        .setup(Checkpoint::KernelInitTaskReady)
}

fn wake_and_enable_kernel_init_task(ctx: &mut Context) -> EventResult {
    if ctx.kernel_init_task.state() != State::Ready
        || ctx.scheduler.state() != State::Online
        || ctx
            .scheduler
            .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
            .is_none()
        || ctx.cpu_group.state() != State::Ready
        || ctx.boot_current_cpu.state() != State::Online
        || ctx.boot_cpu_local_interrupt.state() != State::Ready
        || ctx.boot_cpu_current_task.state() != State::Ready
        || !ctx.boot_cpu_current_task.current_is_boot_task()
        || ctx.kernel_init_task_pi_lock.state() != State::Ready
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
        || ctx.kernel_init_task.pid() != KERNEL_INIT_PID
        || !ctx.kernel_init_task.sched_entity_ready()
    {
        return failed_condition(
            LifecycleEvent::Enable,
            ctx.kernel_init_task.state(),
            State::Ready,
            State::Online,
        );
    }

    ctx.kernel_init_task_pi_lock.lock_irqsave(
        &mut ctx.boot_cpu_local_interrupt,
        ctx.scheduler.boot_idle_preemption_mut(),
    )?;
    let guarded_result: EventResult = (|| {
        ctx.kernel_init_task.task_mut().set_runtime_running()?;
        let selected_rq = ctx
            .scheduler
            .select_runqueue_for_task(ctx.kernel_init_task.pid(), &ctx.cpu_group)?;
        if !ctx
            .kernel_init_task
            .task_mut()
            .set_task_cpu(selected_rq.cpu_id())
        {
            return failed_condition(
                LifecycleEvent::Enable,
                ctx.kernel_init_task.state(),
                State::Ready,
                State::Online,
            );
        }
        ctx.scheduler.enqueue_task_on_runqueue(
            ctx.kernel_init_task.pid(),
            ctx.kernel_init_task.task_ref(),
            selected_rq,
        )?;
        ctx.kernel_init_task.task_mut().publish_runqueue_binding()?;
        ctx.kernel_init_task
            .task_mut()
            .enable(Checkpoint::KernelInitTaskOnline)
    })();
    let unlock_result = ctx.kernel_init_task_pi_lock.unlock_irqrestore(
        &mut ctx.boot_cpu_local_interrupt,
        ctx.scheduler.boot_idle_preemption_mut(),
    );
    guarded_result.and(unlock_result)
}

fn pin_kernel_init_to_boot_cpu(ctx: &mut Context, cpu_id: usize) -> EventResult {
    if ctx.kernel_init_task.state() != State::Online
        || ctx.kernel_init_task.pid() != KERNEL_INIT_PID
        || ctx.kernel_init_task.cpu_id() != cpu_id
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.scheduler.boot_idle_rcu_read_side().state() != State::Prepared
    {
        return failed_condition(
            LifecycleEvent::Enable,
            ctx.kernel_init_task.state(),
            State::Online,
            State::Online,
        );
    }

    let locks_before = ctx.scheduler.boot_idle_rcu_read_side().read_lock_count();
    ctx.scheduler.boot_idle_rcu_read_side_mut().read_lock()?;
    let entered =
        ctx.scheduler.boot_idle_rcu_read_side().read_lock_count() == locks_before.wrapping_add(1);
    let guarded_result = ctx
        .kernel_init_task
        .task_mut()
        .pin_to_cpu(cpu_id)
        .and_then(|()| {
            ctx.kernel_init_task
                .commit_boot_cpu_pin_observation(entered)
        });
    let unlock_result = ctx.scheduler.boot_idle_rcu_read_side_mut().read_unlock();
    guarded_result.and(unlock_result)?;
    let balanced = ctx.scheduler.boot_idle_rcu_read_side().balanced();
    ctx.kernel_init_task
        .commit_pid_lookup_guard_balanced(balanced)
}

fn preset_kthreadd_task(ctx: &mut Context) -> EventResult {
    if ctx.kthreadd_task.state() != State::Base
        || ctx.task_creation_core.state() != State::Ready
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.credential_core.state() != State::Prepared
        || ctx.task_file_context.state() != State::Prepared
        || ctx.boot_task.state() != State::OnCpu
    {
        return failed_condition(
            LifecycleEvent::Preset,
            ctx.kthreadd_task.state(),
            State::Base,
            State::Prepared,
        );
    }

    ctx.kthreadd_task.commit_preset_metadata()?;
    ctx.kthreadd_task
        .task_mut()
        .preset(Checkpoint::KthreaddTaskPrepared)?;
    copy_kthreadd_task(ctx)
}

fn copy_kthreadd_task(ctx: &mut Context) -> EventResult {
    if ctx.kthreadd_task.state() != State::Prepared
        || ctx.task_creation_core.state() != State::Ready
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.scheduler.state() != State::Online
        || ctx
            .scheduler
            .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
            .is_none()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kthreadd_task.state(),
            State::Prepared,
            State::Ready,
        );
    }

    let copy_result = ctx.task_creation_core.copy_process(
        TaskCopyProcessInputs {
            src_task: &ctx.boot_task,
            root_pid_namespace: &ctx.root_pid_namespace,
            credential_core: &ctx.credential_core,
            signal_core: &ctx.signal_core,
            task_file_context: &ctx.task_file_context,
            security_core: &ctx.security_core,
            scheduler: &ctx.scheduler,
            cpu_group: &ctx.cpu_group,
            entry: TaskEntry::Kthreadd,
        },
        ctx.kthreadd_task.state(),
        ctx.kthreadd_task.entry(),
    )?;
    if copy_result.entry() != TaskEntry::Kthreadd
        || !copy_result.task_struct_allocated()
        || !copy_result.thread_context_ready()
        || !copy_result.sched_entity_ready()
        || !copy_result.task_state_new()
        || !copy_result.task_not_enqueued()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kthreadd_task.state(),
            State::Prepared,
            State::Ready,
        );
    }

    let stack_top = allocate_kernel_stack(
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &mut ctx.page_table_caches,
        &ctx.config,
    )?;
    ctx.kthreadd_task.commit_copy_process_metadata(
        copy_result.thread_context_ready(),
        copy_result.sched_entity_ready(),
        stack_top,
    )?;
    ctx.kthreadd_task
        .task_mut()
        .init_switch_context(kthreadd_entry, stack_top);
    Ok(())
}

fn setup_kthreadd_task(ctx: &mut Context) -> EventResult {
    if ctx.kthreadd_task.state() != State::Prepared
        || !ctx.kthreadd_task.thread_context_ready()
        || !ctx.kthreadd_task.sched_entity_ready()
        || ctx.kthreadd_task.kernel_stack_top() == 0
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kthreadd_task.state(),
            State::Prepared,
            State::Ready,
        );
    }
    ctx.kthreadd_task
        .task_mut()
        .setup(Checkpoint::KthreaddTaskReady)
}

fn wake_and_enable_kthreadd_task(ctx: &mut Context) -> EventResult {
    if ctx.kthreadd_task.state() != State::Ready
        || ctx.scheduler.state() != State::Online
        || ctx
            .scheduler
            .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
            .is_none()
        || ctx.cpu_group.state() != State::Ready
        || ctx.boot_current_cpu.state() != State::Online
        || ctx.boot_cpu_local_interrupt.state() != State::Ready
        || ctx.boot_cpu_current_task.state() != State::Ready
        || !ctx.boot_cpu_current_task.current_is_boot_task()
        || ctx.kthreadd_task_pi_lock.state() != State::Ready
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
        || ctx.kthreadd_task.pid() != KTHREADD_PID
        || !ctx.kthreadd_task.sched_entity_ready()
    {
        return failed_condition(
            LifecycleEvent::Enable,
            ctx.kthreadd_task.state(),
            State::Ready,
            State::Online,
        );
    }

    ctx.kthreadd_task_pi_lock.lock_irqsave(
        &mut ctx.boot_cpu_local_interrupt,
        ctx.scheduler.boot_idle_preemption_mut(),
    )?;
    let guarded_result: EventResult = (|| {
        ctx.kthreadd_task.task_mut().set_runtime_running()?;
        let selected_rq = ctx
            .scheduler
            .select_runqueue_for_task(ctx.kthreadd_task.pid(), &ctx.cpu_group)?;
        if !ctx
            .kthreadd_task
            .task_mut()
            .set_task_cpu(selected_rq.cpu_id())
        {
            return failed_condition(
                LifecycleEvent::Enable,
                ctx.kthreadd_task.state(),
                State::Ready,
                State::Online,
            );
        }
        ctx.scheduler.enqueue_task_on_runqueue(
            ctx.kthreadd_task.pid(),
            ctx.kthreadd_task.task_ref(),
            selected_rq,
        )?;
        ctx.kthreadd_task.task_mut().publish_runqueue_binding()?;
        ctx.kthreadd_task
            .task_mut()
            .enable(Checkpoint::KthreaddTaskOnline)
    })();
    let unlock_result = ctx.kthreadd_task_pi_lock.unlock_irqrestore(
        &mut ctx.boot_cpu_local_interrupt,
        ctx.scheduler.boot_idle_preemption_mut(),
    );
    guarded_result.and(unlock_result)
}

fn publish_kthreadd_global_ref(ctx: &mut Context) -> EventResult {
    if ctx.kthreadd_task.state() != State::Online
        || ctx.kthreadd_task.pid() != KTHREADD_PID
        || !ctx.kthreadd_task.running()
        || !ctx.kthreadd_task.enqueued()
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.scheduler.boot_idle_rcu_read_side().state() != State::Prepared
    {
        return failed_condition(
            LifecycleEvent::Enable,
            ctx.kthreadd_task.state(),
            State::Online,
            State::Online,
        );
    }

    let locks_before = ctx.scheduler.boot_idle_rcu_read_side().read_lock_count();
    ctx.scheduler.boot_idle_rcu_read_side_mut().read_lock()?;
    let entered =
        ctx.scheduler.boot_idle_rcu_read_side().read_lock_count() == locks_before.wrapping_add(1);
    let guarded_result = ctx.kthreadd_task.commit_global_ref_metadata(entered);
    if guarded_result.is_ok() {
        crate::checkpoint::checkpoint(Checkpoint::KthreaddTaskGlobalRefBound);
    }
    let unlock_result = ctx.scheduler.boot_idle_rcu_read_side_mut().read_unlock();
    guarded_result.and(unlock_result)?;
    let balanced = ctx.scheduler.boot_idle_rcu_read_side().balanced();
    ctx.kthreadd_task.commit_pid_lookup_guard_balanced(balanced)
}

fn setup_boot_idle_flow(ctx: &mut Context) -> EventResult {
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

fn prepare_boot_idle_entry(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_flow
        .prepare_idle_entry(&ctx.boot_task, &ctx.scheduler, &ctx.cpu_group)
}

fn run_boot_idle_loop(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_flow.run_idle_loop(
        &mut ctx.scheduler,
        &ctx.cpu_group,
        &mut ctx.kernel_init_task,
        &mut ctx.kernel_init_flow,
        &ctx.user_app_flow,
        &mut ctx.kthreadd_task,
        &mut ctx.kthreadd_flow,
        &mut ctx.user_task_set,
        &ctx.boot_task,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    )
}

fn boot_idle_continuation(ctx: &mut Context) -> ! {
    loop {
        let result = ctx.scheduler.schedule_idle(
            &ctx.cpu_group,
            &mut ctx.kernel_init_task,
            &mut ctx.kernel_init_flow,
            &ctx.user_app_flow,
            &mut ctx.kthreadd_task,
            &mut ctx.kthreadd_flow,
            &ctx.boot_idle_flow,
            &mut ctx.user_task_set,
            &mut ctx.boot_cpu_local_interrupt,
            &mut ctx.boot_cpu_current_task,
        );
        crate::phases::shutdown_on_error(result, "boot idle schedule loop failed\n");
    }
}

fn setup_boot_init_schedule_handoff(ctx: &mut Context) -> EventResult {
    if ctx.scheduler.state() != State::Online
        || !boot_init_rest_init_is_online()
        || !boot_init_rest_init_facts_stable(ctx)
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
    setup_boot_idle_flow(ctx)
}

fn enter_boot_idle_startup_context(ctx: &mut Context) -> EventResult {
    if !boot_init_schedule_handoff_is_online()
        || !boot_idle_restore_ready(ctx)
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

    crate::phases::state::mark_checked(
        &BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::BootInitScheduleHandoffPhaseOnline,
    )
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
    boot_init_rest_init_is_online() && boot_init_schedule_handoff_is_online()
}

pub fn boot_init_rest_init_is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE) == State::Online
}

pub fn boot_init_schedule_handoff_is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_SCHEDULE_HANDOFF_PHASE_STATE) == State::Online
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub fn boot_idle_entry_is_online() -> bool {
    crate::phases::state::load(&BOOT_IDLE_ENTRY_PHASE_STATE) == State::Online
}

pub fn dispatch_ready() -> bool {
    let ctx = crate::context::context_ref();
    crate::phases::boot_init::is_online()
        && ctx.scheduler.schedule_passes() != 0
        && ctx.scheduler.current_runqueue_resolve_passes() != 0
        && ctx.scheduler.pick_next_task_passes() != 0
        && ctx.scheduler.switch_to_passes() != 0
        && ctx.boot_cpu_current_task.switch_committed_count() != 0
        && ctx.boot_cpu_current_task.current_is_kernel_init()
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
}

pub fn precommit_ready() -> bool {
    boot_init_schedule_handoff_phase_ready(crate::context::context_ref())
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
        && !ctx.kthreadd_task.schedule_loop_active()
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
    boot_init_rest_init_facts_stable(ctx)
        && ctx.scheduler.schedule_passes() == 0
        && ctx.scheduler.current_runqueue_resolve_passes() == 0
        && ctx.scheduler.pick_next_task_passes() == 0
        && ctx.scheduler.switch_to_passes() == 0
        && ctx.boot_cpu_current_task.switch_committed_count() == 0
        && ctx.boot_cpu_current_task.current_is_boot_task()
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

fn boot_idle_entry_phase_ready(ctx: &Context) -> bool {
    boot_idle_restore_ready(ctx)
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

fn boot_idle_restore_ready(ctx: &Context) -> bool {
    boot_init_rest_init_facts_stable(ctx)
        && crate::phases::boot_init::is_online()
        && ctx.scheduler.schedule_passes() != 0
        && ctx.scheduler.current_runqueue_resolve_passes() != 0
        && ctx.scheduler.pick_next_task_passes() != 0
        && ctx.scheduler.switch_to_passes() != 0
        && ctx.scheduler.identity_switch_passes() == 0
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
        && ctx.scheduler.kernel_init_stack_switch_returned_count() == 1
        && ctx.boot_cpu_current_task.current_is_boot_task()
        && ctx.boot_idle_flow.state() == State::Ready
        && ctx.boot_idle_flow.active()
        && ctx.boot_idle_flow.owner() == ctx.boot_task.task_ref()
        && ctx.boot_task.task().active_flow() == ctx.boot_idle_flow.flow_ref()
}

fn boot_init_rest_init_facts_stable(ctx: &Context) -> bool {
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
        && !ctx.kthreadd_task.schedule_loop_active()
        && ctx.kthreadd_task.enqueued()
        && ctx.system_state.state() == State::Ready
        && ctx.system_state.value() == SystemStateValue::Scheduling
        && ctx.kthreadd_ready_gate.state() == State::Online
        && ctx.kthreadd_ready_gate.completion().complete_committed()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}
