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
 */

object LocalIrqEnablePhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IrqTimeInitPhase.state == State::Ready;
                    IrqController.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                    Tick.state == State::Ready;
                    TimerWheel.state == State::Ready;
                    HrtimerCore.state == State::Ready;
                    Softirq.state == State::Ready;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    Randomness.state == State::Ready;
                    SbiIpi.state == State::Ready;
                    IpiMux.state == State::Ready;
                    SmpCallFunction.state == State::Ready;
                    InterruptStream.state == State::Ready;
                    cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
                    early_boot_irqs_disabled_true();
                }

                drives {
                    InterruptStream.Transition::Enable;
                }

                ensures {
                    local_irq_enable_phase_ready(LocalIrqEnablePhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    boot_cpu_local_irq_enabled();
                    early_boot_irqs_disabled_false();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    time_read_smoke_available(RiscvTimerProvider);
                    clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }

                deferred {
                    "local_irq_enable() 只打开 boot CPU sstatus.SIE 总入口；PLIC UART source gate 和 root supervisor external input gate 仍由后续 UartExternalIrqEnable 显式打开。";
                    "周期 tick、完整 softirq 执行、IPI runtime、workqueue worker、RCU GP kthread、task concurrency 和 secondary CPU execution 仍不得随本阶段隐式 Online。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            LocalIrqEnablePhase.state == State::Ready;
            IrqTimeInitPhase.state == State::Ready;
            InterruptStream.state == State::Online;
            local_irq_enable_phase_ready(LocalIrqEnablePhase);
            interrupt_concurrency_open_for_boot_cpu();
            boot_cpu_local_irq_enabled();
            early_boot_irqs_disabled_false();
            task_concurrency_closed();
            smp_concurrency_closed();
            time_read_smoke_available(RiscvTimerProvider);
            clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
        }
    }
}
