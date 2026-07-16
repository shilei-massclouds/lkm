/*
 * Kernel System Specification
 *
 * This file defines the formal kernel system object and composes phase trees.
 *
 */

include "../phases/boot/entry-prelude/main.spec";
include "../phases/boot/main.spec";
include "../phases/interrupt/main.spec";
include "../phases/up-multitask/main.spec";
include "../phases/smp-runtime/main.spec";
include "../phases/payload/main.spec";

/*
 * Kernel 代表内核运行实例, 由 OpenSBI 交接控制权后启动。
 * 按照内核生命周期编排准备期、引导期、中断期、单核多任务期、
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

                within SingleTaskContext {
                    drives {
                        EntryPreludePhase.Transition::Preset;
                    }
                }

                ensures {
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
                    BootPhase.Transition::Preset;
                    InterruptPhase.Transition::Preset;
                }

                ensures {
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
             * Enable 启动完整内核系统实例，进入多任务、推进 SMP/runtime 和
             * selected payload 不返回交接边界。
             */
            on Transition::Enable -> State::Online {
                drives {
                    UpMultitaskPhase.Transition::Preset;
                    SmpRuntimePhase.Transition::Preset;
                    PayloadPhase.Transition::Preset;
                }

                ensures {
                    UpMultitaskPhase.state == State::Online;
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
