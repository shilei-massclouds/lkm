/*
 * Interrupt Phase Specification
 *
 * InterruptPhase starts when IRQ/time facilities begin setup. Its currently
 * expanded subphases are IrqTimeInitPhase, LocalIrqEnablePhase,
 * IrqOpenPreparePhase and ProcessPreparePhase. IrqTimeInitPhase keeps the
 * boot CPU interrupt gate closed so its setup body remains under the global
 * exclusive boot context. LocalIrqEnablePhase then covers the
 * local_irq_enable() boundary. IrqOpenPreparePhase covers the interrupt-open
 * late core/platform preparation boundary. ProcessPreparePhase prepares
 * PID/task/cred/VMA and security foundations before rest_init() creates the
 * first tasks.
 */

include "irq-time-init/main.spec";
include "local-irq-enable/main.spec";
include "irq-open-prepare/main.spec";
include "process-prepare/main.spec";

/*
 * InterruptPhase 表示中断期阶段对象。它负责推进当前模型已经展开的中断期子阶段。
 */
object InterruptPhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    /*
     * Base 表示中断期阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        transitions {
            /*
             * Setup 顺序推进当前已经正式规格化的中断期子阶段。
             * InterruptPhase 不直接依赖 PreparePhase；它从 BootPhase.Ready 接续。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    BootPhase.state == State::Ready;
                    SchedInitPhase.state == State::Ready;
                }

                drives {
                    IrqTimeInitPhase.Transition::Setup;
                    LocalIrqEnablePhase.Transition::Setup;
                    IrqOpenPreparePhase.Transition::Setup;
                    ProcessPreparePhase.Transition::Setup;
                }
            }
        }
    }

    /*
     * Ready 表示当前模型已经展开的中断期子阶段均已完成。
     */
    state State::Ready {
        invariant {
            BootPhase.state == State::Ready;
            SchedInitPhase.state == State::Ready;
            IrqTimeInitPhase.state == State::Ready;
            LocalIrqEnablePhase.state == State::Ready;
            IrqOpenPreparePhase.state == State::Ready;
            ProcessPreparePhase.state == State::Ready;
        }
    }
}
