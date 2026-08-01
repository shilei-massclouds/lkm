/*
 * SMP Bringup Phase Specification
 *
 * This is SMP Runtime Phase subphase 2. It covers BP-side smp_init() from
 * idle_threads_init() through smp_cpus_done(), plus the AP-side bringup phases
 * reached through RISC-V SBI HSM ordered booting. BP and AP execution are
 * separate phase lines: the BP issues hart_start requests and waits on Linux
 * completion gates, while each AP runs its own secondary_start_sbi entry,
 * smp_callin() body and online-idle handoff before producing the ack facts.
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
 * 准备 inactive idle task。每个 secondary CPU 都有自己的 idle task 和
 * 对应 kernel stack / pt_regs 栈顶；这些对象是 CPU 关联的 per-CPU
 * task/stack 事实，不是 CpuGroup 拥有的 CPU 本体。它不启动 CPU；各 idle
 * Task 在 hart_start 前已由 init_idle()-equivalent construction 建立为
 * OnCpu/Reserved/Invalid 的 rq->idle/rq->curr carrier，不属于普通 runnable
 * class queue。AP 真实进入 secondary entry 后只激活 Live authority；keyed
 * HSM Startup 再启动同 logical-id 的 initial idle Flow。
 */
object SecondaryIdleTaskSet: TaskSet {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PreSmpInitPhase.state == State::Online;
                    CpuGroup.state == State::Ready;
                    Cpu0Scheduler.state == State::Online;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    secondary_idle_tasks_prepared(CpuGroup);
                    secondary_idle_task_per_secondary_cpu(CpuGroup);
                    secondary_idle_task_bound_to_cpu_ref(CpuGroup);
                    secondary_idle_task_has_dedicated_stack(CpuGroup);
                    secondary_idle_task_pt_regs_stack_pointer_ready(CpuGroup);
                    secondary_idle_tasks_inactive(CpuGroup);
                    secondary_idle_tasks_on_cpu_reserved(CpuGroup);
                    secondary_idle_task_breakpoints_invalid(CpuGroup);
                    secondary_idle_flows_base(CpuGroup);
                    task_ap_idle_reserved_for_cpu(ApIdleTask);
                    task_execution_authority_is(
                        ApIdleTask,
                        TaskExecutionAuthority::Reserved
                    );
                    task_breakpoint_state_is(
                        ApIdleTask,
                        TaskBreakpointState::Invalid
                    );
                    task_initial_flow_is(ApIdleTask, ApIdleFlow);
                    task_owns_flow(ApIdleTask, ApIdleFlow);
                    task_flow_owner_is(ApIdleFlow, ApIdleTask);
                    task_flow_parent_is(ApIdleFlow, ApIdleTask);
                    ap_idle_flow_key_matches_task(ApIdleFlow, ApIdleTask);
                    secondary_cpus_present_but_not_online(CpuGroup);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            secondary_idle_tasks_prepared(CpuGroup);
            secondary_idle_task_per_secondary_cpu(CpuGroup);
            secondary_idle_task_bound_to_cpu_ref(CpuGroup);
            secondary_idle_task_has_dedicated_stack(CpuGroup);
            secondary_idle_task_pt_regs_stack_pointer_ready(CpuGroup);
            secondary_idle_tasks_inactive(CpuGroup);
            secondary_idle_tasks_on_cpu_reserved(CpuGroup);
            secondary_idle_task_breakpoints_invalid(CpuGroup);
            secondary_idle_flows_base(CpuGroup);
            task_ap_idle_reserved_for_cpu(ApIdleTask);
            task_execution_authority_is(
                ApIdleTask,
                TaskExecutionAuthority::Reserved
            );
            task_breakpoint_state_is(
                ApIdleTask,
                TaskBreakpointState::Invalid
            );
            task_initial_flow_is(ApIdleTask, ApIdleFlow);
            task_owns_flow(ApIdleTask, ApIdleFlow);
            task_flow_owner_is(ApIdleFlow, ApIdleTask);
            task_flow_parent_is(ApIdleFlow, ApIdleTask);
            ap_idle_flow_key_matches_task(ApIdleFlow, ApIdleTask);
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
                    PreSmpInitPhase.state == State::Online;
                    CpuGroup.state == State::Ready;
                    CpuGroup.cpus[0].state == State::Online;
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
 * CpuStartProvider 表示 BP side 的 arch/SBI CPU start provider。RISC-V
 * ordered booting 路径必须对齐 Linux cpu_ops_sbi.cpu_start():
 * 为每个目标 secondary CPU 写入 struct sbi_hart_boot_data 等价事实
 * { task_ptr, stack_ptr }，用 smp_mb() 等价 ordering 发布，然后调用
 * SBI_EXT_HSM_HART_START(hartid, __pa_symbol(secondary_start_sbi), hsm_data)。
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
                    SBI.state == State::Ready;
                    sbi_hsm_extension_available(SBI);
                    SbiIpi.state == State::Ready;
                    CpuAddRemoveLock.state == State::Ready;
                    CpuHotplugLock.state == State::Ready;
                    cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
                    smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
                }

                /*
                 * Pointwise representative of prepare_secondary_entry(): the
                 * BP publishes the target CPU's formal trap entry before HSM
                 * handoff.  The total interrupt gate is then opened for AP
                 * bringup, while syscall remains Prepared until user service
                 * activation.
                 */
                drives {
                    CpuGroup.Action::PrepareSecondaryTrapEntry;
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
                    bp_selects_secondary_start_sbi_entry(CpuStartProvider);
                    sbi_hsm_hart_start_requests_issued(CpuStartProvider, CpuGroup);
                    sbi_hsm_hart_start_return_observed(CpuStartProvider, CpuGroup);
                    sbi_hart_boot_data_per_secondary_cpu(CpuStartProvider, CpuGroup);
                    sbi_hart_boot_data_task_ptr_is_secondary_idle_task(
                        CpuStartProvider,
                        SecondaryIdleTaskSet
                    );
                    sbi_hart_boot_data_stack_ptr_is_secondary_pt_regs_stack(
                        CpuStartProvider,
                        SecondaryIdleTaskSet
                    );
                    cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
                    cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
                    sbi_boot_data_publish_barriers_observed(CpuStartProvider);
                    sbi_hsm_startup_signal_keyed_by_logical_id(CpuStartProvider, ApIdleFlow);
                    ap_idle_flow_hsm_startup_keyed(ApIdleFlow);
                    ap_idle_flow_key_matches_task(ApIdleFlow, ApIdleTask);
                    sbi_hsm_startup_targets_task_initial_flow(
                        CpuStartProvider,
                        ApIdleTask,
                        ApIdleFlow
                    );
                }

                emits {
                    ApIdleTask.Action::ActivateHsmAuthority;
                    ApIdleFlow.Action::AssignCpuRef(ApCPURef);
                    ApIdleFlow.Transition::Preset;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_start_provider_ready(CpuStartProvider);
            bp_cpu_start_requests_issued(CpuStartProvider, CpuGroup);
            bp_selects_secondary_start_sbi_entry(CpuStartProvider);
            sbi_hsm_hart_start_requests_issued(CpuStartProvider, CpuGroup);
            sbi_hsm_hart_start_return_observed(CpuStartProvider, CpuGroup);
            sbi_hart_boot_data_per_secondary_cpu(CpuStartProvider, CpuGroup);
            sbi_hart_boot_data_task_ptr_is_secondary_idle_task(
                CpuStartProvider,
                SecondaryIdleTaskSet
            );
            sbi_hart_boot_data_stack_ptr_is_secondary_pt_regs_stack(
                CpuStartProvider,
                SecondaryIdleTaskSet
            );
            cpu_add_remove_mutex_guard_used(CpuStartProvider, CpuAddRemoveLock);
            cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
            sbi_boot_data_publish_barriers_observed(CpuStartProvider);
            sbi_hsm_startup_signal_keyed_by_logical_id(CpuStartProvider, ApIdleFlow);
            sbi_hsm_startup_targets_task_initial_flow(
                CpuStartProvider,
                ApIdleTask,
                ApIdleFlow
            );
        }
    }
}

/*
 * ApEntryPreludePhase 是每个 AP 从 SBI HSM 进入 secondary_start_sbi 后
 * 执行的 AP 专属入口先导期。它不同于 BP BootInitFlow.Preset：不建立
 * CurrentCPU，不清 BSS，不解析 boot args；它消费 HSM boot data，
 * 建立 AP 当前 idle task 指针、AP 栈/pt_regs 指针，切到已存在的
 * SwapperVm，并安装正式 trap vector。boot-data/tp 验证后，同一入口直接
 * 建立该 idle Task 的 OnCpu 与 initial idle Flow Startup，不发送 Scheduler
 * Continue。该对象是以 secondary logical_id
 * 为 target key 的 replicated phase family；每个 AP 有独立四态。
 */
object ApEntryPreludePhase: PhaseObject {
    initial_state: State::Base;
    parent: ApIdleFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    CpuStartProvider.state == State::Ready;
                    ApIdleTask.state == State::OnCpu;
                    task_execution_authority_is(
                        ApIdleTask,
                        TaskExecutionAuthority::Live
                    );
                    task_authority_activated_by_hsm(ApIdleTask);
                    ap_idle_flow_hsm_startup_keyed(ApIdleFlow);
                    CpuGroup.state == State::Ready;
                    Vm.state == State::Online;
                    KernelAddrSpace.state == State::Online;
                    PhysicalDirect.state == State::Ready;
                    TrampolineVm.state == State::Ready;
                    SwapperVm.state == State::Ready;
                    CurrentCPU.trap.state == State::Ready;
                    CurrentCPU.trap.exception.state == State::Ready;
                    sbi_hsm_hart_start_requests_issued(CpuStartProvider, CpuGroup);
                    sbi_hart_boot_data_per_secondary_cpu(CpuStartProvider, CpuGroup);
                    secondary_idle_task_per_secondary_cpu(CpuGroup);
                    secondary_cpus_present_but_not_online(CpuGroup);
                    cpu_active_translation_controller_absent_for_ref(ApCPURef);
                    translation_live_satp_absent_for_ref(ApCPURef);
                    translation_initial_activation_entry_satp_for_ref_is(
                        ApCPURef,
                        0
                    );
                }

                drives {
                    PhysicalDirect.Action::ActivateOnCpu(ApCPURef);
                    TrampolineVm.Action::ActivateOnCpu(ApCPURef);
                    SwapperVm.Action::ActivateOnCpu(ApCPURef);
                }

                ensures {
                    ap_secondary_start_sbi_entry_reached(CpuGroup);
                    ap_entry_uses_logical_secondary_cpu(CpuGroup);
                    ap_entry_does_not_create_boot_current_cpu(CpuGroup);
                    ap_entry_consumes_sbi_hart_boot_data(CpuStartProvider, CpuGroup);
                    ap_current_task_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
                    ap_stack_is_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
                    ap_pt_regs_pointer_established(CpuGroup, SecondaryIdleTaskSet);
                    ap_kernel_fpu_vector_disabled(CpuGroup);
                    ap_interrupts_masked_on_entry(CpuGroup);
                    ap_switches_to_swapper_vm(SwapperVm);
                    cpu_active_translation_controller_for_ref_is(
                        ApCPURef,
                        TranslationControllerKind::SwapperVm
                    );
                    ap_formal_trap_entry_installed(CurrentCPU.trap, CurrentCPU.trap.exception);
                    ap_entry_boot_data_logical_id_matches_target(CpuGroup);
                    ap_entry_boot_data_stack_pointer_matches_target(
                        CpuGroup,
                        SecondaryIdleTaskSet
                    );
                    ap_entry_real_sp_in_secondary_idle_task_stack(
                        CpuGroup,
                        SecondaryIdleTaskSet
                    );
                    ap_entry_tp_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ap_secondary_start_sbi_entry_reached(CpuGroup);
            ap_entry_uses_logical_secondary_cpu(CpuGroup);
            ap_entry_does_not_create_boot_current_cpu(CpuGroup);
            ap_entry_consumes_sbi_hart_boot_data(CpuStartProvider, CpuGroup);
            ap_current_task_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
            ap_stack_is_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
            ap_pt_regs_pointer_established(CpuGroup, SecondaryIdleTaskSet);
            ap_switches_to_swapper_vm(SwapperVm);
            ap_formal_trap_entry_installed(CurrentCPU.trap, CurrentCPU.trap.exception);
            ap_entry_boot_data_logical_id_matches_target(CpuGroup);
            ap_entry_boot_data_stack_pointer_matches_target(CpuGroup, SecondaryIdleTaskSet);
            ap_entry_real_sp_in_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
            ap_entry_tp_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ap_entry_boot_data_logical_id_matches_target(CpuGroup);
                    ap_entry_boot_data_stack_pointer_matches_target(CpuGroup, SecondaryIdleTaskSet);
                    ap_entry_real_sp_in_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
                    ap_entry_tp_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ap_secondary_start_sbi_entry_reached(CpuGroup);
            ap_entry_consumes_sbi_hart_boot_data(CpuStartProvider, CpuGroup);
            ap_current_task_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
            ap_stack_is_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
            ap_entry_real_sp_in_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
            ap_entry_tp_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    ap_secondary_start_sbi_entry_reached(CpuGroup);
                    ap_entry_consumes_sbi_hart_boot_data(CpuStartProvider, CpuGroup);
                    ap_entry_real_sp_in_secondary_idle_task_stack(
                        CpuGroup,
                        SecondaryIdleTaskSet
                    );
                    ap_entry_tp_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
                }
            }
        }
    }

    state State::Online {
        invariant {
            ap_secondary_start_sbi_entry_reached(CpuGroup);
            ap_entry_consumes_sbi_hart_boot_data(CpuStartProvider, CpuGroup);
            ap_current_task_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
            ap_stack_is_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
            ap_entry_real_sp_in_secondary_idle_task_stack(CpuGroup, SecondaryIdleTaskSet);
            ap_entry_tp_is_secondary_idle_task(CpuGroup, SecondaryIdleTaskSet);
        }
    }
}

/*
 * ApSmpCallinPhase 是 AP 的 C/Rust bringup 主体，对齐 Linux smp_callin()。
 * 它在 AP 已经具备 current idle task 和正式 trap vector 后运行，发布
 * set_cpu_online() 与 complete(cpu_running) 事实。该对象按同一个
 * secondary logical_id replicate，并 pointwise 依赖前一 family Online。
 */
object ApSmpCallinPhase: PhaseObject {
    initial_state: State::Base;
    parent: ApIdleFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    ApEntryPreludePhase.state == State::Online;
                    CpuHotplugSyncSet.state == State::Prepared;
                    SbiIpi.state == State::Ready;
                    InitMM.state == State::Ready;
                    cpu_hotplug_cpu_running_completion_ready(CpuGroup);
                }

                ensures {
                    ap_smp_callin_reached(CpuGroup);
                    ap_current_active_mm_is_init_mm(CpuGroup, InitMM);
                    ap_topology_recorded(CpuGroup);
                    ap_notify_cpu_starting_observed(CpuGroup);
                    ap_ipi_enable_observed(ApSmpCallinPhase);
                    ap_cpu_online_fact_published(CpuGroup);
                    ap_cache_tlb_flush_summary_observed(ApSmpCallinPhase);
                    ap_cpu_running_completion_produced(CpuHotplugSyncSet);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ap_smp_callin_reached(CpuGroup);
            ap_current_active_mm_is_init_mm(CpuGroup, InitMM);
            ap_topology_recorded(CpuGroup);
            ap_ipi_enable_observed(ApSmpCallinPhase);
            ap_cpu_online_fact_published(CpuGroup);
            ap_cache_tlb_flush_summary_observed(ApSmpCallinPhase);
            ap_cpu_running_completion_produced(CpuHotplugSyncSet);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ApEntryPreludePhase.state == State::Online;
                    ap_cpu_running_completion_produced(CpuHotplugSyncSet);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ApEntryPreludePhase.state == State::Online;
            ap_smp_callin_reached(CpuGroup);
            ap_cpu_running_completion_produced(CpuHotplugSyncSet);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    ApEntryPreludePhase.state == State::Online;
                    ap_cpu_running_completion_produced(CpuHotplugSyncSet);
                }
            }
        }
    }

    state State::Online {
        invariant {
            ApEntryPreludePhase.state == State::Online;
            ap_smp_callin_reached(CpuGroup);
            ap_cpu_running_completion_produced(CpuHotplugSyncSet);
            ap_ipi_enable_observed(ApSmpCallinPhase);
            ap_cache_tlb_flush_summary_observed(ApSmpCallinPhase);
        }
    }
}

/*
 * ApOnlineIdlePhase 表示 AP 在 complete(cpu_running) 之后打开本地中断，
 * 进入 cpu_startup_entry(CPUHP_AP_ONLINE_IDLE)，并由 CPUHP AP online
 * idle 边界产生 done_up completion。当前不展开完整 idle loop、AP 调度
 * 或 hotplug callback，只要求 AP 已进入独立 idle/park 运行线。该对象按
 * secondary logical_id replicate，并 pointwise 依赖 callin family Online。
 */
object ApOnlineIdlePhase: PhaseObject {
    initial_state: State::Base;
    parent: ApIdleFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    ApSmpCallinPhase.state == State::Online;
                    CpuHotplugSyncSet.state == State::Prepared;
                    cpu_hotplug_done_up_completion_ready(CpuGroup);
                    ap_cpu_running_completion_produced(CpuHotplugSyncSet);
                }

                ensures {
                    ap_local_irq_enable_observed(ApOnlineIdlePhase);
                    ap_cpu_startup_entry_reached(ApOnlineIdlePhase);
                    ap_cpuhp_online_idle_reached(ApOnlineIdlePhase);
                    ap_done_up_completion_produced(CpuHotplugSyncSet);
                    ap_idle_or_park_loop_selected(CpuGroup);
                    ap_does_not_run_bp_payload_or_syscalls(CpuGroup);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ap_local_irq_enable_observed(ApOnlineIdlePhase);
            ap_cpu_startup_entry_reached(ApOnlineIdlePhase);
            ap_cpuhp_online_idle_reached(ApOnlineIdlePhase);
            ap_done_up_completion_produced(CpuHotplugSyncSet);
            ap_idle_or_park_loop_selected(CpuGroup);
            ap_does_not_run_bp_payload_or_syscalls(CpuGroup);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ApSmpCallinPhase.state == State::Online;
                    ap_done_up_completion_produced(CpuHotplugSyncSet);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ApSmpCallinPhase.state == State::Online;
            ap_done_up_completion_produced(CpuHotplugSyncSet);
            ap_idle_or_park_loop_selected(CpuGroup);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    ApSmpCallinPhase.state == State::Online;
                    ap_done_up_completion_produced(CpuHotplugSyncSet);
                    ap_idle_or_park_loop_selected(CpuGroup);
                    ap_idle_or_park_loop_entered(CpuGroup);
                }
            }
        }
    }

    state State::Online {
        invariant {
            ApSmpCallinPhase.state == State::Online;
            ap_done_up_completion_produced(CpuHotplugSyncSet);
            ap_idle_or_park_loop_selected(CpuGroup);
            ap_idle_or_park_loop_entered(CpuGroup);
            ap_does_not_run_bp_payload_or_syscalls(CpuGroup);
        }
    }
}

/*
 * SecondaryCpuStartupAck 表示 BP 侧观察 AP 已完成 cpu_running。AP 生产
 * 该 completion 的路径由 ApSmpCallinPhase 建模；本对象只覆盖 BP
 * wait_for_completion_timeout() 一侧的 wait.lock 观察边界。
 */
object SecondaryCpuStartupAck: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CpuStartProvider.state == State::Ready;
                    ApSmpCallinPhase.state == State::Online;
                    CpuHotplugSyncSet.state == State::Prepared;
                    cpu_hotplug_write_guard_used(CpuStartProvider, CpuHotplugLock);
                    sbi_boot_data_publish_barriers_observed(CpuStartProvider);
                    ap_cpu_running_completion_produced(CpuHotplugSyncSet);
                }

                within CpuRunningCompletionWaitLockContext {
                    ensures {
                        raw_spinlock_irqsave_entered(CpuRunningWaitLock, CurrentCPU);
                        raw_spinlock_irqrestore_exited(CpuRunningWaitLock, CurrentCPU);
                        completion_wait_lock_irqsave_entered(CpuHotplugSyncSet, CurrentCPU);
                        completion_wait_lock_irqrestore_exited(
                            CpuHotplugSyncSet,
                            CurrentCPU
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
                    ap_smp_callin_ack_matches_secondary_cpu(CpuGroup);
                    cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ap_startup_acknowledged(CpuGroup);
            cpu_running_completion_observed(CpuGroup);
            ap_smp_callin_ack_matches_secondary_cpu(CpuGroup);
            cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
        }
    }
}

/*
 * SecondaryCpuOnlineAck 表示 BP 侧观察 AP 已到达 CPUHP_AP_ONLINE_IDLE 并
 * complete done_up。CpuGroup online 集合只能在该 AP ack 事实之后更新；
 * BP 不得仅凭 start request 模拟推进 secondary online。
 */
object SecondaryCpuOnlineAck: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SecondaryCpuStartupAck.state == State::Ready;
                    ApOnlineIdlePhase.state == State::Online;
                    CpuHotplugSyncSet.state == State::Prepared;
                    SbiIpi.state == State::Ready;
                    cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
                    ap_done_up_completion_produced(CpuHotplugSyncSet);
                    ap_idle_or_park_loop_entered(CpuGroup);
                }

                within DoneUpCompletionWaitLockContext {
                    ensures {
                        raw_spinlock_irqsave_entered(DoneUpWaitLock, CurrentCPU);
                        raw_spinlock_irqrestore_exited(DoneUpWaitLock, CurrentCPU);
                        completion_wait_lock_irqsave_entered(CpuHotplugSyncSet, CurrentCPU);
                        completion_wait_lock_irqrestore_exited(
                            CpuHotplugSyncSet,
                            CurrentCPU
                        );
                        completion_done_increment_guarded_by_wait_lock(CpuHotplugSyncSet);
                        completion_wake_guarded_by_wait_lock(CpuHotplugSyncSet);
                        done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
                    }
                }

                drives {
                    Cpu1Scheduler.Transition::Enable;
                    Cpu2Scheduler.Transition::Enable;
                    Cpu3Scheduler.Transition::Enable;
                    Cpu4Scheduler.Transition::Enable;
                    Cpu5Scheduler.Transition::Enable;
                    Cpu6Scheduler.Transition::Enable;
                    Cpu7Scheduler.Transition::Enable;
                }

                ensures {
                    ap_online_acknowledged(CpuGroup);
                    done_up_completion_observed(CpuGroup);
                    secondary_cpus_online_after_ap_ack(CpuGroup);
                    secondary_cpus_online(CpuGroup);
                    Cpu1Scheduler.state == State::Online;
                    Cpu2Scheduler.state == State::Online;
                    Cpu3Scheduler.state == State::Online;
                    Cpu4Scheduler.state == State::Online;
                    Cpu5Scheduler.state == State::Online;
                    Cpu6Scheduler.state == State::Online;
                    Cpu7Scheduler.state == State::Online;
                    smp_concurrency_open(CpuGroup);
                    ap_idle_entry_detail_deferred(CpuGroup);
                    done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
                    ap_local_irq_enable_observed(ApOnlineIdlePhase);
                    ap_cache_tlb_flush_summary_observed(ApSmpCallinPhase);
                    ap_ipi_enable_observed(ApSmpCallinPhase);
                    ap_hotplug_thread_memory_barrier_pair_deferred(SecondaryCpuOnlineAck);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ap_online_acknowledged(CpuGroup);
            done_up_completion_observed(CpuGroup);
            secondary_cpus_online_after_ap_ack(CpuGroup);
            secondary_cpus_online(CpuGroup);
            smp_concurrency_open(CpuGroup);
            ap_idle_entry_detail_deferred(CpuGroup);
            done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
            ap_local_irq_enable_observed(ApOnlineIdlePhase);
            ap_cache_tlb_flush_summary_observed(ApSmpCallinPhase);
            ap_ipi_enable_observed(ApSmpCallinPhase);
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
 * SmpBringupPhase 表示 smp_init() 的 BP/AP 组合边界。BP side 由
 * KernelInitTask 驱动 idle_threads_init()/bringup_nonboot_cpus() 并等待
 * completions；AP side 由 HSM 启动后的 replicated ApEntryPreludePhase、
 * ApSmpCallinPhase 和 ApOnlineIdlePhase 驱动。三个 family 的 sibling
 * drives 按 logical_id pointwise 解释：同一 AP 严格顺序，不同 AP 允许
 * 交错。二者通过 cpu_running 和 done_up completion 连接。
 */
object SmpBringupPhase: PhaseObject {
    initial_state: State::Base;
    parent: KernelInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PreSmpInitPhase.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                    BootIdleSetup.state == State::Ready;
                    BootIdleFlow.state == State::Ready;
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
                }

                ensures {
                    secondary_idle_tasks_prepared(CpuGroup);
                    secondary_idle_task_per_secondary_cpu(CpuGroup);
                    secondary_idle_task_has_dedicated_stack(CpuGroup);
                    secondary_idle_tasks_on_cpu_reserved(CpuGroup);
                    secondary_idle_task_breakpoints_invalid(CpuGroup);
                    secondary_idle_flows_base(CpuGroup);
                    cpu_hotplug_sync_gates_prepared(CpuGroup);
                    cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
                    smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
                }

                emits {
                    CpuStartProvider.Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            SecondaryIdleTaskSet.state == State::Prepared;
            SmpbootThreadsLock.state == State::Ready;
            CpuHotplugSyncSet.state == State::Prepared;
            CpuAddRemoveLock.state == State::Ready;
            secondary_idle_tasks_on_cpu_reserved(CpuGroup);
            secondary_idle_task_breakpoints_invalid(CpuGroup);
            secondary_idle_flows_base(CpuGroup);
            cpu_hotplug_read_guard_used(CpuHotplugSyncSet, CpuHotplugLock);
            smpboot_threads_mutex_guard_used(CpuHotplugSyncSet, SmpbootThreadsLock);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PreSmpInitPhase.state == State::Online;
                    SmpBringupBoundary.state == State::Ready;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            PreSmpInitPhase.state == State::Online;
            SmpBringupBoundary.state == State::Ready;
            secondary_cpus_online(CpuGroup);
            smp_concurrency_open(CpuGroup);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    smp_bringup_phase_ready(SmpBringupPhase);
                    secondary_cpus_online(CpuGroup);
                    smp_concurrency_open(CpuGroup);
                }
            }
        }
    }

    state State::Online {
        invariant {
            PreSmpInitPhase.state == State::Online;
            smp_bringup_phase_ready(SmpBringupPhase);
            SmpBringupBoundary.state == State::Ready;
            secondary_cpus_online(CpuGroup);
            smp_concurrency_open(CpuGroup);
        }
    }

    actions {
        /*
         * AP Flow completion resumes the BP wait/ack continuation.  The AP
         * phases have already run pointwise on the AP execution line.
         */
        Action::ObserveApCompletion {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Prepared;
                ApIdleFlow.state == State::Online;
                ApEntryPreludePhase.state == State::Online;
                ApSmpCallinPhase.state == State::Online;
                ApOnlineIdlePhase.state == State::Online;
            }
            drives {
                SecondaryCpuStartupAck.Transition::Setup;
                SecondaryCpuOnlineAck.Transition::Setup;
                SmpBringupBoundary.Transition::Setup;
            }
            ensures {
                smp_bringup_phase_ready(SmpBringupPhase);
                ap_secondary_start_sbi_entry_reached(CpuGroup);
                ap_smp_callin_reached(CpuGroup);
                ap_cpu_running_completion_produced(CpuHotplugSyncSet);
                ap_done_up_completion_produced(CpuHotplugSyncSet);
                cpu_running_completion_observed(CpuGroup);
                done_up_completion_observed(CpuGroup);
                secondary_cpus_online_after_ap_ack(CpuGroup);
                secondary_cpus_online(CpuGroup);
                smp_concurrency_open(CpuGroup);
                cpu_running_wait_lock_guard_used(CpuHotplugSyncSet, CpuRunningWaitLock);
                done_up_wait_lock_guard_used(CpuHotplugSyncSet, DoneUpWaitLock);
                ap_cache_tlb_flush_summary_observed(ApSmpCallinPhase);
                ap_ipi_enable_observed(ApSmpCallinPhase);
                ap_local_irq_enable_observed(ApOnlineIdlePhase);
                smp_bringup_full_ap_cpu_local_chain_deferred(SmpBringupPhase);
            }

            deferred smp_bringup.001 {
                category: DeferredCategory::ModelDetail;
                summary: "Complete each AP CurrentTask, CurrentCPU and InterruptType resolution chain.";
                evidence { smp_bringup_full_ap_cpu_local_chain_deferred(SmpBringupPhase); }
                close_when: "Every online AP has a complete CPU-local identity/control/task chain with SMP tests.";
            }
            deferred smp_bringup.002 {
                category: DeferredCategory::Protocol;
                summary: "Prove the AP hotplug-thread should_run smp_mb pairing.";
                evidence { ap_hotplug_thread_memory_barrier_pair_deferred(SecondaryCpuOnlineAck); }
                close_when: "The publish/observe memory-order proof and stress tests cover the should_run handoff.";
            }
            deferred smp_bringup.003 {
                category: DeferredCategory::ModelDetail;
                summary: "Complete AP hotplug-thread callback execution semantics.";
                evidence { ap_hotplug_callback_details_deferred(); }
                close_when: "Callback ordering, failure and CPU online/offline tests pass.";
            }

            emits {
                Transition::Setup;
            }
        }
    }
}
