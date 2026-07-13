/*
 * Interrupt Phase Specification
 *
 * InterruptPhase starts when IRQ/time facilities begin setup. Its currently
 * expanded subphases are IrqTimeInitPhase, LocalIrqEnablePhase,
 * IrqOpenPreparePhase and ProcessPreparePhase. IrqTimeInitPhase keeps the
 * boot CPU interrupt gate closed inside SingleTaskContext.
 * LocalIrqEnablePhase then covers the local_irq_enable() boundary without an
 * outer phase context: it is an independent operation whose transition itself
 * changes the boot CPU local interrupt context from disabled to enabled.
 * IrqOpenPreparePhase and ProcessPreparePhase run in the corresponding
 * single-task interrupt-stream context before rest_init() creates the first
 * tasks.
 */

include "irq-time-init/main.spec";
include "local-irq-enable/main.spec";
include "irq-open-prepare/main.spec";
include "process-prepare/main.spec";

context SingleTaskInterruptStreamContext: Context {
    /*
     * This is the boot execution context after LocalIrqEnablePhase. It keeps
     * single-CPU/single-task/non-preemptible facts, but changes
     * local_interrupts to enabled for the boot CPU.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: enabled;
            preemption: disabled;
        }
    }
}

/*
 * InterruptPhase 表示中断期阶段对象。它负责推进当前模型已经展开的中断期子阶段。
 */
object InterruptPhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    /*
     * Base 表示中断期阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        transitions {
            /*
             * Preset 顺序推进当前已经正式规格化的中断期子阶段。
             * InterruptPhase 从 BootPhase.Online 接续。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SchedInitPhase.state == State::Online;
                }

                within SingleTaskContext {
                    drives {
                        IrqTimeInitPhase.Transition::Preset;
                    }
                }

                drives {
                    LocalIrqEnablePhase.Transition::Setup;
                }

                within SingleTaskInterruptStreamContext {
                    drives {
                        IrqOpenPreparePhase.Transition::Setup;
                        ProcessPreparePhase.Transition::Setup;
                    }
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
                    IrqTimeInitPhase.state == State::Online;
                    LocalIrqEnablePhase.state == State::Ready;
                    IrqOpenPreparePhase.state == State::Ready;
                    ProcessPreparePhase.state == State::Ready;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootPhase.state == State::Online;
            SchedInitPhase.state == State::Online;
            IrqTimeInitPhase.state == State::Online;
            LocalIrqEnablePhase.state == State::Ready;
            IrqOpenPreparePhase.state == State::Ready;
            ProcessPreparePhase.state == State::Ready;
        }

        transitions {
            on Transition::Enable -> State::Online {
            }
        }
    }

    state State::Online {
    }
}
