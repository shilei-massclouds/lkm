/*
 * Kernel System Specification
 *
 * This file defines the formal kernel system object and composes the model
 * phase tree. The formal directory entry is spec/model/main.spec.
 */

include "objects/main.spec";
include "prepare/main.spec";
include "phases/boot/main.spec";
include "phases/interrupt/main.spec";
include "phases/up-multitask/main.spec";
include "phases/smp-runtime/main.spec";
include "phases/payload/main.spec";

/*
 * Kernel 表示 KernelProject.Enable 后启动出来的内核系统实例。
 * 它替代旧的临时启动时间轴对象，按内核系统生命周期编排准备期、引导期、
 * 中断期、单核多任务期、多核运行期以及 payload 交接阶段。
 */
object Kernel: KernelObject {
    initial_state: State::Base;
    parent: KernelProject;

    /*
     * Base 表示内核系统规格对象已经进入模型空间，但尚未推进到中断开启边界。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立内核系统规格前置，推进准备期、当前已经展开的引导期
             * 和中断期阶段，使目标 Prepared 表示中断已开启。
             */
            on Transition::Preset -> State::Prepared {
                drives {
                    PreparePhase.Transition::Setup;
                    PreparePhase.Transition::Enable;
                    BootPhase.Transition::Setup;
                    InterruptPhase.Transition::Setup;
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    /*
     * Prepared 表示准备期、引导期和中断期边界已经生效，本地中断入口和
     * IRQ/time 设施已经建立。
     */
    state State::Prepared {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
        }

        transitions {
            /*
             * Setup 推进单核多任务阶段，使目标 Ready 表示进入多任务。
             */
            on Transition::Setup -> State::Ready {
                drives {
                    UpMultitaskPhase.Transition::Setup;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示中断期阶段已经完成且已经进入单核多任务，可以继续推进
     * SMP/runtime 与 payload 交接。
     */
    state State::Ready {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
            UpMultitaskPhase.state == State::Ready;
        }

        transitions {
            /*
             * Enable 启动完整内核系统实例，推进多核运行和 selected payload
             * 不返回交接边界。
             */
            on Transition::Enable -> State::Online {
                drives {
                    SmpRuntimePhase.Transition::Setup;
                    PayloadPhase.Transition::Setup;
                    PayloadPhase.Transition::Enable;
                }
            }
        }
    }

    /*
     * Online 表示内核启动编排链已经移交给 selected payload，内核系统实例在线。
     */
    state State::Online {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
            UpMultitaskPhase.state == State::Ready;
            SmpRuntimePhase.state == State::Ready;
            PayloadPhase.state == State::Online;
        }
    }
}
