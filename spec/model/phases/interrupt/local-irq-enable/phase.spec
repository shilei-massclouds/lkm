/*
 * Local IRQ Enable Phase Specification
 *
 * This is InterruptPhase subphase 2. It starts after IrqTimeInitPhase has
 * completed IRQ/time initialization with boot CPU local interrupts still
 * disabled. It covers the start_kernel() boundary that clears
 * early_boot_irqs_disabled and executes local_irq_enable().
 */

/*
 * LocalIrqEnablePhase isolates the interrupt-open boundary from the globally
 * exclusive IRQ/time initialization body. It opens only the boot CPU local
 * interrupt total gate (sstatus.SIE); individual source gates, periodic tick,
 * full softirq execution, IPI runtime, workqueue workers, RCU GP kthreads,
 * task concurrency and SMP concurrency remain closed or deferred.
 *
 * This phase deliberately has no outer `within` context. The operation itself
 * is the boundary that changes the boot CPU local interrupt context from
 * disabled to enabled, so no lexical context can truthfully hold for the whole
 * transition.
 */

object LocalIrqEnablePhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    IrqTimeInitPhase.state == State::Online;
                    IrqController.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    Tick.state == State::Ready;
                    TimerWheel.state == State::Ready;
                    HrtimerCore.state == State::Ready;
                    Softirq.state == State::Ready;
                    Workqueue.state == State::Prepared;
                    RcuCore.state == State::Ready;
                    TasksRcu.state == State::Prepared;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    Randomness.state == State::Ready;
                    SbiIpi.state == State::Ready;
                    IpiMux.state == State::Ready;
                    SmpCallFunction.state == State::Ready;
                    InterruptStream.state == State::Ready;
                    cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
                    early_boot_irqs_disabled_true();
                    irq_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    plic_irq_domain_enable_deferred(PlicIrqDomain);
                    irq_handler_registry_source_enable_deferred(IrqHandlerRegistry);
                }

                drives {
                    InterruptStream.Transition::Enable;
                }

                ensures {
                    local_irq_enable_phase_ready(LocalIrqEnablePhase);
                    local_irq_enable_phase_has_no_within_context(LocalIrqEnablePhase);
                    early_boot_irqs_disabled_cleared_before_local_irq_enable(LocalIrqEnablePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    boot_cpu_local_irq_enabled();
                    early_boot_irqs_disabled_false();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    irq_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    riscv_intc_external_input_enable_deferred(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
                    plic_irq_domain_enable_deferred(PlicIrqDomain);
                    irq_handler_registry_source_enable_deferred(IrqHandlerRegistry);
                    softirq_execution_closed(Softirq);
                    workqueue_workers_not_running(Workqueue);
                    rcu_gp_threads_deferred(RcuCore);
                    tasks_rcu_gp_threads_deferred(TasksRcu);
                    sbi_ipi_enable_deferred(SbiIpi);
                    smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
                    time_read_smoke_available(RiscvTimerProvider);
                    clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }

                deferred {
                    "local_irq_enable() 只打开 boot CPU sstatus.SIE 总入口；PLIC UART source gate 和 root supervisor external input gate 仍由后续 UartExternalIrqEnable 显式打开。";
                    "周期 tick、完整 softirq 执行、IPI runtime、workqueue worker、RCU GP kthread、task concurrency 和 secondary CPU execution 仍不得随本阶段隐式 Online。";
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
                    local_irq_enable_phase_ready(LocalIrqEnablePhase);
                    local_irq_enable_phase_has_no_within_context(LocalIrqEnablePhase);
                    early_boot_irqs_disabled_cleared_before_local_irq_enable(LocalIrqEnablePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    InterruptStream.state == State::Online;
                    boot_cpu_local_irq_enabled();
                    early_boot_irqs_disabled_false();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            LocalIrqEnablePhase.state == State::Ready;
            IrqTimeInitPhase.state == State::Online;
            InterruptStream.state == State::Online;
            PlicIrqDomain.state == State::Ready;
            IrqHandlerRegistry.state == State::Ready;
            Softirq.state == State::Ready;
            Workqueue.state == State::Prepared;
            RcuCore.state == State::Ready;
            TasksRcu.state == State::Prepared;
            SbiIpi.state == State::Ready;
            IpiMux.state == State::Ready;
            SmpCallFunction.state == State::Ready;
            local_irq_enable_phase_ready(LocalIrqEnablePhase);
            local_irq_enable_phase_has_no_within_context(LocalIrqEnablePhase);
            early_boot_irqs_disabled_cleared_before_local_irq_enable(LocalIrqEnablePhase);
            interrupt_concurrency_open_for_boot_cpu();
            boot_cpu_local_irq_enabled();
            early_boot_irqs_disabled_false();
            task_concurrency_closed();
            smp_concurrency_closed();
            irq_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            riscv_intc_external_input_enable_deferred(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
            plic_irq_domain_enable_deferred(PlicIrqDomain);
            irq_handler_registry_source_enable_deferred(IrqHandlerRegistry);
            softirq_execution_closed(Softirq);
            workqueue_workers_not_running(Workqueue);
            rcu_gp_threads_deferred(RcuCore);
            tasks_rcu_gp_threads_deferred(TasksRcu);
            sbi_ipi_enable_deferred(SbiIpi);
            smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
            time_read_smoke_available(RiscvTimerProvider);
            clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    local_irq_enable_phase_ready(LocalIrqEnablePhase);
                    local_irq_enable_phase_has_no_within_context(LocalIrqEnablePhase);
                    early_boot_irqs_disabled_cleared_before_local_irq_enable(LocalIrqEnablePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    InterruptStream.state == State::Online;
                    boot_cpu_local_irq_enabled();
                    early_boot_irqs_disabled_false();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }
            }
        }
    }

    state State::Online {
    }
}
