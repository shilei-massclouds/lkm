/*
 * IRQ and Time Init Phase Specification
 *
 * This subphase starts after sched_init/context_tracking_init and now ends
 * after the boot CPU local_irq_enable() boundary. Moving the interrupt-open
 * action here lets this phase expose a real IRQ/timer acceptance test while
 * task and SMP concurrency remain closed.
 */

/*
 * IrqController 表示 early_irq_init()/init_IRQ() 建立的 IRQ descriptor、
 * IRQ domain 和 RISC-V INTC 基础。当前只要求 boot CPU timer interrupt 可映射。
 */
object IrqController: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    SlubAllocator.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    irq_descriptors_ready(IrqController);
                    irq_domain_ready(IrqController, DeviceTree);
                    riscv_intc_domain_ready(IrqController, DeviceTree);
                    boot_cpu_timer_irq_mapping_ready(IrqController);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_descriptors_ready(IrqController);
            irq_domain_ready(IrqController, DeviceTree);
            riscv_intc_domain_ready(IrqController, DeviceTree);
            boot_cpu_timer_irq_mapping_ready(IrqController);
        }
    }
}

/*
 * IrqDispatchTree 表示 generic IRQ 分派树/路由表。它承接 IRQ domain
 * 映射结果，把硬件 interrupt cause 路由到具体 handler action；当前最小
 * 闭环只要求 RISC-V supervisor timer interrupt route 可用。
 */
object IrqDispatchTree: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    IrqController.state == State::Ready;
                    InterruptStream.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    irq_dispatch_tree_ready(IrqDispatchTree, IrqController);
                    irq_dispatch_fallback_route_ready(IrqDispatchTree);
                    irq_dispatch_timer_route_ready(IrqDispatchTree, IrqController);
                    irq_dispatch_boot_cpu_route_ready(IrqDispatchTree, BootCPU);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_dispatch_tree_ready(IrqDispatchTree, IrqController);
            irq_dispatch_fallback_route_ready(IrqDispatchTree);
            irq_dispatch_timer_route_ready(IrqDispatchTree, IrqController);
            irq_dispatch_boot_cpu_route_ready(IrqDispatchTree, BootCPU);
        }
    }
}

/*
 * Tick 表示 tick_init() 建立的 tick/broadcast 控制壳。RISC-V timer
 * provider 注册 clockevent 后补完 boot CPU tick device，使其 Ready。
 */
object Tick: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    TickBroadcast.Event::Preset;
                }

                ensures {
                    tick_control_ready(Tick, CpuGroup);
                    tick_nohz_trimmed(Tick);
                    tick_broadcast_prepared(TickBroadcast);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            TickBroadcast.state == State::Prepared;
            tick_control_ready(Tick, CpuGroup);
            tick_broadcast_prepared(TickBroadcast);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RiscvTimerProvider.state == State::Ready;
                }

                drives {
                    TickBroadcast.Event::Setup;
                }

                ensures {
                    boot_cpu_tick_device_ready(Tick, RiscvTimerProvider);
                    tick_broadcast_ready(TickBroadcast);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            TickBroadcast.state == State::Ready;
            boot_cpu_tick_device_ready(Tick, RiscvTimerProvider);
            tick_broadcast_ready(TickBroadcast);
        }
    }
}

object TickBroadcast: KernelObject {
    initial_state: State::Base;
    parent: Tick;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                ensures {
                    tick_broadcast_masks_ready(TickBroadcast);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            tick_broadcast_masks_ready(TickBroadcast);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    HrtimerCore.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                }

                ensures {
                    tick_broadcast_masks_ready(TickBroadcast);
                    tick_broadcast_clockevent_ready(TickBroadcast, RiscvTimerProvider);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tick_broadcast_masks_ready(TickBroadcast);
            tick_broadcast_clockevent_ready(TickBroadcast, RiscvTimerProvider);
        }
    }
}

/*
 * TimerWheel 表示 init_timers() 建立的低精度 timer wheel 基础。
 */
object TimerWheel: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Softirq.state == State::Prepared;
                    BootInitTask.state == State::Online;
                }

                ensures {
                    timer_wheel_ready(TimerWheel, CpuGroup);
                    cpu_timer_bases_ready(TimerWheel, PerCpuStorage);
                    boot_init_task_posix_cpu_timer_work_ready(BootInitTask);
                    timer_softirq_registered(Softirq, TimerWheel);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            timer_wheel_ready(TimerWheel, CpuGroup);
            cpu_timer_bases_ready(TimerWheel, PerCpuStorage);
            boot_init_task_posix_cpu_timer_work_ready(BootInitTask);
            timer_softirq_registered(Softirq, TimerWheel);
        }
    }
}

/*
 * HrtimerCore 表示 hrtimers_init() 建立的高精度 timer 基础。
 */
object HrtimerCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Softirq.state == State::Prepared;
                }

                ensures {
                    hrtimer_core_ready(HrtimerCore, CpuGroup);
                    boot_cpu_hrtimer_base_ready(HrtimerCore);
                    hrtimer_softirq_registered(Softirq, HrtimerCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            hrtimer_core_ready(HrtimerCore, CpuGroup);
            boot_cpu_hrtimer_base_ready(HrtimerCore);
            hrtimer_softirq_registered(Softirq, HrtimerCore);
        }
    }
}

/*
 * Timekeeper 表示 timekeeping_init() 建立的 timekeeper 与默认 clocksource。
 */
object Timekeeper: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Tick.state == State::Prepared;
                    StaticBranch.state == State::Ready;
                }

                drives {
                    ClocksourceCore.Event::Preset;
                    JiffiesClocksource.Event::Preset;
                }

                ensures {
                    timekeeper_ready(Timekeeper);
                    wall_time_basis_ready(Timekeeper);
                    monotonic_time_basis_ready(Timekeeper);
                    raw_time_basis_ready(Timekeeper);
                    current_clocksource_is_jiffies(Timekeeper, JiffiesClocksource);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ClocksourceCore.state == State::Prepared;
            JiffiesClocksource.state == State::Prepared;
            timekeeper_ready(Timekeeper);
            wall_time_basis_ready(Timekeeper);
            monotonic_time_basis_ready(Timekeeper);
            raw_time_basis_ready(Timekeeper);
            current_clocksource_is_jiffies(Timekeeper, JiffiesClocksource);
        }
    }
}

object ClocksourceCore: KernelObject {
    initial_state: State::Base;
    parent: Timekeeper;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                ensures {
                    clocksource_core_registry_ready(ClocksourceCore);
                    clocksource_watchdog_deferred(ClocksourceCore);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            clocksource_core_registry_ready(ClocksourceCore);
            clocksource_watchdog_deferred(ClocksourceCore);
        }
    }
}

object JiffiesClocksource: KernelObject {
    initial_state: State::Base;
    parent: Timekeeper;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                ensures {
                    jiffies_clocksource_available(JiffiesClocksource);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            jiffies_clocksource_available(JiffiesClocksource);
        }
    }
}

/*
 * RiscvTimerProvider 表示 RISC-V time_init() 的 timer provider 路径。
 */
object RiscvTimerProvider: HardwareObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    IrqController.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                    Timekeeper.state == State::Ready;
                    HrtimerCore.state == State::Ready;
                    Tick.state == State::Prepared;
                    SBI.state == State::Ready;
                }

                ensures {
                    riscv_timebase_ready(RiscvTimerProvider, DeviceTree);
                    riscv_clocksource_registered(RiscvTimerProvider, ClocksourceCore);
                    riscv_clockevent_registered(RiscvTimerProvider);
                    riscv_timer_irq_mapping_ready(RiscvTimerProvider, IrqController);
                    riscv_timer_interrupt_action_ready(IrqDispatchTree, RiscvTimerProvider);
                    riscv_timer_sbi_programming_ready(RiscvTimerProvider, SBI);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            riscv_timebase_ready(RiscvTimerProvider, DeviceTree);
            riscv_clocksource_registered(RiscvTimerProvider, ClocksourceCore);
            riscv_clockevent_registered(RiscvTimerProvider);
            riscv_timer_irq_mapping_ready(RiscvTimerProvider, IrqController);
            riscv_timer_interrupt_action_ready(IrqDispatchTree, RiscvTimerProvider);
            riscv_timer_sbi_programming_ready(RiscvTimerProvider, SBI);
        }
    }
}

/*
 * SbiIpi 表示 RISC-V smp_prepare_cpus() 前的 SBI IPI 基础。
 */
object SbiIpi: HardwareObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SBI.state == State::Ready;
                    IrqController.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    sbi_ipi_ready(SbiIpi, SBI, CpuGroup);
                    sbi_ipi_irq_mapping_ready(SbiIpi, IrqController);
                    sbi_ipi_enable_deferred(SbiIpi);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            sbi_ipi_ready(SbiIpi, SBI, CpuGroup);
            sbi_ipi_irq_mapping_ready(SbiIpi, IrqController);
            sbi_ipi_enable_deferred(SbiIpi);
        }
    }
}

/*
 * SmpCallFunction 表示 call_function_init() 建立的 SMP function-call 基础。
 */
object SmpCallFunction: TaskObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SbiIpi.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    smp_call_function_ready(SmpCallFunction, CpuGroup);
                    call_single_queue_ready(SmpCallFunction, PerCpuStorage);
                    smp_call_function_ipi_route_ready(SmpCallFunction, SbiIpi);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            smp_call_function_ready(SmpCallFunction, CpuGroup);
            call_single_queue_ready(SmpCallFunction, PerCpuStorage);
            smp_call_function_ipi_route_ready(SmpCallFunction, SbiIpi);
        }
    }
}

/*
 * IrqTimeInitPhase 表示 BootPhase 的第六个子阶段。它在阶段末尾打开
 * boot CPU 本地中断总入口，使后续 smoke 可以真实验收 timer interrupt。
 */
object IrqTimeInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SchedInitPhase.state == State::Ready;
                    Scheduler.state == State::Online;
                    RcuCore.state == State::Ready;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Prepared;
                    Randomness.state == State::Prepared;
                    InterruptStream.state == State::Ready;
                }

                drives {
                    IrqController.Event::Setup;
                    IrqDispatchTree.Event::Setup;
                    Tick.Event::Preset;
                    TimerWheel.Event::Setup;
                    HrtimerCore.Event::Setup;
                    Timekeeper.Event::Setup;
                    RiscvTimerProvider.Event::Setup;
                    Tick.Event::Setup;
                    Softirq.Event::Setup;
                    Randomness.Event::Setup;
                    SbiIpi.Event::Setup;
                    SmpCallFunction.Event::Setup;
                    InterruptStream.Event::Enable;
                }

                ensures {
                    irq_time_init_ready(IrqTimeInitPhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    boot_cpu_timer_interrupt_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }

                deferred {
                    "early_irq_init() 的完整 irq_desc allocator 细节暂缓；当前只要求 descriptor/domain 壳和 timer IRQ mapping。";
                    "perf_event_init() 暂缓：perf core 和 PMU registry 不属于当前最小 IRQ/timer 验收闭环。";
                    "profile_init() 暂缓：profile buffer 和 proc export 后续再建模。";
                    "late_time_init hook 不在本阶段执行；当前 RISC-V 路径无 hook。";
                    "RiscvTimerProvider.enable() 暂缓：正式周期 tick 服务属于中断打开后的运行期推进。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_time_init_ready(IrqTimeInitPhase);
                    IrqController.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                    Tick.state == State::Ready;
            TickBroadcast.state == State::Ready;
            TimerWheel.state == State::Ready;
            HrtimerCore.state == State::Ready;
            Timekeeper.state == State::Ready;
            ClocksourceCore.state == State::Prepared;
            JiffiesClocksource.state == State::Prepared;
            RiscvTimerProvider.state == State::Ready;
            Softirq.state == State::Ready;
            Randomness.state == State::Ready;
            SbiIpi.state == State::Ready;
            SmpCallFunction.state == State::Ready;
            InterruptStream.state == State::Online;
            interrupt_concurrency_open_for_boot_cpu();
            task_concurrency_closed();
            smp_concurrency_closed();
            boot_cpu_timer_interrupt_smoke_available(RiscvTimerProvider, IrqDispatchTree);
        }
    }
}
