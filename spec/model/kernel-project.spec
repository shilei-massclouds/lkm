/*
 * Kernel Project Specification
 *
 * KernelProject models the lifecycle of the kernel engineering work product:
 * establish the kernel system specification, build the image, then boot and
 * evaluate the resulting kernel instance.
 */

include "kernel.spec";

object KernelProject: ProjectObject {
    initial_state: State::Base;
    parent: ComputerProject;

    /*
     * Base 表示内核工程已经进入模型空间，但尚未建立内核系统规格。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立内核系统规格和工程模型前置。
             */
            on Transition::Preset -> State::Prepared {
                ensures {
                    kernel_system_spec_established();
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    /*
     * Prepared 表示内核系统规格已经建立，可以在规格约束下生成代码并组装 image。
     */
    state State::Prepared {
        invariant {
            kernel_system_spec_established();
        }

        transitions {
            /*
             * Setup 定义 lds/conf、生成代码并构造内核 image。
             */
            on Transition::Setup -> State::Ready {
                ensures {
                    kernel_image_constructed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示内核 image 已经构造完成，可以启动并评估。
     */
    state State::Ready {
        invariant {
            kernel_image_constructed();
        }

        transitions {
            /*
             * Enable 启动内核实例并执行测试/评估；Kernel 自身的完成事件链
             * 负责从 Preset 自动推进到 Online。
             */
            on Transition::Enable -> State::Online {
                drives {
                    Kernel.Transition::Preset;
                }
            }
        }
    }

    /*
     * Online 表示内核工程产物已经启动为在线内核实例。
     */
    state State::Online {
        invariant {
            Kernel.state == State::Online;
        }
    }
}
