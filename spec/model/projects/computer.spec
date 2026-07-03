/*
 * Computer Project Specification
 *
 * ComputerProject is the formal top-level object for the current model.  It
 * keeps the hardware/firmware preconditions needed by the kernel project in
 * scope without expanding the full computer construction model yet.
 */

include "kernel.spec";

object ComputerProject: ProjectObject {
    initial_state: State::Base;

    /*
     * Base 表示计算机工程已经进入模型空间，但尚未确认当前内核工程所需的
     * 计算机规格前置条件。
     */
    state State::Base {
        transitions {
            /*
             * Preset 记录当前项目采用的计算机规格前置：riscv64 ISA 与当前
             * SoC 规格已经可作为后续硬件/固件构造的标准来源。细节暂不展开
             * 为独立硬件工程对象。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                emits {
                    Transition::Setup;
                }

                deferred {
                    riscv64_soc_standard_established();
                }
            }
        }
    }

    /*
     * Prepared 表示计算机规格前置已经建立，后续可以构造硬件/固件并启动内核工程。
     */
    state State::Prepared {
        invariant {
            Riscv64.state == State::Online;
        }

        transitions {
            /*
             * Setup 建立当前项目所需的硬件/固件前置，使内核工程可以被启动。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                }

                emits {
                    Transition::Enable;
                }

                deferred {
                    computer_hardware_constructed();
                    computer_firmware_constructed();
                }
            }
        }
    }

    /*
     * Ready 表示当前计算机工程已经完成硬件/固件前置，可以启动内核工程。
     */
    state State::Ready {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSbiFirmware.state == State::Online;
        }

        transitions {
            /*
             * Enable 构造并评估内核工程产物；KernelProject 自身的完成事件链
             * 负责从 Preset 自动推进到 Online。
             */
            on Transition::Enable -> State::Online {
                drives {
                    KernelProject.Transition::Preset;
                }
            }
        }
    }

    /*
     * Online 表示包含硬件、固件、内核和 selected payload 的完整系统已在线。
     */
    state State::Online {
        invariant {
            KernelProject.state == State::Online;
        }
    }
}
