/*
 * Runtime Core Phase Specification
 *
 * This is SMP Runtime Phase subphase 2. It covers the boot processor side of
 * kernel_init_freeable() from sched_init_smp() through page_alloc_init_late(),
 * after secondary CPUs have become online and before do_basic_setup().
 */

/*
 * SchedulerSmpRuntime 表示 sched_init_smp() 的对象级 action。Scheduler
 * 主对象已经 Online；这里补齐 SMP sched domain、PID 1 affinity 和 RT/DL
 * SMP 后置状态，不重新推进 Scheduler 生命周期。
 */
object SchedulerSmpRuntime: KernelObject {
    initial_state: State::Base;
    parent: Scheduler;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SmpBringupPhase.state == State::Ready;
                    Scheduler.state == State::Online;
                    KernelInitTask.state == State::Online;
                    CpuGroup.state == State::Ready;
                    SecondaryCpuOnlineAck.state == State::Ready;
                    SmpBringupBoundary.state == State::Ready;
                }

                ensures {
                    scheduler_smp_initialized(Scheduler);
                    scheduler_smp_domains_ready(Scheduler, CpuGroup);
                    kernel_init_boot_cpu_affinity_released(KernelInitTask);
                    kernel_init_pf_no_setaffinity_cleared(KernelInitTask);
                    scheduler_rt_dl_smp_ready(Scheduler);
                    scheduler_granularity_refreshed(Scheduler);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            scheduler_smp_initialized(Scheduler);
            scheduler_smp_domains_ready(Scheduler, CpuGroup);
            kernel_init_boot_cpu_affinity_released(KernelInitTask);
            kernel_init_pf_no_setaffinity_cleared(KernelInitTask);
            scheduler_rt_dl_smp_ready(Scheduler);
            scheduler_granularity_refreshed(Scheduler);
        }
    }
}

/*
 * WorkqueueTopology 表示 workqueue_init_topology()。它消耗 PreSMP 时期
 * Workqueue.Ready 的 worker 创建边界，在 CPU topology 稳定后发布
 * topology-aware unbound workqueue 事实。为避免破坏前序阶段对
 * Workqueue.Ready 的历史不变式，当前 formal 用 action object 承载该
 * 第三步初始化，而不改变 Workqueue 主对象状态。
 */
object WorkqueueTopology: KernelObject {
    initial_state: State::Base;
    parent: Workqueue;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Workqueue.state == State::Ready;
                    SchedulerSmpRuntime.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    SecondaryCpuOnlineAck.state == State::Ready;
                }

                ensures {
                    workqueue_topology_ready(Workqueue, CpuGroup);
                    workqueue_pod_types_ready(Workqueue);
                    workqueue_unbound_pools_rebound(Workqueue, CpuGroup);
                    workqueue_max_active_topology_ready(Workqueue);
                    workqueue_workers_not_running(Workqueue);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            workqueue_topology_ready(Workqueue, CpuGroup);
            workqueue_pod_types_ready(Workqueue);
            workqueue_unbound_pools_rebound(Workqueue, CpuGroup);
            workqueue_max_active_topology_ready(Workqueue);
            workqueue_workers_not_running(Workqueue);
        }
    }
}

/*
 * AsyncCoreDeferred 保留 async_init() 在时序上的位置。当前对象级轮次
 * 不展开 async domain、cookie、pending list、wait queue 和 worker
 * 运行细节。
 */
object AsyncCoreDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    WorkqueueTopology.state == State::Ready;
                }

                ensures {
                    async_core_setup_deferred();
                    async_workqueue_creation_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            async_core_setup_deferred();
            async_workqueue_creation_deferred();
        }
    }
}

/*
 * PadataCoreDeferred 保留 padata_init() 在时序上的位置。当前不展开
 * padata instance、work array、free list 和具体使用方。
 */
object PadataCoreDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    AsyncCoreDeferred.state == State::Ready;
                    CpuHotplugState.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    padata_core_setup_deferred();
                    padata_hotplug_steps_deferred();
                    padata_work_array_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            padata_core_setup_deferred();
            padata_hotplug_steps_deferred();
            padata_work_array_deferred();
        }
    }
}

/*
 * PageAllocatorLate 表示 page_alloc_init_late()。PageAllocator 主对象在
 * mm core init 后已 Ready；本对象发布 late 收尾事实和当前配置下的
 * trimmed/no-op 路径，不改变 PageAllocator 主生命周期。
 */
object PageAllocatorLate: KernelObject {
    initial_state: State::Base;
    parent: PageAllocator;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PageAllocator.state == State::Ready;
                    PadataCoreDeferred.state == State::Ready;
                    WorkqueueTopology.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    SecondaryCpuOnlineAck.state == State::Ready;
                }

                ensures {
                    page_allocator_late_ready(PageAllocator);
                    memory_stats_printed(PageAllocator);
                    buffer_init_ready(PageAllocator);
                    memblock_private_discarded(PageAllocator);
                    zone_contiguous_ready(PageAllocator);
                    page_allocator_sysctl_ready(PageAllocator);
                    deferred_struct_page_init_trimmed();
                    page_extension_late_trimmed();
                    shuffle_page_allocator_late_trimmed();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_allocator_late_ready(PageAllocator);
            memory_stats_printed(PageAllocator);
            buffer_init_ready(PageAllocator);
            memblock_private_discarded(PageAllocator);
            zone_contiguous_ready(PageAllocator);
            page_allocator_sysctl_ready(PageAllocator);
            deferred_struct_page_init_trimmed();
            page_extension_late_trimmed();
            shuffle_page_allocator_late_trimmed();
        }
    }
}

/*
 * RuntimeCoreBoundary 聚合 sched_init_smp() 到 page_alloc_init_late() 的
 * BP-side 完成边界，并把下一子阶段入口固定为 do_basic_setup()。
 */
object RuntimeCoreBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SchedulerSmpRuntime.state == State::Ready;
                    WorkqueueTopology.state == State::Ready;
                    AsyncCoreDeferred.state == State::Ready;
                    PadataCoreDeferred.state == State::Ready;
                    PageAllocatorLate.state == State::Ready;
                }

                ensures {
                    runtime_core_boundary_ready(RuntimeCoreBoundary);
                    do_basic_setup_next_boundary();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            runtime_core_boundary_ready(RuntimeCoreBoundary);
            do_basic_setup_next_boundary();
        }
    }
}

/*
 * RuntimeCorePhase 表示 SMP Runtime Phase 的运行核心补全期。当前继续
 * 以 BP 主线为中心；AP 侧启动后的本地细节已由 SmpBringupPhase 的
 * summary facts 表示，不在本子阶段重新展开。
 */
object RuntimeCorePhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SmpBringupPhase.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    Scheduler.state == State::Online;
                    Workqueue.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    SecondaryCpuOnlineAck.state == State::Ready;
                    SmpBringupBoundary.state == State::Ready;
                }

                drives {
                    SchedulerSmpRuntime.Event::Setup;
                    WorkqueueTopology.Event::Setup;
                    AsyncCoreDeferred.Event::Setup;
                    PadataCoreDeferred.Event::Setup;
                    PageAllocatorLate.Event::Setup;
                    RuntimeCoreBoundary.Event::Setup;
                }

                ensures {
                    runtime_core_phase_ready(RuntimeCorePhase);
                    scheduler_smp_initialized(Scheduler);
                    kernel_init_boot_cpu_affinity_released(KernelInitTask);
                    workqueue_topology_ready(Workqueue, CpuGroup);
                    async_core_setup_deferred();
                    padata_core_setup_deferred();
                    page_allocator_late_ready(PageAllocator);
                    runtime_core_boundary_ready(RuntimeCoreBoundary);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SmpBringupPhase.state == State::Ready;
            SchedulerSmpRuntime.state == State::Ready;
            WorkqueueTopology.state == State::Ready;
            AsyncCoreDeferred.state == State::Ready;
            PadataCoreDeferred.state == State::Ready;
            PageAllocatorLate.state == State::Ready;
            RuntimeCoreBoundary.state == State::Ready;
            runtime_core_phase_ready(RuntimeCorePhase);
        }
    }
}
