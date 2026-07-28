/*
 * IRQ-Open Prepare Phase Specification
 *
 * This is the third interrupt-oriented leaf directly driven by
 * BootInitFlow.Setup. It starts after LocalIrqEnablePhase has
 * opened the boot CPU local interrupt gate and covers the Linux start_kernel()
 * segment from kmem_cache_init_late() through arch_cpu_finalize_init().
 */

/*
 * Console 表示 console_init() 建立的正式 console 准备边界。当前只要求
 * line discipline registry 和配置内 early register 路径进入 Prepared；
 * 真实设备 probe、boot console 注销和完整 handoff 仍是条件事实。
 */
object Console: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PrintkBuffer.state == State::Ready;
                    EarlyCon.state == State::Online;
                }

                drives {
                    TtyLineDisciplineRegistry.Transition::Preset;
                    ConsoleDriverSet.Transition::Preset;
                }

                ensures {
                    console_prepared(Console, PrintkBuffer);
                    console_initcall_table_scanned(Console, Lds);
                    console_real_device_probe_deferred(Console);
                    console_earlycon_handoff_conditional(Console, EarlyCon);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            TtyLineDisciplineRegistry.state == State::Prepared;
            ConsoleDriverSet.state == State::Prepared;
            console_prepared(Console, PrintkBuffer);
            console_initcall_table_scanned(Console, Lds);
            console_real_device_probe_deferred(Console);
            console_earlycon_handoff_conditional(Console, EarlyCon);
        }
    }
}

/*
 * TtyLineDisciplineRegistry 表示 console_init() 中注册 N_TTY line
 * discipline 的全局 registry 准备事实。
 */
object TtyLineDisciplineRegistry: ConsoleObject {
    initial_state: State::Base;
    parent: Console;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    tty_line_discipline_registry_prepared(TtyLineDisciplineRegistry);
                    n_tty_line_discipline_registered(TtyLineDisciplineRegistry);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            tty_line_discipline_registry_prepared(TtyLineDisciplineRegistry);
            n_tty_line_discipline_registered(TtyLineDisciplineRegistry);
        }
    }
}

/*
 * ConsoleDriverSet 表示当前配置下 console initcall 表的 early register
 * 结果。Prepared 不要求真实串口 console 已完成完整 device probe。
 */
object ConsoleDriverSet: ConsoleObject {
    initial_state: State::Base;
    parent: Console;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                }

                ensures {
                    console_driver_set_static_entries_ready(ConsoleDriverSet, Lds);
                    console_driver_set_early_registered(ConsoleDriverSet);
                    serial_console_probe_deferred(ConsoleDriverSet);
                    boot_console_unregister_deferred(ConsoleDriverSet);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            console_driver_set_static_entries_ready(ConsoleDriverSet, Lds);
            console_driver_set_early_registered(ConsoleDriverSet);
            serial_console_probe_deferred(ConsoleDriverSet);
            boot_console_unregister_deferred(ConsoleDriverSet);
        }
    }
}

/*
 * SchedClock 表示 sched_clock_init() 后 generic sched clock core 进入
 * 启动期可用边界。它不改变 RiscvTimerProvider 的生命周期状态。
 */
context SchedClockLocalInterruptContext: Context {
    /*
     * Linux sched_clock_init() temporarily disables local interrupts around
     * generic_sched_clock_init(). The outer phase has local interrupts enabled,
     * so this records the local irq disable/enable window explicitly.
     */
    guard {
        entered_by {
            BootCpuLocalInterrupt.Transition::Disable;
        }

        exited_by {
            BootCpuLocalInterrupt.Transition::Enable;
        }
    }

    obj_refs {
        SchedClock;
        BootCpuLocalInterrupt;
    }
}

object SchedClock: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    HrtimerCore.state == State::Ready;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    StaticBranch.state == State::Ready;
                    BootCpuLocalInterrupt.state == State::Ready;
                    boot_cpu_local_irq_enabled();
                }

                within SchedClockLocalInterruptContext {
                    ensures {
                        sched_clock_setup_local_irq_guard_used(SchedClock, BootCpuLocalInterrupt);
                    }
                }

                ensures {
                    sched_clock_ready(SchedClock, RiscvTimerProvider);
                    sched_clock_running_key_enabled(SchedClock, StaticBranch);
                    sched_clock_reader_ready(SchedClock);
                    sched_clock_timer_ready(SchedClock, HrtimerCore);
                    sched_clock_setup_local_irq_disable_enable_used(SchedClock);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            sched_clock_ready(SchedClock, RiscvTimerProvider);
            sched_clock_running_key_enabled(SchedClock, StaticBranch);
            sched_clock_reader_ready(SchedClock);
            sched_clock_timer_ready(SchedClock, HrtimerCore);
            sched_clock_setup_local_irq_disable_enable_used(SchedClock);
            sched_clock_setup_local_irq_guard_used(SchedClock, BootCpuLocalInterrupt);
        }
    }
}

/*
 * DelayLoop 表示 calibrate_delay() 后 busy-wait delay API 的参数基础。
 * udelay/ndelay/mdelay 是 Ready 后 action，不再推进生命周期状态。
 */
object DelayLoop: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    RiscvTimerProvider.state == State::Ready;
                    CpuGroup.cpus[0].state == State::Online;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    delay_loop_ready(DelayLoop, RiscvTimerProvider);
                    lpj_fine_consumed_from_timer_provider(DelayLoop, RiscvTimerProvider);
                    boot_cpu_loops_per_jiffy_ready(DelayLoop, CpuGroup.cpus[0]);
                    global_loops_per_jiffy_ready(DelayLoop);
                    delay_api_actions_available(DelayLoop);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            delay_loop_ready(DelayLoop, RiscvTimerProvider);
            boot_cpu_loops_per_jiffy_ready(DelayLoop, CpuGroup.cpus[0]);
            global_loops_per_jiffy_ready(DelayLoop);
            delay_api_actions_available(DelayLoop);
        }
    }
}

/*
 * IrqOpenPrepareTrimmedPaths 保留 start_kernel() 中落在本子阶段、但当前
 * ../linux-6.12/.config 下为空、不可达或暂缓展开的调用点。
 * 这些事实必须结构化记录，不能只留在 checkpoint 或 markdown 表格。
 */
object IrqOpenPrepareTrimmedPaths: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                    Console.state == State::Prepared;
                    PageAllocator.state == State::Ready;
                }

                ensures {
                    irq_open_prepare_trimmed_paths_ready(IrqOpenPrepareTrimmedPaths);
                    irq_open_panic_later_clear(IrqOpenPrepareTrimmedPaths);
                    irq_open_lockdep_init_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                    irq_open_lockdep_trimmed_because_config_debug_lock_alloc_disabled(IrqOpenPrepareTrimmedPaths);
                    irq_open_locking_selftest_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                    irq_open_locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled(IrqOpenPrepareTrimmedPaths);
                    irq_open_initrd_bounds_trimmed(IrqOpenPrepareTrimmedPaths);
                    irq_open_initrd_trimmed_because_config_blk_dev_initrd_disabled(IrqOpenPrepareTrimmedPaths);
                    irq_open_page_allocator_per_cpu_pagesets_deferred(IrqOpenPrepareTrimmedPaths, PageAllocator);
                    irq_open_numa_policy_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                    irq_open_numa_policy_trimmed_because_config_numa_disabled(IrqOpenPrepareTrimmedPaths);
                    irq_open_acpi_early_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                    irq_open_acpi_early_trimmed_because_config_acpi_disabled(IrqOpenPrepareTrimmedPaths);
                    irq_open_late_time_init_hook_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                    irq_open_late_time_init_hook_unset_on_riscv(IrqOpenPrepareTrimmedPaths);
                    irq_open_trimmed_paths_position_preserved(IrqOpenPrepareTrimmedPaths);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            irq_open_prepare_trimmed_paths_ready(IrqOpenPrepareTrimmedPaths);
            irq_open_panic_later_clear(IrqOpenPrepareTrimmedPaths);
            irq_open_lockdep_init_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_lockdep_trimmed_because_config_debug_lock_alloc_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_locking_selftest_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_initrd_bounds_trimmed(IrqOpenPrepareTrimmedPaths);
            irq_open_initrd_trimmed_because_config_blk_dev_initrd_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_page_allocator_per_cpu_pagesets_deferred(IrqOpenPrepareTrimmedPaths, PageAllocator);
            irq_open_numa_policy_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_numa_policy_trimmed_because_config_numa_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_acpi_early_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_acpi_early_trimmed_because_config_acpi_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_late_time_init_hook_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_late_time_init_hook_unset_on_riscv(IrqOpenPrepareTrimmedPaths);
            irq_open_trimmed_paths_position_preserved(IrqOpenPrepareTrimmedPaths);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    SchedClock.state == State::Ready;
                    DelayLoop.state == State::Ready;
                }

                ensures {
                    irq_open_arch_cpu_finalize_init_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                    irq_open_arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled(IrqOpenPrepareTrimmedPaths);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_open_prepare_trimmed_paths_ready(IrqOpenPrepareTrimmedPaths);
            irq_open_panic_later_clear(IrqOpenPrepareTrimmedPaths);
            irq_open_lockdep_init_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_lockdep_trimmed_because_config_debug_lock_alloc_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_locking_selftest_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_initrd_bounds_trimmed(IrqOpenPrepareTrimmedPaths);
            irq_open_initrd_trimmed_because_config_blk_dev_initrd_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_page_allocator_per_cpu_pagesets_deferred(IrqOpenPrepareTrimmedPaths, PageAllocator);
            irq_open_numa_policy_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_numa_policy_trimmed_because_config_numa_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_acpi_early_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_acpi_early_trimmed_because_config_acpi_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_late_time_init_hook_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_late_time_init_hook_unset_on_riscv(IrqOpenPrepareTrimmedPaths);
            irq_open_arch_cpu_finalize_init_trimmed_noop(IrqOpenPrepareTrimmedPaths);
            irq_open_arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled(IrqOpenPrepareTrimmedPaths);
            irq_open_trimmed_paths_position_preserved(IrqOpenPrepareTrimmedPaths);
        }
    }
}

/*
 * IrqOpenPreparePhase 表示 BootInitFlow.Setup 的第三个 interrupt 叶阶段。它承接已开放
 * boot CPU 本地中断总入口的事实，建立中断开放后到进程准备期前的 late
 * core/platform 准备边界。
 */
object IrqOpenPreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    LocalIrqEnablePhase.state == State::Online;
                    InterruptStream.state == State::Online;
                    IrqDispatchTree.state == State::Ready;
                    SbiIpi.state == State::Ready;
                    Tick.state == State::Ready;
                    TimerWheel.state == State::Ready;
                    HrtimerCore.state == State::Ready;
                    Softirq.state == State::Ready;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    SmpCallFunction.state == State::Ready;
                    Workqueue.state == State::Prepared;
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                }

                drives {
                    Console.Transition::Preset;
                    IrqOpenPrepareTrimmedPaths.Transition::Preset;
                    SchedClock.Transition::Setup;
                    DelayLoop.Transition::Setup;
                    IrqOpenPrepareTrimmedPaths.Transition::Setup;
                }

                ensures {
                    irq_open_prepare_ready(IrqOpenPreparePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    boot_cpu_local_irq_enabled();
                    early_boot_irqs_disabled_false();
                    softirq_execution_closed(Softirq);
                    workqueue_workers_not_running(Workqueue);
                    sbi_ipi_enable_deferred(SbiIpi);
                    smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
                    slub_flush_workqueue_ready(SlubSubsystem, Workqueue);
                    irq_open_prepare_trimmed_paths_ready(IrqOpenPrepareTrimmedPaths);
                    irq_open_page_allocator_per_cpu_pagesets_deferred(IrqOpenPrepareTrimmedPaths, PageAllocator);
                    irq_open_real_console_handoff_deferred(IrqOpenPreparePhase);
                    irq_open_slub_full_enable_deferred(IrqOpenPreparePhase);
                    next_interrupt_subphase_is_process_prepare();
                }

                deferred irq_open.001 {
                    category: DeferredCategory::ModelDetail;
                    summary: "Complete the page-allocator per-CPU pageset fast path.";
                    evidence { irq_open_page_allocator_per_cpu_pagesets_deferred(IrqOpenPrepareTrimmedPaths, PageAllocator); }
                    close_when: "Per-CPU pageset lifecycle, refill/drain and SMP tests pass.";
                }
                deferred irq_open.002 {
                    category: DeferredCategory::Feature;
                    summary: "Complete real console device probing.";
                    evidence { console_real_device_probe_deferred(Console); }
                    close_when: "Conditional real-console probe and failure tests pass.";
                }
                deferred irq_open.003 {
                    category: DeferredCategory::Protocol;
                    summary: "Unregister the boot console after a real console is ready.";
                    evidence { boot_console_unregister_deferred(ConsoleDriverSet); }
                    close_when: "Boot-console unregister ordering and replay tests pass.";
                }
                deferred irq_open.004 {
                    category: DeferredCategory::Protocol;
                    summary: "Complete the boot-to-real-console handoff protocol.";
                    evidence { irq_open_real_console_handoff_deferred(IrqOpenPreparePhase); }
                    close_when: "Buffer replay, ownership transfer and conditional handoff tests pass.";
                }
                deferred irq_open.005 {
                    category: DeferredCategory::Feature;
                    summary: "Enable the SLUB subsystem through Linux slab_state=FULL.";
                    evidence { irq_open_slub_full_enable_deferred(IrqOpenPreparePhase); }
                    close_when: "slab_sysfs_init and late SLUB enable behavior are modeled and tested.";
                }
                trimmed irq_open.006 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "lockdep_init is absent because CONFIG_DEBUG_LOCK_ALLOC=n.";
                    evidence {
                        irq_open_lockdep_init_trimmed_noop(IrqOpenPrepareTrimmedPaths);
                        irq_open_lockdep_trimmed_because_config_debug_lock_alloc_disabled(IrqOpenPrepareTrimmedPaths);
                    }
                    revisit_when: "The reference configuration enables CONFIG_DEBUG_LOCK_ALLOC.";
                }
                trimmed irq_open.007 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "locking selftests are absent because CONFIG_DEBUG_LOCKING_API_SELFTESTS=n.";
                    evidence { irq_open_locking_selftest_trimmed_noop(IrqOpenPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables locking API selftests.";
                }
                trimmed irq_open.008 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "initrd bounds setup is absent because CONFIG_BLK_DEV_INITRD=n.";
                    evidence { irq_open_initrd_bounds_trimmed(IrqOpenPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_BLK_DEV_INITRD.";
                }
                trimmed irq_open.009 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "NUMA policy setup is a no-op because CONFIG_NUMA=n.";
                    evidence { irq_open_numa_policy_trimmed_noop(IrqOpenPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_NUMA.";
                }
                trimmed irq_open.010 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "early ACPI setup is absent because CONFIG_ACPI=n.";
                    evidence { irq_open_acpi_early_trimmed_noop(IrqOpenPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_ACPI.";
                }
                trimmed irq_open.011 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "arch_cpu_finalize_init is absent because CONFIG_ARCH_HAS_CPU_FINALIZE_INIT=n.";
                    evidence { irq_open_arch_cpu_finalize_init_trimmed_noop(IrqOpenPrepareTrimmedPaths); }
                    revisit_when: "The target configuration enables CONFIG_ARCH_HAS_CPU_FINALIZE_INIT.";
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    irq_open_prepare_ready(IrqOpenPreparePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    LocalIrqEnablePhase.state == State::Online;
                    Console.state == State::Prepared;
                    IrqOpenPrepareTrimmedPaths.state == State::Ready;
                    SchedClock.state == State::Ready;
                    DelayLoop.state == State::Ready;
                    slub_flush_workqueue_ready(SlubSubsystem, Workqueue);
                    task_concurrency_closed();
                    smp_concurrency_closed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            IrqTimeInitPhase.state == State::Online;
            LocalIrqEnablePhase.state == State::Online;
            InterruptStream.state == State::Online;
            SbiIpi.state == State::Ready;
            Softirq.state == State::Ready;
            SmpCallFunction.state == State::Ready;
            SlubSubsystem.state == State::Ready;
            KmallocCaches.state == State::Ready;
            Workqueue.state == State::Prepared;
            Console.state == State::Prepared;
            TtyLineDisciplineRegistry.state == State::Prepared;
            ConsoleDriverSet.state == State::Prepared;
            IrqOpenPrepareTrimmedPaths.state == State::Ready;
            SchedClock.state == State::Ready;
            DelayLoop.state == State::Ready;
            irq_open_prepare_ready(IrqOpenPreparePhase);
            interrupt_concurrency_open_for_boot_cpu();
            task_concurrency_closed();
            smp_concurrency_closed();
            boot_cpu_local_irq_enabled();
            early_boot_irqs_disabled_false();
            softirq_execution_closed(Softirq);
            workqueue_workers_not_running(Workqueue);
            sbi_ipi_enable_deferred(SbiIpi);
            smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
            slub_flush_workqueue_ready(SlubSubsystem, Workqueue);
            irq_open_page_allocator_per_cpu_pagesets_deferred(IrqOpenPrepareTrimmedPaths, PageAllocator);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    irq_open_prepare_ready(IrqOpenPreparePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    LocalIrqEnablePhase.state == State::Online;
                    Console.state == State::Prepared;
                    IrqOpenPrepareTrimmedPaths.state == State::Ready;
                    SchedClock.state == State::Ready;
                    DelayLoop.state == State::Ready;
                    slub_flush_workqueue_ready(SlubSubsystem, Workqueue);
                    task_concurrency_closed();
                    smp_concurrency_closed();
                }
            }
        }
    }

    state State::Online {
    }
}
