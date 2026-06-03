/*
 * Interrupt Phase Specification
 *
 * InterruptPhase starts when IRQ/time facilities begin setup. Its first
 * currently expanded subphases are IrqTimeInitPhase, IrqOpenPreparePhase and
 * ProcessPreparePhase. IrqTimeInitPhase starts with the boot CPU interrupt
 * gate still closed and ends after local_irq_enable() opens that gate.
 * IrqOpenPreparePhase then covers the interrupt-open late core/platform
 * preparation boundary. ProcessPreparePhase prepares PID/task/cred/VMA and
 * security foundations before rest_init() creates the first tasks.
 */

include "irq-time-init/main.spec";
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
        events {
            /*
             * Setup 顺序推进当前已经正式规格化的中断期子阶段。
             * InterruptPhase 不直接依赖 PreparePhase；它从 BootPhase.Ready 接续。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    BootPhase.state == State::Ready;
                    SchedInitPhase.state == State::Ready;
                }

                drives {
                    IrqTimeInitPhase.Event::Setup;
                    IrqOpenPreparePhase.Event::Setup;
                    ProcessPreparePhase.Event::Setup;
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
            IrqOpenPreparePhase.state == State::Ready;
            ProcessPreparePhase.state == State::Ready;
        }
    }
}
