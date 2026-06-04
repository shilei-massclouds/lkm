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
    ctx.rcu_scheduler_start
        .setup(&ctx.rcu_core, &ctx.scheduler, &ctx.cpu_group)?;
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
    ctx.kernel_init_task.enable(&ctx.scheduler)?;
    ctx.kernel_init_affinity.setup(
        &mut ctx.kernel_init_task,
        &ctx.root_pid_namespace,
        &ctx.scheduler,
    )?;
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
    ctx.kthreadd_task.enable(&ctx.scheduler)?;
    ctx.system_state.preset()?;
    ctx.system_state
        .setup(&ctx.kernel_init_task, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate
        .setup(&ctx.kernel_init_task, &ctx.kthreadd_task)?;
    ctx.kthreadd_ready_gate.enable(
        &ctx.system_state,
        &ctx.kthreadd_task,
        &mut ctx.kernel_init_task,
    )?;
    ctx.kernel_init_dispatch_gate.setup(
        &mut ctx.scheduler,
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.system_state,
        &ctx.kthreadd_ready_gate,
    )
}

fn setup_boot_idle_tail(ctx: &mut Context) -> EventResult {
    ctx.boot_idle_runtime.setup(
        &ctx.scheduler,
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.kthreadd_ready_gate,
        &ctx.kernel_init_dispatch_gate,
        &ctx.cpu_group,
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
        Checkpoint::KernelInitDispatchGateReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&REST_INIT_PHASE_STATE) == State::Ready
}

pub fn dispatch_ready() -> bool {
    crate::phases::state::load(&REST_INIT_PHASE_STATE) != State::Base
        && crate::context::context().kernel_init_dispatch_gate.state() == State::Ready
}

fn rest_init_phase_ready(ctx: &Context) -> bool {
    rest_init_dispatch_ready(ctx)
        && ctx.boot_idle_runtime.state() == State::Ready
        && ctx.boot_idle_runtime.first_schedule_committed()
        && ctx.boot_idle_runtime.cpu_startup_entry_ready()
        && ctx.boot_idle_runtime.boot_init_handoff_complete()
        && ctx.boot_idle_runtime.boot_cpu_hotplug_online()
        && ctx.boot_idle_runtime.secondary_cpus_not_started()
        && ctx.boot_idle_runtime.real_task_switch_deferred()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn rest_init_dispatch_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::process_prepare::is_ready()
        && ctx.rcu_scheduler_start.state() == State::Ready
        && ctx.rcu_scheduler_start.scheduler_active()
        && ctx.rcu_scheduler_start.single_online_cpu()
        && ctx.rcu_scheduler_start.gp_threads_still_deferred()
        && ctx.kernel_init_task.state() == State::Online
        && ctx.kernel_init_task.pid() == 1
        && ctx.kernel_init_task.entry() == TaskEntry::KernelInit
        && ctx.kernel_init_task.kind() == TaskKind::UserModeThread
        && ctx.kernel_init_task.clone_fs()
        && !ctx.kernel_init_task.user_mm_created()
        && ctx.kernel_init_task.thread_context_ready()
        && ctx.kernel_init_task.sched_entity_ready()
        && ctx.kernel_init_task.enqueued()
        && !ctx.kernel_init_task.waiting_for_kthreadd_done()
        && ctx.kernel_init_task.released_for_pre_smp_init()
        && ctx.kernel_init_task.pinned_to_boot_cpu()
        && ctx.kernel_init_task.pf_no_setaffinity()
        && ctx.kernel_init_task.cpu_id() == ctx.scheduler.boot_runqueue().cpu_id()
        && ctx.kernel_init_affinity.state() == State::Ready
        && ctx.kernel_init_affinity.pid_lookup_used_root_namespace()
        && ctx.kthreadd_task.state() == State::Online
        && ctx.kthreadd_task.pid() == 2
        && ctx.kthreadd_task.entry() == TaskEntry::Kthreadd
        && ctx.kthreadd_task.kind() == TaskKind::KernelThread
        && ctx.kthreadd_task.clone_fs()
        && ctx.kthreadd_task.clone_files()
        && ctx.kthreadd_task.thread_context_ready()
        && ctx.kthreadd_task.sched_entity_ready()
        && ctx.kthreadd_task.global_ref_bound()
        && ctx.kthreadd_task.enqueued()
        && ctx.system_state.state() == State::Ready
        && ctx.system_state.value() == SystemStateValue::Scheduling
        && ctx.kthreadd_ready_gate.state() == State::Online
        && !ctx.kthreadd_ready_gate.pending()
        && ctx.kthreadd_ready_gate.completed()
        && ctx.kthreadd_ready_gate.release_committed()
        && ctx.kernel_init_dispatch_gate.state() == State::Ready
        && ctx.kernel_init_dispatch_gate.schedule_committed()
        && ctx.kernel_init_dispatch_gate.kernel_init_dispatched()
        && ctx.kernel_init_dispatch_gate.boot_idle_tail_pending()
        && runtime_services_still_deferred(&ctx.workqueue, &ctx.rcu_core, &ctx.cpu_group)
}

fn checkpoint_numa_default_policy_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::NumaDefaultPolicyNoop);
    Ok(())
}
