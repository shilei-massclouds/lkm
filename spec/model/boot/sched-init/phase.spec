/*
 * Sched Init Phase Specification
 *
 * This subphase starts after mm_core_init() returns and ends after the
 * context_tracking_init() call point. It builds the boot CPU scheduler
 * foundation and early asynchronous support while interrupts are still closed.
 */

/*
 * Scheduler 表示启动期调度器基础对象。本阶段只要求 boot CPU 的 runqueue、
 * idle task 关联和主动调度入口可用；完整 SMP 调度拓扑留给后续阶段。
 */
object Scheduler: SchedulerObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    MmCoreInitPhase.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    StaticBranch.state == State::Ready;
                }

                drives {
                    DefaultSchedRootDomain.Event::Setup;
                    BitWaitQueueTable.Event::Preset;
                }

                ensures {
                    scheduler_preset_ready(Scheduler);
                    sched_class_skeletons_deferred(Scheduler);
                    scheduler_default_root_domain_ready(Scheduler, DefaultSchedRootDomain);
                    bit_wait_queue_table_ready(BitWaitQueueTable);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            DefaultSchedRootDomain.state == State::Ready;
            BitWaitQueueTable.state == State::Prepared;
            scheduler_preset_ready(Scheduler);
            scheduler_default_root_domain_ready(Scheduler, DefaultSchedRootDomain);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    BootInitTask.state == State::Online;
                    InitMM.state == State::Ready;
                }

                drives {
                    BootRunQueue.Event::Setup;
                    BootIdleTask.Event::Setup;
                }

                ensures {
                    scheduler_runqueues_ready(Scheduler, CpuGroup);
                    boot_runqueue_ready(BootRunQueue, BootCPU);
                    runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(BootRunQueueRef);
                    runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(CurrentRunQueueRef);
                    runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
                    current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
                    current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
                    boot_idle_task_ready(BootIdleTask, BootInitTask, BootRunQueue);
                    current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
                    task_cpu_ref_is(BootIdleTask, BootCPURef);
                    scheduler_schedule_event_available(Scheduler);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootRunQueue.state == State::Ready;
            BootIdleTask.state == State::Ready;
            scheduler_runqueues_ready(Scheduler, CpuGroup);
            runqueue_ref_ready(BootRunQueueRef);
            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
            runqueue_ref_ready(CurrentRunQueueRef);
            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
            current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
            current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
            current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
            boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
            task_cpu_ref_is(BootIdleTask, BootCPURef);
            scheduler_schedule_event_available(Scheduler);
        }

        events {
            on Event::Enable -> State::Online {
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
            BootIdleTask.state == State::Ready;
            runqueue_ref_ready(BootRunQueueRef);
            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
            runqueue_ref_ready(CurrentRunQueueRef);
            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
            current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
            current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
            task_cpu_ref_is(BootIdleTask, BootCPURef);
            scheduler_schedule_event_available(Scheduler);
        }
    }
}

/*
 * DefaultSchedRootDomain 表示 sched_init() 中建立的默认 root domain。
 */
object DefaultSchedRootDomain: TaskObject {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    default_sched_root_domain_ready(DefaultSchedRootDomain, CpuGroup);
                    sched_smp_topology_deferred(DefaultSchedRootDomain);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            default_sched_root_domain_ready(DefaultSchedRootDomain, CpuGroup);
        }
    }
}

/*
 * BitWaitQueueTable 表示 wait_bit_init() 建立的 bit wait 全局 bucket 表。
 */
object BitWaitQueueTable: TaskObject {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                ensures {
                    bit_wait_queue_table_ready(BitWaitQueueTable);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            bit_wait_queue_table_ready(BitWaitQueueTable);
        }
    }
}

/*
 * BootRunQueue 表示 boot CPU 的 runqueue 元数据。
 */
object BootRunQueue: RunQueue {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DefaultSchedRootDomain.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    boot_runqueue_ready(BootRunQueue, BootCPU);
                    runqueue_runtime_state_is(BootRunQueue, RunQueueRuntimeState::None);
                    runqueue_task_refs_empty(BootRunQueue);
                    runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(BootRunQueueRef);
                    runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
                    runqueue_ref_ready(CurrentRunQueueRef);
                    runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
                    boot_runqueue_attached_to_root_domain(BootRunQueue, DefaultSchedRootDomain);
                    boot_runqueue_class_queues_ready(BootRunQueue);
                    boot_runqueue_balance_push_disabled(BootRunQueue);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            boot_runqueue_ready(BootRunQueue, BootCPU);
            runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
            runqueue_ref_ready(BootRunQueueRef);
            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
            runqueue_ref_ready(CurrentRunQueueRef);
            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
            boot_runqueue_attached_to_root_domain(BootRunQueue, DefaultSchedRootDomain);
            boot_runqueue_class_queues_ready(BootRunQueue);
        }
    }
}

/*
 * BootIdleTask 表示启动线程在 sched_init() 中转换出的 boot CPU idle task
 * 规格身份。它不创建新 task，而是复用当前 BootInitTask/current。
 */
object BootIdleTask: Task {
    initial_state: State::Base;
    parent: BootRunQueue;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    BootRunQueue.state == State::Ready;
                    BootCpuCurrentTask.state == State::Ready;
                    BootInitTask.state == State::Online;
                    InitMM.state == State::Ready;
                }

                drives {
                    BootIdlePreemption.Event::Setup;
                    BootCpuCurrentTask.Action::SetCurrent(task: BootIdleTask);
                    BootIdleTask.Action::SetTaskCpu(BootCPURef);
                }

                ensures {
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
                    current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    boot_cpu_current_is_idle_task(BootCPU, BootIdleTask);
                    task_cpu_ref_is(BootIdleTask, BootCPURef);
                }
            }
        }
    }

    state State::Ready {
        invariant {
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
        events {
            on Event::Setup -> State::Ready {
                ensures {
                    task_preemption_control_ready(BootIdleTask);
                    task_preemption_disabled(BootIdleTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            task_preemption_control_ready(BootIdleTask);
        }
    }
}

/*
 * RadixTree 表示 radix_tree_init() 建立的全局 radix tree node 分配基础。
 */
object RadixTree: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                }

                ensures {
                    radix_tree_node_cache_ready(RadixTree, SlubAllocator);
                    radix_tree_cpuhp_dead_step_registered(RadixTree, CpuHotplugState);
                    radix_tree_node_api_ready(RadixTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            radix_tree_node_cache_ready(RadixTree, SlubAllocator);
            radix_tree_cpuhp_dead_step_registered(RadixTree, CpuHotplugState);
            radix_tree_node_api_ready(RadixTree);
        }
    }
}

/*
 * MapleTree 表示 maple_tree_init() 建立的全局 maple node 分配基础。
 */
object MapleTree: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SlubAllocator.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    maple_tree_node_cache_ready(MapleTree, SlubAllocator);
                    maple_tree_node_api_ready(MapleTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            maple_tree_node_cache_ready(MapleTree, SlubAllocator);
            maple_tree_node_api_ready(MapleTree);
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
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    PageAllocator.state == State::Ready;
                    SlubAllocator.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    workqueue_early_framework_ready(Workqueue, CpuGroup);
                    workqueue_system_queues_ready(Workqueue);
                    workqueue_worker_pools_prepared(Workqueue);
                    workqueue_unbound_cpumask_ready(Workqueue);
                    workqueue_workers_not_running(Workqueue);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            workqueue_early_framework_ready(Workqueue, CpuGroup);
            workqueue_workers_not_running(Workqueue);
        }

        events {
            /*
             * Setup 对应 PreSmpInitPhase 中 workqueue_init()。它创建
             * rescuer 和初始 worker 壳，打开 worker 创建属性，但仍不表示
             * SMP topology 感知和完整 worker 运行期服务已经 online。
             */
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Preset -> State::Prepared {
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

        events {
            /*
             * Setup 对应 softirq_init()。TimerWheel/HrtimerCore 已经在本阶段
             * 前半段注册 TIMER_SOFTIRQ/HRTIMER_SOFTIRQ；这里补齐 tasklet 队列
             * 与 TASKLET/HI softirq action。
             */
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Prepared;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    TasksRcu.Event::Preset;
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
        events {
            on Event::Preset -> State::Prepared {
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

        events {
            /*
             * Setup 对应 PreSmpInitPhase 中 rcu_init_tasks_generic()。
             */
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    MmCoreInitPhase.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    SlubAllocator.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                    StaticBranch.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                }

                drives {
                    Scheduler.Event::Preset;
                    Scheduler.Event::Setup;
                    Scheduler.Event::Enable;
                    RadixTree.Event::Setup;
                    MapleTree.Event::Setup;
                    Workqueue.Event::Preset;
                    Softirq.Event::Preset;
                    RcuCore.Event::Setup;
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
