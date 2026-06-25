/*
 * Sched Init Phase Specification
 *
 * This subphase starts after mm_core_init() returns and ends after the
 * context_tracking_init() call point. It builds the boot CPU scheduler
 * foundation and early asynchronous support while interrupts are still closed.
 */

lock BootRunQueueLock: RawSpinLock;
lock BootIdlePiLock: RawSpinLock;

context RunQueueRootAttachContext: ResourceExclusiveContext {
    /*
     * rq_attach_root() attaches a runqueue to def_root_domain while holding
     * rq->__lock through rq_lock_irqsave()/rq_unlock_irqrestore().
     */
    guard {
        lock_ref: BootRunQueueLock;

        entered_by {
            BootRunQueueLock.Transition::LockIrqSave;
        }

        exited_by {
            BootRunQueueLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        BootRunQueue;
        BootInitPreemption;
        DefaultSchedRootDomain;
        Scheduler;
    }
}

context BootIdlePiLockContext: ResourceExclusiveContext {
    /*
     * init_idle() first takes idle->pi_lock with raw_spin_lock_irqsave().
     * The guard records the real irq-save protocol; the outer boot phase
     * context alone is not enough to erase this lock boundary.
     */
    guard {
        lock_ref: BootIdlePiLock;

        entered_by {
            BootIdlePiLock.Transition::LockIrqSave;
        }

        exited_by {
            BootIdlePiLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        BootIdleTask;
        BootRunQueue;
        BootCpuCurrentTask;
        Scheduler;
    }
}

context BootRunQueueLockContext: ResourceExclusiveContext {
    /*
     * init_idle() takes rq->__lock with raw_spin_rq_lock() while the outer
     * BootIdlePiLockContext already holds idle->pi_lock and has saved and
     * disabled local interrupts. This guard models the ordinary raw rq lock
     * boundary; it contributes the held runqueue lock and protected object
     * scope without pretending to run a second irq-save protocol.
     */
    guard {
        lock_ref: BootRunQueueLock;

        entered_by {
            BootRunQueueLock.Action::Acquire;
        }

        exited_by {
            BootRunQueueLock.Action::Release;
        }
    }

    obj_refs {
        BootRunQueue;
        BootIdleTask;
        BootCpuCurrentTask;
        Scheduler;
    }
}

context BootIdleRcuReadSideContext: Context {
    /*
     * init_idle() deliberately wraps __set_task_cpu(idle, cpu) in
     * rcu_read_lock()/rcu_read_unlock() to satisfy Linux's PROVE_RCU checks
     * while rq->__lock is held and idle->cpu is not yet published.
     *
     * This is the first, incomplete RCU read-side slice: it records the
     * lexical read-side guard required by init_idle(), but it is not the full
     * RCU read-side subsystem semantics.
     */
    guard {
        lock_ref: BootIdleRcuReadSide;

        entered_by {
            BootIdleRcuReadSide.Transition::ReadLock;
        }

        exited_by {
            BootIdleRcuReadSide.Transition::ReadUnlock;
        }
    }

    obj_refs {
        BootIdleTask;
        BootRunQueue;
        BootCpuCurrentTask;
        Scheduler;
    }
}

context BitWaitQueueTableInitContext: Context {
    /*
     * wait_bit_init() runs as a boot-time initialization loop before
     * ordinary task, interrupt, or SMP concurrency is opened. The protected
     * lexical body corresponds to initializing every bit_wait_table bucket as
     * a wait_queue_head_t: init_waitqueue_head() initializes the bucket's
     * internal spinlock and empty list head. This context records that
     * synchronization-object initialization scope; it does not model runtime
     * waitqueue enqueue/wakeup locking.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        BitWaitQueueTable;
        Scheduler;
    }
}

context WorkqueuePoolMutexContext: ResourceExclusiveContext {
    /*
     * workqueue_init_early() reaches alloc_workqueue(), which takes
     * wq_pool_mutex while allocating/linking PWQs and adding each system
     * workqueue to the global workqueues list. The guard is still modeled even
     * though boot concurrency is otherwise closed, because this mutex owns the
     * Linux workqueue/pool publication protocol.
     */
    guard {
        lock_ref: WorkqueuePoolMutex;

        entered_by {
            WorkqueuePoolMutex.Transition::Lock(BootInitTaskRef);
        }

        exited_by {
            WorkqueuePoolMutex.Transition::Unlock(BootInitTaskRef);
        }
    }

    obj_refs {
        Workqueue;
        WorkqueuePoolMutex;
    }
}

context WorkqueueStructMutexContext: ResourceExclusiveContext {
    /*
     * alloc_workqueue() initializes each workqueue_struct mutex and takes it
     * while linking pool_workqueue entries and adjusting max_active. The
     * current model aggregates the system workqueue set behind one
     * WorkqueueStructMutex fact rather than naming every system queue mutex.
     */
    guard {
        lock_ref: WorkqueueStructMutex;

        entered_by {
            WorkqueueStructMutex.Transition::Lock(BootInitTaskRef);
        }

        exited_by {
            WorkqueueStructMutex.Transition::Unlock(BootInitTaskRef);
        }
    }

    obj_refs {
        Workqueue;
        WorkqueueStructMutex;
    }
}

/*
 * BootInitPreemption 表示 rq_attach_root() 调用点仍在 BootInitTask/current
 * 身份下执行时的 preemption control 视图。它只用于
 * RunQueueRootAttachContext 的 rq_lock_irqsave() 协议；不能与
 * init_idle_preempt_count() 建立的 BootIdlePreemption 混用。
 */
object BootInitPreemption: PreemptionControl {
    initial_state: State::Base;
    parent: BootInitTask;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    task_preemption_control_ready(BootInitTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootInitTask);
                    task_preemption_disabled(BootInitTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            task_preemption_control_ready(BootInitTask);
            task_preempt_count_initialized_to_init_preempt_count(BootInitTask);
            task_preemption_disabled(BootInitTask);
        }
    }
}

/*
 * Scheduler 表示启动期调度器基础编排对象。本阶段只要求 boot CPU 的
 * CPU-owned runqueue、CPU-owned idle task 关联和主动调度入口可用；同时记录
 * Linux for_each_possible_cpu() 已基于 CpuGroup possible 集合初始化 runqueue
 * 元数据，并把这些 runqueue attach 到 DefaultSchedRootDomain。完整 SMP 调度
 * 拓扑留给后续阶段。
 */
object Scheduler: SchedulerObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    MmCoreInitPhase.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    StaticBranch.state == State::Ready;
                }

                drives {
                    DefaultSchedRootDomain.Transition::Setup;
                    BitWaitQueueTable.Transition::Preset;
                    BootIdleRcuReadSide.Transition::Preset;
                }

                ensures {
                    scheduler_preset_ready(Scheduler);
                    sched_class_skeletons_deferred(Scheduler);
                    scheduler_default_root_domain_ready(Scheduler, DefaultSchedRootDomain);
                    default_sched_root_domain_covers_cpu_group_possible(
                        DefaultSchedRootDomain,
                        CpuGroup
                    );
                    bit_wait_queue_table_ready(BitWaitQueueTable);
                    bit_wait_queue_table_bucket_count_matches_wait_table_size(
                        BitWaitQueueTable
                    );
                    bit_wait_queue_table_bucket_waitqueues_ready(BitWaitQueueTable);
                    bit_wait_queue_table_bucket_locks_ready(BitWaitQueueTable);
                    bit_wait_queue_table_bucket_lists_empty(BitWaitQueueTable);
                    rcu_read_side_ready(BootIdleRcuReadSide);
                    rcu_read_side_incomplete_first_slice(BootIdleRcuReadSide);
                    rcu_read_side_full_semantics_deferred(BootIdleRcuReadSide);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            DefaultSchedRootDomain.state == State::Ready;
            BitWaitQueueTable.state == State::Prepared;
            BootIdleRcuReadSide.state == State::Prepared;
            scheduler_preset_ready(Scheduler);
            scheduler_default_root_domain_ready(Scheduler, DefaultSchedRootDomain);
            bit_wait_queue_table_ready(BitWaitQueueTable);
            bit_wait_queue_table_bucket_count_matches_wait_table_size(
                BitWaitQueueTable
            );
            bit_wait_queue_table_bucket_waitqueues_ready(BitWaitQueueTable);
            bit_wait_queue_table_bucket_locks_ready(BitWaitQueueTable);
            bit_wait_queue_table_bucket_lists_empty(BitWaitQueueTable);
            default_sched_root_domain_covers_cpu_group_possible(
                DefaultSchedRootDomain,
                CpuGroup
            );
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    BootInitTask.state == State::Online;
                    InitMM.state == State::Ready;
                    BootIdleRcuReadSide.state == State::Prepared;
                }

                drives {
                    BootRunQueue.Transition::Setup;
                    BootIdleTask.Transition::Setup;
                }

                ensures {
                    scheduler_runqueues_ready(Scheduler, CpuGroup);
                    scheduler_possible_cpu_runqueues_ready(Scheduler, CpuGroup);
                    scheduler_orchestrates_cpu_owned_runqueues(Scheduler, CpuGroup);
                    cpu_owns_runqueue(BootCPU, BootRunQueue);
                    cpu_owns_idle_task(BootCPU, BootIdleTask);
                    cpu_runqueue_idle_is_cpu_idle_task(
                        BootCPU,
                        BootRunQueue,
                        BootIdleTask
                    );
                    task_preemption_control_ready(BootInitTask);
                    task_preemption_disabled(BootInitTask);
                    boot_runqueue_ready(BootRunQueue, BootCPU);
                    runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(BootRunQueueRef);
                    runqueue_ref_cpu_is(BootRunQueueRef, BootCPURef);
                    runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(CurrentRunQueueRef);
                    runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
                    current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
                    current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
                    boot_idle_task_ready(BootIdleTask, BootInitTask, BootRunQueue);
                    current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
                    task_cpu_ref_is(BootIdleTask, BootCPURef);
                    scheduler_possible_cpu_runqueues_attached_to_default_root_domain(
                        Scheduler,
                        CpuGroup,
                        DefaultSchedRootDomain
                    );
                    scheduler_schedule_event_available(Scheduler);
                    rcu_read_side_ready(BootIdleRcuReadSide);
                    rcu_read_side_incomplete_first_slice(BootIdleRcuReadSide);
                    rcu_read_side_full_semantics_deferred(BootIdleRcuReadSide);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootRunQueue.state == State::Ready;
            BootInitPreemption.state == State::Ready;
            BootIdleTask.state == State::Ready;
            scheduler_runqueues_ready(Scheduler, CpuGroup);
            scheduler_possible_cpu_runqueues_ready(Scheduler, CpuGroup);
            scheduler_orchestrates_cpu_owned_runqueues(Scheduler, CpuGroup);
            cpu_owns_runqueue(BootCPU, BootRunQueue);
            cpu_owns_idle_task(BootCPU, BootIdleTask);
            cpu_runqueue_idle_is_cpu_idle_task(BootCPU, BootRunQueue, BootIdleTask);
            task_preemption_control_ready(BootInitTask);
            task_preemption_disabled(BootInitTask);
            scheduler_possible_cpu_runqueues_attached_to_default_root_domain(
                Scheduler,
                CpuGroup,
                DefaultSchedRootDomain
            );
            runqueue_ref_ready(BootRunQueueRef);
            runqueue_ref_cpu_is(BootRunQueueRef, BootCPURef);
            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
            runqueue_ref_ready(CurrentRunQueueRef);
            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
            current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
            current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
            current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
            boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
            task_cpu_ref_is(BootIdleTask, BootCPURef);
            scheduler_schedule_event_available(Scheduler);
            rcu_read_side_ready(BootIdleRcuReadSide);
            rcu_read_side_incomplete_first_slice(BootIdleRcuReadSide);
            rcu_read_side_full_semantics_deferred(BootIdleRcuReadSide);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    scheduler_running_flag_set(Scheduler);
                    scheduler_schedule_event_available(Scheduler);
                }
            }
        }
    }

    state State::Online {
        invariant {
            scheduler_running_flag_set(Scheduler);
            BootRunQueue.state == State::Ready;
            BootInitPreemption.state == State::Ready;
            BootIdleTask.state == State::Ready;
            scheduler_orchestrates_cpu_owned_runqueues(Scheduler, CpuGroup);
            cpu_owns_runqueue(BootCPU, BootRunQueue);
            cpu_owns_idle_task(BootCPU, BootIdleTask);
            cpu_runqueue_idle_is_cpu_idle_task(BootCPU, BootRunQueue, BootIdleTask);
            task_preemption_control_ready(BootInitTask);
            task_preemption_disabled(BootInitTask);
            runqueue_ref_ready(BootRunQueueRef);
            runqueue_ref_cpu_is(BootRunQueueRef, BootCPURef);
            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
            runqueue_ref_ready(CurrentRunQueueRef);
            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
            current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
            current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
            task_cpu_ref_is(BootIdleTask, BootCPURef);
            scheduler_schedule_event_available(Scheduler);
            rcu_read_side_ready(BootIdleRcuReadSide);
            rcu_read_side_incomplete_first_slice(BootIdleRcuReadSide);
            rcu_read_side_full_semantics_deferred(BootIdleRcuReadSide);
        }
    }
}

/*
 * DefaultSchedRootDomain 表示 sched_init() 中建立的默认 root domain。它是
 * Scheduler 的调度覆盖视图，不拥有 CPU 本体；其 covered_cpus 由
 * CpuGroup.possible_cpus 的 CpuRef 集合建立。
 */
object DefaultSchedRootDomain: TaskObject {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    default_sched_root_domain_ready(DefaultSchedRootDomain, CpuGroup);
                    default_sched_root_domain_covers_cpu_group_possible(
                        DefaultSchedRootDomain,
                        CpuGroup
                    );
                    default_sched_root_domain_covered_cpus_are_cpu_refs(
                        DefaultSchedRootDomain,
                        CpuGroup
                    );
                    default_sched_root_domain_does_not_own_cpu_bodies(
                        DefaultSchedRootDomain
                    );
                    cpu_group_possible_contains(CpuGroup, BootCPURef);
                    default_sched_root_domain_covers_cpu_ref(
                        DefaultSchedRootDomain,
                        BootCPURef
                    );
                    sched_smp_topology_deferred(DefaultSchedRootDomain);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            default_sched_root_domain_ready(DefaultSchedRootDomain, CpuGroup);
            default_sched_root_domain_covers_cpu_group_possible(
                DefaultSchedRootDomain,
                CpuGroup
            );
            default_sched_root_domain_covered_cpus_are_cpu_refs(
                DefaultSchedRootDomain,
                CpuGroup
            );
            default_sched_root_domain_does_not_own_cpu_bodies(DefaultSchedRootDomain);
            default_sched_root_domain_covers_cpu_ref(
                DefaultSchedRootDomain,
                BootCPURef
            );
            sched_smp_topology_deferred(DefaultSchedRootDomain);
        }
    }
}

/*
 * BitWaitQueueTable 表示 wait_bit_init() 建立的 bit wait 全局 bucket 表。
 * 每个 bucket 是 wait_queue_head_t；wait_bit_init() 对所有 bucket 调用
 * init_waitqueue_head()，因此本阶段必须记录每个 bucket 的内部 spinlock
 * 已初始化、wait list 为空，以及 bucket 数量匹配 Linux WAIT_TABLE_SIZE。
 */
object BitWaitQueueTable: TaskObject {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                within BitWaitQueueTableInitContext {
                    ensures {
                        bit_wait_queue_table_bucket_count_matches_wait_table_size(
                            BitWaitQueueTable
                        );
                        bit_wait_queue_table_bucket_waitqueues_ready(BitWaitQueueTable);
                        bit_wait_queue_table_bucket_locks_ready(BitWaitQueueTable);
                        bit_wait_queue_table_bucket_lists_empty(BitWaitQueueTable);
                    }
                }

                ensures {
                    bit_wait_queue_table_ready(BitWaitQueueTable);
                    bit_wait_queue_table_bucket_count_matches_wait_table_size(
                        BitWaitQueueTable
                    );
                    bit_wait_queue_table_bucket_waitqueues_ready(BitWaitQueueTable);
                    bit_wait_queue_table_bucket_locks_ready(BitWaitQueueTable);
                    bit_wait_queue_table_bucket_lists_empty(BitWaitQueueTable);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            bit_wait_queue_table_ready(BitWaitQueueTable);
            bit_wait_queue_table_bucket_count_matches_wait_table_size(
                BitWaitQueueTable
            );
            bit_wait_queue_table_bucket_waitqueues_ready(BitWaitQueueTable);
            bit_wait_queue_table_bucket_locks_ready(BitWaitQueueTable);
            bit_wait_queue_table_bucket_lists_empty(BitWaitQueueTable);
        }
    }
}

/*
 * BootIdleRcuReadSide is the named SchedInitPhase instance of the common
 * RcuReadSide type. It is intentionally incomplete: this object only models
 * the rcu_read_lock()/rcu_read_unlock() guard around init_idle()'s
 * __set_task_cpu() call. Full RCU reader nesting, preemptible-RCU accounting,
 * quiescent-state reporting, lockdep/debug checks, and scheduler/RCU context
 * switch integration remain deferred to the RcuCore/runtime RCU model.
 */
object BootIdleRcuReadSide: RcuReadSide {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    rcu_read_side_ready(BootIdleRcuReadSide);
                    rcu_read_side_incomplete_first_slice(BootIdleRcuReadSide);
                    rcu_read_side_full_semantics_deferred(BootIdleRcuReadSide);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            rcu_read_side_ready(BootIdleRcuReadSide);
            rcu_read_side_incomplete_first_slice(BootIdleRcuReadSide);
            rcu_read_side_full_semantics_deferred(BootIdleRcuReadSide);
        }
    }
}

/*
 * BootRunQueue 表示 BootCPU.RunQueue 的物化实例。Linux 同时在
 * for_each_possible_cpu() 中初始化所有 possible CPU 的 rq；当前模型用
 * Scheduler/CpuGroup 上的聚合事实表达全 possible 集合，用 BootRunQueue
 * 继续承载 boot CPU 的可直接观测 rq。Scheduler 只编排 setup，不拥有
 * 该 runqueue 本体。
 */
object BootRunQueue: RunQueue {
    initial_state: State::Base;
    parent: BootCPU;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    DefaultSchedRootDomain.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    BootInitTask.state == State::Online;
                }

                drives {
                    BootInitPreemption.Transition::Setup;
                }

                within RunQueueRootAttachContext {
                    ensures {
                        raw_spinlock_irqsave_entered(BootRunQueueLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(BootRunQueueLock, BootCurrentCPU);
                        boot_runqueue_attached_to_root_domain(
                            BootRunQueue,
                            DefaultSchedRootDomain
                        );
                        boot_runqueue_root_attach_held_runqueue_lock(
                            BootRunQueue,
                            BootRunQueueLock
                        );
                    }
                }

                ensures {
                    cpu_owns_runqueue(BootCPU, BootRunQueue);
                    boot_runqueue_ready(BootRunQueue, BootCPU);
                    raw_spinlock_initialized(BootRunQueueLock);
                    raw_spinlock_ready(BootRunQueueLock);
                    boot_runqueue_lock_ready(BootRunQueue, BootRunQueueLock);
                    boot_runqueue_possible_cpu_set_covered_by_cpu_group(BootRunQueue, CpuGroup);
                    cpu_group_possible_contains(CpuGroup, BootCPURef);
                    default_sched_root_domain_covers_cpu_ref(
                        DefaultSchedRootDomain,
                        BootCPURef
                    );
                    boot_runqueue_cpu_ref_covered_by_root_domain(
                        BootRunQueue,
                        DefaultSchedRootDomain,
                        BootCPURef
                    );
                    runqueue_runtime_state_is(BootRunQueue, RunQueueRuntimeState::None);
                    runqueue_task_refs_empty(BootRunQueue);
                    runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(BootRunQueueRef);
                    runqueue_ref_cpu_is(BootRunQueueRef, BootCPURef);
                    runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(CurrentRunQueueRef);
                    runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
                    boot_runqueue_attached_to_root_domain(BootRunQueue, DefaultSchedRootDomain);
                    boot_runqueue_root_attach_held_runqueue_lock(
                        BootRunQueue,
                        BootRunQueueLock
                    );
                    boot_runqueue_class_queues_ready(BootRunQueue);
                    boot_runqueue_balance_push_disabled(BootRunQueue);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_owns_runqueue(BootCPU, BootRunQueue);
            boot_runqueue_ready(BootRunQueue, BootCPU);
            raw_spinlock_ready(BootRunQueueLock);
            boot_runqueue_lock_ready(BootRunQueue, BootRunQueueLock);
            boot_runqueue_possible_cpu_set_covered_by_cpu_group(BootRunQueue, CpuGroup);
            cpu_group_possible_contains(CpuGroup, BootCPURef);
            default_sched_root_domain_covers_cpu_ref(DefaultSchedRootDomain, BootCPURef);
            boot_runqueue_cpu_ref_covered_by_root_domain(
                BootRunQueue,
                DefaultSchedRootDomain,
                BootCPURef
            );
            runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
            runqueue_ref_ready(BootRunQueueRef);
            runqueue_ref_cpu_is(BootRunQueueRef, BootCPURef);
            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
            runqueue_ref_ready(CurrentRunQueueRef);
            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
            boot_runqueue_attached_to_root_domain(BootRunQueue, DefaultSchedRootDomain);
            boot_runqueue_root_attach_held_runqueue_lock(BootRunQueue, BootRunQueueLock);
            boot_runqueue_class_queues_ready(BootRunQueue);
        }
    }
}

/*
 * BootIdleTask 表示 BootCPU.IdleTask 的物化实例，也就是启动线程在
 * sched_init() 中转换出的 boot CPU idle task 规格身份。它不创建新 task，
 * 而是复用当前 BootInitTask/current；BootCPU.RunQueue.idle 指向它。
 */
object BootIdleTask: Task {
    initial_state: State::Base;
    parent: BootCPU;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    BootRunQueue.state == State::Ready;
                    BootCpuCurrentTask.state == State::Ready;
                    BootInitTask.state == State::Online;
                    InitMM.state == State::Ready;
                    raw_spinlock_ready(BootRunQueueLock);
                }

                drives {
                    BootIdlePreemption.Transition::Setup;
                }

                within BootIdlePiLockContext {
                    within BootRunQueueLockContext {
                        depends_on {
                            BootRunQueue.state == State::Ready;
                            raw_spinlock_ready(BootRunQueueLock);
                        }

                        drives {
                            BootCpuCurrentTask.Action::SetCurrent(task: BootIdleTask);
                        }

                        within BootIdleRcuReadSideContext {
                            drives {
                                BootIdleTask.Action::SetTaskCpu(BootCPURef);
                            }

                            ensures {
                                rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                                rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                                boot_idle_task_cpu_set_under_rcu_read(
                                    BootIdleTask,
                                    BootCPURef
                                );
                            }
                        }

                        ensures {
                            raw_spinlock_irqsave_entered(BootIdlePiLock, BootCurrentCPU);
                            raw_spinlock_irqrestore_exited(BootIdlePiLock, BootCurrentCPU);
                            raw_spinlock_initialized(BootIdlePiLock);
                            raw_spinlock_ready(BootIdlePiLock);
                            boot_idle_pi_lock_ready(BootIdleTask, BootIdlePiLock);
                            boot_idle_init_held_pi_lock(BootIdleTask, BootIdlePiLock);
                            raw_spinlock_acquired(BootRunQueueLock);
                            raw_spinlock_released(BootRunQueueLock);
                            boot_idle_init_held_runqueue_lock(BootRunQueue, BootRunQueueLock);
                            rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                            rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                            boot_idle_task_cpu_set_under_rcu_read(BootIdleTask, BootCPURef);
                            boot_runqueue_current_published_with_rcu(BootRunQueue, BootIdleTask);
                        }
                    }
                }

                ensures {
                    cpu_owns_idle_task(BootCPU, BootIdleTask);
                    cpu_runqueue_idle_is_cpu_idle_task(
                        BootCPU,
                        BootRunQueue,
                        BootIdleTask
                    );
                    boot_idle_task_ready(BootIdleTask, BootInitTask, BootRunQueue);
                    boot_idle_task_reuses_current_init_task(BootIdleTask, BootInitTask);
                    boot_idle_task_uses_init_mm_lazy_tlb(BootIdleTask, InitMM);
                    task_ref_targets(BootIdleTaskRef, BootIdleTask);
                    task_ref_ready(BootIdleTaskRef);
                    task_ref_targets(CurrentTaskRef, BootIdleTask);
                    task_ref_ready(CurrentTaskRef);
                    current_task_ref_private_to_cpu(CurrentTaskRef, BootCurrentCPU);
                    current_task_ref_targets_cpu_task(CurrentTaskRef, BootCurrentCPU, BootIdleTask);
                    current_task_ref_from_cpu_view(CurrentTaskRef, BootCurrentCPU, BootIdleTask);
                    current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
                    current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
                    task_thread_context_owned(BootIdleTask, BootIdleTask.thread_context);
                    task_thread_context_core_register_set(BootIdleTask.thread_context);
                    task_preemption_control_ready(BootIdleTask);
                    raw_spinlock_initialized(BootIdlePiLock);
                    raw_spinlock_ready(BootIdlePiLock);
                    boot_idle_pi_lock_ready(BootIdleTask, BootIdlePiLock);
                    boot_idle_init_held_pi_lock(BootIdleTask, BootIdlePiLock);
                    boot_idle_init_held_runqueue_lock(BootRunQueue, BootRunQueueLock);
                    boot_idle_task_cpu_set_under_rcu_read(BootIdleTask, BootCPURef);
                    boot_runqueue_current_published_with_rcu(BootRunQueue, BootIdleTask);
                    current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
                    task_cpu_ref_is(BootIdleTask, BootCPURef);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_owns_idle_task(BootCPU, BootIdleTask);
            cpu_runqueue_idle_is_cpu_idle_task(BootCPU, BootRunQueue, BootIdleTask);
            boot_idle_task_ready(BootIdleTask, BootInitTask, BootRunQueue);
            boot_idle_task_reuses_current_init_task(BootIdleTask, BootInitTask);
            task_ref_targets(BootIdleTaskRef, BootIdleTask);
            task_ref_ready(BootIdleTaskRef);
            task_ref_targets(CurrentTaskRef, BootIdleTask);
            task_ref_ready(CurrentTaskRef);
            current_task_ref_private_to_cpu(CurrentTaskRef, BootCurrentCPU);
            current_task_ref_targets_cpu_task(CurrentTaskRef, BootCurrentCPU, BootIdleTask);
            current_task_ref_from_cpu_view(CurrentTaskRef, BootCurrentCPU, BootIdleTask);
            current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
            current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
            task_thread_context_owned(BootIdleTask, BootIdleTask.thread_context);
            task_thread_context_core_register_set(BootIdleTask.thread_context);
            task_preemption_control_ready(BootIdleTask);
            raw_spinlock_ready(BootIdlePiLock);
            boot_idle_pi_lock_ready(BootIdleTask, BootIdlePiLock);
            boot_idle_task_cpu_set_under_rcu_read(BootIdleTask, BootCPURef);
            boot_runqueue_current_published_with_rcu(BootRunQueue, BootIdleTask);
            current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
            boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
            task_cpu_ref_is(BootIdleTask, BootCPURef);
        }
    }
}

/*
 * BootIdlePreemption 表示 boot idle/current task 的 preempt_count 控制视图。
 * raw spinlock 等运行期路径通过 current task slot 找到当前任务，再作用于该
 * 任务的抢占控制。
 */
object BootIdlePreemption: PreemptionControl {
    initial_state: State::Base;
    parent: BootIdleTask;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    task_preemption_control_ready(BootIdleTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootIdleTask);
                    task_preemption_disabled(BootIdleTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            task_preemption_control_ready(BootIdleTask);
            task_preempt_count_initialized_to_init_preempt_count(BootIdleTask);
        }
    }
}

/*
 * RadixTree 表示 radix_tree_init() 建立的全局 radix tree node 分配基础。
 */
object RadixTree: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    SlubCacheRegistry.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                }

                ensures {
                    radix_tree_node_cache_ready(RadixTree, SlubSubsystem);
                    radix_tree_node_cache_registered_in_slub_registry(
                        RadixTree,
                        SlubCacheRegistry
                    );
                    radix_tree_cpuhp_dead_step_registered(RadixTree, CpuHotplugState);
                    radix_tree_node_api_ready(RadixTree);
                    radix_tree_node_rcu_free_callback_deferred(RadixTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            radix_tree_node_cache_ready(RadixTree, SlubSubsystem);
            radix_tree_node_cache_registered_in_slub_registry(RadixTree, SlubCacheRegistry);
            radix_tree_cpuhp_dead_step_registered(RadixTree, CpuHotplugState);
            radix_tree_node_api_ready(RadixTree);
            radix_tree_node_rcu_free_callback_deferred(RadixTree);
        }
    }
}

/*
 * MapleTree 表示 maple_tree_init() 建立的全局 maple node 分配基础。
 */
object MapleTree: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    SlubCacheRegistry.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    maple_tree_node_cache_ready(MapleTree, SlubSubsystem);
                    maple_tree_node_cache_registered_in_slub_registry(
                        MapleTree,
                        SlubCacheRegistry
                    );
                    maple_tree_node_api_ready(MapleTree);
                    maple_tree_node_rcu_free_callback_deferred(MapleTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            maple_tree_node_cache_ready(MapleTree, SlubSubsystem);
            maple_tree_node_cache_registered_in_slub_registry(MapleTree, SlubCacheRegistry);
            maple_tree_node_api_ready(MapleTree);
            maple_tree_node_rcu_free_callback_deferred(MapleTree);
        }
    }
}

/*
 * WorkqueuePoolMutex 表示 Linux workqueue.c 的静态 wq_pool_mutex。
 * workqueue_init_early() 通过 alloc_workqueue() 持有它来保护 worker
 * pools、PWQ 分配和全局 workqueues list 发布。
 */
object WorkqueuePoolMutex: Mutex {
    initial_state: State::Base;
    parent: Workqueue;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    mutex_storage_bound(WorkqueuePoolMutex);
                    mutex_init_kind_recorded(WorkqueuePoolMutex);
                    mutex_preset_respects_init_kind(WorkqueuePoolMutex);
                    mutex_owns_wait_queue(WorkqueuePoolMutex);
                    mutex_wait_lock_internal_deferred(WorkqueuePoolMutex);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    mutex_initialized(WorkqueuePoolMutex);
                    mutex_ready(WorkqueuePoolMutex);
                    mutex_unlocked(WorkqueuePoolMutex);
                    mutex_wait_queue_ready(WorkqueuePoolMutex);
                    mutex_recursive_locking_forbidden(WorkqueuePoolMutex);
                    mutex_unlock_requires_owner(WorkqueuePoolMutex);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mutex_ready(WorkqueuePoolMutex);
            mutex_unlocked(WorkqueuePoolMutex);
            mutex_wait_queue_ready(WorkqueuePoolMutex);
        }
    }
}

/*
 * WorkqueueStructMutex aggregates the per-workqueue workqueue_struct->mutex
 * instances used while linking PWQs and adjusting max_active during
 * alloc_workqueue(). The current model treats the early system workqueue set
 * as one aggregate object; it does not yet name each system queue separately.
 */
object WorkqueueStructMutex: Mutex {
    initial_state: State::Base;
    parent: Workqueue;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    mutex_storage_bound(WorkqueueStructMutex);
                    mutex_init_kind_recorded(WorkqueueStructMutex);
                    mutex_preset_respects_init_kind(WorkqueueStructMutex);
                    mutex_owns_wait_queue(WorkqueueStructMutex);
                    mutex_wait_lock_internal_deferred(WorkqueueStructMutex);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    mutex_initialized(WorkqueueStructMutex);
                    mutex_ready(WorkqueueStructMutex);
                    mutex_unlocked(WorkqueueStructMutex);
                    mutex_wait_queue_ready(WorkqueueStructMutex);
                    mutex_recursive_locking_forbidden(WorkqueueStructMutex);
                    mutex_unlock_requires_owner(WorkqueueStructMutex);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mutex_ready(WorkqueueStructMutex);
            mutex_unlocked(WorkqueueStructMutex);
            mutex_wait_queue_ready(WorkqueueStructMutex);
        }
    }
}

/*
 * Workqueue 表示 workqueue_init_early() 建立的 early workqueue 框架。
 * Prepared 只表示可创建 workqueue 和排队/取消 work item，不表示 worker
 * kthread 已经执行。
 */
object Workqueue: TaskObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PageAllocator.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    SlubCacheRegistry.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    BootInitTask.state == State::Online;
                    task_ref_ready(BootInitTaskRef);
                }

                drives {
                    WorkqueuePoolMutex.Transition::Preset;
                    WorkqueuePoolMutex.Transition::Setup;
                    WorkqueueStructMutex.Transition::Preset;
                    WorkqueueStructMutex.Transition::Setup;
                }

                within WorkqueuePoolMutexContext {
                    ensures {
                        workqueue_pool_mutex_guard_used(
                            Workqueue,
                            WorkqueuePoolMutex
                        );
                        workqueue_system_queue_count_matches_linux_early(Workqueue);
                        workqueue_pool_workqueue_cache_ready(Workqueue);
                        pool_workqueue_cache_registered_in_slub_registry(
                            Workqueue,
                            SlubCacheRegistry
                        );
                        workqueue_cpu_worker_pools_ready(Workqueue);
                        workqueue_bh_pools_ready(Workqueue);
                        workqueue_attrs_ready(Workqueue);
                        workqueue_system_affinity_pods_ready(Workqueue);
                    }

                    within WorkqueueStructMutexContext {
                        ensures {
                            workqueue_struct_mutex_guard_used(
                                Workqueue,
                                WorkqueueStructMutex
                            );
                            workqueue_system_queues_ready(Workqueue);
                            workqueue_worker_pools_prepared(Workqueue);
                        }
                    }
                }

                ensures {
                    mutex_storage_bound(WorkqueuePoolMutex);
                    mutex_init_kind_recorded(WorkqueuePoolMutex);
                    mutex_preset_respects_init_kind(WorkqueuePoolMutex);
                    mutex_owns_wait_queue(WorkqueuePoolMutex);
                    mutex_wait_lock_internal_deferred(WorkqueuePoolMutex);
                    mutex_initialized(WorkqueuePoolMutex);
                    mutex_ready(WorkqueuePoolMutex);
                    mutex_unlocked(WorkqueuePoolMutex);
                    mutex_wait_queue_ready(WorkqueuePoolMutex);
                    mutex_storage_bound(WorkqueueStructMutex);
                    mutex_init_kind_recorded(WorkqueueStructMutex);
                    mutex_preset_respects_init_kind(WorkqueueStructMutex);
                    mutex_owns_wait_queue(WorkqueueStructMutex);
                    mutex_wait_lock_internal_deferred(WorkqueueStructMutex);
                    mutex_initialized(WorkqueueStructMutex);
                    mutex_ready(WorkqueueStructMutex);
                    mutex_unlocked(WorkqueueStructMutex);
                    mutex_wait_queue_ready(WorkqueueStructMutex);
                    workqueue_pool_mutex_ready(Workqueue, WorkqueuePoolMutex);
                    workqueue_struct_mutex_ready(Workqueue, WorkqueueStructMutex);
                    workqueue_pool_mutex_guard_used(
                        Workqueue,
                        WorkqueuePoolMutex
                    );
                    workqueue_struct_mutex_guard_used(
                        Workqueue,
                        WorkqueueStructMutex
                    );
                    workqueue_early_framework_ready(Workqueue, CpuGroup);
                    workqueue_system_queues_ready(Workqueue);
                    workqueue_system_queue_count_matches_linux_early(Workqueue);
                    workqueue_worker_pools_prepared(Workqueue);
                    workqueue_cpu_worker_pools_ready(Workqueue);
                    workqueue_bh_pools_ready(Workqueue);
                    workqueue_pool_workqueue_cache_ready(Workqueue);
                    pool_workqueue_cache_registered_in_slub_registry(
                        Workqueue,
                        SlubCacheRegistry
                    );
                    workqueue_unbound_cpumask_ready(Workqueue);
                    workqueue_attrs_ready(Workqueue);
                    workqueue_system_affinity_pods_ready(Workqueue);
                    workqueue_pool_attach_mutex_deferred(Workqueue);
                    workqueue_mayday_lock_deferred(Workqueue);
                    workqueue_manager_wait_deferred(Workqueue);
                    workqueue_workers_not_running(Workqueue);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            workqueue_early_framework_ready(Workqueue, CpuGroup);
            workqueue_system_queues_ready(Workqueue);
            workqueue_system_queue_count_matches_linux_early(Workqueue);
            workqueue_worker_pools_prepared(Workqueue);
            workqueue_cpu_worker_pools_ready(Workqueue);
            workqueue_bh_pools_ready(Workqueue);
            workqueue_pool_workqueue_cache_ready(Workqueue);
            pool_workqueue_cache_registered_in_slub_registry(
                Workqueue,
                SlubCacheRegistry
            );
            workqueue_unbound_cpumask_ready(Workqueue);
            workqueue_attrs_ready(Workqueue);
            workqueue_system_affinity_pods_ready(Workqueue);
            workqueue_pool_mutex_ready(Workqueue, WorkqueuePoolMutex);
            workqueue_struct_mutex_ready(Workqueue, WorkqueueStructMutex);
            workqueue_pool_mutex_guard_used(Workqueue, WorkqueuePoolMutex);
            workqueue_struct_mutex_guard_used(Workqueue, WorkqueueStructMutex);
            workqueue_pool_attach_mutex_deferred(Workqueue);
            workqueue_mayday_lock_deferred(Workqueue);
            workqueue_manager_wait_deferred(Workqueue);
            workqueue_workers_not_running(Workqueue);
        }

        transitions {
            /*
             * Setup 对应 PreSmpInitPhase 中 workqueue_init()。它创建
             * rescuer 和初始 worker 壳，打开 worker 创建属性，但仍不表示
             * SMP topology 感知和完整 worker 运行期服务已经 online。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    KthreaddTask.state == State::Online;
                    PageAllocator.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    workqueue_ready_before_smp(Workqueue);
                    workqueue_rescuers_ready(Workqueue, KthreaddTask);
                    workqueue_initial_workers_created(Workqueue, CpuGroup);
                    workqueue_worker_creation_open(Workqueue);
                    workqueue_watchdog_ready(Workqueue);
                    workqueue_smp_topology_deferred(Workqueue);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            workqueue_ready_before_smp(Workqueue);
            workqueue_worker_creation_open(Workqueue);
            workqueue_smp_topology_deferred(Workqueue);
        }
    }
}

/*
 * Softirq 在本阶段只建立 action table 和 pending bit 承载壳，供 RCU
 * 注册 RCU_SOFTIRQ；tasklet 队列和实际执行路径留给下一子阶段。
 */
object Softirq: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    softirq_action_table_ready(Softirq);
                    softirq_slots_ready(Softirq);
                    softirq_pending_set_ready(Softirq, PerCpuStorage);
                    softirq_execution_closed(Softirq);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            softirq_action_table_ready(Softirq);
            softirq_pending_set_ready(Softirq, PerCpuStorage);
            softirq_execution_closed(Softirq);
        }

        transitions {
            /*
             * Setup 对应 softirq_init()。TimerWheel/HrtimerCore 已经在本阶段
             * 前半段注册 TIMER_SOFTIRQ/HRTIMER_SOFTIRQ；这里补齐 tasklet 队列
             * 与 TASKLET/HI softirq action。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    TimerWheel.state == State::Ready;
                    HrtimerCore.state == State::Ready;
                }

                ensures {
                    softirq_action_table_ready(Softirq);
                    softirq_pending_set_ready(Softirq, PerCpuStorage);
                    softirq_tasklet_queues_ready(Softirq, PerCpuStorage);
                    softirq_tasklet_actions_registered(Softirq);
                    softirq_timer_actions_registered(Softirq, TimerWheel, HrtimerCore);
                    softirq_execution_closed(Softirq);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            softirq_action_table_ready(Softirq);
            softirq_pending_set_ready(Softirq, PerCpuStorage);
            softirq_tasklet_queues_ready(Softirq, PerCpuStorage);
            softirq_tasklet_actions_registered(Softirq);
            softirq_timer_actions_registered(Softirq, TimerWheel, HrtimerCore);
            softirq_execution_closed(Softirq);
        }
    }
}

/*
 * RcuCore 表示 rcu_init() 建立的 TREE_RCU/PREEMPT_RCU/Tasks RCU 共同
 * 启动基础。运行期 GP kthread 与完全 online 语义留给后续阶段。
 */
object RcuCore: TaskObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Prepared;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    TasksRcu.Transition::Preset;
                }

                ensures {
                    rcu_core_ready(RcuCore, CpuGroup);
                    rcu_boot_cpu_online_ready(RcuCore, BootCPU);
                    rcu_softirq_registered(RcuCore, Softirq);
                    rcu_workqueues_ready(RcuCore, Workqueue);
                    tasks_rcu_prepared(TasksRcu);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            TasksRcu.state == State::Prepared;
            rcu_core_ready(RcuCore, CpuGroup);
            rcu_boot_cpu_online_ready(RcuCore, BootCPU);
            rcu_softirq_registered(RcuCore, Softirq);
            rcu_workqueues_ready(RcuCore, Workqueue);
        }
    }
}

/*
 * TasksRcu 表示 rcu_init() 内 tasks_cblist_init_generic() 建立的 Tasks
 * RCU callback-list 壳。GP kthread 创建留给后续 rcu_init_tasks_generic()。
 */
object TasksRcu: TaskObject {
    initial_state: State::Base;
    parent: RcuCore;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    tasks_rcu_callback_lists_ready(TasksRcu, PerCpuStorage);
                    tasks_rcu_enabled_flavors_recorded(TasksRcu);
                    tasks_rcu_gp_threads_deferred(TasksRcu);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            tasks_rcu_callback_lists_ready(TasksRcu, PerCpuStorage);
            tasks_rcu_enabled_flavors_recorded(TasksRcu);
        }

        transitions {
            /*
             * Setup 对应 PreSmpInitPhase 中 rcu_init_tasks_generic()。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    tasks_rcu_ready(TasksRcu);
                    tasks_rcu_gp_threads_created(TasksRcu, KthreaddTask);
                    tasks_rcu_enabled_flavors_recorded(TasksRcu);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tasks_rcu_ready(TasksRcu);
            tasks_rcu_gp_threads_created(TasksRcu, KthreaddTask);
        }
    }
}

/*
 * SchedInitPhase 表示 BootPhase 的第五个子阶段。
 */
object SchedInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    MmCoreInitPhase.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                    StaticBranch.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                }

                drives {
                    Scheduler.Transition::Preset;
                    Scheduler.Transition::Setup;
                    Scheduler.Transition::Enable;
                    RadixTree.Transition::Setup;
                    MapleTree.Transition::Setup;
                    Workqueue.Transition::Preset;
                    Softirq.Transition::Preset;
                    RcuCore.Transition::Setup;
                }

                ensures {
                    sched_init_ready(SchedInitPhase);
                    scheduler_schedule_smoke_ready(Scheduler);
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                }

                deferred {
                    "early_trace_init()/trace_init() 暂缓：Linux tracing core 与项目 checkpoint trace 的关系后续单独收敛。";
                    "housekeeping_init() 暂缓：nohz_full/isolcpus/CPU isolation 参数路径尚未进入当前 formal。";
                    "SchedClass 细分暂缓：当前只保留调度类壳和 boot CPU runqueue 语义。";
                    "Workqueue.setup()/enable() 暂缓：worker kthread 创建和执行边界属于后续多任务/SMP 路径。";
                    "TasksRcu.setup() 暂缓：GP kthread 创建留给后续 rcu_init_tasks_generic()。";
                    "context_tracking_init() 当前 CONFIG_CONTEXT_TRACKING_USER_FORCE=n，为 trimmed/no-op。";
                    "poking_init()/ftrace_init() 当前 RISC-V/default_config 下为 trimmed/no-op。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            sched_init_ready(SchedInitPhase);
            Scheduler.state == State::Online;
            BootRunQueue.state == State::Ready;
            BootIdleTask.state == State::Ready;
            RadixTree.state == State::Ready;
            MapleTree.state == State::Ready;
            Workqueue.state == State::Prepared;
            Softirq.state == State::Prepared;
            RcuCore.state == State::Ready;
            TasksRcu.state == State::Prepared;
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
        }
    }
}
