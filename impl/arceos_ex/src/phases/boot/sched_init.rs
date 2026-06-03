use crate::{
    context::Context,
    objects::{
        earlycon, printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static SCHED_INIT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::SchedInitPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex sched init event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    checkpoint_poking_init_noop()?;
    checkpoint_ftrace_init_noop()?;
    ctx.scheduler.preset(
        &ctx.cpu_group,
        &ctx.cpu_id_map,
        &ctx.per_cpu_storage,
        &ctx.static_branch,
    )?;
    ctx.scheduler.setup(
        &ctx.cpu_group,
        &ctx.cpu_id_map,
        &ctx.per_cpu_storage,
        &ctx.init_task,
        &ctx.init_mm,
    )?;
    ctx.scheduler.enable()?;
    checkpoint_irqs_disabled()?;
    ctx.radix_tree
        .setup(&ctx.slub_allocator, &ctx.cpu_hotplug_state)?;
    ctx.maple_tree.setup(&ctx.slub_allocator)?;
    ctx.workqueue.preset(
        &ctx.page_allocator,
        &ctx.slub_allocator,
        &ctx.cpu_group,
        &ctx.per_cpu_storage,
    )?;
    ctx.softirq.preset(&ctx.per_cpu_storage)?;
    ctx.rcu_core.setup(
        &ctx.scheduler,
        &ctx.workqueue,
        &ctx.softirq,
        &ctx.cpu_group,
        &ctx.per_cpu_storage,
    )?;
    checkpoint_trace_init_deferred()?;
    checkpoint_context_tracking_noop()
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !sched_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&SCHED_INIT_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &SCHED_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::SchedInitPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&SCHED_INIT_PHASE_STATE) == State::Ready
}

fn sched_init_phase_ready(ctx: &Context) -> bool {
    crate::phases::boot::mm_core_init::is_ready()
        && ctx.scheduler.state() == State::Online
        && ctx.scheduler.scheduler_running()
        && ctx.scheduler.default_root_domain().state() == State::Ready
        && ctx.scheduler.default_root_domain().possible_cpu_count()
            == ctx.cpu_group.possible_cpu_count()
        && ctx.scheduler.default_root_domain().smp_topology_deferred()
        && ctx.scheduler.bit_wait_queue_table().state() == State::Prepared
        && ctx.scheduler.bit_wait_queue_table().bucket_count() != 0
        && ctx.scheduler.boot_runqueue().state() == State::Ready
        && ctx.scheduler.boot_runqueue().cpu_id() == 0
        && ctx.scheduler.boot_runqueue().boot_hartid() == ctx.cpu_group.boot_hartid()
        && ctx.scheduler.boot_runqueue().class_queues_ready()
        && ctx.scheduler.boot_runqueue().attached_to_root_domain()
        && !ctx.scheduler.boot_runqueue().balance_push_enabled()
        && ctx.scheduler.boot_idle_task().state() == State::Ready
        && ctx.scheduler.boot_idle_task().cpu_id() == ctx.scheduler.boot_runqueue().cpu_id()
        && ctx.scheduler.boot_idle_task().task_id() == ctx.scheduler.boot_runqueue().curr_task_id()
        && ctx.scheduler.boot_idle_task().uses_current_init_task()
        && ctx.scheduler.boot_idle_task().lazy_tlb_mm_ready()
        && ctx.scheduler.boot_idle_task().no_set_affinity()
        && ctx.radix_tree.state() == State::Ready
        && ctx.radix_tree.node_cache_ready()
        && ctx.radix_tree.cpuhp_step() != 0
        && ctx.radix_tree.node_api_ready()
        && ctx.maple_tree.state() == State::Ready
        && ctx.maple_tree.node_cache_ready()
        && ctx.maple_tree.node_api_ready()
        && ctx.workqueue.state() == State::Prepared
        && ctx.workqueue.system_queues_ready()
        && ctx.workqueue.worker_pools_prepared()
        && ctx.workqueue.unbound_cpumask_ready()
        && ctx.workqueue.bh_pools_ready()
        && !ctx.workqueue.workers_running()
        && ctx.workqueue.possible_cpu_count() == ctx.cpu_group.possible_cpu_count()
        && ctx.softirq.state() == State::Prepared
        && ctx.softirq.action_table_ready()
        && ctx.softirq.slot_count() != 0
        && ctx.softirq.pending_set_ready()
        && !ctx.softirq.execution_open()
        && ctx.rcu_core.state() == State::Ready
        && ctx.rcu_core.tasks_rcu().state() == State::Prepared
        && ctx.rcu_core.boot_cpu_online_ready()
        && ctx.rcu_core.softirq_registered()
        && ctx.rcu_core.workqueues_ready()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.rcu_core.tasks_rcu().callback_lists_ready()
        && ctx.rcu_core.tasks_rcu().enabled_flavor_count() != 0
        && ctx.rcu_core.tasks_rcu().gp_threads_deferred()
        && printk::is_ready()
        && earlycon::is_online()
        && !crate::arch::riscv64::csr::supervisor_interrupts_enabled()
}

fn checkpoint_poking_init_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::PokingInitNoop);
    Ok(())
}

fn checkpoint_ftrace_init_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::FtraceInitNoop);
    Ok(())
}

fn checkpoint_irqs_disabled() -> EventResult {
    if crate::arch::riscv64::csr::supervisor_interrupts_enabled() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Base,
            State::Ready,
        );
    }

    crate::trace::checkpoint(Checkpoint::SchedInitIrqsDisabledCheckpoint);
    Ok(())
}

fn checkpoint_trace_init_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::TraceInitDeferred);
    Ok(())
}

fn checkpoint_context_tracking_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::ContextTrackingInitNoop);
    Ok(())
}
