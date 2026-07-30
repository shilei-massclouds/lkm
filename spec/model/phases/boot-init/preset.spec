/*
 * BootInitFlow Preset Object Specification
 *
 * These objects support BootInitFlow.Preset from kernel entry through the
 * point where EarlyVm is online and BootInitFlow.Prepared can be committed.
 */

/*
 * BootTask 的定义集中在 task.spec。入口 tp binding 是 CurrentTask 的
 * CPU-local 上下文 Action，不创建独立对象、状态或 CurrentTaskSlot。
 */
predicate boot_task_current_binding_established<C: CPU, T: Task>(cpu: C, task: T) -> bool;
predicate boot_task_current_binding_refreshed_for_active_controller<C: CPU, T: Task>(cpu: C, task: T) -> bool;
predicate boot_task_preemption_is_static_initial_property<T: Task>(task: T) -> bool;
predicate boot_task_task_stack_binding_diagnostic_clear() -> bool;
predicate boot_task_stack_current_binding_established<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate boot_task_stack_current_binding_refreshed_for_active_controller<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;

/* Early virtual-memory systems are defined independently in objects/*.spec. */

/*
 * BootCpuRegisters 表示 CpuGroup.cpus[0] 天然存在且可访问的启动相关寄存器子集。
 * Online 只保证这些寄存器属性可访问；a0/a1 由 OpenSBI 交接确定，
 * 其余寄存器由内核入口阶段逐步更新。
 */
object BootCpuRegisters: HardwareObject {
    initial_state: State::Online;
    parent: CpuGroup.cpus[0];
    source: hardware::boot_cpu_registers;

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

    state State::Online {
        invariant {
            attrs_accessible(self);
        }
    }
}

/*
 * Soc 表示片上系统平台对象。入口前导期只建模平台早期预置的边界。
 */
predicate soc_full_early_platform_model_deferred<T>(soc: T) -> bool;

object Soc: HardwareObject {
    initial_state: State::Base;
    parent: Kernel;

    /*
     * Base 表示 SoC 平台早期预置尚未执行。
     */
    state State::Base {
        transitions {
            /*
             * Preset 执行 SoC 平台相关的早期预置。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    CpuGroup.state == State::Prepared;
                }

                may_change {
                    // 待补充：平台相关早期状态点。
                }

                ensures {
                    soc_early_platform_ready();
                    soc_full_early_platform_model_deferred(Soc);
                }
            }
        }
    }

    /*
     * Prepared 表示 SoC 平台早期预置已完成到入口前导期所需边界。
     */
    state State::Prepared {
        invariant {
            CpuGroup.state == State::Prepared;
            soc_early_platform_ready();
            soc_full_early_platform_model_deferred(Soc);
        }

        deferred soc.001 {
            category: DeferredCategory::ModelDetail;
            summary: "Define the complete SoC-specific early platform state points beyond boot CPU organization.";
            evidence { soc_full_early_platform_model_deferred(Soc); }
            close_when: "The supported SoC early platform facts and their implementation checkpoints are explicitly modeled and tested.";
        }
    }
}
