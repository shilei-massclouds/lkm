/*
 * IRQ and Time Init Phase Specification
 *
 * This is InterruptPhase subphase 1. It starts after
 * sched_init/context_tracking_init and ends with IRQ/time infrastructure ready
 * while the boot CPU local interrupt gate remains closed. The following
 * LocalIrqEnablePhase owns the local_irq_enable() boundary, so this phase can
 * remain covered by the global exclusive boot context.
 */

context IrqTimeInitGlobalExclusiveContext: Context {
    /*
     * This context captures the start_kernel() section before
     * local_irq_enable(): one boot CPU, one boot task, local interrupts still
     * disabled, and preemption disabled. The following LocalIrqEnablePhase is
     * intentionally outside this context.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        IrqTimeInitPhase;
        IrqController;
        RiscvIntc;
        IrqChipInitTable;
        PlicDriver;
        PlicIrqDomain;
        IrqHandlerRegistry;
        IrqDispatchTree;
        Tick;
        TimerWheel;
        SrcuCore;
        HrtimerCore;
        Timekeeper;
        RiscvTimerProvider;
        Softirq;
        Randomness;
        BootStackCanary;
        PerfEventCore;
        ProfileCore;
        SbiIpi;
        IpiMux;
        SmpCallFunction;
        InterruptStream;
        BootCpuLocalInterrupt;
    }
}

context IrqControllerDescInitContext: Context {
    /*
     * early_irq_init()/init_IRQ() initializes irq_desc and the generic IRQ
     * domain shell while the boot context is still globally exclusive.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        IrqController;
    }
}

context PlicIrqDomainMappingContext: Context {
    /*
     * PLIC irqdomain setup/mapping owns the logical IRQ allocator and mapping
     * table publication boundary. It defines source gates but does not enable
     * them.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        PlicIrqDomain;
        PlicIrqMapping;
    }
}

context TimerBaseInitContext: Context {
    /*
     * init_timers() initializes per-CPU timer bases, base locks, pending maps
     * and empty wheel vectors before TIMER_SOFTIRQ can run.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        TimerWheel;
        Softirq;
        PerCpuStorage;
    }
}

context HrtimerBaseInitContext: Context {
    /*
     * hrtimers_init() initializes hrtimer cpu bases, clock bases, base locks
     * and empty active queues before HRTIMER_SOFTIRQ can run.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        HrtimerCore;
        Softirq;
        PerCpuStorage;
    }
}

type TimekeeperSeqWriteSectionType: KernelObject {
    processes {
        Transition::Enter {
            state_effect: StateEffect::Conditional;
            ensures {
                timekeeper_seqwrite_entered(TimekeeperSeqWriteSection);
            }
        }

        Transition::Exit {
            state_effect: StateEffect::Conditional;
            ensures {
                timekeeper_seqwrite_exited(TimekeeperSeqWriteSection);
            }
        }
    }
}

object TimekeeperSeqWriteSection: TimekeeperSeqWriteSectionType {
    initial_state: State::Ready;

    state State::Ready {
    }
}

context TimekeeperSeqWriteContext: Context {
    /*
     * timekeeping_init() writes tk_core under the timekeeper seqcount writer
     * protocol. Reader retry semantics remain a later runtime refinement.
     */
    guard {
        entered_by {
            TimekeeperSeqWriteSection.Transition::Enter;
        }

        exited_by {
            TimekeeperSeqWriteSection.Transition::Exit;
        }
    }

    obj_refs {
        Timekeeper;
        ClocksourceCore;
        JiffiesClocksource;
    }
}

context SrcuBootListDrainContext: Context {
    /*
     * srcu_init() flips srcu_init_done and drains the boot SRCU list while
     * timer/workqueue infrastructure needed for delayed SRCU callbacks exists.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        SrcuCore;
        RcuCore;
        TimerWheel;
        Workqueue;
    }
}

context BootStackCanaryInitContext: Context {
    /*
     * boot_init_stack_canary() depends on random_init() and seeds the current
     * boot task/per-task canary while ordinary task concurrency is still closed.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        BootStackCanary;
        Randomness;
        BootInitStack;
    }
}

context PerfPmusSrcuInitContext: Context {
    /*
     * perf_event_init() initializes PMU registry state and pmus_srcu before
     * runtime PMU registration or event allocation is opened.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        PerfEventCore;
        SrcuCore;
        CpuGroup;
    }
}

context SmpCallFunctionInitContext: Context {
    /*
     * call_function_init() initializes per-CPU call_single queues and locks.
     * Runtime IPI delivery remains deferred until interrupt/SMP concurrency is
     * explicitly opened.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }

    obj_refs {
        SmpCallFunction;
        IpiMux;
        PerCpuStorage;
    }
}

/*
 * IrqController 表示 early_irq_init()/init_IRQ() 建立的 IRQ descriptor、
 * generic IRQ domain 壳。RISC-V 本地中断控制器、IPI mux 和 PLIC 另行建模。
 */
object IrqController: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                within IrqControllerDescInitContext {
                    ensures {
                        irq_controller_desc_init_context_used(IrqController);
                        irq_desc_locks_ready(IrqController);
                        sparse_irq_tree_lock_deferred(IrqController);
                        irq_domain_mutex_deferred(IrqController);
                    }
                }

                ensures {
                    irq_descriptors_ready(IrqController);
                    irq_domain_ready(IrqController, DeviceTree);
                    irq_allocator_minimal_ready(IrqController, PageAllocator, SlubSubsystem);
                    irq_desc_locks_ready(IrqController);
                    sparse_irq_tree_lock_deferred(IrqController);
                    irq_domain_mutex_deferred(IrqController);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_descriptors_ready(IrqController);
            irq_domain_ready(IrqController, DeviceTree);
            irq_allocator_minimal_ready(IrqController, PageAllocator, SlubSubsystem);
            irq_desc_locks_ready(IrqController);
            sparse_irq_tree_lock_deferred(IrqController);
            irq_domain_mutex_deferred(IrqController);
        }
    }
}

/*
 * IrqChipInitTable 表示 irqchip_init()/of_irq_init() 可遍历的 irqchip
 * init entry 视图。当前 target 必须由 retained LDS section 承载；
 * init_IRQ() 调用链只能通过遍历该 section 找到具体 irqchip driver。
 */
object IrqChipInitTable: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    IrqController.state == State::Ready;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    irqchip_init_table_static_entries_ready(IrqChipInitTable);
                    irqchip_init_table_lds_section_ready(IrqChipInitTable);
                    irqchip_init_table_entry_view_ready(IrqChipInitTable);
                    of_irq_init_interrupt_controller_scan_ready(IrqChipInitTable, DeviceTree);
                    of_irq_init_parent_first_order_ready(IrqChipInitTable, DeviceTree);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            irqchip_init_table_static_entries_ready(IrqChipInitTable);
            irqchip_init_table_lds_section_ready(IrqChipInitTable);
            irqchip_init_table_entry_view_ready(IrqChipInitTable);
            of_irq_init_interrupt_controller_scan_ready(IrqChipInitTable, DeviceTree);
            of_irq_init_parent_first_order_ready(IrqChipInitTable, DeviceTree);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PlicDriver.state == State::Prepared;
                    RiscvIntc.state == State::Ready;
                    DeviceTree.state == State::Ready;
                    Ioremap.state == State::Ready;
                    VmallocAllocator.state == State::Ready;
                    PageTableCaches.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                }

                drives {
                    Plic.Transition::Preset;
                }

                ensures {
                    init_irq_calls_irqchip_init(IrqTimeInitPhase, IrqChipInitTable);
                    irqchip_init_calls_of_irq_init(IrqChipInitTable);
                    of_irq_init_traverses_irqchip_lds_section(IrqChipInitTable);
                    of_irq_init_invokes_plic_init_callback(IrqChipInitTable, PlicDriver, Plic);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irqchip_init_table_static_entries_ready(IrqChipInitTable);
            irqchip_init_table_lds_section_ready(IrqChipInitTable);
            irqchip_init_table_entry_view_ready(IrqChipInitTable);
            of_irq_init_interrupt_controller_scan_ready(IrqChipInitTable, DeviceTree);
            of_irq_init_parent_first_order_ready(IrqChipInitTable, DeviceTree);
            init_irq_calls_irqchip_init(IrqTimeInitPhase, IrqChipInitTable);
            irqchip_init_calls_of_irq_init(IrqChipInitTable);
            of_irq_init_traverses_irqchip_lds_section(IrqChipInitTable);
            of_irq_init_invokes_plic_init_callback(IrqChipInitTable, PlicDriver, Plic);
        }
    }
}

/*
 * RiscvIntc 表示每 hart 直连 CPU 的 RISC-V local interrupt controller。
 * 当前要求 boot CPU 的 INTC domain 建立，并能映射 timer/software/external
 * 三条本地 cause。所有中断类型必须通过命名 cause ref 表达；RISC-V
 * supervisor external interrupt 在本模型中命名为
 * InterruptCauseRef::SupervisorExternalIrq，代码中可对应 EXT_IRQ/SEI
 * 等架构命名，但不得在规格正文中依赖裸数字。
 *
 * SupervisorExternalIrq 必须能作为 root entry 转交给 PLIC chained
 * handler，不能在 root INTC 内直接知道 UART 等叶子设备。它的输入
 * gate 独立于 sstatus.SIE 总开关：只有 root INTC/InterruptStream 的
 * supervisor external enable gate 打开后，PLIC 输出才可能进入 CPU。
 */
object RiscvIntc: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    processes {
        Action::EnableExternalInput(gate: IrqGateRef) -> IrqGateRef {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                irq_gate_defined(self, gate);
                irq_gate_closed(self, gate);
                riscv_intc_external_input_enable_deferred(self, gate, InterruptCauseRef::SupervisorExternalIrq);
            }
            ensures {
                irq_gate_open(self, gate);
                riscv_intc_external_input_enabled(self, gate, InterruptCauseRef::SupervisorExternalIrq);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IrqController.state == State::Ready;
                    DeviceTree.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    riscv_intc_domain_ready(RiscvIntc, IrqController, DeviceTree);
                    riscv_intc_boot_cpu_local_causes_ready(RiscvIntc, BootCPU);
                    riscv_intc_timer_pin_ready(RiscvIntc);
                    riscv_intc_software_pin_ready(RiscvIntc);
                    riscv_intc_external_pin_ready(RiscvIntc);
                    boot_cpu_timer_irq_mapping_ready(RiscvIntc);
                    boot_cpu_software_irq_mapping_ready(RiscvIntc);
                    boot_cpu_external_irq_mapping_reserved(RiscvIntc);
                    riscv_intc_external_irq_named_cause(RiscvIntc, InterruptCauseRef::SupervisorExternalIrq);
                    riscv_intc_external_irq_entry_ready(RiscvIntc);
                    riscv_intc_external_irq_does_not_dispatch_leaf_device(RiscvIntc);
                    irq_gate_defined(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    irq_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    riscv_intc_external_input_gate_defined(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
                    riscv_intc_external_input_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    riscv_intc_external_input_enable_deferred(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            riscv_intc_domain_ready(RiscvIntc, IrqController, DeviceTree);
            riscv_intc_boot_cpu_local_causes_ready(RiscvIntc, BootCPU);
            riscv_intc_timer_pin_ready(RiscvIntc);
            riscv_intc_software_pin_ready(RiscvIntc);
            riscv_intc_external_pin_ready(RiscvIntc);
            boot_cpu_timer_irq_mapping_ready(RiscvIntc);
            boot_cpu_software_irq_mapping_ready(RiscvIntc);
            boot_cpu_external_irq_mapping_reserved(RiscvIntc);
            riscv_intc_external_irq_named_cause(RiscvIntc, InterruptCauseRef::SupervisorExternalIrq);
            riscv_intc_external_irq_entry_ready(RiscvIntc);
            riscv_intc_external_irq_does_not_dispatch_leaf_device(RiscvIntc);
            irq_gate_defined(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            irq_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            riscv_intc_external_input_gate_defined(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
            riscv_intc_external_input_gate_closed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            riscv_intc_external_input_enable_deferred(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
        }
    }
}

/*
 * IrqDispatchTree 表示 generic IRQ 分派树/路由表。它承接 IRQ domain
 * 映射结果，把硬件 interrupt cause 路由到具体 handler action；当前要求
 * timer route 可用，并建立 SupervisorExternalIrq -> PLIC chained handler ->
 * PLIC irqdomain -> IRQ action 的运行期分发契约。这个对象描述软件
 * 溯源/分发链，不替代物理传播链上的 source enable/input enable gate。
 */
object IrqDispatchTree: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IrqController.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    Plic.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    InterruptStream.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    irq_dispatch_tree_ready(IrqDispatchTree, IrqController, RiscvIntc);
                    irq_dispatch_fallback_route_ready(IrqDispatchTree);
                    irq_dispatch_timer_route_ready(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_software_route_reserved(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_external_route_ready(IrqDispatchTree, RiscvIntc, Plic);
                    irq_dispatch_external_route_uses_named_cause(IrqDispatchTree, InterruptCauseRef::SupervisorExternalIrq);
                    irq_dispatch_external_route_uses_plic_chained_handler(IrqDispatchTree, Plic);
                    irq_dispatch_external_route_uses_plic_irq_domain(IrqDispatchTree, PlicIrqDomain);
                    irq_dispatch_external_route_claims_before_dispatch(IrqDispatchTree, Plic);
                    irq_dispatch_external_route_completes_after_handler(IrqDispatchTree, Plic);
                    irq_dispatch_boot_cpu_route_ready(IrqDispatchTree, BootCPU);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_dispatch_tree_ready(IrqDispatchTree, IrqController, RiscvIntc);
            irq_dispatch_fallback_route_ready(IrqDispatchTree);
            irq_dispatch_timer_route_ready(IrqDispatchTree, RiscvIntc);
            irq_dispatch_software_route_reserved(IrqDispatchTree, RiscvIntc);
            irq_dispatch_external_route_ready(IrqDispatchTree, RiscvIntc, Plic);
            irq_dispatch_external_route_uses_named_cause(IrqDispatchTree, InterruptCauseRef::SupervisorExternalIrq);
            irq_dispatch_external_route_uses_plic_chained_handler(IrqDispatchTree, Plic);
            irq_dispatch_external_route_uses_plic_irq_domain(IrqDispatchTree, PlicIrqDomain);
            irq_dispatch_external_route_claims_before_dispatch(IrqDispatchTree, Plic);
            irq_dispatch_external_route_completes_after_handler(IrqDispatchTree, Plic);
            irq_dispatch_boot_cpu_route_ready(IrqDispatchTree, BootCPU);
        }
    }
}

/*
 * IrqDomain 是 IRQ core 的通用映射契约：负责把硬件 IRQ 编号/
 * interrupt specifier 翻译并映射为 logical IRQ。它不负责硬件 enable、
 * handler 注册或 interrupt dispatch。
 */
type IrqDomain: InterruptObject {
    processes {
        Action::TranslateIrqSpecifier(resource: PlatformIrqResourceRef) -> HwirqRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                irq_domain_translate_specifier_ready(self);
                platform_irq_resource_raw_specifier_ready(resource);
            }
            ensures {
                irq_domain_translated_specifier(self, resource);
                hwirq_ref_ready(HwirqRef::PlicUart0);
            }
        }

        Action::MapHwirq(hwirq: HwirqRef, mapping: PlicIrqMappingRef) -> LogicalIrqRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                irq_domain_logical_irq_allocator_ready(self);
                irq_domain_mapping_table_ready(self);
                hwirq_ref_ready(hwirq);
            }
            ensures {
                irq_domain_hwirq_mapped(self, hwirq, LogicalIrqRef::Uart0);
                irq_domain_mapping_record_ready(self, mapping);
                logical_irq_ref_ready(LogicalIrqRef::Uart0);
            }
        }

        Action::ResolveHwirq(hwirq: HwirqRef) -> LogicalIrqRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                irq_domain_mapping_table_ready(self);
                hwirq_ref_ready(hwirq);
                irq_domain_hwirq_mapped(self, hwirq, LogicalIrqRef::Uart0);
            }
            ensures {
                irq_domain_resolves_hwirq_to_logical_irq(self, hwirq, LogicalIrqRef::Uart0);
                logical_irq_ref_ready(LogicalIrqRef::Uart0);
            }
        }

        Action::EnableSourceGate(hwirq: HwirqRef, gate: IrqGateRef) -> IrqGateRef {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                irq_domain_hwirq_mapped(self, hwirq, LogicalIrqRef::Uart0);
                irq_gate_defined(PlicIrqMapping, gate);
                irq_gate_closed(PlicIrqMapping, gate);
                plic_irq_mapping_source_enable_deferred(PlicIrqMapping, gate);
            }
            ensures {
                irq_gate_open(PlicIrqMapping, gate);
                plic_irq_mapping_source_enabled(PlicIrqMapping, gate);
            }
        }
    }
}

type HwirqRef {
}

type InterruptCauseRef {
}

type IrqGateRef {
}

/*
 * IrqGate 表示中断物理传播链上的受控开关。gate 的身份和当前状态可以在
 * 建立拓扑/映射时定义；Enable 是后续显式动作。当前阶段只要求两个 gate
 * 被命名并保持 closed，不能因为建立 route/mapping/request_irq 而隐式打开。
 */
type IrqGate: InterruptObject {
    processes {
        Action::Enable(gate: IrqGateRef) -> IrqGateRef {
            state_effect: StateEffect::Conditional;
            depends_on {
                irq_gate_defined(self, gate);
                irq_gate_closed(self, gate);
            }
            ensures {
                irq_gate_open(self, gate);
            }
        }
    }
}

type LogicalIrqRef {
}

type PlicIrqMappingRef {
}

type PlatformIrqResourceRef {
}

type IrqActionRef {
}

predicate irq_domain_owner_irqchip<T, C>(domain: T, irqchip: C) -> bool;
predicate irq_domain_hwirq_valid_range_ready<T>(domain: T) -> bool;
predicate irq_domain_logical_irq_allocator_ready<T>(domain: T) -> bool;
predicate irq_domain_mapping_table_ready<T>(domain: T) -> bool;
predicate irq_domain_translate_specifier_ready<T>(domain: T) -> bool;
predicate irq_domain_translated_specifier<T, R>(domain: T, resource: R) -> bool;
predicate irq_domain_hwirq_mapped<T, H, L>(domain: T, hwirq: H, logical_irq: L) -> bool;
predicate irq_domain_mapping_record_ready<T, M>(domain: T, mapping: M) -> bool;
predicate irq_domain_resolves_hwirq_to_logical_irq<T, H, L>(domain: T, hwirq: H, logical_irq: L) -> bool;
predicate hwirq_ref_ready<T>(hwirq: T) -> bool;
predicate logical_irq_ref_ready<T>(logical_irq: T) -> bool;
predicate irq_gate_defined<T, G>(owner: T, gate: G) -> bool;
predicate irq_gate_closed<T, G>(owner: T, gate: G) -> bool;
predicate irq_gate_open<T, G>(owner: T, gate: G) -> bool;
predicate irq_gate_enable_action_committed<T, G>(owner: T, gate: G) -> bool;

predicate plic_irq_domain_owner_bound<T, P>(domain: T, plic: P) -> bool;
predicate plic_irq_domain_source_range_bound<T, P>(domain: T, plic: P) -> bool;
predicate plic_irq_domain_source_zero_reserved<T>(domain: T) -> bool;
predicate plic_irq_domain_one_cell_specifier<T>(domain: T) -> bool;
predicate plic_irq_domain_mapping_table_ready<T>(domain: T) -> bool;
predicate plic_irq_domain_logical_allocator_ready<T>(domain: T) -> bool;
predicate plic_irq_domain_translate_ops_ready<T>(domain: T) -> bool;
predicate plic_irq_domain_enable_deferred<T>(domain: T) -> bool;
predicate plic_irq_domain_dispatch_ops_ready<T>(domain: T) -> bool;

predicate plic_irq_mapping_domain_bound<T, D>(mapping: T, domain: D) -> bool;
predicate plic_irq_mapping_source_valid<T, P>(mapping: T, plic: P) -> bool;
predicate plic_irq_mapping_source_zero_rejected<T>(mapping: T) -> bool;
predicate plic_irq_mapping_source_range_checked<T, P>(mapping: T, plic: P) -> bool;
predicate plic_irq_mapping_logical_irq_assigned<T, L>(mapping: T, logical_irq: L) -> bool;
predicate plic_irq_mapping_duplicate_source_idempotent<T, D>(mapping: T, domain: D) -> bool;
predicate plic_irq_mapping_source_gate_defined<T, G, H>(mapping: T, gate: G, hwirq: H) -> bool;
predicate plic_irq_mapping_source_gate_closed<T, G>(mapping: T, gate: G) -> bool;
predicate plic_irq_mapping_source_enable_deferred<T, G>(mapping: T, gate: G) -> bool;
predicate plic_irq_mapping_source_enabled<T, G>(mapping: T, gate: G) -> bool;
predicate plic_irq_mapping_source_not_enabled<T>(mapping: T) -> bool;
predicate plic_irq_mapping_handler_not_registered<T>(mapping: T) -> bool;

predicate irq_handler_registry_ready<T>(registry: T) -> bool;
predicate irq_handler_registry_action_table_ready<T>(registry: T) -> bool;
predicate irq_handler_registry_owner_irq_core<T>(registry: T) -> bool;
predicate irq_handler_registry_requires_mapped_logical_irq<T, D>(registry: T, domain: D) -> bool;
predicate irq_handler_registry_duplicate_policy_ready<T>(registry: T) -> bool;
predicate irq_handler_registry_unmapped_reject_ready<T>(registry: T) -> bool;
predicate irq_handler_registry_hardirq_context_guard_ready<T>(registry: T) -> bool;
predicate irq_handler_registry_source_enable_deferred<T>(registry: T) -> bool;
predicate irq_handler_registry_dispatch_ready<T>(registry: T) -> bool;
predicate irq_handler_registry_registered_action<T, A>(registry: T, action: A) -> bool;
predicate irq_handler_registry_dispatch_requires_hardirq_context<T>(registry: T) -> bool;
predicate irq_handler_registry_dispatch_requires_registered_action<T, A>(registry: T, action: A) -> bool;
predicate irq_handler_registry_dispatch_uses_logical_irq<T, L>(registry: T, logical_irq: L) -> bool;

predicate irq_action_logical_irq_bound<T, L>(action: T, logical_irq: L) -> bool;
predicate irq_action_device_bound<T, D>(action: T, device: D) -> bool;
predicate irq_action_handler_bound<T>(action: T) -> bool;
predicate irq_action_hardirq_context_required<T>(action: T) -> bool;
predicate irq_action_mapped_irq_required<T, M>(action: T, mapping: M) -> bool;
predicate irq_action_duplicate_registration_rejected<T>(action: T) -> bool;
predicate irq_action_unmapped_registration_rejected<T>(action: T) -> bool;
predicate irq_action_does_not_enable_source<T>(action: T) -> bool;
predicate irq_action_dispatch_ready<T>(action: T) -> bool;
predicate irq_action_handler_runs_after_plic_claim<T, P>(action: T, plic: P) -> bool;
predicate irq_action_handler_runs_before_plic_complete<T, P>(action: T, plic: P) -> bool;

predicate uart_irq_chain_kunit_observer_ready<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_read_only<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_does_not_call_handler<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_does_not_claim_or_complete<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_does_not_modify_plic_state<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_reads_registered_action<T, R, A>(observer: T, registry: R, action: A) -> bool;
predicate uart_irq_chain_kunit_observer_reads_real_path_result<T, P, R, U>(observer: T, plic: P, registry: R, probe: U) -> bool;
predicate uart_irq_chain_kunit_observer_reads_dispatch_contract<T, P, R>(observer: T, plic: P, registry: R) -> bool;

predicate riscv_intc_external_irq_entry_ready<T>(intc: T) -> bool;
predicate riscv_intc_external_irq_named_cause<T, C>(intc: T, cause: C) -> bool;
predicate riscv_intc_external_irq_forwards_to_plic<T, P>(intc: T, plic: P) -> bool;
predicate riscv_intc_external_irq_does_not_dispatch_leaf_device<T>(intc: T) -> bool;
predicate riscv_intc_external_input_gate_defined<T, G, C>(intc: T, gate: G, cause: C) -> bool;
predicate riscv_intc_external_input_gate_closed<T, G>(intc: T, gate: G) -> bool;
predicate riscv_intc_external_input_enable_deferred<T, G, C>(intc: T, gate: G, cause: C) -> bool;
predicate riscv_intc_external_input_enabled<T, G, C>(intc: T, gate: G, cause: C) -> bool;
predicate irq_dispatch_external_route_uses_named_cause<T, C>(dispatch_tree: T, cause: C) -> bool;
predicate irq_dispatch_external_route_ready<T, R, P>(dispatch_tree: T, riscv_intc: R, plic: P) -> bool;
predicate irq_dispatch_external_route_uses_plic_chained_handler<T, P>(dispatch_tree: T, plic: P) -> bool;
predicate irq_dispatch_external_route_uses_plic_irq_domain<T, D>(dispatch_tree: T, domain: D) -> bool;
predicate irq_dispatch_external_route_claims_before_dispatch<T, P>(dispatch_tree: T, plic: P) -> bool;
predicate irq_dispatch_external_route_completes_after_handler<T, P>(dispatch_tree: T, plic: P) -> bool;
predicate plic_chained_handler_ready<T, R>(plic: T, riscv_intc: R) -> bool;
predicate plic_claim_action_ready<T>(plic: T) -> bool;
predicate plic_complete_action_ready<T>(plic: T) -> bool;
predicate plic_claim_reads_claim_register<T>(plic: T) -> bool;
predicate plic_claim_returns_zero_when_no_pending_source<T>(plic: T) -> bool;
predicate plic_complete_writes_claimed_source<T>(plic: T) -> bool;
predicate plic_chained_handler_claim_loop_until_zero<T>(plic: T) -> bool;
predicate plic_chained_handler_zero_claim_stops_dispatch<T>(plic: T) -> bool;
predicate plic_chained_handler_completes_each_claimed_source<T>(plic: T) -> bool;
predicate plic_claim_before_generic_irq_dispatch<T>(plic: T) -> bool;
predicate plic_complete_after_irq_action_handler<T>(plic: T) -> bool;
predicate uart_external_irq_enable_ready<T>(enable: T) -> bool;
predicate uart_external_irq_enable_opens_plic_source_gate<T, D, G>(enable: T, domain: D, gate: G) -> bool;
predicate uart_external_irq_enable_opens_root_input_gate<T, R, G>(enable: T, riscv_intc: R, gate: G) -> bool;
predicate uart_external_irq_enable_requires_registered_handler<T, A>(enable: T, action: A) -> bool;
predicate uart_external_irq_enable_keeps_uart_trigger_deferred<T, P>(enable: T, plic: P) -> bool;
predicate uart_interrupt_chain_probe_ready<T>(probe: T) -> bool;
predicate uart_interrupt_chain_probe_triggers_uart_once<T, U>(probe: T, uart: U) -> bool;
predicate uart_interrupt_chain_probe_observes_plic_claim<T, P>(probe: T, plic: P) -> bool;
predicate uart_interrupt_chain_probe_observes_irq_dispatch<T, R>(probe: T, registry: R) -> bool;
predicate uart_interrupt_chain_probe_observes_uart_handler<T, A>(probe: T, action: A) -> bool;
predicate uart_interrupt_chain_probe_observes_plic_complete<T, P>(probe: T, plic: P) -> bool;
predicate uart_interrupt_chain_probe_observes_plic_loop_exit<T, P>(probe: T, plic: P) -> bool;
predicate uart_interrupt_chain_probe_observes_irq_cycle_closure<T, P, R>(probe: T, plic: P, registry: R) -> bool;
predicate uart_interrupt_chain_probe_preserves_polling_console<T, U>(probe: T, uart: U) -> bool;

/*
 * IrqHandlerRegistry 是 IRQ core 侧的 handler/action registry。它记录
 * request_irq 风格的 handler 绑定，并提供 logical IRQ -> action 的
 * dispatch 契约；source enable 仍属于 irqchip/source 层。
 */
object IrqHandlerRegistry: InterruptObject {
    initial_state: State::Base;

    processes {
        Action::RequestIrq(logical_irq: LogicalIrqRef, action: IrqActionRef) -> IrqActionRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                PlicIrqDomain.state == State::Ready;
                PlicIrqMapping.state == State::Ready;
                logical_irq_ref_ready(logical_irq);
                irq_handler_registry_action_table_ready(self);
                irq_handler_registry_requires_mapped_logical_irq(self, PlicIrqDomain);
            }

            ensures {
                irq_handler_registry_registered_action(self, action);
                irq_action_logical_irq_bound(action, logical_irq);
                irq_action_mapped_irq_required(action, PlicIrqMappingRef::Uart0);
                irq_action_hardirq_context_required(action);
                irq_action_duplicate_registration_rejected(action);
                irq_action_unmapped_registration_rejected(action);
                irq_action_does_not_enable_source(action);
                irq_action_dispatch_ready(action);
                irq_handler_registry_dispatch_requires_registered_action(self, action);
            }
        }

        Action::Dispatch(logical_irq: LogicalIrqRef) -> IrqActionRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                irq_handler_registry_dispatch_ready(self);
                irq_handler_registry_registered_action(self, IrqActionRef::Ns16550aUart);
                irq_handler_registry_dispatch_requires_hardirq_context(self);
                logical_irq_ref_ready(logical_irq);
            }

            ensures {
                irq_handler_registry_dispatch_uses_logical_irq(self, logical_irq);
                irq_handler_registry_dispatch_requires_registered_action(self, IrqActionRef::Ns16550aUart);
                irq_action_dispatch_ready(IrqAction);
                irq_action_handler_runs_after_plic_claim(IrqAction, Plic);
                irq_action_handler_runs_before_plic_complete(IrqAction, Plic);
            }
        }
    }

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IrqController.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                }

                ensures {
                    irq_handler_registry_ready(IrqHandlerRegistry);
                    irq_handler_registry_action_table_ready(IrqHandlerRegistry);
                    irq_handler_registry_owner_irq_core(IrqHandlerRegistry);
                    irq_handler_registry_requires_mapped_logical_irq(IrqHandlerRegistry, PlicIrqDomain);
                    irq_handler_registry_duplicate_policy_ready(IrqHandlerRegistry);
                    irq_handler_registry_unmapped_reject_ready(IrqHandlerRegistry);
                    irq_handler_registry_hardirq_context_guard_ready(IrqHandlerRegistry);
                    irq_handler_registry_source_enable_deferred(IrqHandlerRegistry);
                    irq_handler_registry_dispatch_ready(IrqHandlerRegistry);
                    irq_handler_registry_dispatch_requires_hardirq_context(IrqHandlerRegistry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_handler_registry_ready(IrqHandlerRegistry);
            irq_handler_registry_action_table_ready(IrqHandlerRegistry);
            irq_handler_registry_owner_irq_core(IrqHandlerRegistry);
            irq_handler_registry_requires_mapped_logical_irq(IrqHandlerRegistry, PlicIrqDomain);
            irq_handler_registry_duplicate_policy_ready(IrqHandlerRegistry);
            irq_handler_registry_unmapped_reject_ready(IrqHandlerRegistry);
            irq_handler_registry_hardirq_context_guard_ready(IrqHandlerRegistry);
            irq_handler_registry_source_enable_deferred(IrqHandlerRegistry);
            irq_handler_registry_dispatch_ready(IrqHandlerRegistry);
            irq_handler_registry_dispatch_requires_hardirq_context(IrqHandlerRegistry);
        }
    }
}

/*
 * IrqAction 表示 request_irq() 创建的一条 logical IRQ -> handler 记录。
 * 它要求 hardirq context，并允许被 IRQ core dispatch；但本对象仍不拥有
 * PLIC source enable，不能把 UART console 提前声明为 interrupt-driven。
 */
object IrqAction: InterruptObject {
    initial_state: State::Base;
    parent: IrqHandlerRegistry;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IrqHandlerRegistry.state == State::Ready;
                    PlicIrqMapping.state == State::Ready;
                }

                drives {
                    IrqHandlerRegistry.Action::RequestIrq(LogicalIrqRef::Uart0, IrqActionRef::Ns16550aUart);
                }

                ensures {
                    irq_handler_registry_registered_action(IrqHandlerRegistry, IrqActionRef::Ns16550aUart);
                    irq_action_logical_irq_bound(IrqAction, LogicalIrqRef::Uart0);
                    irq_action_device_bound(IrqAction, DeviceRef::Ns16550aSerial);
                    irq_action_handler_bound(IrqAction);
                    irq_action_hardirq_context_required(IrqAction);
                    irq_action_mapped_irq_required(IrqAction, PlicIrqMappingRef::Uart0);
                    irq_action_duplicate_registration_rejected(IrqAction);
                    irq_action_unmapped_registration_rejected(IrqAction);
                    irq_action_does_not_enable_source(IrqAction);
                    irq_action_dispatch_ready(IrqAction);
                    irq_action_handler_runs_after_plic_claim(IrqAction, Plic);
                    irq_action_handler_runs_before_plic_complete(IrqAction, Plic);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_handler_registry_registered_action(IrqHandlerRegistry, IrqActionRef::Ns16550aUart);
            irq_action_logical_irq_bound(IrqAction, LogicalIrqRef::Uart0);
            irq_action_device_bound(IrqAction, DeviceRef::Ns16550aSerial);
            irq_action_handler_bound(IrqAction);
            irq_action_hardirq_context_required(IrqAction);
            irq_action_mapped_irq_required(IrqAction, PlicIrqMappingRef::Uart0);
            irq_action_duplicate_registration_rejected(IrqAction);
            irq_action_unmapped_registration_rejected(IrqAction);
            irq_action_does_not_enable_source(IrqAction);
            irq_action_dispatch_ready(IrqAction);
            irq_action_handler_runs_after_plic_claim(IrqAction, Plic);
            irq_action_handler_runs_before_plic_complete(IrqAction, Plic);
        }
    }
}

/*
 * UartIrqChainKunitObserver 是 checkpoint KUnit 侧的只读观察者。
 * 它建立测试检查机制，但不得充当或干预中断处理流程。
 */
object UartIrqChainKunitObserver: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Plic.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    IrqAction.state == State::Ready;
                }

                ensures {
                    uart_irq_chain_kunit_observer_ready(UartIrqChainKunitObserver);
                    uart_irq_chain_kunit_observer_read_only(UartIrqChainKunitObserver);
                    uart_irq_chain_kunit_observer_does_not_call_handler(UartIrqChainKunitObserver);
                    uart_irq_chain_kunit_observer_does_not_claim_or_complete(UartIrqChainKunitObserver);
                    uart_irq_chain_kunit_observer_does_not_modify_plic_state(UartIrqChainKunitObserver);
                    uart_irq_chain_kunit_observer_reads_registered_action(
                        UartIrqChainKunitObserver,
                        IrqHandlerRegistry,
                        IrqActionRef::Ns16550aUart
                    );
                    uart_irq_chain_kunit_observer_reads_real_path_result(
                        UartIrqChainKunitObserver,
                        Plic,
                        IrqHandlerRegistry,
                        UartInterruptChainProbe
                    );
                    uart_irq_chain_kunit_observer_reads_dispatch_contract(
                        UartIrqChainKunitObserver,
                        Plic,
                        IrqHandlerRegistry
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uart_irq_chain_kunit_observer_ready(UartIrqChainKunitObserver);
            uart_irq_chain_kunit_observer_read_only(UartIrqChainKunitObserver);
            uart_irq_chain_kunit_observer_does_not_call_handler(UartIrqChainKunitObserver);
            uart_irq_chain_kunit_observer_does_not_claim_or_complete(UartIrqChainKunitObserver);
            uart_irq_chain_kunit_observer_does_not_modify_plic_state(UartIrqChainKunitObserver);
            uart_irq_chain_kunit_observer_reads_registered_action(
                UartIrqChainKunitObserver,
                IrqHandlerRegistry,
                IrqActionRef::Ns16550aUart
            );
            uart_irq_chain_kunit_observer_reads_real_path_result(
                UartIrqChainKunitObserver,
                Plic,
                IrqHandlerRegistry,
                UartInterruptChainProbe
            );
            uart_irq_chain_kunit_observer_reads_dispatch_contract(
                UartIrqChainKunitObserver,
                Plic,
                IrqHandlerRegistry
            );
        }
    }
}

/*
 * UartExternalIrqEnable 是 UART 外部中断物理传播链的显式 enable 边界。
 * 它发生在 UART source mapping 和 request_irq action 记录之后，打开
 * PLIC UART source gate 和 root INTC SupervisorExternalIrq input gate；
 * 但不制造 UART interrupt trigger，也不把 serial8250 console 改为
 * interrupt-driven。
 */
object UartExternalIrqEnable: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Plic.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                    PlicIrqMapping.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    IrqAction.state == State::Ready;
                    Uart8250Port.state == State::Ready;
                }

                drives {
                    PlicIrqDomain.Action::EnableSourceGate(HwirqRef::PlicUart0, IrqGateRef::PlicUartSource);
                    RiscvIntc.Action::EnableExternalInput(IrqGateRef::RootSupervisorExternalInput);
                }

                ensures {
                    uart_external_irq_enable_ready(UartExternalIrqEnable);
                    uart_external_irq_enable_requires_registered_handler(UartExternalIrqEnable, IrqAction);
                    uart_external_irq_enable_opens_plic_source_gate(UartExternalIrqEnable, PlicIrqDomain, IrqGateRef::PlicUartSource);
                    uart_external_irq_enable_opens_root_input_gate(UartExternalIrqEnable, RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    irq_gate_open(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    irq_gate_open(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    irq_gate_enable_action_committed(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    irq_gate_enable_action_committed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    plic_irq_mapping_source_enabled(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    riscv_intc_external_input_enabled(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
                    uart_external_irq_enable_keeps_uart_trigger_deferred(UartExternalIrqEnable, Plic);
                    plic_uart_source_trigger_deferred(Plic);
                    uart8250_port_interrupt_output_still_deferred(Uart8250Port);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uart_external_irq_enable_ready(UartExternalIrqEnable);
            uart_external_irq_enable_requires_registered_handler(UartExternalIrqEnable, IrqAction);
            uart_external_irq_enable_opens_plic_source_gate(UartExternalIrqEnable, PlicIrqDomain, IrqGateRef::PlicUartSource);
            uart_external_irq_enable_opens_root_input_gate(UartExternalIrqEnable, RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            irq_gate_open(PlicIrqMapping, IrqGateRef::PlicUartSource);
            irq_gate_open(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            irq_gate_enable_action_committed(PlicIrqMapping, IrqGateRef::PlicUartSource);
            irq_gate_enable_action_committed(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
            plic_irq_mapping_source_enabled(PlicIrqMapping, IrqGateRef::PlicUartSource);
            riscv_intc_external_input_enabled(RiscvIntc, IrqGateRef::RootSupervisorExternalInput, InterruptCauseRef::SupervisorExternalIrq);
            uart_external_irq_enable_keeps_uart_trigger_deferred(UartExternalIrqEnable, Plic);
            plic_uart_source_trigger_deferred(Plic);
            uart8250_port_interrupt_output_still_deferred(Uart8250Port);
        }
    }
}

/*
 * UartInterruptChainProbe 是生产侧的一次性外部中断链验证边界。
 * 它在两个传播 gate 已打开后，按 Linux-like 8250 THRI 形状设置
 * UART interrupt-output 条件、启用 THRI 并制造一次真实 TX empty edge，
 * 之后由真实 trap/root INTC/PLIC/IRQ core 路径完成 claim -> dispatch
 * -> handler -> complete。KUnit 只读取这个结果，不得触发该流程。
 */
object UartInterruptChainProbe: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    UartExternalIrqEnable.state == State::Ready;
                    Plic.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                    PlicIrqMapping.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                    IrqAction.state == State::Ready;
                    Uart8250Port.state == State::Ready;
                    irq_gate_open(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    irq_gate_open(RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                }

                drives {
                    Uart8250Port.Action::TriggerInterrupt(InterruptCauseRef::UartThre);
                    RiscvIntc.Action::HandleExternalInput(InterruptCauseRef::SupervisorExternalIrq);
                    Plic.Action::Claim(HwirqRef::PlicUart0);
                    PlicIrqDomain.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqHandlerRegistry.Action::Dispatch(LogicalIrqRef::Uart0);
                    IrqAction.Action::Handle(IrqActionRef::Ns16550aUart);
                    Plic.Action::Complete(HwirqRef::PlicUart0);
                }

                ensures {
                    uart_interrupt_chain_probe_ready(UartInterruptChainProbe);
                    uart_interrupt_chain_probe_triggers_uart_once(UartInterruptChainProbe, Uart8250Port);
                    uart_interrupt_chain_probe_observes_plic_claim(UartInterruptChainProbe, Plic);
                    uart_interrupt_chain_probe_observes_irq_dispatch(UartInterruptChainProbe, IrqHandlerRegistry);
                    uart_interrupt_chain_probe_observes_uart_handler(UartInterruptChainProbe, IrqAction);
                    uart_interrupt_chain_probe_observes_plic_complete(UartInterruptChainProbe, Plic);
                    uart_interrupt_chain_probe_observes_plic_loop_exit(UartInterruptChainProbe, Plic);
                    uart_interrupt_chain_probe_observes_irq_cycle_closure(UartInterruptChainProbe, Plic, IrqHandlerRegistry);
                    uart_interrupt_chain_probe_preserves_polling_console(UartInterruptChainProbe, Uart8250Port);
                    plic_chained_handler_claim_loop_until_zero(Plic);
                    plic_chained_handler_zero_claim_stops_dispatch(Plic);
                    plic_chained_handler_completes_each_claimed_source(Plic);
                    plic_claim_before_generic_irq_dispatch(Plic);
                    plic_complete_after_irq_action_handler(Plic);
                    irq_action_handler_runs_after_plic_claim(IrqAction, Plic);
                    irq_action_handler_runs_before_plic_complete(IrqAction, Plic);
                    uart8250_port_interrupt_output_still_deferred(Uart8250Port);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uart_interrupt_chain_probe_ready(UartInterruptChainProbe);
            uart_interrupt_chain_probe_triggers_uart_once(UartInterruptChainProbe, Uart8250Port);
            uart_interrupt_chain_probe_observes_plic_claim(UartInterruptChainProbe, Plic);
            uart_interrupt_chain_probe_observes_irq_dispatch(UartInterruptChainProbe, IrqHandlerRegistry);
            uart_interrupt_chain_probe_observes_uart_handler(UartInterruptChainProbe, IrqAction);
            uart_interrupt_chain_probe_observes_plic_complete(UartInterruptChainProbe, Plic);
            uart_interrupt_chain_probe_observes_plic_loop_exit(UartInterruptChainProbe, Plic);
            uart_interrupt_chain_probe_observes_irq_cycle_closure(UartInterruptChainProbe, Plic, IrqHandlerRegistry);
            uart_interrupt_chain_probe_preserves_polling_console(UartInterruptChainProbe, Uart8250Port);
            plic_chained_handler_claim_loop_until_zero(Plic);
            plic_chained_handler_zero_claim_stops_dispatch(Plic);
            plic_chained_handler_completes_each_claimed_source(Plic);
            uart8250_port_interrupt_output_still_deferred(Uart8250Port);
        }
    }
}

/*
 * PlicIrqDomain 是 PLIC provider 创建的具体 IRQ domain 实例。
 * 它只建立 source/specifier 到 logical IRQ 的映射能力，不开放 source
 * enable、handler 或 dispatch。
 */
object PlicIrqDomain: IrqDomain {
    initial_state: State::Base;
    parent: Plic;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Plic.state == State::Ready;
                    IrqController.state == State::Ready;
                }

                ensures {
                    irq_domain_owner_irqchip(PlicIrqDomain, Plic);
                    plic_irq_domain_owner_bound(PlicIrqDomain, Plic);
                    plic_irq_domain_translate_ops_ready(PlicIrqDomain);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            irq_domain_owner_irqchip(PlicIrqDomain, Plic);
            plic_irq_domain_owner_bound(PlicIrqDomain, Plic);
            plic_irq_domain_translate_ops_ready(PlicIrqDomain);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Plic.state == State::Ready;
                    IrqController.state == State::Ready;
                }

                within PlicIrqDomainMappingContext {
                    ensures {
                        plic_irq_domain_mapping_context_used(PlicIrqDomain);
                        plic_irq_domain_mapping_table_ready(PlicIrqDomain);
                        plic_irq_domain_logical_allocator_ready(PlicIrqDomain);
                        plic_irq_domain_enable_deferred(PlicIrqDomain);
                    }
                }

                ensures {
                    irq_domain_hwirq_valid_range_ready(PlicIrqDomain);
                    irq_domain_logical_irq_allocator_ready(PlicIrqDomain);
                    irq_domain_mapping_table_ready(PlicIrqDomain);
                    irq_domain_translate_specifier_ready(PlicIrqDomain);
                    plic_irq_domain_source_range_bound(PlicIrqDomain, Plic);
                    plic_irq_domain_source_zero_reserved(PlicIrqDomain);
                    plic_irq_domain_one_cell_specifier(PlicIrqDomain);
                    plic_irq_domain_mapping_table_ready(PlicIrqDomain);
                    plic_irq_domain_logical_allocator_ready(PlicIrqDomain);
                    plic_irq_domain_enable_deferred(PlicIrqDomain);
                    plic_irq_domain_dispatch_ops_ready(PlicIrqDomain);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_domain_owner_irqchip(PlicIrqDomain, Plic);
            irq_domain_hwirq_valid_range_ready(PlicIrqDomain);
            irq_domain_logical_irq_allocator_ready(PlicIrqDomain);
            irq_domain_mapping_table_ready(PlicIrqDomain);
            irq_domain_translate_specifier_ready(PlicIrqDomain);
            plic_irq_domain_owner_bound(PlicIrqDomain, Plic);
            plic_irq_domain_source_range_bound(PlicIrqDomain, Plic);
            plic_irq_domain_source_zero_reserved(PlicIrqDomain);
            plic_irq_domain_one_cell_specifier(PlicIrqDomain);
            plic_irq_domain_mapping_table_ready(PlicIrqDomain);
            plic_irq_domain_logical_allocator_ready(PlicIrqDomain);
            plic_irq_domain_translate_ops_ready(PlicIrqDomain);
            plic_irq_domain_enable_deferred(PlicIrqDomain);
            plic_irq_domain_dispatch_ops_ready(PlicIrqDomain);
        }
    }
}

/*
 * PlicIrqMapping 是 PlicIrqDomain 内的一条 source -> logical IRQ 记录。
 * Setup 建立映射记录，同时定义 UART source gate 并保持 gate closed /
 * source disabled / handler unregistered。source gate 的 Enable action
 * 后续显式展开，不由 mapping 或 request_irq 隐式执行。
 */
object PlicIrqMapping: InterruptObject {
    initial_state: State::Base;
    parent: PlicIrqDomain;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PlicIrqDomain.state == State::Ready;
                    Plic.state == State::Ready;
                }

                within PlicIrqDomainMappingContext {
                    ensures {
                        plic_irq_domain_mapping_context_used(PlicIrqMapping);
                        plic_irq_mapping_source_gate_defined(PlicIrqMapping, IrqGateRef::PlicUartSource, HwirqRef::PlicUart0);
                        plic_irq_mapping_source_gate_closed(PlicIrqMapping, IrqGateRef::PlicUartSource);
                        plic_irq_mapping_source_enable_deferred(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    }
                }

                ensures {
                    irq_domain_hwirq_mapped(PlicIrqDomain, HwirqRef::PlicUart0, LogicalIrqRef::Uart0);
                    irq_domain_mapping_record_ready(PlicIrqDomain, PlicIrqMappingRef::Uart0);
                    logical_irq_ref_ready(LogicalIrqRef::Uart0);
                    plic_irq_mapping_domain_bound(PlicIrqMapping, PlicIrqDomain);
                    plic_irq_mapping_source_valid(PlicIrqMapping, Plic);
                    plic_irq_mapping_source_zero_rejected(PlicIrqMapping);
                    plic_irq_mapping_source_range_checked(PlicIrqMapping, Plic);
                    plic_irq_mapping_logical_irq_assigned(PlicIrqMapping, LogicalIrqRef::Uart0);
                    plic_irq_mapping_duplicate_source_idempotent(PlicIrqMapping, PlicIrqDomain);
                    irq_gate_defined(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    irq_gate_closed(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    plic_irq_mapping_source_gate_defined(PlicIrqMapping, IrqGateRef::PlicUartSource, HwirqRef::PlicUart0);
                    plic_irq_mapping_source_gate_closed(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    plic_irq_mapping_source_enable_deferred(PlicIrqMapping, IrqGateRef::PlicUartSource);
                    plic_irq_mapping_source_not_enabled(PlicIrqMapping);
                    plic_irq_mapping_handler_not_registered(PlicIrqMapping);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_domain_hwirq_mapped(PlicIrqDomain, HwirqRef::PlicUart0, LogicalIrqRef::Uart0);
            irq_domain_mapping_record_ready(PlicIrqDomain, PlicIrqMappingRef::Uart0);
            logical_irq_ref_ready(LogicalIrqRef::Uart0);
            plic_irq_mapping_domain_bound(PlicIrqMapping, PlicIrqDomain);
            plic_irq_mapping_source_valid(PlicIrqMapping, Plic);
            plic_irq_mapping_source_zero_rejected(PlicIrqMapping);
            plic_irq_mapping_source_range_checked(PlicIrqMapping, Plic);
            plic_irq_mapping_logical_irq_assigned(PlicIrqMapping, LogicalIrqRef::Uart0);
            plic_irq_mapping_duplicate_source_idempotent(PlicIrqMapping, PlicIrqDomain);
            irq_gate_defined(PlicIrqMapping, IrqGateRef::PlicUartSource);
            irq_gate_closed(PlicIrqMapping, IrqGateRef::PlicUartSource);
            plic_irq_mapping_source_gate_defined(PlicIrqMapping, IrqGateRef::PlicUartSource, HwirqRef::PlicUart0);
            plic_irq_mapping_source_gate_closed(PlicIrqMapping, IrqGateRef::PlicUartSource);
            plic_irq_mapping_source_enable_deferred(PlicIrqMapping, IrqGateRef::PlicUartSource);
            plic_irq_mapping_source_not_enabled(PlicIrqMapping);
            plic_irq_mapping_handler_not_registered(PlicIrqMapping);
        }
    }
}

/*
 * Tick 表示 tick_init() 建立的 tick/broadcast 控制壳。RISC-V timer
 * provider 注册 clockevent 后补完 boot CPU tick device，使其 Ready。
 */
object Tick: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    TickBroadcast.Transition::Preset;
                }

                ensures {
                    tick_control_ready(Tick, CpuGroup);
                    tick_nohz_trimmed(Tick);
                    tick_broadcast_prepared(TickBroadcast);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            TickBroadcast.state == State::Prepared;
            tick_control_ready(Tick, CpuGroup);
            tick_broadcast_prepared(TickBroadcast);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    RiscvTimerProvider.state == State::Ready;
                }

                drives {
                    TickBroadcast.Transition::Setup;
                }

                ensures {
                    boot_cpu_tick_device_ready(Tick, RiscvTimerProvider);
                    tick_broadcast_ready(TickBroadcast);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            TickBroadcast.state == State::Ready;
            boot_cpu_tick_device_ready(Tick, RiscvTimerProvider);
            tick_broadcast_ready(TickBroadcast);
        }
    }
}

object TickBroadcast: KernelObject {
    initial_state: State::Base;
    parent: Tick;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    tick_broadcast_masks_ready(TickBroadcast);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            tick_broadcast_masks_ready(TickBroadcast);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    HrtimerCore.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                }

                ensures {
                    tick_broadcast_masks_ready(TickBroadcast);
                    tick_broadcast_clockevent_ready(TickBroadcast, RiscvTimerProvider);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tick_broadcast_masks_ready(TickBroadcast);
            tick_broadcast_clockevent_ready(TickBroadcast, RiscvTimerProvider);
        }
    }
}

/*
 * TimerWheel 表示 init_timers() 建立的低精度 timer wheel 基础。
 */
object TimerWheel: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Softirq.state == State::Prepared;
                    BootInitTask.state == State::Online;
                }

                within TimerBaseInitContext {
                    ensures {
                        timer_base_init_context_used(TimerWheel);
                        timer_base_locks_ready(TimerWheel);
                        timer_base_pending_maps_ready(TimerWheel);
                        timer_base_vectors_empty(TimerWheel);
                    }
                }

                ensures {
                    timer_wheel_ready(TimerWheel, CpuGroup);
                    cpu_timer_bases_ready(TimerWheel, PerCpuStorage);
                    timer_base_locks_ready(TimerWheel);
                    timer_base_pending_maps_ready(TimerWheel);
                    timer_base_vectors_empty(TimerWheel);
                    boot_init_task_posix_cpu_timer_work_ready(BootInitTask);
                    timer_softirq_registered(Softirq, TimerWheel);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            timer_wheel_ready(TimerWheel, CpuGroup);
            cpu_timer_bases_ready(TimerWheel, PerCpuStorage);
            timer_base_locks_ready(TimerWheel);
            timer_base_pending_maps_ready(TimerWheel);
            timer_base_vectors_empty(TimerWheel);
            boot_init_task_posix_cpu_timer_work_ready(BootInitTask);
            timer_softirq_registered(Softirq, TimerWheel);
        }
    }
}

/*
 * SrcuCore 表示 srcu_init() 建立的 TREE_SRCU boot-time 基础。它依赖
 * RCU、timer wheel 和 early workqueue 壳，用于让后续 perf/PMU SRCU
 * 初始化有明确基础；每个 srcu_struct 的具体锁仍按对象使用点延后。
 */
object SrcuCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    RcuCore.state == State::Ready;
                    TimerWheel.state == State::Ready;
                    Workqueue.state == State::Prepared;
                }

                within SrcuBootListDrainContext {
                    ensures {
                        srcu_boot_list_drain_context_used(SrcuCore);
                        srcu_boot_list_drained(SrcuCore);
                        srcu_delayed_work_queueing_ready(SrcuCore, TimerWheel, Workqueue);
                    }
                }

                ensures {
                    tree_srcu_enabled(SrcuCore);
                    srcu_init_done(SrcuCore);
                    srcu_boot_list_drained(SrcuCore);
                    srcu_per_struct_locks_deferred(SrcuCore);
                    srcu_delayed_work_queueing_ready(SrcuCore, TimerWheel, Workqueue);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            tree_srcu_enabled(SrcuCore);
            srcu_init_done(SrcuCore);
            srcu_boot_list_drained(SrcuCore);
            srcu_per_struct_locks_deferred(SrcuCore);
            srcu_delayed_work_queueing_ready(SrcuCore, TimerWheel, Workqueue);
        }
    }
}

/*
 * HrtimerCore 表示 hrtimers_init() 建立的高精度 timer 基础。
 */
object HrtimerCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Softirq.state == State::Prepared;
                }

                within HrtimerBaseInitContext {
                    ensures {
                        hrtimer_base_init_context_used(HrtimerCore);
                        hrtimer_base_locks_ready(HrtimerCore);
                        hrtimer_clock_bases_ready(HrtimerCore);
                        hrtimer_active_queues_empty(HrtimerCore);
                    }
                }

                ensures {
                    hrtimer_core_ready(HrtimerCore, CpuGroup);
                    boot_cpu_hrtimer_base_ready(HrtimerCore);
                    hrtimer_base_locks_ready(HrtimerCore);
                    hrtimer_clock_bases_ready(HrtimerCore);
                    hrtimer_active_queues_empty(HrtimerCore);
                    hrtimer_softirq_registered(Softirq, HrtimerCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            hrtimer_core_ready(HrtimerCore, CpuGroup);
            boot_cpu_hrtimer_base_ready(HrtimerCore);
            hrtimer_base_locks_ready(HrtimerCore);
            hrtimer_clock_bases_ready(HrtimerCore);
            hrtimer_active_queues_empty(HrtimerCore);
            hrtimer_softirq_registered(Softirq, HrtimerCore);
        }
    }
}

/*
 * Timekeeper 表示 timekeeping_init() 建立的 timekeeper 与默认 clocksource。
 */
object Timekeeper: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Tick.state == State::Prepared;
                    StaticBranch.state == State::Ready;
                }

                within TimekeeperSeqWriteContext {
                    drives {
                        ClocksourceCore.Transition::Preset;
                        JiffiesClocksource.Transition::Preset;
                    }

                    ensures {
                        timekeeper_seqwrite_context_used(Timekeeper);
                        timekeeper_tk_core_seqcount_ready(Timekeeper);
                        timekeeper_tk_core_write_seqcount_used(Timekeeper);
                        timekeeper_lock_ready(Timekeeper);
                        timekeeper_shadow_timekeeper_ready(Timekeeper);
                    }
                }

                ensures {
                    timekeeper_ready(Timekeeper);
                    wall_time_basis_ready(Timekeeper);
                    monotonic_time_basis_ready(Timekeeper);
                    raw_time_basis_ready(Timekeeper);
                    timekeeper_tk_core_seqcount_ready(Timekeeper);
                    timekeeper_tk_core_write_seqcount_used(Timekeeper);
                    timekeeper_lock_ready(Timekeeper);
                    timekeeper_shadow_timekeeper_ready(Timekeeper);
                    current_clocksource_is_jiffies(Timekeeper, JiffiesClocksource);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ClocksourceCore.state == State::Prepared;
            JiffiesClocksource.state == State::Prepared;
            timekeeper_ready(Timekeeper);
            wall_time_basis_ready(Timekeeper);
            monotonic_time_basis_ready(Timekeeper);
            raw_time_basis_ready(Timekeeper);
            timekeeper_tk_core_seqcount_ready(Timekeeper);
            timekeeper_tk_core_write_seqcount_used(Timekeeper);
            timekeeper_lock_ready(Timekeeper);
            timekeeper_shadow_timekeeper_ready(Timekeeper);
            current_clocksource_is_jiffies(Timekeeper, JiffiesClocksource);
        }
    }
}

object ClocksourceCore: KernelObject {
    initial_state: State::Base;
    parent: Timekeeper;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    clocksource_core_registry_ready(ClocksourceCore);
                    clocksource_watchdog_deferred(ClocksourceCore);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            clocksource_core_registry_ready(ClocksourceCore);
            clocksource_watchdog_deferred(ClocksourceCore);
        }
    }
}

object JiffiesClocksource: KernelObject {
    initial_state: State::Base;
    parent: Timekeeper;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    jiffies_clocksource_available(JiffiesClocksource);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            jiffies_clocksource_available(JiffiesClocksource);
        }
    }
}

/*
 * RiscvTimerProvider 表示 RISC-V time_init() 的 timer provider 路径。
 */
object RiscvTimerProvider: HardwareObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    IrqController.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                    Timekeeper.state == State::Ready;
                    HrtimerCore.state == State::Ready;
                    Tick.state == State::Prepared;
                    SBI.state == State::Ready;
                }

                ensures {
                    riscv_timebase_ready(RiscvTimerProvider, DeviceTree);
                    riscv_clocksource_registered(RiscvTimerProvider, ClocksourceCore);
                    riscv_clockevent_registered(RiscvTimerProvider);
                    riscv_timer_irq_mapping_ready(RiscvTimerProvider, RiscvIntc);
                    riscv_timer_interrupt_action_ready(IrqDispatchTree, RiscvTimerProvider);
                    riscv_timer_sbi_programming_ready(RiscvTimerProvider, SBI);
                    riscv_time_read_action_available(RiscvTimerProvider);
                    riscv_clockevent_oneshot_action_available(RiscvTimerProvider, IrqDispatchTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            riscv_timebase_ready(RiscvTimerProvider, DeviceTree);
            riscv_clocksource_registered(RiscvTimerProvider, ClocksourceCore);
            riscv_clockevent_registered(RiscvTimerProvider);
            riscv_timer_irq_mapping_ready(RiscvTimerProvider, RiscvIntc);
            riscv_timer_interrupt_action_ready(IrqDispatchTree, RiscvTimerProvider);
            riscv_timer_sbi_programming_ready(RiscvTimerProvider, SBI);
            riscv_time_read_action_available(RiscvTimerProvider);
            riscv_clockevent_oneshot_action_available(RiscvTimerProvider, IrqDispatchTree);
        }
    }
}

/*
 * SbiIpi 表示 RISC-V smp_prepare_cpus() 前的 SBI IPI 基础。
 */
object SbiIpi: HardwareObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SBI.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    sbi_ipi_ready(SbiIpi, SBI, CpuGroup);
                    sbi_ipi_irq_mapping_ready(SbiIpi, RiscvIntc);
                    sbi_ipi_send_action_ready(SbiIpi, SBI);
                    sbi_ipi_enable_deferred(SbiIpi);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            sbi_ipi_ready(SbiIpi, SBI, CpuGroup);
            sbi_ipi_irq_mapping_ready(SbiIpi, RiscvIntc);
            sbi_ipi_send_action_ready(SbiIpi, SBI);
            sbi_ipi_enable_deferred(SbiIpi);
        }
    }
}

/*
 * IpiMux 表示 generic IPI-Mux：多个虚拟 IPI 复用到 SBI software IRQ。
 * 当前只要求 boot CPU 上的 mux domain 和基础虚拟 IPI range 可见，secondary
 * CPU enable 仍随 SMP 并发路径后续展开。
 */
object IpiMux: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SbiIpi.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    ipi_mux_domain_ready(IpiMux, SbiIpi);
                    ipi_mux_per_cpu_bits_ready(IpiMux, PerCpuStorage);
                    ipi_mux_virtual_ipi_range_ready(IpiMux);
                    ipi_mux_parent_software_irq_ready(IpiMux, SbiIpi);
                    ipi_mux_secondary_enable_deferred(IpiMux);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ipi_mux_domain_ready(IpiMux, SbiIpi);
            ipi_mux_per_cpu_bits_ready(IpiMux, PerCpuStorage);
            ipi_mux_virtual_ipi_range_ready(IpiMux);
            ipi_mux_parent_software_irq_ready(IpiMux, SbiIpi);
            ipi_mux_secondary_enable_deferred(IpiMux);
        }
    }
}

/*
 * BootStackCanary 表示 boot_init_stack_canary()。当前配置启用
 * STACKPROTECTOR_PER_TASK，因此这里要求 random_init() 已完成并将 boot
 * init task/per-task canary seed 物化；完整 per-task fork 传播留给后续任务模型。
 */
object BootStackCanary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Randomness.state == State::Ready;
                    BootInitStack.state == State::Online;
                }

                within BootStackCanaryInitContext {
                    ensures {
                        boot_stack_canary_init_context_used(BootStackCanary);
                        boot_stack_canary_uses_randomness(BootStackCanary, Randomness);
                        boot_init_task_canary_seeded(BootStackCanary);
                    }
                }

                ensures {
                    stackprotector_enabled(BootStackCanary);
                    per_task_stack_canary_ready(BootStackCanary);
                    boot_stack_canary_uses_randomness(BootStackCanary, Randomness);
                    boot_init_task_canary_seeded(BootStackCanary);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            stackprotector_enabled(BootStackCanary);
            per_task_stack_canary_ready(BootStackCanary);
            boot_stack_canary_uses_randomness(BootStackCanary, Randomness);
            boot_init_task_canary_seeded(BootStackCanary);
        }
    }
}

/*
 * PerfEventCore 表示 perf_event_init() 的启动期 PMU registry/SRCU 基础。
 * 运行期 event cache 分配和硬件 breakpoint 细节后续展开。
 */
object PerfEventCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SrcuCore.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                within PerfPmusSrcuInitContext {
                    ensures {
                        perf_pmus_srcu_init_context_used(PerfEventCore);
                        perf_pmus_srcu_ready(PerfEventCore, SrcuCore);
                        perf_cpu_context_locks_ready(PerfEventCore, CpuGroup);
                    }
                }

                ensures {
                    perf_events_enabled_by_config(PerfEventCore);
                    perf_pmu_idr_ready(PerfEventCore);
                    perf_pmus_srcu_ready(PerfEventCore, SrcuCore);
                    perf_pmu_registry_ready(PerfEventCore);
                    perf_cpu_context_locks_ready(PerfEventCore, CpuGroup);
                    perf_swevent_pmus_registered(PerfEventCore);
                    perf_reboot_notifier_registered(PerfEventCore);
                    perf_event_cache_deferred(PerfEventCore);
                    perf_hw_breakpoint_deferred(PerfEventCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            perf_events_enabled_by_config(PerfEventCore);
            perf_pmu_idr_ready(PerfEventCore);
            perf_pmus_srcu_ready(PerfEventCore, SrcuCore);
            perf_pmu_registry_ready(PerfEventCore);
            perf_cpu_context_locks_ready(PerfEventCore, CpuGroup);
            perf_swevent_pmus_registered(PerfEventCore);
            perf_reboot_notifier_registered(PerfEventCore);
            perf_event_cache_deferred(PerfEventCore);
            perf_hw_breakpoint_deferred(PerfEventCore);
        }
    }
}

/*
 * ProfileCore 表示 profile_init()。当前默认无 profile= 参数，因此
 * CONFIG_PROFILING 路径被记录为参数缺省、buffer 分配裁剪和 proc export 延后。
 */
object ProfileCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Randomness.state == State::Ready;
                    PerfEventCore.state == State::Ready;
                }

                ensures {
                    profile_enabled_by_config(ProfileCore);
                    profile_param_absent(ProfileCore);
                    profile_buffer_allocation_trimmed(ProfileCore);
                    profile_proc_export_deferred(ProfileCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            profile_enabled_by_config(ProfileCore);
            profile_param_absent(ProfileCore);
            profile_buffer_allocation_trimmed(ProfileCore);
            profile_proc_export_deferred(ProfileCore);
        }
    }
}

/*
 * PlicDriver 表示 Linux-like IRQCHIP_DECLARE()/irqchip init entry 层。
 * 它不是普通 PlatformBus driver；其 init callback 由 irqchip_init()
 * 经 of_irq_init() 遍历 LDS section 并根据 DeviceTree compatible 匹配后调度。
 */
object PlicDriver: InterruptObject {
    initial_state: State::Base;
    parent: IrqChipInitTable;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    IrqChipInitTable.state == State::Prepared;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    plic_driver_irqchip_entry_registered(PlicDriver, IrqChipInitTable);
                    plic_driver_registered_in_irqchip_lds_section(PlicDriver, IrqChipInitTable);
                    plic_driver_init_callback_bound(PlicDriver, Plic);
                    plic_driver_compatible_covers_qemu_virt(PlicDriver);
                    plic_driver_probe_depends_on_device_tree(PlicDriver, DeviceTree);
                    plic_driver_probe_runs_in_irq_time_init(PlicDriver);
                    plic_driver_not_platform_bus_probe(PlicDriver);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            plic_driver_irqchip_entry_registered(PlicDriver, IrqChipInitTable);
            plic_driver_registered_in_irqchip_lds_section(PlicDriver, IrqChipInitTable);
            plic_driver_init_callback_bound(PlicDriver, Plic);
            plic_driver_compatible_covers_qemu_virt(PlicDriver);
            plic_driver_probe_depends_on_device_tree(PlicDriver, DeviceTree);
            plic_driver_probe_runs_in_irq_time_init(PlicDriver);
            plic_driver_not_platform_bus_probe(PlicDriver);
        }
    }
}

/*
 * Plic 表示 RISC-V 外部中断 provider。当前阶段已经把 PLIC init callback
 * 挂到 init_IRQ() -> irqchip_init() -> of_irq_init() -> LDS section
 * traversal 链上，并完成 provider 的最小 setup：DT resource 解析、
 * system-irqchip ioremap、external-input context 基础状态和父 INTC
 * external 输入连接。运行期 chained handler、claim/complete 和 generic
 * IRQ dispatch contract 已建立；具体 UART source gate 要等
 * PlicIrqMapping 绑定 HwirqRef::PlicUart0 后定义，source enable 和真实
 * 触发仍后续展开。
 *
 * 物理传播链是 UART -> PLIC -> RiscvIntc -> CPU。当前只建立连接和
 * 处理链 contract，不打开 UART 在 PLIC 上的 source enable gate，也不
 * 打开 root INTC 的 SupervisorExternalIrq input enable gate。
 */
object Plic: InterruptObject {
    initial_state: State::Base;
    parent: RiscvIntc;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Ready {
                depends_on {
                    RiscvIntc.state == State::Ready;
                    IrqChipInitTable.state == State::Prepared;
                    PlicDriver.state == State::Prepared;
                    DeviceTree.state == State::Ready;
                    Ioremap.state == State::Ready;
                    VmallocAllocator.state == State::Ready;
                    PageTableCaches.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                }

                ensures {
                    of_irq_init_matched_plic_compatible(IrqChipInitTable, PlicDriver, DeviceTree);
                    plic_setup_called_by_of_irq_init(Plic, IrqChipInitTable, PlicDriver);
                    plic_node_interrupt_controller_ready(Plic, DeviceTree);
                    plic_provider_discovery_reserved(Plic, DeviceTree);
                    plic_external_parent_reserved(Plic, RiscvIntc);
                    plic_output_connected_to_riscv_intc_external_input(Plic, RiscvIntc);
                    riscv_intc_external_irq_forwards_to_plic(RiscvIntc, Plic);
                    plic_mmio_resource_ready(Plic, DeviceTree);
                    plic_ioremap_mapping_created(Plic, Ioremap, IoMemoryMappingRef::Plic);
                    ioremap_mapping_owner_is_system_irqchip(Ioremap, IoMemoryMappingRef::Plic, Plic);
                    plic_ioremap_owner_is_system_irqchip(Plic, Ioremap, IoMemoryMappingRef::Plic);
                    plic_ioremap_uses_vmalloc_mapping(Plic, VmallocAllocator, IoMemoryMappingRef::Plic);
                    plic_source_count_ready(Plic, DeviceTree);
                    plic_external_input_context_ready(Plic, RiscvIntc);
                    plic_threshold_ready(Plic);
                    plic_priority_ready(Plic);
                    plic_source_enable_ready(Plic);
                    plic_uart_source_trigger_deferred(Plic);
                    plic_chained_handler_ready(Plic, RiscvIntc);
                    plic_claim_action_ready(Plic);
                    plic_complete_action_ready(Plic);
                    plic_claim_reads_claim_register(Plic);
                    plic_claim_returns_zero_when_no_pending_source(Plic);
                    plic_complete_writes_claimed_source(Plic);
                    plic_chained_handler_claim_loop_until_zero(Plic);
                    plic_chained_handler_zero_claim_stops_dispatch(Plic);
                    plic_chained_handler_completes_each_claimed_source(Plic);
                    plic_claim_before_generic_irq_dispatch(Plic);
                    plic_complete_after_irq_action_handler(Plic);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            of_irq_init_matched_plic_compatible(IrqChipInitTable, PlicDriver, DeviceTree);
            plic_setup_called_by_of_irq_init(Plic, IrqChipInitTable, PlicDriver);
            plic_node_interrupt_controller_ready(Plic, DeviceTree);
            plic_provider_discovery_reserved(Plic, DeviceTree);
            plic_external_parent_reserved(Plic, RiscvIntc);
            plic_output_connected_to_riscv_intc_external_input(Plic, RiscvIntc);
            riscv_intc_external_irq_forwards_to_plic(RiscvIntc, Plic);
            plic_mmio_resource_ready(Plic, DeviceTree);
            plic_ioremap_mapping_created(Plic, Ioremap, IoMemoryMappingRef::Plic);
            ioremap_mapping_owner_is_system_irqchip(Ioremap, IoMemoryMappingRef::Plic, Plic);
            plic_ioremap_owner_is_system_irqchip(Plic, Ioremap, IoMemoryMappingRef::Plic);
            plic_ioremap_uses_vmalloc_mapping(Plic, VmallocAllocator, IoMemoryMappingRef::Plic);
            plic_source_count_ready(Plic, DeviceTree);
            plic_external_input_context_ready(Plic, RiscvIntc);
            plic_threshold_ready(Plic);
            plic_priority_ready(Plic);
            plic_source_enable_ready(Plic);
            plic_uart_source_trigger_deferred(Plic);
            plic_chained_handler_ready(Plic, RiscvIntc);
            plic_claim_action_ready(Plic);
            plic_complete_action_ready(Plic);
            plic_claim_reads_claim_register(Plic);
            plic_claim_returns_zero_when_no_pending_source(Plic);
            plic_complete_writes_claimed_source(Plic);
            plic_chained_handler_claim_loop_until_zero(Plic);
            plic_chained_handler_zero_claim_stops_dispatch(Plic);
            plic_chained_handler_completes_each_claimed_source(Plic);
            plic_claim_before_generic_irq_dispatch(Plic);
            plic_complete_after_irq_action_handler(Plic);
        }
    }
}

/*
 * SmpCallFunction 表示 call_function_init() 建立的 SMP function-call 基础。
 */
object SmpCallFunction: TaskObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IpiMux.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                within SmpCallFunctionInitContext {
                    ensures {
                        smp_call_function_init_context_used(SmpCallFunction);
                        call_single_queue_locks_ready(SmpCallFunction, PerCpuStorage);
                        smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
                    }
                }

                ensures {
                    smp_call_function_ready(SmpCallFunction, CpuGroup);
                    call_single_queue_ready(SmpCallFunction, PerCpuStorage);
                    call_single_queue_locks_ready(SmpCallFunction, PerCpuStorage);
                    smp_call_function_ipi_route_ready(SmpCallFunction, IpiMux);
                    smp_call_function_ipi_mux_ready(SmpCallFunction, IpiMux);
                    smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            smp_call_function_ready(SmpCallFunction, CpuGroup);
            call_single_queue_ready(SmpCallFunction, PerCpuStorage);
            call_single_queue_locks_ready(SmpCallFunction, PerCpuStorage);
            smp_call_function_ipi_route_ready(SmpCallFunction, IpiMux);
            smp_call_function_ipi_mux_ready(SmpCallFunction, IpiMux);
            smp_call_function_runtime_ipi_delivery_deferred(SmpCallFunction);
        }
    }
}

/*
 * IrqTimeInitPhase 表示 InterruptPhase 的第一个子阶段。它建立
 * early_irq_init()/init_IRQ()/timer/timekeeping/random/IPI/call-function
 * 基础，但不打开 boot CPU 本地中断总入口；local_irq_enable() 独立由
 * LocalIrqEnablePhase 承载。
 */
object IrqTimeInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SchedInitPhase.state == State::Ready;
                    Scheduler.state == State::Online;
                    RcuCore.state == State::Ready;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Prepared;
                    Randomness.state == State::Prepared;
                    InterruptStream.state == State::Ready;
                }

                within IrqTimeInitGlobalExclusiveContext {
                    drives {
                        IrqController.Transition::Setup;
                        RiscvIntc.Transition::Setup;
                        IrqChipInitTable.Transition::Preset;
                        PlicDriver.Transition::Preset;
                        IrqChipInitTable.Transition::Setup;
                        PlicIrqDomain.Transition::Preset;
                        PlicIrqDomain.Transition::Setup;
                        IrqHandlerRegistry.Transition::Setup;
                        IrqDispatchTree.Transition::Setup;
                        Tick.Transition::Preset;
                        TimerWheel.Transition::Setup;
                        SrcuCore.Transition::Setup;
                        HrtimerCore.Transition::Setup;
                        Timekeeper.Transition::Setup;
                        RiscvTimerProvider.Transition::Setup;
                        Tick.Transition::Setup;
                        Softirq.Transition::Setup;
                        Randomness.Transition::Setup;
                        BootStackCanary.Transition::Setup;
                        PerfEventCore.Transition::Setup;
                        ProfileCore.Transition::Setup;
                        SbiIpi.Transition::Setup;
                        IpiMux.Transition::Setup;
                        SmpCallFunction.Transition::Setup;
                    }

                    ensures {
                        irq_time_init_global_exclusive_context_used(IrqTimeInitPhase);
                        cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
                        interrupt_concurrency_closed();
                        early_boot_irqs_disabled_true();
                    }
                }

                ensures {
                    irq_time_init_ready(IrqTimeInitPhase);
                    irq_time_init_global_exclusive_context_used(IrqTimeInitPhase);
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    early_boot_irqs_disabled_true();
                    time_read_smoke_available(RiscvTimerProvider);
                }

                deferred {
                    "early_irq_init() 的完整 irq_desc allocator 细节暂缓；当前只要求 descriptor/domain 壳和 timer IRQ mapping。";
                    "perf_event_init() 的 event cache 分配、硬件 breakpoint 细节和运行期 PMU event 生命周期暂缓；当前只要求 PMU registry、pmus_srcu 和 CPU context locks 基础。";
                    "profile_init() 的 profile buffer 分配和 proc export 暂缓；当前默认无 profile= 参数，只记录裁剪边界。";
                    "late_time_init hook 不在本阶段执行；当前 RISC-V 路径无 hook。";
                    "RiscvTimerProvider.enable() 暂缓：正式周期 tick 服务属于中断打开后的运行期推进。";
                    "更完整 UART RX、ordinary TTY runtime/FIFO 策略和复杂并发策略暂缓；当前 IRQ-time/initcall 路径已建立 root INTC -> PLIC chained handler -> irqdomain -> action 的 dispatch contract，并在 InitcallPhase 后续边界由 Serial8250Console.Enable/Serial8250ConsoleIrqTxProbe 完成 interrupt-driven TX 首轮。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_time_init_ready(IrqTimeInitPhase);
            IrqController.state == State::Ready;
            RiscvIntc.state == State::Ready;
            IrqDispatchTree.state == State::Ready;
            IrqChipInitTable.state == State::Ready;
            PlicDriver.state == State::Prepared;
            Plic.state == State::Ready;
            PlicIrqDomain.state == State::Ready;
            IrqHandlerRegistry.state == State::Ready;
            Tick.state == State::Ready;
            TickBroadcast.state == State::Ready;
            TimerWheel.state == State::Ready;
            SrcuCore.state == State::Ready;
            HrtimerCore.state == State::Ready;
            Timekeeper.state == State::Ready;
            ClocksourceCore.state == State::Prepared;
            JiffiesClocksource.state == State::Prepared;
            RiscvTimerProvider.state == State::Ready;
            Softirq.state == State::Ready;
            Randomness.state == State::Ready;
            BootStackCanary.state == State::Ready;
            PerfEventCore.state == State::Ready;
            ProfileCore.state == State::Ready;
            IpiMux.state == State::Ready;
            SbiIpi.state == State::Ready;
            SmpCallFunction.state == State::Ready;
            InterruptStream.state == State::Ready;
            cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
            irq_time_init_global_exclusive_context_used(IrqTimeInitPhase);
            interrupt_concurrency_closed();
            task_concurrency_closed();
            smp_concurrency_closed();
            early_boot_irqs_disabled_true();
            time_read_smoke_available(RiscvTimerProvider);
        }
    }
}
