/*
 * Pre-SMP Init Phase Specification
 *
 * This is SMP Runtime Phase subphase 1. It covers kernel_init_freeable()
 * from opening the full GFP allocation mask through the point immediately
 * before smp_init(). It is driven by KernelInitTask after the rest_init
 * scheduler fork boundary, while BootInitTask may still complete its boot
 * idle tail.
 */

/*
 * PageAllocatorFullGfpMask 表示 kernel_init_freeable() 中
 * gfp_allowed_mask = __GFP_BITS_MASK 形成的分配策略边界。
 */
object PageAllocatorFullGfpMask: MemoryObject {
    initial_state: State::Base;
    parent: PageAllocator;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    PageAllocator.state == State::Ready;
                }

                ensures {
                    page_allocator_full_gfp_mask_open(PageAllocator);
                    blocking_gfp_allocations_allowed(PageAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_allocator_full_gfp_mask_open(PageAllocator);
            blocking_gfp_allocations_allowed(PageAllocator);
        }
    }
}

/*
 * PreSmpCpuTopology 表示 smp_prepare_cpus(setup_max_cpus) 在 smp_init()
 * 之前发布的 topology/present 边界。它不启动 secondary CPU。
 */
object PreSmpCpuTopology: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    CpuGroup.state == State::Ready;
                    BootCPU.state == State::Online;
                }

                ensures {
                    cpu_topology_prepared_for_smp(CpuGroup);
                    boot_cpu_topology_recorded(CpuGroup, BootCPU);
                    secondary_cpus_present(CpuGroup);
                    secondary_cpus_present_but_not_online(CpuGroup);
                    smp_concurrency_closed();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_topology_prepared_for_smp(CpuGroup);
            boot_cpu_topology_recorded(CpuGroup, BootCPU);
            secondary_cpus_present_but_not_online(CpuGroup);
            smp_concurrency_closed();
        }
    }
}

/*
 * VmstatCore 表示 init_mm_internals() 建立的 mm/vmstat 运行支撑壳。
 * procfs 可见导出仍保持 deferred。
 */
object VmstatCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Workqueue.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    PageAllocatorFullGfpMask.state == State::Ready;
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                }

                ensures {
                    vmstat_core_prepared(VmstatCore);
                    vmstat_mm_percpu_workqueue_ready(VmstatCore, Workqueue);
                    vmstat_cpuhp_state_registered(VmstatCore);
                    vmstat_shepherd_work_started(VmstatCore);
                    vmstat_proc_exports_deferred();
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            vmstat_core_prepared(VmstatCore);
            vmstat_mm_percpu_workqueue_ready(VmstatCore, Workqueue);
            vmstat_proc_exports_deferred();
        }
    }
}

/*
 * PreSmpInitcallTable 表示 do_pre_smp_initcalls() 对 early initcall 区间的
 * 一次驱动。它不建模为长期 registry，只发布当前阶段早期动作事实。
 */
object PreSmpInitcallTable: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    RcuCore.state == State::Ready;
                    Softirq.state == State::Ready;
                    Scheduler.state == State::Online;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    pre_smp_initcalls_early_level_ran(PreSmpInitcallTable);
                    rcu_gp_kthread_ready(RcuCore);
                    softirq_ksoftirqd_ready(Softirq);
                    scheduler_migration_ready(Scheduler, BootCPU);
                    cpu_stopper_prepared();
                    memory_zero_page_bound();
                    address_space_id_ready();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            pre_smp_initcalls_early_level_ran(PreSmpInitcallTable);
            rcu_gp_kthread_ready(RcuCore);
            softirq_ksoftirqd_ready(Softirq);
            scheduler_migration_ready(Scheduler, BootCPU);
        }
    }
}

/*
 * PreSmpInitBoundary 聚合 set_mems_allowed()/cad_pid/lockup detector/smp_init
 * 等当前阶段不推进为主对象的边界事实。
 */
object PreSmpInitBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    PreSmpInitcallTable.state == State::Ready;
                }

                ensures {
                    mems_allowed_trimmed_noop();
                    cad_pid_binding_deferred();
                    lockup_detector_deferred();
                    smp_init_not_called();
                    secondary_cpus_present_but_not_online(CpuGroup);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            smp_init_not_called();
            secondary_cpus_present_but_not_online(CpuGroup);
        }
    }
}

/*
 * PreSmpInitPhase 表示 KernelInitTask 在 kernel_init_freeable() 中推进的
 * SMP 启动前初始化段。它是 SmpRuntimePhase 的首个子阶段，依赖
 * KernelInitTask 已被 kthreadd_done 释放且 Scheduler.Action::Schedule
 * 已提交，同时要求 KernelInitTask 的创建入口已由 TaskCreationCore
 * 绑定为 TaskEntry::KernelInit 并指向 SmpRuntimePhase 入口；它不由
 * RestInitPhase.Ready 作为普通 sibling 顺序启动。
 */
object PreSmpInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
                    task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    PageAllocator.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Workqueue.state == State::Prepared;
                    RcuCore.state == State::Ready;
                    Softirq.state == State::Ready;
                    Scheduler.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                drives {
                    PageAllocatorFullGfpMask.Transition::Setup;
                    PreSmpCpuTopology.Transition::Setup;
                    Workqueue.Transition::Setup;
                    VmstatCore.Transition::Preset;
                    TasksRcu.Transition::Setup;
                    PreSmpInitcallTable.Transition::Setup;
                    PreSmpInitBoundary.Transition::Setup;
                }

                ensures {
                    pre_smp_init_ready(PreSmpInitPhase);
                    task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
                    task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    page_allocator_full_gfp_mask_open(PageAllocator);
                    cpu_topology_prepared_for_smp(CpuGroup);
                    workqueue_ready_before_smp(Workqueue);
                    vmstat_core_prepared(VmstatCore);
                    tasks_rcu_ready(TasksRcu);
                    pre_smp_initcalls_early_level_ran(PreSmpInitcallTable);
                    smp_init_not_called();
                    secondary_cpus_present_but_not_online(CpuGroup);
                }

                deferred {
                    "cad_pid 绑定留给系统控制路径。";
                    "procfs vmstat 导出留给 VFS/procfs 路径。";
                    "lockup detector 留给运行期诊断模型。";
                    "smp_init() 是下一子阶段 SmpBringupPhase 的入口。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kernel_init_released_for_pre_smp_init(KernelInitTask);
            task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
            task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
            kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            PageAllocator.state == State::Ready;
            PageAllocatorFullGfpMask.state == State::Ready;
            PreSmpCpuTopology.state == State::Ready;
            Workqueue.state == State::Ready;
            VmstatCore.state == State::Prepared;
            TasksRcu.state == State::Ready;
            PreSmpInitcallTable.state == State::Ready;
            PreSmpInitBoundary.state == State::Ready;
            pre_smp_init_ready(PreSmpInitPhase);
            smp_init_not_called();
            secondary_cpus_present_but_not_online(CpuGroup);
        }
    }
}
