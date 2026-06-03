/*
 * Rest Init Phase Specification
 *
 * This is UP Multitask Phase subphase 1. It covers Linux rest_init(): RCU
 * scheduler start, creation of PID 1 and kthreadd, publication of
 * SYSTEM_SCHEDULING, completion of kthreadd_done, and boot idle runtime entry.
 * It stops before KernelInitTask enters kernel_init_freeable().
 */

/*
 * RcuSchedulerStart 表示 rcu_scheduler_starting() 后 RCU 已经离开 early boot
 * 调度盲区。它不重新推进 RcuCore 生命周期，也不创建 GP kthread。
 */
object RcuSchedulerStart: TaskObject {
    initial_state: State::Base;
    parent: RcuCore;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RcuCore.state == State::Ready;
                    Scheduler.state == State::Online;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    rcu_scheduler_starting_ready(RcuSchedulerStart, RcuCore);
                    rcu_scheduler_active_level_init(RcuSchedulerStart);
                    rcu_single_online_cpu_at_scheduler_start(RcuSchedulerStart, CpuGroup);
                    rcu_gp_threads_still_deferred(RcuCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            rcu_scheduler_starting_ready(RcuSchedulerStart, RcuCore);
            rcu_scheduler_active_level_init(RcuSchedulerStart);
            rcu_gp_threads_still_deferred(RcuCore);
        }
    }
}

/*
 * KernelInitTask 表示 user_mode_thread(kernel_init, NULL, CLONE_FS) 创建的
 * PID 1。它在本阶段变为 Online，但其 kernel_init_freeable() 执行属于下一子阶段。
 */
object KernelInitTask: TaskObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    BootInitTask.state == State::Online;
                }

                ensures {
                    kernel_init_spawn_spec_ready(KernelInitTask);
                    kernel_init_entry_selected(KernelInitTask);
                    kernel_init_clone_fs_flag_set(KernelInitTask);
                    kernel_init_not_user_mm_yet(KernelInitTask);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            kernel_init_spawn_spec_ready(KernelInitTask);
            kernel_init_entry_selected(KernelInitTask);
            kernel_init_clone_fs_flag_set(KernelInitTask);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                ensures {
                    kernel_init_task_ready(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_thread_context_ready(KernelInitTask);
                    kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
                    kernel_init_waits_for_kthreadd_done(KernelInitTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kernel_init_task_ready(KernelInitTask);
            kernel_init_task_pid_is_one(KernelInitTask);
            kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
            kernel_init_waits_for_kthreadd_done(KernelInitTask);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                ensures {
                    kernel_init_task_online(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_task_enqueued(KernelInitTask, Scheduler);
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                }
            }
        }
    }

    state State::Online {
        invariant {
            kernel_init_task_online(KernelInitTask);
            kernel_init_task_pid_is_one(KernelInitTask);
            kernel_init_task_enqueued(KernelInitTask, Scheduler);
        }
    }
}

/*
 * KernelInitAffinity 表示 rest_init() 中对 PID 1 设置 PF_NO_SETAFFINITY 并
 * 临时固定到 boot CPU。
 */
object KernelInitAffinity: TaskObject {
    initial_state: State::Base;
    parent: KernelInitTask;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    RootPidNamespace.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    kernel_init_affinity_ready(KernelInitAffinity, KernelInitTask);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    kernel_init_pid_lookup_used_root_namespace(KernelInitTask, RootPidNamespace);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kernel_init_affinity_ready(KernelInitAffinity, KernelInitTask);
            kernel_init_pf_no_setaffinity(KernelInitTask);
            kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
        }
    }
}

/*
 * KthreaddTask 表示 kernel_thread(kthreadd, NULL, CLONE_FS | CLONE_FILES)
 * 创建的全局内核线程管理者。
 */
object KthreaddTask: TaskObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    BootInitTask.state == State::Online;
                }

                ensures {
                    kthreadd_spawn_spec_ready(KthreaddTask);
                    kthreadd_entry_selected(KthreaddTask);
                    kthreadd_clone_fs_files_flags_set(KthreaddTask);
                    kthreadd_is_kernel_thread_provider(KthreaddTask);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            kthreadd_spawn_spec_ready(KthreaddTask);
            kthreadd_entry_selected(KthreaddTask);
            kthreadd_is_kernel_thread_provider(KthreaddTask);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                ensures {
                    kthreadd_task_ready(KthreaddTask);
                    kthreadd_task_pid_allocated(KthreaddTask, RootPidNamespace);
                    kthreadd_thread_context_ready(KthreaddTask);
                    kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
                    kthreadd_global_ref_bound(KthreaddTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kthreadd_task_ready(KthreaddTask);
            kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
            kthreadd_global_ref_bound(KthreaddTask);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                ensures {
                    kthreadd_task_online(KthreaddTask);
                    kthreadd_task_enqueued(KthreaddTask, Scheduler);
                    kthreadd_global_ref_bound(KthreaddTask);
                }
            }
        }
    }

    state State::Online {
        invariant {
            kthreadd_task_online(KthreaddTask);
            kthreadd_task_enqueued(KthreaddTask, Scheduler);
            kthreadd_global_ref_bound(KthreaddTask);
        }
    }
}

/*
 * SystemState 表示 Linux 全局 system_state 枚举。生命周期 Ready 对应本阶段
 * 把内部值推进到 SYSTEM_SCHEDULING。
 */
object SystemState: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                ensures {
                    system_state_booting(SystemState);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            system_state_booting(SystemState);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    system_state_scheduling(SystemState);
                    up_multitask_scheduling_open();
                    smp_concurrency_closed();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            system_state_scheduling(SystemState);
            up_multitask_scheduling_open();
            smp_concurrency_closed();
        }
    }
}

/*
 * KthreaddReadyGate 表示静态 completion kthreadd_done。Setup 建立 pending 门；
 * Enable 表示 complete(&kthreadd_done) 已发布，PID 1 可继续执行下一子阶段。
 */
object KthreaddReadyGate: TaskObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    kthreadd_ready_gate_ready(KthreaddReadyGate);
                    kthreadd_ready_gate_pending(KthreaddReadyGate);
                    kernel_init_waits_for_kthreadd_done(KernelInitTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kthreadd_ready_gate_ready(KthreaddReadyGate);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    SystemState.state == State::Ready;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
                }
            }
        }
    }

    state State::Online {
        invariant {
            kthreadd_ready_gate_completed(KthreaddReadyGate);
            kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
            kernel_init_released_for_pre_smp_init(KernelInitTask);
        }
    }
}

/*
 * BootIdleRuntime 表示 schedule_preempt_disabled() 后 cpu_startup_entry()
 * 确认 boot CPU idle runtime 入口。它复用 SchedInitPhase 已建立的 BootIdleTask。
 */
object BootIdleRuntime: TaskObject {
    initial_state: State::Base;
    parent: BootIdleTask;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    BootIdleTask.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                    KthreaddReadyGate.state == State::Online;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    scheduler_first_schedule_committed(Scheduler);
                    boot_idle_runtime_ready(BootIdleRuntime, BootIdleTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleRuntime, BootCPU);
                    boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
                    boot_cpu_hotplug_state_online(BootCPU);
                    secondary_cpus_not_started(CpuGroup);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            scheduler_first_schedule_committed(Scheduler);
            boot_idle_runtime_ready(BootIdleRuntime, BootIdleTask);
            boot_idle_cpu_startup_entry_ready(BootIdleRuntime, BootCPU);
            boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
            secondary_cpus_not_started(CpuGroup);
        }
    }
}

/*
 * RestInitPhase 表示 UP MultitaskPhase 的第一个子阶段。它创建 PID 1 和
 * kthreadd，发布 SYSTEM_SCHEDULING，并完成 boot idle runtime 入口。
 */
object RestInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: UpMultitaskPhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ProcessPreparePhase.state == State::Ready;
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                    BootIdleTask.state == State::Ready;
                    RcuCore.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    InterruptStream.state == State::Online;
                }

                drives {
                    RcuSchedulerStart.Event::Setup;
                    KernelInitTask.Event::Preset;
                    KernelInitTask.Event::Setup;
                    KernelInitTask.Event::Enable;
                    KernelInitAffinity.Event::Setup;
                    KthreaddTask.Event::Preset;
                    KthreaddTask.Event::Setup;
                    KthreaddTask.Event::Enable;
                    SystemState.Event::Preset;
                    SystemState.Event::Setup;
                    KthreaddReadyGate.Event::Setup;
                    KthreaddReadyGate.Event::Enable;
                    BootIdleRuntime.Event::Setup;
                }

                ensures {
                    rest_init_ready(RestInitPhase);
                    up_multitask_runtime_ready(RestInitPhase, KernelInitTask, KthreaddTask, BootIdleRuntime);
                    kernel_init_task_created(KernelInitTask);
                    kthreadd_task_created(KthreaddTask);
                    system_state_scheduling(SystemState);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    boot_cpu_idle_runtime_entered(BootIdleRuntime);
                    task_concurrency_open();
                    smp_concurrency_closed();
                    workqueue_workers_still_deferred();
                    rcu_gp_threads_still_deferred(RcuCore);
                    numa_default_policy_trimmed();
                }

                deferred {
                    "KernelInitTask 执行 kernel_init_freeable() 留给下一子阶段 PreSmpInitPhase。";
                    "KthreaddTask 消费 kthread_create_list 和后续 kthread 创建服务留给运行期模型。";
                    "真实抢占、上下文切换、任务栈切换和 idle loop 不在当前对象级实现中执行，只发布 rest_init 边界事实。";
                    "workqueue worker、Tasks RCU GP kthread、secondary CPU 启动仍保持 deferred，后续阶段再推进。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ProcessPreparePhase.state == State::Ready;
            RcuSchedulerStart.state == State::Ready;
            KernelInitTask.state == State::Online;
            KernelInitAffinity.state == State::Ready;
            KthreaddTask.state == State::Online;
            SystemState.state == State::Ready;
            KthreaddReadyGate.state == State::Online;
            BootIdleRuntime.state == State::Ready;
            rest_init_ready(RestInitPhase);
            system_state_scheduling(SystemState);
            kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            task_concurrency_open();
            smp_concurrency_closed();
        }
    }
}
