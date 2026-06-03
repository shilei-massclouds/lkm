/*
 * IRQ-Open Prepare Phase Specification
 *
 * This is InterruptPhase subphase 2. It starts after IrqTimeInitPhase has
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
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    PrintkBuffer.state == State::Ready;
                    EarlyCon.state == State::Online;
                    StaticObjects.state == State::Online;
                }

                drives {
                    TtyLineDisciplineRegistry.Event::Preset;
                    ConsoleDriverSet.Event::Preset;
                }

                ensures {
                    console_prepared(Console, PrintkBuffer);
                    console_initcall_table_scanned(Console, StaticObjects);
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
            console_initcall_table_scanned(Console, StaticObjects);
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
        events {
            on Event::Preset -> State::Prepared {
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
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    StaticObjects.state == State::Online;
                }

                ensures {
                    console_driver_set_early_registered(ConsoleDriverSet);
                    serial_console_probe_deferred(ConsoleDriverSet);
                    boot_console_unregister_deferred(ConsoleDriverSet);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
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
object SchedClock: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    HrtimerCore.state == State::Ready;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    StaticBranch.state == State::Ready;
                }

                ensures {
                    sched_clock_ready(SchedClock, RiscvTimerProvider);
                    sched_clock_running_key_enabled(SchedClock, StaticBranch);
                    sched_clock_reader_ready(SchedClock);
                    sched_clock_timer_ready(SchedClock, HrtimerCore);
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RiscvTimerProvider.state == State::Ready;
                    BootCPU.state == State::Online;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    delay_loop_ready(DelayLoop, RiscvTimerProvider);
                    lpj_fine_consumed_from_timer_provider(DelayLoop, RiscvTimerProvider);
                    boot_cpu_loops_per_jiffy_ready(DelayLoop, BootCPU);
                    global_loops_per_jiffy_ready(DelayLoop);
                    delay_api_actions_available(DelayLoop);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            delay_loop_ready(DelayLoop, RiscvTimerProvider);
            boot_cpu_loops_per_jiffy_ready(DelayLoop, BootCPU);
            global_loops_per_jiffy_ready(DelayLoop);
            delay_api_actions_available(DelayLoop);
        }
    }
}

/*
 * IrqOpenPreparePhase 表示 InterruptPhase 的第二个子阶段。它承接已开放
 * boot CPU 本地中断总入口的事实，建立中断开放后到进程准备期前的 late
 * core/platform 准备边界。
 */
object IrqOpenPreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    IrqTimeInitPhase.state == State::Ready;
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
                    SlubAllocator.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                }

                drives {
                    Console.Event::Preset;
                    SchedClock.Event::Setup;
                    DelayLoop.Event::Setup;
                }

                ensures {
                    irq_open_prepare_ready(IrqOpenPreparePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    boot_cpu_local_irq_enabled();
                    early_boot_irqs_disabled_false();
                    slub_flush_workqueue_ready(SlubAllocator, Workqueue);
                    page_allocator_per_cpu_pagesets_deferred(PageAllocator);
                    lockdep_path_trimmed();
                    locking_selftest_path_trimmed();
                    initrd_bounds_path_trimmed();
                    numa_policy_path_trimmed();
                    acpi_early_path_trimmed();
                    late_time_init_hook_trimmed();
                    arch_cpu_finalize_init_trimmed();
                    next_interrupt_subphase_is_process_prepare();
                }

                deferred {
                    "setup_per_cpu_pageset() 作为 PageAllocator.setup() 的 per-CPU pageset 快速路径细项暂缓，不引入新 lifecycle slot。";
                    "完整 console device probe、boot console 注销和 real console handoff 属于条件结果或后续设备初始化，不作为本阶段固定后置条件。";
                    "SlubAllocator.enable()/Linux slab_state=FULL 留给 slab_sysfs_init() 等后续 late initcall，不在本阶段推进。";
                    "Lockdep、locking selftest、initrd bounds、NUMA policy、ACPI early、late_time_init hook 和 arch_cpu_finalize_init 在当前 RISC-V default_config 下为 trimmed/no-op。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            IrqTimeInitPhase.state == State::Ready;
            InterruptStream.state == State::Online;
            SlubAllocator.state == State::Ready;
            KmallocCaches.state == State::Ready;
            Workqueue.state == State::Prepared;
            Console.state == State::Prepared;
            TtyLineDisciplineRegistry.state == State::Prepared;
            ConsoleDriverSet.state == State::Prepared;
            SchedClock.state == State::Ready;
            DelayLoop.state == State::Ready;
            irq_open_prepare_ready(IrqOpenPreparePhase);
            interrupt_concurrency_open_for_boot_cpu();
            task_concurrency_closed();
            smp_concurrency_closed();
            boot_cpu_local_irq_enabled();
            early_boot_irqs_disabled_false();
            slub_flush_workqueue_ready(SlubAllocator, Workqueue);
        }
    }
}
