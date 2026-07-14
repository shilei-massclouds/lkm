use super::{
    cpu_control::LocalInterruptControl,
    cpu_group::CpuGroup,
    per_cpu_storage::PerCpuStorage,
    scheduler::Scheduler,
    softirq::Softirq,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    workqueue::Workqueue,
};
use crate::checkpoint::Checkpoint;

pub struct RcuCore {
    lifecycle: Lifecycle,
    tasks_rcu: TasksRcu,
    boot_cpu_online_ready: bool,
    softirq_registered: bool,
    workqueues_ready: bool,
    node_tree_ready: bool,
    node_locks_ready: bool,
    node_waitqueues_ready: bool,
    node_poll_work_ready: bool,
    percpu_data_ready: bool,
    kfree_batch_ready: bool,
    kfree_shrinker_registered: bool,
    pm_notifier_registered: bool,
    gp_threads_deferred: bool,
    runtime_read_side_full_semantics_deferred: bool,
    scheduler_starting_ready: bool,
    scheduler_active_init: bool,
    scheduler_start_single_online_cpu: bool,
    scheduler_start_local_irq_guarded: bool,
    scheduler_start_local_irq_save_count: usize,
    scheduler_start_local_irq_restore_count: usize,
    gp_seq_baseline_synced: bool,
    inkernel_boot_ended: bool,
    unexpedite_gp_atomic_decrement_recorded: bool,
    async_relax_config_lazy_trimmed: bool,
    normal_after_boot_write_once_trimmed_or_recorded: bool,
    boot_ended_publish_recorded: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl RcuCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            tasks_rcu: TasksRcu::new(),
            boot_cpu_online_ready: false,
            softirq_registered: false,
            workqueues_ready: false,
            node_tree_ready: false,
            node_locks_ready: false,
            node_waitqueues_ready: false,
            node_poll_work_ready: false,
            percpu_data_ready: false,
            kfree_batch_ready: false,
            kfree_shrinker_registered: false,
            pm_notifier_registered: false,
            gp_threads_deferred: true,
            runtime_read_side_full_semantics_deferred: true,
            scheduler_starting_ready: false,
            scheduler_active_init: false,
            scheduler_start_single_online_cpu: false,
            scheduler_start_local_irq_guarded: false,
            scheduler_start_local_irq_save_count: 0,
            scheduler_start_local_irq_restore_count: 0,
            gp_seq_baseline_synced: false,
            inkernel_boot_ended: false,
            unexpedite_gp_atomic_decrement_recorded: false,
            async_relax_config_lazy_trimmed: false,
            normal_after_boot_write_once_trimmed_or_recorded: false,
            boot_ended_publish_recorded: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn tasks_rcu(&self) -> &TasksRcu {
        &self.tasks_rcu
    }

    pub fn tasks_rcu_mut(&mut self) -> &mut TasksRcu {
        &mut self.tasks_rcu
    }

    pub const fn boot_cpu_online_ready(&self) -> bool {
        self.boot_cpu_online_ready
    }

    pub const fn softirq_registered(&self) -> bool {
        self.softirq_registered
    }

    pub const fn workqueues_ready(&self) -> bool {
        self.workqueues_ready
    }

    pub const fn node_tree_ready(&self) -> bool {
        self.node_tree_ready
    }

    pub const fn node_locks_ready(&self) -> bool {
        self.node_locks_ready
    }

    pub const fn node_waitqueues_ready(&self) -> bool {
        self.node_waitqueues_ready
    }

    pub const fn node_poll_work_ready(&self) -> bool {
        self.node_poll_work_ready
    }

    pub const fn percpu_data_ready(&self) -> bool {
        self.percpu_data_ready
    }

    pub const fn kfree_batch_ready(&self) -> bool {
        self.kfree_batch_ready
    }

    pub const fn kfree_shrinker_registered(&self) -> bool {
        self.kfree_shrinker_registered
    }

    pub const fn pm_notifier_registered(&self) -> bool {
        self.pm_notifier_registered
    }

    pub const fn gp_threads_deferred(&self) -> bool {
        self.gp_threads_deferred
    }

    pub const fn runtime_read_side_full_semantics_deferred(&self) -> bool {
        self.runtime_read_side_full_semantics_deferred
    }

    pub const fn scheduler_starting_ready(&self) -> bool {
        self.scheduler_starting_ready
    }

    pub const fn scheduler_active_init(&self) -> bool {
        self.scheduler_active_init
    }

    pub const fn scheduler_start_single_online_cpu(&self) -> bool {
        self.scheduler_start_single_online_cpu
    }

    pub const fn scheduler_start_local_irq_guarded(&self) -> bool {
        self.scheduler_start_local_irq_guarded
    }

    pub const fn scheduler_start_local_irq_save_count(&self) -> usize {
        self.scheduler_start_local_irq_save_count
    }

    pub const fn scheduler_start_local_irq_restore_count(&self) -> usize {
        self.scheduler_start_local_irq_restore_count
    }

    pub const fn gp_seq_baseline_synced(&self) -> bool {
        self.gp_seq_baseline_synced
    }

    pub const fn inkernel_boot_ended(&self) -> bool {
        self.inkernel_boot_ended
    }

    pub const fn unexpedite_gp_atomic_decrement_recorded(&self) -> bool {
        self.unexpedite_gp_atomic_decrement_recorded
    }

    pub const fn async_relax_config_lazy_trimmed(&self) -> bool {
        self.async_relax_config_lazy_trimmed
    }

    pub const fn normal_after_boot_write_once_trimmed_or_recorded(&self) -> bool {
        self.normal_after_boot_write_once_trimmed_or_recorded
    }

    pub const fn boot_ended_publish_recorded(&self) -> bool {
        self.boot_ended_publish_recorded
    }

    pub fn setup(
        &mut self,
        scheduler: &Scheduler,
        workqueue: &Workqueue,
        softirq: &mut Softirq,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || workqueue.state() != State::Prepared
            || softirq.state() != State::Prepared
            || cpu_group.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_setup();
        }

        softirq.register_rcu_action()?;
        self.tasks_rcu.preset(per_cpu_storage, workqueue)?;
        self.boot_cpu_online_ready = cpu_group.boot_cpu_state() == State::Online;
        self.softirq_registered = softirq.rcu_action_registered();
        self.workqueues_ready = workqueue.system_queues_ready();
        self.node_tree_ready = cpu_group.possible_cpu_count() != 0;
        self.node_locks_ready = true;
        self.node_waitqueues_ready = true;
        self.node_poll_work_ready = true;
        self.percpu_data_ready = per_cpu_storage.state() == State::Ready;
        self.kfree_batch_ready = workqueue.system_queues_ready();
        self.kfree_shrinker_registered = true;
        self.pm_notifier_registered = true;
        self.gp_threads_deferred = true;
        self.runtime_read_side_full_semantics_deferred = true;
        self.scheduler_starting_ready = false;
        self.scheduler_active_init = false;
        self.scheduler_start_single_online_cpu = false;
        self.scheduler_start_local_irq_guarded = false;
        self.scheduler_start_local_irq_save_count = 0;
        self.scheduler_start_local_irq_restore_count = 0;
        self.gp_seq_baseline_synced = false;
        self.inkernel_boot_ended = false;
        self.unexpedite_gp_atomic_decrement_recorded = false;
        self.async_relax_config_lazy_trimmed = false;
        self.normal_after_boot_write_once_trimmed_or_recorded = false;
        self.boot_ended_publish_recorded = false;
        if !self.boot_cpu_online_ready
            || !self.softirq_registered
            || !self.workqueues_ready
            || !self.node_tree_ready
            || !self.node_locks_ready
            || !self.node_waitqueues_ready
            || !self.node_poll_work_ready
            || !self.percpu_data_ready
            || !self.kfree_batch_ready
            || !self.kfree_shrinker_registered
            || !self.pm_notifier_registered
            || !self.runtime_read_side_full_semantics_deferred
        {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RcuCoreReady,
        )
    }

    pub fn scheduler_start(
        &mut self,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || scheduler.state() != State::Online
            || cpu_group.state() != State::Ready
            || local_interrupt.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.scheduler_start_single_online_cpu = cpu_group.boot_cpu_state() == State::Online;
        if !self.scheduler_start_single_online_cpu || !self.gp_threads_deferred {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        let saved_before = local_interrupt.saved_and_disabled_count();
        local_interrupt.save_and_disable()?;
        let guarded_result: EventResult = {
            self.scheduler_starting_ready = true;
            self.scheduler_active_init = true;
            self.gp_seq_baseline_synced = true;
            self.scheduler_start_local_irq_guarded = local_interrupt.disabled();
            self.scheduler_start_local_irq_save_count = local_interrupt.saved_and_disabled_count();
            crate::checkpoint::checkpoint(Checkpoint::RcuSchedulerStartingReady);
            Ok(())
        };
        let restore_result = local_interrupt.restore();
        if guarded_result.is_ok() && restore_result.is_ok() {
            self.scheduler_start_local_irq_restore_count = local_interrupt.restored_count();
            self.scheduler_start_local_irq_guarded &= self.scheduler_start_local_irq_save_count
                == saved_before.wrapping_add(1)
                && self.scheduler_start_local_irq_restore_count != 0;
        }
        guarded_result.and(restore_result)
    }

    pub fn end_inkernel_boot(&mut self) -> bool {
        if self.lifecycle.state() != State::Ready || self.inkernel_boot_ended {
            return false;
        }

        self.unexpedite_gp_atomic_decrement_recorded = true;
        self.async_relax_config_lazy_trimmed = true;
        self.normal_after_boot_write_once_trimmed_or_recorded = true;
        self.boot_ended_publish_recorded = true;
        self.inkernel_boot_ended = true;
        crate::checkpoint::checkpoint(Checkpoint::RcuInkernelBootEnded);
        true
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct TasksRcu {
    lifecycle: Lifecycle,
    callback_lists_ready: bool,
    enabled_flavor_count: usize,
    percpu_arrays_ready: bool,
    percpu_locks_ready: bool,
    percpu_work_ready: bool,
    barrier_heads_ready: bool,
    gp_threads_deferred: bool,
    gp_threads_ready: bool,
}

impl TasksRcu {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            callback_lists_ready: false,
            enabled_flavor_count: 0,
            percpu_arrays_ready: false,
            percpu_locks_ready: false,
            percpu_work_ready: false,
            barrier_heads_ready: false,
            gp_threads_deferred: true,
            gp_threads_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn callback_lists_ready(&self) -> bool {
        self.callback_lists_ready
    }

    pub const fn enabled_flavor_count(&self) -> usize {
        self.enabled_flavor_count
    }

    pub const fn percpu_arrays_ready(&self) -> bool {
        self.percpu_arrays_ready
    }

    pub const fn percpu_locks_ready(&self) -> bool {
        self.percpu_locks_ready
    }

    pub const fn percpu_work_ready(&self) -> bool {
        self.percpu_work_ready
    }

    pub const fn barrier_heads_ready(&self) -> bool {
        self.barrier_heads_ready
    }

    pub const fn gp_threads_deferred(&self) -> bool {
        self.gp_threads_deferred
    }

    pub const fn gp_threads_ready(&self) -> bool {
        self.gp_threads_ready
    }

    fn preset(&mut self, per_cpu_storage: &PerCpuStorage, workqueue: &Workqueue) -> EventResult {
        if self.lifecycle.state() != State::Base
            || per_cpu_storage.state() != State::Ready
            || workqueue.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.callback_lists_ready = true;
        self.enabled_flavor_count = 2;
        self.percpu_arrays_ready = true;
        self.percpu_locks_ready = true;
        self.percpu_work_ready = workqueue.system_queues_ready();
        self.barrier_heads_ready = true;
        self.gp_threads_deferred = true;
        self.gp_threads_ready = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TasksRcuPrepared,
        )
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.callback_lists_ready
            || self.enabled_flavor_count == 0
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.gp_threads_deferred = false;
        self.gp_threads_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::TasksRcuReady,
        )
    }
}
