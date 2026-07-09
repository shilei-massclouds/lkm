/*
 * Kernel System Specification
 *
 * This file defines the formal kernel system object and composes the sibling
 * model object and phase trees. The formal directory entry is spec/model/main.spec.
 *
 * Four-level chain:
 * spec/charter/systems/kernel.md -> this model -> spec/coding/systems/kernel.spec
 * -> impl/arceos_ex/src/systems/kernel.rs.
 */

include "../phases/boot/main.spec";
include "../phases/interrupt/main.spec";
include "../phases/up-multitask/main.spec";
include "../phases/smp-runtime/main.spec";
include "../phases/payload/main.spec";

/*
 * Kernel 表示 OpenSBI.Enable 完成控制权交接后触发出的内核系统实例。
 * 它替代旧的临时启动时间轴对象，按内核系统生命周期编排准备期、引导期、
 * 中断期、单核多任务期、多核运行期以及 payload 交接阶段。
 */
object Kernel: KernelObject {
    initial_state: State::Base;
    parent: KernelProject;

    /*
     * Base 表示内核系统规格对象已经进入模型空间，但尚未推进到 Boot ready 边界。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立内核系统规格前置，验证 project-level 启动输入，
             * 推进当前已经展开的引导期阶段，使目标 Prepared 表示 Boot ready。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                drives {
                    BootPhase.Transition::Setup;
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    /*
     * Prepared 表示 project-level 启动输入和引导期边界已经生效。
     */
    state State::Prepared {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSbiFirmware.state == State::Online;
            Lds.state == State::Online;
            Config.state == State::Online;
            BootPhase.state == State::Ready;
        }

        transitions {
            /*
             * Setup 推进中断期阶段，使目标 Ready 表示中断期已完成。
             */
            on Transition::Setup -> State::Ready {
                drives {
                    InterruptPhase.Transition::Setup;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示引导期和中断期阶段已经完成，可以继续推进多任务、
     * SMP/runtime 与 payload 交接。
     */
    state State::Ready {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSbiFirmware.state == State::Online;
            Lds.state == State::Online;
            Config.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
        }

        transitions {
            /*
             * Enable 启动完整内核系统实例，进入多任务、推进 SMP/runtime 和
             * selected payload 不返回交接边界。
             */
            on Transition::Enable -> State::Online {
                drives {
                    UpMultitaskPhase.Transition::Setup;
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
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSbiFirmware.state == State::Online;
            Lds.state == State::Online;
            Config.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
            UpMultitaskPhase.state == State::Ready;
            SmpRuntimePhase.state == State::Ready;
            PayloadPhase.state == State::Online;
        }
    }
}
