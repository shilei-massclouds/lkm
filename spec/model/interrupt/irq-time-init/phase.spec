/*
 * IRQ and Time Init Phase Specification
 *
 * This is InterruptPhase subphase 1. It starts after
 * sched_init/context_tracking_init and ends after the boot CPU
 * local_irq_enable() boundary. Moving the interrupt-open action here lets this
 * phase expose a real IRQ/timer acceptance test while task and SMP concurrency
 * remain closed.
 */

/*
 * IrqController 表示 early_irq_init()/init_IRQ() 建立的 IRQ descriptor、
 * generic IRQ domain 壳。RISC-V 本地中断控制器、IPI mux 和 PLIC 另行建模。
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
                    irq_allocator_minimal_ready(IrqController, PageAllocator, SlubAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_descriptors_ready(IrqController);
            irq_domain_ready(IrqController, DeviceTree);
            irq_allocator_minimal_ready(IrqController, PageAllocator, SlubAllocator);
        }
    }
}

/*
 * RiscvIntc 表示每 hart 直连 CPU 的 RISC-V local interrupt controller。
 * 当前要求 boot CPU 的 INTC domain 建立，并能映射 timer/software/external
 * 三条本地 cause；external provider 仍由 PLIC 占位而不开放运行期路由。
 */
object RiscvIntc: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    IrqController.state == State::Ready;
                    DeviceTree.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    riscv_intc_domain_ready(RiscvIntc, IrqController, DeviceTree);
                    riscv_intc_boot_cpu_local_causes_ready(RiscvIntc, BootCPU);
                    riscv_intc_timer_pin_ready(RiscvIntc);
                    riscv_intc_software_pin_ready(RiscvIntc);
                    riscv_intc_external_pin_ready(RiscvIntc);
                    boot_cpu_timer_irq_mapping_ready(RiscvIntc);
                    boot_cpu_software_irq_mapping_ready(RiscvIntc);
                    boot_cpu_external_irq_mapping_reserved(RiscvIntc);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            riscv_intc_domain_ready(RiscvIntc, IrqController, DeviceTree);
            riscv_intc_boot_cpu_local_causes_ready(RiscvIntc, BootCPU);
            riscv_intc_timer_pin_ready(RiscvIntc);
            riscv_intc_software_pin_ready(RiscvIntc);
            riscv_intc_external_pin_ready(RiscvIntc);
            boot_cpu_timer_irq_mapping_ready(RiscvIntc);
            boot_cpu_software_irq_mapping_ready(RiscvIntc);
            boot_cpu_external_irq_mapping_reserved(RiscvIntc);
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
                    RiscvIntc.state == State::Ready;
                    InterruptStream.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    irq_dispatch_tree_ready(IrqDispatchTree, IrqController, RiscvIntc);
                    irq_dispatch_fallback_route_ready(IrqDispatchTree);
                    irq_dispatch_timer_route_ready(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_software_route_reserved(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_external_route_deferred(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_boot_cpu_route_ready(IrqDispatchTree, BootCPU);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_dispatch_tree_ready(IrqDispatchTree, IrqController, RiscvIntc);
            irq_dispatch_fallback_route_ready(IrqDispatchTree);
            irq_dispatch_timer_route_ready(IrqDispatchTree, RiscvIntc);
            irq_dispatch_software_route_reserved(IrqDispatchTree, RiscvIntc);
            irq_dispatch_external_route_deferred(IrqDispatchTree, RiscvIntc);
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
                    RiscvIntc.state == State::Ready;
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
                    riscv_timer_irq_mapping_ready(RiscvTimerProvider, RiscvIntc);
                    riscv_timer_interrupt_action_ready(IrqDispatchTree, RiscvTimerProvider);
                    riscv_timer_sbi_programming_ready(RiscvTimerProvider, SBI);
                    riscv_time_read_action_available(RiscvTimerProvider);
                    riscv_clockevent_oneshot_action_available(RiscvTimerProvider, IrqDispatchTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            riscv_timebase_ready(RiscvTimerProvider, DeviceTree);
            riscv_clocksource_registered(RiscvTimerProvider, ClocksourceCore);
            riscv_clockevent_registered(RiscvTimerProvider);
            riscv_timer_irq_mapping_ready(RiscvTimerProvider, RiscvIntc);
            riscv_timer_interrupt_action_ready(IrqDispatchTree, RiscvTimerProvider);
            riscv_timer_sbi_programming_ready(RiscvTimerProvider, SBI);
            riscv_time_read_action_available(RiscvTimerProvider);
            riscv_clockevent_oneshot_action_available(RiscvTimerProvider, IrqDispatchTree);
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
                    RiscvIntc.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    sbi_ipi_ready(SbiIpi, SBI, CpuGroup);
                    sbi_ipi_irq_mapping_ready(SbiIpi, RiscvIntc);
                    sbi_ipi_send_action_ready(SbiIpi, SBI);
                    sbi_ipi_enable_deferred(SbiIpi);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            sbi_ipi_ready(SbiIpi, SBI, CpuGroup);
            sbi_ipi_irq_mapping_ready(SbiIpi, RiscvIntc);
            sbi_ipi_send_action_ready(SbiIpi, SBI);
            sbi_ipi_enable_deferred(SbiIpi);
        }
    }
}

/*
 * IpiMux 表示 generic IPI-Mux：多个虚拟 IPI 复用到 SBI software IRQ。
 * 当前只要求 boot CPU 上的 mux domain 和基础虚拟 IPI range 可见，secondary
 * CPU enable 仍随 SMP 并发路径后续展开。
 */
object IpiMux: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SbiIpi.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    ipi_mux_domain_ready(IpiMux, SbiIpi);
                    ipi_mux_per_cpu_bits_ready(IpiMux, PerCpuStorage);
                    ipi_mux_virtual_ipi_range_ready(IpiMux);
                    ipi_mux_parent_software_irq_ready(IpiMux, SbiIpi);
                    ipi_mux_secondary_enable_deferred(IpiMux);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ipi_mux_domain_ready(IpiMux, SbiIpi);
            ipi_mux_per_cpu_bits_ready(IpiMux, PerCpuStorage);
            ipi_mux_virtual_ipi_range_ready(IpiMux);
            ipi_mux_parent_software_irq_ready(IpiMux, SbiIpi);
            ipi_mux_secondary_enable_deferred(IpiMux);
        }
    }
}

/*
 * Plic 表示 RISC-V 外部中断 provider 的占位。当前阶段只确认 PLIC/PLIC-like
 * provider 发现路径被保留，不创建 external IRQ 运行期路由。
 */
object Plic: InterruptObject {
    initial_state: State::Base;
    parent: RiscvIntc;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    RiscvIntc.state == State::Ready;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    plic_provider_discovery_reserved(Plic, DeviceTree);
                    plic_external_parent_reserved(Plic, RiscvIntc);
                    plic_external_irq_route_deferred(Plic);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            plic_provider_discovery_reserved(Plic, DeviceTree);
            plic_external_parent_reserved(Plic, RiscvIntc);
            plic_external_irq_route_deferred(Plic);
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
                    IpiMux.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    smp_call_function_ready(SmpCallFunction, CpuGroup);
                    call_single_queue_ready(SmpCallFunction, PerCpuStorage);
                    smp_call_function_ipi_route_ready(SmpCallFunction, IpiMux);
                    smp_call_function_ipi_mux_ready(SmpCallFunction, IpiMux);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            smp_call_function_ready(SmpCallFunction, CpuGroup);
            call_single_queue_ready(SmpCallFunction, PerCpuStorage);
            smp_call_function_ipi_route_ready(SmpCallFunction, IpiMux);
            smp_call_function_ipi_mux_ready(SmpCallFunction, IpiMux);
        }
    }
}

/*
 * IrqTimeInitPhase 表示 InterruptPhase 的第一个子阶段。它在阶段末尾打开
 * boot CPU 本地中断总入口，使后续 smoke 可以真实验收 timer interrupt。
 */
object IrqTimeInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

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
                    RiscvIntc.Event::Setup;
                    IrqDispatchTree.Event::Setup;
                    Plic.Event::Preset;
                    Tick.Event::Preset;
                    TimerWheel.Event::Setup;
                    HrtimerCore.Event::Setup;
                    Timekeeper.Event::Setup;
                    RiscvTimerProvider.Event::Setup;
                    Tick.Event::Setup;
                    Softirq.Event::Setup;
                    Randomness.Event::Setup;
                    SbiIpi.Event::Setup;
                    IpiMux.Event::Setup;
                    SmpCallFunction.Event::Setup;
                    InterruptStream.Event::Enable;
                }

                ensures {
                    irq_time_init_ready(IrqTimeInitPhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    time_read_smoke_available(RiscvTimerProvider);
                    clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }

                deferred {
                    "early_irq_init() 的完整 irq_desc allocator 细节暂缓；当前只要求 descriptor/domain 壳和 timer IRQ mapping。";
                    "perf_event_init() 暂缓：perf core 和 PMU registry 不属于当前最小 IRQ/timer 验收闭环。";
                    "profile_init() 暂缓：profile buffer 和 proc export 后续再建模。";
                    "late_time_init hook 不在本阶段执行；当前 RISC-V 路径无 hook。";
                    "RiscvTimerProvider.enable() 暂缓：正式周期 tick 服务属于中断打开后的运行期推进。";
                    "PLIC external IRQ route 暂缓：当前只保留 provider discovery/parent reserved，不开放外部中断 handler。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_time_init_ready(IrqTimeInitPhase);
            IrqController.state == State::Ready;
            RiscvIntc.state == State::Ready;
            IrqDispatchTree.state == State::Ready;
            Plic.state == State::Prepared;
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
            IpiMux.state == State::Ready;
            SbiIpi.state == State::Ready;
            SmpCallFunction.state == State::Ready;
            InterruptStream.state == State::Online;
            interrupt_concurrency_open_for_boot_cpu();
            task_concurrency_closed();
            smp_concurrency_closed();
            time_read_smoke_available(RiscvTimerProvider);
            clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
        }
    }
}
