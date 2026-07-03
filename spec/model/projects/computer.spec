/*
 * Computer Project Specification
 *
 * ComputerProject is the formal top-level object for the current model.  It
 * keeps the architecture, boot ABI, and firmware preconditions needed by the
 * kernel project in scope without expanding the full computer construction
 * model yet.
 */

include "../objects/main.spec";

/*
 * Riscv64 表示当前模型引用的 RISC-V 64 位体系结构对象。
 * 它提供入口前导期需要读取或改写的通用寄存器和监管者 CSR。
 */
object Riscv64: IsaObject {
    initial_state: State::Online;
    source: external_spec::riscv_isa;

    attrs {
        a0: Gpr<HartId>;
        a1: Gpr<PhysAddr<Dtb>>;
        sp: Gpr<Addr>;
        tp: Gpr<Addr>;
        gp: Gpr<Addr>;

        sstatus: Csr<Sstatus>;
        sie: Csr<Sie>;
        sip: Csr<Sip>;
        stvec: Csr<TrapVector>;
        sscratch: Csr<usize>;
        satp: Csr<Satp>;
    }

    /*
     * Online 表示体系结构对象在推导起点已经可用。
     * 本状态只要求当前模型声明的体系结构属性都可读取。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
        }
    }
}

/*
 * BootArgs 表示启动 ABI 对入口寄存器的语义解释。
 * RISC-V64 下 a0 是启动 hartid，a1 是原始 dtb 物理地址。
 */
object BootArgs: PrepareObject {
    initial_state: State::Online;

    attrs {
        boot_hartid: HartId;
        dtb_pa: PhysAddr<Dtb>;
    }

    /*
     * Online 表示启动参数在入口前导期开始前已经由启动 ABI 给出。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            boot_hartid == Riscv64.a0;
            dtb_pa == Riscv64.a1;
        }
    }
}

/*
 * SbiSpec 表示 RISC-V SBI 规范中当前模型依赖的 HSM 语义。
 * 它描述规范能力，不描述具体固件版本的实现细节。
 */
object SbiSpec: PrepareObject {
    initial_state: State::Online;
    source: external_spec::riscv_sbi;

    state State::Online {
        invariant {
            sbi_hsm_available();
        }
    }
}

/*
 * OpenSbiFirmware 表示当前启动路径中由 OpenSBI 固件提供的 SBI 交接语义。
 * 它把 SBI 规范能力落实到本次内核入口的固件状态。
 */
object OpenSbiFirmware: PrepareObject {
    initial_state: State::Online;
    source: firmware::opensbi;

    state State::Online {
        invariant {
            SbiSpec.state == State::Online;
            BootArgs.state == State::Online;
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
            firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
        }
    }
}

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
             * Preset 记录当前项目采用的计算机规格前置：Riscv64 ISA 已经
             * 可作为后续硬件/固件构造的标准来源。细节暂不展开为独立硬件
             * 工程对象。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                emits {
                    Transition::Setup;
                }

                deferred {
                    /* 对应 Riscv64 的外部 ISA/SoC 标准事实。 */
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
             * Setup 建立当前项目所需的硬件/固件前置：BootArgs 描述入口
             * ABI 交接，SbiSpec/OpenSbiFirmware 描述固件能力与本次入口状态。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    BootArgs.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                }

                emits {
                    Transition::Enable;
                }

                deferred {
                    /* 对应 BootArgs 所承载的启动硬件/ABI 交接事实。 */
                    computer_hardware_constructed();
                    /* 对应 SbiSpec 与 OpenSbiFirmware 所承载的固件事实。 */
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
            BootArgs.state == State::Online;
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
