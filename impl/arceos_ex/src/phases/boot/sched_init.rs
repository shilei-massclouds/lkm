use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        earlycon,
        mm_core::NamedSlubCacheKind,
        printk,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static SCHED_INIT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(ctx),
        "arceos_ex sched init preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::SchedInitPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared_with_check(ctx)),
        "arceos_ex sched init preset failed\n",
    );
    setup(ctx)
}

fn preset_start(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&SCHED_INIT_PHASE_STATE);
    if state != State::Base || !preset_dependencies_ready(ctx) {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_dependencies_ready(ctx: &Context) -> bool {
    crate::phases::boot::mm_core_init::is_online()
        && ctx.page_allocator.state() == State::Ready
        && ctx.slub_subsystem.state() == State::Ready
        && ctx.slub_subsystem.kmalloc_caches().state() == State::Ready
        && ctx.cpu_group.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && ctx.cpu_hotplug_state.state() == State::Ready
        && ctx.static_branch.state() == State::Ready
        && printk::is_ready()
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.sched_init_prelude_trimmed_paths.setup()?;
    ctx.scheduler
        .preset(&ctx.cpu_group, &ctx.per_cpu_storage, &ctx.static_branch)?;
    ctx.scheduler.setup(
        &ctx.cpu_group,
        &ctx.per_cpu_storage,
        &mut ctx.boot_task,
        &ctx.init_mm,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
    )?;
    ctx.scheduler.enable()?;
    checkpoint_irqs_disabled()?;
    ctx.radix_tree
        .setup(&mut ctx.slub_subsystem, &ctx.cpu_hotplug_state)?;
    ctx.maple_tree.setup(&mut ctx.slub_subsystem)?;
    ctx.workqueue.preset(
        &ctx.page_allocator,
        &mut ctx.slub_subsystem,
        &ctx.cpu_group,
        &ctx.per_cpu_storage,
        &ctx.boot_task,
    )?;
    ctx.softirq.preset(&ctx.per_cpu_storage)?;
    ctx.rcu_core.setup(
        &ctx.scheduler,
        &ctx.workqueue,
        &mut ctx.softirq,
        &ctx.cpu_group,
        &ctx.per_cpu_storage,
    )?;
    ctx.sched_init_trace_context_boundaries
        .setup(&ctx.sched_init_prelude_trimmed_paths, &ctx.rcu_core)
}

fn adopt_prepared_with_check(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&SCHED_INIT_PHASE_STATE);
    if state != State::Base
        || !sched_init_phase_ready(ctx)
        || crate::arch::riscv64::csr::supervisor_interrupts_enabled()
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    crate::phases::state::mark_checked(
        &SCHED_INIT_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::SchedInitPhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(adopt_ready(ctx), "arceos_ex sched init setup failed\n");
    enable(ctx)
}

fn adopt_ready(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&SCHED_INIT_PHASE_STATE);
    if state != State::Prepared || !sched_init_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Setup, state, State::Prepared, State::Ready);
    }

    crate::phases::state::mark_checked(
        &SCHED_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::SchedInitPhaseReady,
    )
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(enable_event(ctx), "arceos_ex sched init enable failed\n");
    crate::phases::boot_init::setup_after_sched_init()
}

fn enable_event(ctx: &mut Context) -> EventResult {
    let state = crate::phases::state::load(&SCHED_INIT_PHASE_STATE);
    if state != State::Ready || !sched_init_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }

    crate::phases::state::mark_checked(
        &SCHED_INIT_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::SchedInitPhaseOnline,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&SCHED_INIT_PHASE_STATE) == State::Online
}

fn sched_init_phase_ready(ctx: &Context) -> bool {
    let boot_cpu = ctx.cpu_group.boot_cpu();
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };
    let boot_runqueue = ctx.scheduler.boot_runqueue();
    let boot_idle_setup_state = boot_scheduler_view.idle_task();

    crate::phases::boot::mm_core_init::is_online()
        && ctx.scheduler.state() == State::Online
        && ctx.scheduler.scheduler_running()
        && ctx.scheduler.default_root_domain().state() == State::Ready
        && ctx.scheduler.default_root_domain().possible_cpu_count()
            == ctx.cpu_group.possible_cpu_count()
        && ctx.scheduler.default_root_domain().covered_cpu_count()
            == ctx.cpu_group.possible_cpu_count()
        && ctx.scheduler.default_root_domain().covered_cpu_ref(0) == ctx.cpu_group.boot_cpu_ref()
        && ctx
            .scheduler
            .default_root_domain()
            .covers_cpu_group_possible(&ctx.cpu_group)
        && ctx.scheduler.default_root_domain().smp_topology_deferred()
        && ctx.scheduler.bit_wait_queue_table().state() == State::Prepared
        && ctx.scheduler.bit_wait_queue_table().bucket_count() != 0
        && ctx
            .scheduler
            .bit_wait_queue_table()
            .bucket_count_matches_wait_table_size()
        && ctx
            .scheduler
            .bit_wait_queue_table()
            .bucket_waitqueues_ready()
        && ctx.scheduler.bit_wait_queue_table().bucket_locks_ready()
        && ctx.scheduler.bit_wait_queue_table().bucket_lists_empty()
        && boot_runqueue.state() == State::Ready
        && boot_runqueue.cpu_ref().is_boot_cpu()
        && boot_cpu
            .map(|cpu| boot_runqueue.cpu_ref() == cpu.cpu_ref())
            .unwrap_or(false)
        && boot_runqueue.class_queues_ready()
        && boot_runqueue.attached_to_root_domain()
        && boot_runqueue.root_attach_held_runqueue_lock()
        && ctx.scheduler.boot_runqueue_lock().state() == State::Ready
        && !ctx.scheduler.boot_runqueue_lock().locked()
        && ctx.scheduler.boot_runqueue_lock().acquired_count() != 0
        && ctx.scheduler.boot_runqueue_lock().released_count() != 0
        && ctx.scheduler.boot_runqueue_lock().irqsave_entered_count() != 0
        && ctx.scheduler.boot_runqueue_lock().irqrestore_exited_count() != 0
        && ctx.scheduler.boot_init_preemption().state() == State::Ready
        && ctx.scheduler.boot_init_preemption().disabled()
        && ctx
            .scheduler
            .default_root_domain()
            .covers_cpu_ref(boot_runqueue.cpu_ref())
        && !boot_runqueue.balance_push_enabled()
        && ctx
            .scheduler
            .boot_cpu_owned_scheduler_view_ready(&ctx.cpu_group)
        && boot_idle_setup_state.state() == State::Ready
        && boot_scheduler_view.runqueue_idle_task_matches()
        && boot_idle_setup_state.uses_current_init_task()
        && boot_idle_setup_state.lazy_tlb_mm_ready()
        && boot_idle_setup_state.no_set_affinity()
        && ctx.scheduler.boot_idle_pi_lock().state() == State::Ready
        && !ctx.scheduler.boot_idle_pi_lock().locked()
        && ctx.scheduler.boot_idle_pi_lock().irqsave_entered_count() != 0
        && ctx.scheduler.boot_idle_pi_lock().irqrestore_exited_count() != 0
        && ctx.scheduler.boot_idle_rcu_read_side().state() == State::Prepared
        && ctx
            .scheduler
            .boot_idle_rcu_read_side()
            .incomplete_first_slice()
        && ctx
            .scheduler
            .boot_idle_rcu_read_side()
            .full_semantics_deferred()
        && ctx.scheduler.boot_idle_rcu_read_side().read_lock_count() != 0
        && ctx.scheduler.boot_idle_rcu_read_side().read_unlock_count() != 0
        && ctx.scheduler.boot_idle_rcu_read_side().balanced()
        && ctx.scheduler.boot_idle_preemption().state() == State::Ready
        && ctx.scheduler.boot_idle_preemption().disabled()
        && ctx.boot_cpu_current_task.state() == State::Ready
        && ctx.boot_cpu_current_task.current_is_boot_task()
        && ctx.radix_tree.state() == State::Ready
        && ctx.radix_tree.node_cache_ready()
        && ctx.radix_tree.registered_in_slub_registry()
        && ctx.radix_tree.node_cache_object_size() != 0
        && ctx
            .slub_subsystem
            .cache_registry()
            .named_cache(NamedSlubCacheKind::RadixTreeNode)
            .map(|cache| cache.object_size() == ctx.radix_tree.node_cache_object_size())
            == Some(true)
        && ctx.radix_tree.cpuhp_step() != 0
        && ctx.radix_tree.node_api_ready()
        && ctx.radix_tree.node_rcu_free_callback_deferred()
        && ctx.maple_tree.state() == State::Ready
        && ctx.maple_tree.node_cache_ready()
        && ctx.maple_tree.registered_in_slub_registry()
        && ctx.maple_tree.node_cache_object_size() != 0
        && ctx
            .slub_subsystem
            .cache_registry()
            .named_cache(NamedSlubCacheKind::MapleNode)
            .map(|cache| cache.object_size() == ctx.maple_tree.node_cache_object_size())
            == Some(true)
        && ctx.maple_tree.node_api_ready()
        && ctx.maple_tree.node_rcu_free_callback_deferred()
        && ctx.workqueue.state() == State::Prepared
        && ctx.workqueue.system_queues_ready()
        && ctx.workqueue.system_queue_count_matches_linux_early()
        && ctx.workqueue.worker_pools_prepared()
        && ctx.workqueue.cpu_worker_pools_ready()
        && ctx.workqueue.unbound_cpumask_ready()
        && ctx.workqueue.bh_pools_ready()
        && ctx.workqueue.pool_workqueue_cache_ready()
        && ctx.workqueue.registered_in_slub_registry()
        && ctx.workqueue.pool_workqueue_cache_object_size() != 0
        && ctx
            .slub_subsystem
            .cache_registry()
            .named_cache(NamedSlubCacheKind::PoolWorkqueue)
            .map(|cache| cache.object_size() == ctx.workqueue.pool_workqueue_cache_object_size())
            == Some(true)
        && ctx.workqueue.attrs_ready()
        && ctx.workqueue.system_affinity_pods_ready()
        && ctx.workqueue.pool_mutex().ready()
        && ctx.workqueue.pool_mutex().boot_init_task_guard_completed()
        && ctx.workqueue.struct_mutex().ready()
        && ctx
            .workqueue
            .struct_mutex()
            .boot_init_task_guard_completed()
        && ctx.workqueue.pool_mutex_guard_used()
        && ctx.workqueue.struct_mutex_guard_used()
        && ctx.workqueue.pool_attach_mutex_deferred()
        && ctx.workqueue.mayday_lock_deferred()
        && ctx.workqueue.manager_wait_deferred()
        && !ctx.workqueue.workers_running()
        && ctx.workqueue.possible_cpu_count() == ctx.cpu_group.possible_cpu_count()
        && ctx.softirq.state() == State::Prepared
        && ctx.softirq.action_table_ready()
        && ctx.softirq.slot_count() != 0
        && ctx.softirq.pending_set_ready()
        && ctx.softirq.rcu_action_registered()
        && !ctx.softirq.execution_open()
        && ctx.rcu_core.state() == State::Ready
        && ctx.rcu_core.tasks_rcu().state() == State::Prepared
        && ctx.rcu_core.boot_cpu_online_ready()
        && ctx.rcu_core.softirq_registered()
        && ctx.rcu_core.workqueues_ready()
        && ctx.rcu_core.node_tree_ready()
        && ctx.rcu_core.node_locks_ready()
        && ctx.rcu_core.node_waitqueues_ready()
        && ctx.rcu_core.node_poll_work_ready()
        && ctx.rcu_core.percpu_data_ready()
        && ctx.rcu_core.kfree_batch_ready()
        && ctx.rcu_core.kfree_shrinker_registered()
        && ctx.rcu_core.pm_notifier_registered()
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.rcu_core.runtime_read_side_full_semantics_deferred()
        && ctx.rcu_core.tasks_rcu().callback_lists_ready()
        && ctx.rcu_core.tasks_rcu().enabled_flavor_count() != 0
        && ctx.rcu_core.tasks_rcu().percpu_arrays_ready()
        && ctx.rcu_core.tasks_rcu().percpu_locks_ready()
        && ctx.rcu_core.tasks_rcu().percpu_work_ready()
        && ctx.rcu_core.tasks_rcu().barrier_heads_ready()
        && ctx.rcu_core.tasks_rcu().gp_threads_deferred()
        && ctx.sched_init_prelude_trimmed_paths.state() == State::Ready
        && ctx
            .sched_init_prelude_trimmed_paths
            .poking_init_trimmed_noop()
        && ctx
            .sched_init_prelude_trimmed_paths
            .ftrace_init_trimmed_noop()
        && ctx
            .sched_init_prelude_trimmed_paths
            .ftrace_trimmed_because_mcount_record_disabled()
        && ctx
            .sched_init_prelude_trimmed_paths
            .early_trace_init_deferred()
        && ctx.sched_init_prelude_trimmed_paths.position_preserved()
        && ctx.sched_init_trace_context_boundaries.state() == State::Ready
        && ctx
            .sched_init_trace_context_boundaries
            .trace_init_deferred()
        && ctx
            .sched_init_trace_context_boundaries
            .context_tracking_init_trimmed_noop()
        && ctx
            .sched_init_trace_context_boundaries
            .context_tracking_trimmed_because_user_force_disabled()
        && ctx.sched_init_trace_context_boundaries.position_preserved()
        && printk::is_ready()
        && (earlycon::is_online() || printk::console_handoff_complete())
        && !crate::arch::riscv64::csr::supervisor_interrupts_enabled()
}

fn checkpoint_irqs_disabled() -> EventResult {
    if crate::arch::riscv64::csr::supervisor_interrupts_enabled() {
        return failed_condition(
            LifecycleEvent::Preset,
            State::Base,
            State::Base,
            State::Prepared,
        );
    }

    crate::checkpoint::checkpoint(Checkpoint::SchedInitIrqsDisabledCheckpoint);
    Ok(())
}
