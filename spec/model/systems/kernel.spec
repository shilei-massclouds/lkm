/*
 * Kernel System Specification
 *
 * This file defines the formal kernel system object and composes phase trees.
 *
 */

include "../phases/boot/entry-prelude/main.spec";
include "../phases/boot/main.spec";
include "../phases/interrupt/main.spec";
include "../phases/boot-init/main.spec";
include "../phases/smp-runtime/main.spec";
include "../phases/payload/main.spec";

/*
 * Kernel 代表内核运行实例, 由 OpenSBI 交接控制权后启动。
 * 按照内核生命周期驱动 BootInitFlow，并在真实 PID 1 栈入口继续
 * 多核运行期以及应用交接期。
 */
object Kernel: KernelObject {
    initial_state: State::Base;
    parent: KernelProject;

    /*
     * Base 表示内核映像已经驻留在内存中，但是 Kernel 尚未收到启动事件、
     * 尚未开始内核生命周期。
     */
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                /*
                * 检查硬件/固件规范，确保固件服务可用，确认内核Lds和Config存在。
                */
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSBI.state == State::Online;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                drives {
                    BootInitFlow.Transition::Preset;
                }

                ensures {
                    BootInitFlow.state == State::Prepared;
                    EntryPreludePhase.state == State::Online;
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    /*
     * Prepared 状态：入口前导期完成，具备进入引导期的条件。
     */
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    BootInitFlow.Transition::Setup;
                }

                ensures {
                    BootInitFlow.state == State::Ready;
                    BootPhase.state == State::Online;
                    InterruptPhase.state == State::Online;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示入口前导期、引导期和中断期阶段已经完成，可以继续推进多任务、
     * SMP/runtime 与 payload 交接。
     */
    state State::Ready {
        transitions {
            /*
             * Enable 先提交 BootInitFlow.Online，再执行真实首次调度切换；
             * KernelInitTask 的实际入口随后推进 SMP/runtime 和 payload。
             */
            on Transition::Enable -> State::Online {
                drives {
                    BootInitFlow.Transition::Enable;
                    Scheduler.Action::Schedule;
                    SmpRuntimePhase.Transition::Preset;
                    PayloadPhase.Transition::Preset;
                }

                ensures {
                    BootInitFlow.state == State::Online;
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    SmpRuntimePhase.state == State::Online;
                    PayloadPhase.state == State::Online;
                }
            }
        }
    }

    /*
     * Online 表示内核启动编排链已经移交给 selected payload，内核进入正式服务状态。
     */
    state State::Online {
    }
}
