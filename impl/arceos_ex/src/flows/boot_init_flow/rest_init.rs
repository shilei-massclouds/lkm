//! BootInitRestInitPhase lowering owned by BootInitFlow.

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

pub(super) fn preset(ctx: &mut Context) -> ! {
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
    super::setup_after_boot_init_rest_init()
}

fn require_boot_init_rest_init_preset() -> EventResult {
    let state = crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE);
    if state != State::Base || !crate::phases::interrupt::process_prepare::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn setup_boot_init_rest_init(ctx: &mut Context) -> EventResult {
    let cpu_group_ready = ctx.cpu_group.state() == State::Ready;
    let boot_cpu_online = ctx.cpu_group.boot_cpu_state() == State::Online;
    {
        let Context {
            rcu_core,
            cpu_group,
            ..
        } = ctx;
        let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
        else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Ready,
                State::Ready,
            );
        };
        rcu_core.scheduler_start(scheduler, cpu_group_ready, boot_cpu_online, local_interrupt)?;
    }
    preset_kernel_init_task(ctx)?;
    setup_kernel_init_task(ctx)?;
    crate::checkpoint::dispatch(Checkpoint::KernelInitTaskReady, ctx);
    ctx.kernel_init_flow.bind_fixed(&mut ctx.kernel_init_task)?;
    ctx.kernel_init_flow.publish(&ctx.kernel_init_task)?;
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
    ctx.kthreadd_flow.bind_fixed(&mut ctx.kthreadd_task)?;
    ctx.kthreadd_flow.publish(&ctx.kthreadd_task)?;
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
    let Context {
        kthreadd_ready_gate,
        system_state,
        kthreadd_task,
        kthreadd_ready_gate_wait_lock,
        cpu_group,
        ..
    } = ctx;
    let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
    else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    kthreadd_ready_gate.complete(
        system_state,
        kthreadd_task,
        kthreadd_ready_gate_wait_lock,
        local_interrupt,
        scheduler,
    )
}

fn kernel_init_pi_lock_irqsave(ctx: &mut Context) -> EventResult {
    let Context {
        kernel_init_task_pi_lock,
        cpu_group,
        ..
    } = ctx;
    let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
    else {
        return failed_condition(
            LifecycleEvent::Enable,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    kernel_init_task_pi_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())
}

fn kernel_init_pi_unlock_irqrestore(ctx: &mut Context) -> EventResult {
    let Context {
        kernel_init_task_pi_lock,
        cpu_group,
        ..
    } = ctx;
    let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
    else {
        return failed_condition(
            LifecycleEvent::Enable,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    kernel_init_task_pi_lock
        .unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut())
}

fn kthreadd_pi_lock_irqsave(ctx: &mut Context) -> EventResult {
    let Context {
        kthreadd_task_pi_lock,
        cpu_group,
        ..
    } = ctx;
    let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
    else {
        return failed_condition(
            LifecycleEvent::Enable,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    kthreadd_task_pi_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())
}

fn kthreadd_pi_unlock_irqrestore(ctx: &mut Context) -> EventResult {
    let Context {
        kthreadd_task_pi_lock,
        cpu_group,
        ..
    } = ctx;
    let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
    else {
        return failed_condition(
            LifecycleEvent::Enable,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    kthreadd_task_pi_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut())
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
        || ctx.scheduler().state() != State::Online
        || ctx
            .scheduler()
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

    let Ok(current_task) = ctx.current_task() else {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kernel_init_task.state(),
            State::Prepared,
            State::Ready,
        );
    };
    let copy_result = {
        let Context {
            task_creation_core,
            cpu_group,
            boot_task,
            root_pid_namespace,
            credential_core,
            signal_core,
            task_file_context,
            security_core,
            kernel_init_task,
            ..
        } = ctx;
        let Some(scheduler) = cpu_group.boot_scheduler() else {
            return failed_condition(
                LifecycleEvent::Setup,
                kernel_init_task.state(),
                State::Prepared,
                State::Ready,
            );
        };
        task_creation_core.copy_process(
            TaskCopyProcessInputs {
                src_task: boot_task.task(),
                src_task_ref: boot_task.task_ref(),
                current_task,
                root_pid_namespace,
                credential_core,
                signal_core,
                task_file_context,
                security_core,
                scheduler,
                cpu_group,
                entry: TaskEntry::KernelInit,
            },
            kernel_init_task.state(),
            kernel_init_task.entry(),
        )
    }
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
    ctx.kernel_init_task.task_mut().init_switch_context(
        kernel_init_entry,
        stack_top - crate::objects::rest_init::KERNEL_TASK_STACK_SIZE,
        stack_top,
    );
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
        || ctx.scheduler().state() != State::Online
        || ctx
            .scheduler()
            .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
            .is_none()
        || ctx.cpu_group.state() != State::Ready
        || ctx.cpu_group.boot_cpu_state() != State::Online
        || ctx.boot_cpu_local_interrupt().local_state() != State::Ready
        || !ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.boot_task.task_ref()))
        || ctx.kernel_init_task_pi_lock.state() != State::Ready
        || ctx.scheduler().boot_idle_preemption().state() != State::Ready
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

    kernel_init_pi_lock_irqsave(ctx)?;
    let guarded_result: EventResult = (|| {
        ctx.kernel_init_task.task_mut().set_runtime_running()?;
        let task_pid = ctx.kernel_init_task.pid();
        let selected_rq = ctx.scheduler_mut().select_scheduler_for_task(task_pid)?;
        let Some(selected_cpu_ref) = ctx.cpu_group.cpu_ref_at(selected_rq.logical_id()) else {
            return failed_condition(
                LifecycleEvent::Enable,
                ctx.kernel_init_task.state(),
                State::Ready,
                State::Online,
            );
        };
        if !ctx.kernel_init_flow.commit_cpu_ref(selected_cpu_ref) {
            return failed_condition(
                LifecycleEvent::Enable,
                ctx.kernel_init_task.state(),
                State::Ready,
                State::Online,
            );
        }
        let task_ref = ctx.kernel_init_task.task_ref();
        ctx.scheduler_mut()
            .enqueue_task_on_scheduler(task_pid, task_ref, selected_rq)?;
        ctx.kernel_init_task.task_mut().publish_runqueue_binding()?;
        ctx.kernel_init_task
            .task_mut()
            .enable(Checkpoint::KernelInitTaskOnline)
    })();
    let unlock_result = kernel_init_pi_unlock_irqrestore(ctx);
    guarded_result.and(unlock_result)
}

fn pin_kernel_init_to_boot_cpu(ctx: &mut Context, cpu_id: usize) -> EventResult {
    if ctx.kernel_init_task.state() != State::Online
        || ctx.kernel_init_task.pid() != KERNEL_INIT_PID
        || ctx.kernel_init_flow.cpu_id() != cpu_id
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.scheduler().boot_idle_rcu_read_side().state() != State::Prepared
    {
        return failed_condition(
            LifecycleEvent::Enable,
            ctx.kernel_init_task.state(),
            State::Online,
            State::Online,
        );
    }

    let locks_before = ctx.scheduler().boot_idle_rcu_read_side().read_lock_count();
    ctx.scheduler_mut()
        .boot_idle_rcu_read_side_mut()
        .read_lock()?;
    let entered =
        ctx.scheduler().boot_idle_rcu_read_side().read_lock_count() == locks_before.wrapping_add(1);
    let guarded_result = ctx
        .kernel_init_task
        .task_mut()
        .pin_to_cpu(cpu_id)
        .and_then(|()| {
            ctx.kernel_init_task
                .commit_boot_cpu_pin_observation(entered)
        });
    let unlock_result = ctx
        .scheduler_mut()
        .boot_idle_rcu_read_side_mut()
        .read_unlock();
    guarded_result.and(unlock_result)?;
    let balanced = ctx.scheduler().boot_idle_rcu_read_side().balanced();
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
        || ctx.scheduler().state() != State::Online
        || ctx
            .scheduler()
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

    let Ok(current_task) = ctx.current_task() else {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.kthreadd_task.state(),
            State::Prepared,
            State::Ready,
        );
    };
    let copy_result = {
        let Context {
            task_creation_core,
            cpu_group,
            boot_task,
            root_pid_namespace,
            credential_core,
            signal_core,
            task_file_context,
            security_core,
            kthreadd_task,
            ..
        } = ctx;
        let Some(scheduler) = cpu_group.boot_scheduler() else {
            return failed_condition(
                LifecycleEvent::Setup,
                kthreadd_task.state(),
                State::Prepared,
                State::Ready,
            );
        };
        task_creation_core.copy_process(
            TaskCopyProcessInputs {
                src_task: boot_task.task(),
                src_task_ref: boot_task.task_ref(),
                current_task,
                root_pid_namespace,
                credential_core,
                signal_core,
                task_file_context,
                security_core,
                scheduler,
                cpu_group,
                entry: TaskEntry::Kthreadd,
            },
            kthreadd_task.state(),
            kthreadd_task.entry(),
        )?
    };
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
    ctx.kthreadd_task.task_mut().init_switch_context(
        kthreadd_entry,
        stack_top - crate::objects::rest_init::KERNEL_TASK_STACK_SIZE,
        stack_top,
    );
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
        || ctx.scheduler().state() != State::Online
        || ctx
            .scheduler()
            .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
            .is_none()
        || ctx.cpu_group.state() != State::Ready
        || ctx.cpu_group.boot_cpu_state() != State::Online
        || ctx.boot_cpu_local_interrupt().local_state() != State::Ready
        || !ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.boot_task.task_ref()))
        || ctx.kthreadd_task_pi_lock.state() != State::Ready
        || ctx.scheduler().boot_idle_preemption().state() != State::Ready
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

    kthreadd_pi_lock_irqsave(ctx)?;
    let guarded_result: EventResult = (|| {
        ctx.kthreadd_task.task_mut().set_runtime_running()?;
        let task_pid = ctx.kthreadd_task.pid();
        let selected_rq = ctx.scheduler_mut().select_scheduler_for_task(task_pid)?;
        let Some(selected_cpu_ref) = ctx.cpu_group.cpu_ref_at(selected_rq.logical_id()) else {
            return failed_condition(
                LifecycleEvent::Enable,
                ctx.kthreadd_task.state(),
                State::Ready,
                State::Online,
            );
        };
        if !ctx.kthreadd_flow.commit_cpu_ref(selected_cpu_ref) {
            return failed_condition(
                LifecycleEvent::Enable,
                ctx.kthreadd_task.state(),
                State::Ready,
                State::Online,
            );
        }
        let task_ref = ctx.kthreadd_task.task_ref();
        ctx.scheduler_mut()
            .enqueue_task_on_scheduler(task_pid, task_ref, selected_rq)?;
        ctx.kthreadd_task.task_mut().publish_runqueue_binding()?;
        ctx.kthreadd_task
            .task_mut()
            .enable(Checkpoint::KthreaddTaskOnline)
    })();
    let unlock_result = kthreadd_pi_unlock_irqrestore(ctx);
    guarded_result.and(unlock_result)
}

fn publish_kthreadd_global_ref(ctx: &mut Context) -> EventResult {
    if ctx.kthreadd_task.state() != State::Online
        || ctx.kthreadd_task.pid() != KTHREADD_PID
        || !ctx.kthreadd_task.running()
        || !ctx.kthreadd_task.enqueued()
        || ctx.root_pid_namespace.state() != State::Ready
        || ctx.scheduler().boot_idle_rcu_read_side().state() != State::Prepared
    {
        return failed_condition(
            LifecycleEvent::Enable,
            ctx.kthreadd_task.state(),
            State::Online,
            State::Online,
        );
    }

    let locks_before = ctx.scheduler().boot_idle_rcu_read_side().read_lock_count();
    ctx.scheduler_mut()
        .boot_idle_rcu_read_side_mut()
        .read_lock()?;
    let entered =
        ctx.scheduler().boot_idle_rcu_read_side().read_lock_count() == locks_before.wrapping_add(1);
    let guarded_result = ctx.kthreadd_task.commit_global_ref_metadata(entered);
    if guarded_result.is_ok() {
        crate::checkpoint::checkpoint(Checkpoint::KthreaddTaskGlobalRefBound);
    }
    let unlock_result = ctx
        .scheduler_mut()
        .boot_idle_rcu_read_side_mut()
        .read_unlock();
    guarded_result.and(unlock_result)?;
    let balanced = ctx.scheduler().boot_idle_rcu_read_side().balanced();
    ctx.kthreadd_task.commit_pid_lookup_guard_balanced(balanced)
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

pub(super) fn is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_REST_INIT_PHASE_STATE) == State::Online
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
    let Some(boot_scheduler_view) = ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };

    crate::phases::interrupt::process_prepare::is_online()
        && ctx.rcu_core.scheduler_starting_ready()
        && ctx.rcu_core.scheduler_active_init()
        && ctx.rcu_core.scheduler_start_single_online_cpu()
        && ctx.rcu_core.gp_seq_baseline_synced()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.boot_task.task_ref()))
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
        && ctx.kernel_init_flow.cpu_id() == boot_cpu.logical_id()
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
        && ctx.kthreadd_flow.cpu_id() == boot_cpu.logical_id()
        && ctx.scheduler().selected_runqueue_task_id() == ctx.kthreadd_task.pid()
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
        && ctx.scheduler().schedule_passes() == 0
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

pub(super) fn facts_stable(ctx: &Context) -> bool {
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let Some(boot_scheduler_view) = ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };

    crate::phases::interrupt::process_prepare::is_online()
        && ctx.rcu_core.scheduler_starting_ready()
        && ctx.rcu_core.scheduler_active_init()
        && ctx.rcu_core.scheduler_start_single_online_cpu()
        && ctx.rcu_core.gp_seq_baseline_synced()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.boot_task.task_ref()))
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
        && ctx.kernel_init_flow.cpu_id() == boot_cpu.logical_id()
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
        && ctx.kthreadd_flow.cpu_id() == boot_cpu.logical_id()
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
