/*
 * SMP Bringup Phase Specification
 *
 * This is SMP Runtime Phase subphase 1. It covers smp_init() on the boot
 * processor side, from idle_threads_init() through smp_cpus_done(). AP-side
 * internals are summarized as ack-producing actions, but the BP/AP
 * synchronization completions remain explicit model facts.
 */

/*
 * SecondaryIdleTaskSet 表示 idle_threads_init() 为 possible non-boot CPU
 * 准备 inactive idle task。它不启动 CPU，也不让 idle task 进入运行。
 */
object SecondaryIdleTaskSet: TaskObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    PreSmpInitPhase.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Scheduler.state == State::Online;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    secondary_idle_tasks_prepared(CpuGroup);
                    secondary_idle_tasks_inactive(CpuGroup);
                    secondary_cpus_present_but_not_online(CpuGroup);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            secondary_idle_tasks_prepared(CpuGroup);
            secondary_idle_tasks_inactive(CpuGroup);
            secondary_cpus_present_but_not_online(CpuGroup);
        }
    }
}

/*
 * CpuHotplugSyncSet 表示 cpuhp_threads_init() 建立的 hotplug 同步量和
 * boot CPU cpuhp thread 边界。done_down 保留给 teardown/rollback。
 */
object CpuHotplugSyncSet: KernelObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    PreSmpInitPhase.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    BootCPU.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    cpu_hotplug_sync_gates_prepared(CpuGroup);
                    cpu_hotplug_cpu_running_completion_ready(CpuGroup);
                    cpu_hotplug_done_up_completion_ready(CpuGroup);
                    cpu_hotplug_done_down_completion_ready(CpuGroup);
                    boot_cpu_hotplug_thread_online(CpuGroup);
                    secondary_cpu_hotplug_threads_deferred(CpuGroup);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            cpu_hotplug_sync_gates_prepared(CpuGroup);
            cpu_hotplug_cpu_running_completion_ready(CpuGroup);
            cpu_hotplug_done_up_completion_ready(CpuGroup);
            cpu_hotplug_done_down_completion_ready(CpuGroup);
            boot_cpu_hotplug_thread_online(CpuGroup);
            secondary_cpu_hotplug_threads_deferred(CpuGroup);
        }
    }
}

/*
 * CpuStartProvider 表示 BP side 的 arch/SBI CPU start provider。本轮只
 * 记录 BP 已发出启动请求，AP 入口内部按 summary action 处理。
 */
object CpuStartProvider: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                    SecondaryIdleTaskSet.state == State::Prepared;
                    CpuHotplugSyncSet.state == State::Prepared;
                    SbiIpi.state == State::Ready;
                }

                ensures {
                    cpu_start_provider_ready(CpuStartProvider);
                    bp_cpu_start_requests_issued(CpuStartProvider, CpuGroup);
                    ap_entry_detail_deferred(CpuStartProvider);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_start_provider_ready(CpuStartProvider);
            bp_cpu_start_requests_issued(CpuStartProvider, CpuGroup);
            ap_entry_detail_deferred(CpuStartProvider);
        }
    }
}

/*
 * SecondaryCpuStartupAck 表示 AP summary path 完成 cpu_running。
 * 它不展开 secondary_start_sbi/smp_callin 内部细节。
 */
object SecondaryCpuStartupAck: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpuStartProvider.state == State::Ready;
                    CpuHotplugSyncSet.state == State::Prepared;
                }

                ensures {
                    ap_startup_acknowledged(CpuGroup);
                    cpu_running_completion_observed(CpuGroup);
                    ap_secondary_entry_details_deferred(CpuGroup);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ap_startup_acknowledged(CpuGroup);
            cpu_running_completion_observed(CpuGroup);
            ap_secondary_entry_details_deferred(CpuGroup);
        }
    }
}

/*
 * SecondaryCpuOnlineAck 表示 AP 到达 CPUHP_AP_ONLINE_IDLE 并 complete done_up。
 * 当前只发布 BP 可继续执行所需的 online 边界和同步事实。
 */
object SecondaryCpuOnlineAck: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SecondaryCpuStartupAck.state == State::Ready;
                    CpuHotplugSyncSet.state == State::Prepared;
                    SbiIpi.state == State::Ready;
                }

                ensures {
                    ap_online_acknowledged(CpuGroup);
                    done_up_completion_observed(CpuGroup);
                    secondary_cpus_online(CpuGroup);
                    smp_concurrency_open(CpuGroup);
                    ap_idle_entry_detail_deferred(CpuGroup);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ap_online_acknowledged(CpuGroup);
            done_up_completion_observed(CpuGroup);
            secondary_cpus_online(CpuGroup);
            smp_concurrency_open(CpuGroup);
            ap_idle_entry_detail_deferred(CpuGroup);
        }
    }
}

/*
 * SmpBringupBoundary 表示 BP side 的 smp_init() 收尾。RISC-V
 * smp_cpus_done() 当前为空实现。
 */
object SmpBringupBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SecondaryCpuOnlineAck.state == State::Ready;
                }

                ensures {
                    smp_bringup_complete(SmpBringupBoundary);
                    smp_cpus_done_trimmed();
                    ap_hotplug_callback_details_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            smp_bringup_complete(SmpBringupBoundary);
            smp_cpus_done_trimmed();
            ap_hotplug_callback_details_deferred();
        }
    }
}

/*
 * SmpBringupPhase 表示 smp_init() 的 BP-focused 最小正式边界。AP side
 * 细节暂不展开，但 cpu_running/done_up 等同步事实必须保留。
 */
object SmpBringupPhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PreSmpInitPhase.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    BootIdleTask.state == State::Ready;
                    BootIdleRuntime.state == State::Ready;
                    KthreaddTask.state == State::Online;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    SbiIpi.state == State::Ready;
                    Workqueue.state == State::Ready;
                    TasksRcu.state == State::Ready;
                }

                drives {
                    SecondaryIdleTaskSet.Event::Preset;
                    CpuHotplugSyncSet.Event::Preset;
                    CpuStartProvider.Event::Setup;
                    SecondaryCpuStartupAck.Event::Setup;
                    SecondaryCpuOnlineAck.Event::Setup;
                    SmpBringupBoundary.Event::Setup;
                }

                ensures {
                    smp_bringup_phase_ready(SmpBringupPhase);
                    secondary_idle_tasks_prepared(CpuGroup);
                    cpu_hotplug_sync_gates_prepared(CpuGroup);
                    cpu_running_completion_observed(CpuGroup);
                    done_up_completion_observed(CpuGroup);
                    secondary_cpus_online(CpuGroup);
                    smp_concurrency_open(CpuGroup);
                }

                deferred {
                    "AP secondary_start_sbi / smp_callin() 内部细节留给后续 AP 侧展开。";
                    "AP hotplug thread callback 细节留给后续 CPU hotplug 模型。";
                    "RuntimeCorePhase 及后续 SMP Runtime 子阶段暂保持 deferred 边界。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SecondaryIdleTaskSet.state == State::Prepared;
            CpuHotplugSyncSet.state == State::Prepared;
            CpuStartProvider.state == State::Ready;
            SecondaryCpuStartupAck.state == State::Ready;
            SecondaryCpuOnlineAck.state == State::Ready;
            SmpBringupBoundary.state == State::Ready;
            smp_bringup_phase_ready(SmpBringupPhase);
            secondary_cpus_online(CpuGroup);
            smp_concurrency_open(CpuGroup);
        }
    }
}
