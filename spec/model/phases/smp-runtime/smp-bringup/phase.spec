/*
 * SMP Bringup Phase Specification
 *
 * This is SMP Runtime Phase subphase 2. It covers smp_init() on the boot
 * processor side, from idle_threads_init() through smp_cpus_done(). AP-side
 * internals are summarized as ack-producing actions, but the BP/AP
 * synchronization completions remain explicit model facts.
 */

lock CpuRunningWaitLock: RawSpinLock;
lock DoneUpWaitLock: RawSpinLock;

/*
 * SmpbootThreadsLock 表示 Linux kernel/smpboot.c 的静态
 * smpboot_threads_lock。cpuhp_threads_init() 通过
 * smpboot_register_percpu_thread(&cpuhp_threads) 在 cpus_read_lock()
 * 内持有该 mutex，创建/唤醒当前 online CPU 的 cpuhp/%u 线程并把模板加入
 * hotplug_threads list。secondary CPU 的线程创建细节仍保持 deferred。
 */
object SmpbootThreadsLock: Mutex {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    mutex_storage_bound(SmpbootThreadsLock);
                    mutex_init_kind_recorded(SmpbootThreadsLock);
                    mutex_preset_respects_init_kind(SmpbootThreadsLock);
                    mutex_owns_wait_queue(SmpbootThreadsLock);
                    mutex_wait_lock_internal_deferred(SmpbootThreadsLock);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    mutex_initialized(SmpbootThreadsLock);
                    mutex_ready(SmpbootThreadsLock);
                    mutex_unlocked(SmpbootThreadsLock);
                    mutex_wait_queue_ready(SmpbootThreadsLock);
                    mutex_recursive_locking_forbidden(SmpbootThreadsLock);
                    mutex_unlock_requires_owner(SmpbootThreadsLock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mutex_ready(SmpbootThreadsLock);
            mutex_unlocked(SmpbootThreadsLock);
            mutex_wait_queue_ready(SmpbootThreadsLock);
        }
    }
}

/*
 * CpuAddRemoveLock 表示 Linux kernel/cpu.c 的 cpu_add_remove_lock。
 * cpu_up() 用 cpu_maps_update_begin()/done() 持有它来串行化
 * cpu_online_mask / cpu_present_mask 更新。
 */
object CpuAddRemoveLock: Mutex {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    mutex_storage_bound(CpuAddRemoveLock);
                    mutex_init_kind_recorded(CpuAddRemoveLock);
                    mutex_preset_respects_init_kind(CpuAddRemoveLock);
                    mutex_owns_wait_queue(CpuAddRemoveLock);
                    mutex_wait_lock_internal_deferred(CpuAddRemoveLock);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    mutex_initialized(CpuAddRemoveLock);
                    mutex_ready(CpuAddRemoveLock);
                    mutex_unlocked(CpuAddRemoveLock);
                    mutex_wait_queue_ready(CpuAddRemoveLock);
                    mutex_recursive_locking_forbidden(CpuAddRemoveLock);
                    mutex_unlock_requires_owner(CpuAddRemoveLock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            mutex_ready(CpuAddRemoveLock);
            mutex_unlocked(CpuAddRemoveLock);
            mutex_wait_queue_ready(CpuAddRemoveLock);
        }
    }
}

/*
 * cpuhp_threads_init() reaches smpboot_register_percpu_thread(), whose
 * lexical guard nesting is cpus_read_lock() followed by
 * mutex_lock(&smpboot_threads_lock). Both protocols stay modeled even though
 * this boot path is still single-task on the BP.
 */
context SmpBringupCpuHotplugReadContext: ResourceExclusiveContext {
    guard {
        lock_ref: CpuHotplugLock;

        entered_by {
            CpuHotplugLock.Transition::ReadLock(KernelInitTaskRef);
        }

        exited_by {
            CpuHotplugLock.Transition::ReadUnlock(KernelInitTaskRef);
        }
    }

    obj_refs {
        CpuHotplugSyncSet;
        SmpbootThreadsLock;
        CpuHotplugLock;
    }
}

context SmpbootThreadsMutexContext: ResourceExclusiveContext {
    guard {
        lock_ref: SmpbootThreadsLock;

        entered_by {
            SmpbootThreadsLock.Transition::Lock(KernelInitTaskRef);
        }

        exited_by {
            SmpbootThreadsLock.Transition::Unlock(KernelInitTaskRef);
        }
    }

    obj_refs {
        CpuHotplugSyncSet;
        SmpbootThreadsLock;
    }
}

/*
 * bringup_nonboot_cpus() calls cpu_up(), which first holds
 * cpu_add_remove_lock through cpu_maps_update_begin()/done(), then _cpu_up()
 * takes cpus_write_lock() while advancing the CPUHP state machine.
 */
context CpuAddRemoveMutexContext: ResourceExclusiveContext {
    guard {
        lock_ref: CpuAddRemoveLock;

        entered_by {
            CpuAddRemoveLock.Transition::Lock(KernelInitTaskRef);
        }

        exited_by {
            CpuAddRemoveLock.Transition::Unlock(KernelInitTaskRef);
        }
    }

    obj_refs {
        CpuStartProvider;
        CpuAddRemoveLock;
        CpuHotplugLock;
    }
}

context CpuHotplugWriteContext: ResourceExclusiveContext {
    guard {
        lock_ref: CpuHotplugLock;

        entered_by {
            CpuHotplugLock.Transition::WriteLock(KernelInitTaskRef);
        }

        exited_by {
            CpuHotplugLock.Transition::WriteUnlock(KernelInitTaskRef);
        }
    }

    obj_refs {
        CpuStartProvider;
        CpuHotplugLock;
    }
}

context CpuRunningCompletionWaitLockContext: ResourceExclusiveContext {
    /*
     * RISC-V __cpu_up() waits on the static cpu_running completion while AP
     * smp_callin() completes it. The generic Completion Type supplies token
     * semantics; this instance records the wait.lock irqsave guard.
     */
    guard {
        lock_ref: CpuRunningWaitLock;

        entered_by {
            CpuRunningWaitLock.Transition::LockIrqSave;
        }

        exited_by {
            CpuRunningWaitLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        CpuHotplugSyncSet;
        SecondaryCpuStartupAck;
    }
}

context DoneUpCompletionWaitLockContext: ResourceExclusiveContext {
    /*
     * CPUHP generic bringup waits for st->done_up; AP reaches
     * cpuhp_online_idle(CPUHP_AP_ONLINE_IDLE) and completes that gate.
     */
    guard {
        lock_ref: DoneUpWaitLock;

        entered_by {
            DoneUpWaitLock.Transition::LockIrqSave;
        }

        exited_by {
            DoneUpWaitLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        CpuHotplugSyncSet;
        SecondaryCpuOnlineAck;
    }
}

/*
 * SecondaryIdleTaskSet 表示 idle_threads_init() 为 possible non-boot CPU
 * 准备 inactive idle task。它不启动 CPU，也不让 idle task 进入运行。
 */
object SecondaryIdleTaskSet: TaskObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
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
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PreSmpInitPhase.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    BootCPU.state == State::Online;
                    KthreaddTask.state == State::Online;
                    CpuHotplugLock.state == State::Ready;
                    SmpbootThreadsLock.state == State::Ready;
                }

                within SmpBringupCpuHotplugReadContext {
                    within SmpbootThreadsMutexContext {
                        ensures {
                            percpu_rwsem_read_lock_entered(CpuHotplugLock, KernelInitTaskRef);
                            percpu_rwsem_read_unlock_exited(CpuHotplugLock, KernelInitTaskRef);
                            mutex_lock_acquired(SmpbootThreadsLock, KernelInitTaskRef);
                            mutex_unlock_released(SmpbootThreadsLock, KernelInitTaskRef);
                            cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
                            smpboot_threads_mutex_guard_used(
                                CpuHotplugSyncSet,
                                SmpbootThreadsLock
                            );
                        }
                    }
                }

                ensures {
                    cpu_hotplug_sync_gates_prepared(CpuGroup);
                    cpu_hotplug_cpu_running_completion_ready(CpuGroup);
                    cpu_hotplug_done_up_completion_ready(CpuGroup);
                    cpu_hotplug_done_down_completion_ready(CpuGroup);
                    boot_cpu_hotplug_thread_online(CpuGroup);
                    secondary_cpu_hotplug_threads_deferred(CpuGroup);
                    cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
                    smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
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
            cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
            smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                    SecondaryIdleTaskSet.state == State::Prepared;
                    CpuHotplugSyncSet.state == State::Prepared;
                    SbiIpi.state == State::Ready;
                    CpuAddRemoveLock.state == State::Ready;
                    CpuHotplugLock.state == State::Ready;
                    cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
                    smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
                }

                within CpuAddRemoveMutexContext {
                    within CpuHotplugWriteContext {
                        ensures {
                            mutex_lock_acquired(CpuAddRemoveLock, KernelInitTaskRef);
                            mutex_unlock_released(CpuAddRemoveLock, KernelInitTaskRef);
                            percpu_rwsem_write_lock_entered(CpuHotplugLock, KernelInitTaskRef);
                            percpu_rwsem_write_unlock_exited(CpuHotplugLock, KernelInitTaskRef);
                            cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
                            cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
                        }
                    }
                }

                ensures {
                    cpu_start_provider_ready(CpuStartProvider);
                    bp_cpu_start_requests_issued(CpuStartProvider, CpuGroup);
                    ap_entry_detail_deferred(CpuStartProvider);
                    cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
                    cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
                    sbi_boot_data_publish_barriers_observed(CpuStartProvider);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_start_provider_ready(CpuStartProvider);
            bp_cpu_start_requests_issued(CpuStartProvider, CpuGroup);
            ap_entry_detail_deferred(CpuStartProvider);
            cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
            cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
            sbi_boot_data_publish_barriers_observed(CpuStartProvider);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CpuStartProvider.state == State::Ready;
                    CpuHotplugSyncSet.state == State::Prepared;
                    cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
                    sbi_boot_data_publish_barriers_observed(CpuStartProvider);
                }

                within CpuRunningCompletionWaitLockContext {
                    ensures {
                        raw_spinlock_irqsave_entered(CpuRunningWaitLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(CpuRunningWaitLock, BootCurrentCPU);
                        completion_wait_lock_irqsave_entered(CpuHotplugSyncSet, BootCurrentCPU);
                        completion_wait_lock_irqrestore_exited(
                            CpuHotplugSyncSet,
                            BootCurrentCPU
                        );
                        completion_done_increment_guarded_by_wait_lock(CpuHotplugSyncSet);
                        completion_wake_guarded_by_wait_lock(CpuHotplugSyncSet);
                        cpu_running_wait_lock_guard_used(
                            CpuHotplugSyncSet,
                            CpuRunningWaitLock
                        );
                    }
                }

                ensures {
                    ap_startup_acknowledged(CpuGroup);
                    cpu_running_completion_observed(CpuGroup);
                    ap_secondary_entry_details_deferred(CpuGroup);
                    cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ap_startup_acknowledged(CpuGroup);
            cpu_running_completion_observed(CpuGroup);
            ap_secondary_entry_details_deferred(CpuGroup);
            cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
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
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SecondaryCpuStartupAck.state == State::Ready;
                    CpuHotplugSyncSet.state == State::Prepared;
                    SbiIpi.state == State::Ready;
                    cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
                }

                within DoneUpCompletionWaitLockContext {
                    ensures {
                        raw_spinlock_irqsave_entered(DoneUpWaitLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(DoneUpWaitLock, BootCurrentCPU);
                        completion_wait_lock_irqsave_entered(CpuHotplugSyncSet, BootCurrentCPU);
                        completion_wait_lock_irqrestore_exited(
                            CpuHotplugSyncSet,
                            BootCurrentCPU
                        );
                        completion_done_increment_guarded_by_wait_lock(CpuHotplugSyncSet);
                        completion_wake_guarded_by_wait_lock(CpuHotplugSyncSet);
                        done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
                    }
                }

                ensures {
                    ap_online_acknowledged(CpuGroup);
                    done_up_completion_observed(CpuGroup);
                    secondary_cpus_online(CpuGroup);
                    smp_concurrency_open(CpuGroup);
                    ap_idle_entry_detail_deferred(CpuGroup);
                    done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
                    ap_local_irq_enable_summary_deferred(SecondaryCpuOnlineAck);
                    ap_cache_tlb_flush_summary_observed(SecondaryCpuOnlineAck);
                    ap_ipi_enable_observed(SecondaryCpuOnlineAck);
                    ap_hotplug_thread_memory_barrier_pair_deferred(SecondaryCpuOnlineAck);
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
            done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
            ap_local_irq_enable_summary_deferred(SecondaryCpuOnlineAck);
            ap_cache_tlb_flush_summary_observed(SecondaryCpuOnlineAck);
            ap_ipi_enable_observed(SecondaryCpuOnlineAck);
            ap_hotplug_thread_memory_barrier_pair_deferred(SecondaryCpuOnlineAck);
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
        transitions {
            on Transition::Setup -> State::Ready {
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
        transitions {
            on Transition::Setup -> State::Ready {
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
                    CpuHotplugLock.state == State::Ready;
                }

                drives {
                    SecondaryIdleTaskSet.Transition::Preset;
                    SmpbootThreadsLock.Transition::Preset;
                    SmpbootThreadsLock.Transition::Setup;
                    CpuHotplugSyncSet.Transition::Preset;
                    CpuAddRemoveLock.Transition::Preset;
                    CpuAddRemoveLock.Transition::Setup;
                    CpuStartProvider.Transition::Setup;
                    SecondaryCpuStartupAck.Transition::Setup;
                    SecondaryCpuOnlineAck.Transition::Setup;
                    SmpBringupBoundary.Transition::Setup;
                }

                ensures {
                    smp_bringup_phase_ready(SmpBringupPhase);
                    secondary_idle_tasks_prepared(CpuGroup);
                    cpu_hotplug_sync_gates_prepared(CpuGroup);
                    cpu_running_completion_observed(CpuGroup);
                    done_up_completion_observed(CpuGroup);
                    secondary_cpus_online(CpuGroup);
                    smp_concurrency_open(CpuGroup);
                    cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
                    smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
                    cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
                    cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
                    sbi_boot_data_publish_barriers_observed(CpuStartProvider);
                    cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
                    done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
                    ap_cache_tlb_flush_summary_observed(SecondaryCpuOnlineAck);
                    ap_ipi_enable_observed(SecondaryCpuOnlineAck);
                }

                deferred {
                    "AP secondary_start_sbi / smp_callin() 内部细节留给后续 AP 侧展开。";
                    "AP local_irq_enable() 的真实 live AP LocalInterruptControl 边界留给后续 AP 当前 CPU 模型，本轮只保留 summary fact。";
                    "AP hotplug thread should_run smp_mb() 配对和 callbacks 内部细节留给后续 CPU hotplug 模型，本轮保留 memory-ordering deferred fact。";
                    "AP hotplug thread callback 细节留给后续 CPU hotplug 模型。";
                    "FinalizePhase 内部的 async/initmem/mapping/sysctl 细节逐步展开，AP 侧仍留给后续模型。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SecondaryIdleTaskSet.state == State::Prepared;
            SmpbootThreadsLock.state == State::Ready;
            CpuHotplugSyncSet.state == State::Prepared;
            CpuAddRemoveLock.state == State::Ready;
            CpuStartProvider.state == State::Ready;
            SecondaryCpuStartupAck.state == State::Ready;
            SecondaryCpuOnlineAck.state == State::Ready;
            SmpBringupBoundary.state == State::Ready;
            smp_bringup_phase_ready(SmpBringupPhase);
            secondary_cpus_online(CpuGroup);
            smp_concurrency_open(CpuGroup);
            cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
            smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
            cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
            cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
            sbi_boot_data_publish_barriers_observed(CpuStartProvider);
            cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
            done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
            ap_local_irq_enable_summary_deferred(SecondaryCpuOnlineAck);
            ap_cache_tlb_flush_summary_observed(SecondaryCpuOnlineAck);
            ap_ipi_enable_observed(SecondaryCpuOnlineAck);
            ap_hotplug_thread_memory_barrier_pair_deferred(SecondaryCpuOnlineAck);
        }
    }
}
