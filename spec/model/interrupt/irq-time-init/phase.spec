/*
 * IRQ and Time Init Phase Specification
 *
 * This is InterruptPhase subphase 1. It starts after
 * sched_init/context_tracking_init and ends after the boot CPU
 * local_irq_enable() boundary. Moving the interrupt-open action here lets this
 * phase expose a real IRQ/timer acceptance test while task and SMP concurrency
 * remain closed.
 */

/*
 * IrqController 表示 early_irq_init()/init_IRQ() 建立的 IRQ descriptor、
 * generic IRQ domain 壳。RISC-V 本地中断控制器、IPI mux 和 PLIC 另行建模。
 */
object IrqController: InterruptObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    SlubAllocator.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    irq_descriptors_ready(IrqController);
                    irq_domain_ready(IrqController, DeviceTree);
                    irq_allocator_minimal_ready(IrqController, PageAllocator, SlubAllocator);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_descriptors_ready(IrqController);
            irq_domain_ready(IrqController, DeviceTree);
            irq_allocator_minimal_ready(IrqController, PageAllocator, SlubAllocator);
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
        events {
            on Event::Preset -> State::Prepared {
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

        events {
            on Event::Setup -> State::Ready {
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
                    Plic.Event::Preset;
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
 * 三条本地 cause；external provider 仍由 PLIC 占位而不开放运行期路由。
 */
object RiscvIntc: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
        }
    }
}

/*
 * IrqDispatchTree 表示 generic IRQ 分派树/路由表。它承接 IRQ domain
 * 映射结果，把硬件 interrupt cause 路由到具体 handler action；当前最小
 * 闭环只要求 RISC-V supervisor timer interrupt route 可用。
 */
object IrqDispatchTree: InterruptObject {
    initial_state: State::Base;
    parent: IrqController;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    IrqController.state == State::Ready;
                    RiscvIntc.state == State::Ready;
                    InterruptStream.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    irq_dispatch_tree_ready(IrqDispatchTree, IrqController, RiscvIntc);
                    irq_dispatch_fallback_route_ready(IrqDispatchTree);
                    irq_dispatch_timer_route_ready(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_software_route_reserved(IrqDispatchTree, RiscvIntc);
                    irq_dispatch_external_route_deferred(IrqDispatchTree, RiscvIntc);
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
            irq_dispatch_external_route_deferred(IrqDispatchTree, RiscvIntc);
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
    }
}

type HwirqRef {
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
predicate hwirq_ref_ready<T>(hwirq: T) -> bool;
predicate logical_irq_ref_ready<T>(logical_irq: T) -> bool;

predicate plic_irq_domain_owner_bound<T, P>(domain: T, plic: P) -> bool;
predicate plic_irq_domain_source_range_bound<T, P>(domain: T, plic: P) -> bool;
predicate plic_irq_domain_source_zero_reserved<T>(domain: T) -> bool;
predicate plic_irq_domain_one_cell_specifier<T>(domain: T) -> bool;
predicate plic_irq_domain_mapping_table_ready<T>(domain: T) -> bool;
predicate plic_irq_domain_logical_allocator_ready<T>(domain: T) -> bool;
predicate plic_irq_domain_translate_ops_ready<T>(domain: T) -> bool;
predicate plic_irq_domain_enable_deferred<T>(domain: T) -> bool;

predicate plic_irq_mapping_domain_bound<T, D>(mapping: T, domain: D) -> bool;
predicate plic_irq_mapping_source_valid<T, P>(mapping: T, plic: P) -> bool;
predicate plic_irq_mapping_source_zero_rejected<T>(mapping: T) -> bool;
predicate plic_irq_mapping_source_range_checked<T, P>(mapping: T, plic: P) -> bool;
predicate plic_irq_mapping_logical_irq_assigned<T, L>(mapping: T, logical_irq: L) -> bool;
predicate plic_irq_mapping_duplicate_source_idempotent<T, D>(mapping: T, domain: D) -> bool;
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
predicate irq_handler_registry_dispatch_deferred<T>(registry: T) -> bool;
predicate irq_handler_registry_registered_action<T, A>(registry: T, action: A) -> bool;

predicate irq_action_logical_irq_bound<T, L>(action: T, logical_irq: L) -> bool;
predicate irq_action_device_bound<T, D>(action: T, device: D) -> bool;
predicate irq_action_handler_bound<T>(action: T) -> bool;
predicate irq_action_hardirq_context_required<T>(action: T) -> bool;
predicate irq_action_mapped_irq_required<T, M>(action: T, mapping: M) -> bool;
predicate irq_action_duplicate_registration_rejected<T>(action: T) -> bool;
predicate irq_action_unmapped_registration_rejected<T>(action: T) -> bool;
predicate irq_action_does_not_enable_source<T>(action: T) -> bool;
predicate irq_action_dispatch_deferred<T>(action: T) -> bool;

predicate uart_irq_chain_kunit_observer_ready<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_read_only<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_does_not_call_handler<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_does_not_claim_or_complete<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_does_not_modify_plic_state<T>(observer: T) -> bool;
predicate uart_irq_chain_kunit_observer_reads_registered_action<T, R, A>(observer: T, registry: R, action: A) -> bool;
predicate uart_irq_chain_kunit_observer_reads_deferred_route<T, P, R>(observer: T, plic: P, registry: R) -> bool;

/*
 * IrqHandlerRegistry 是 IRQ core 侧的 handler/action registry。它只记录
 * request_irq 风格的 handler 绑定，不负责 PLIC source enable 或 dispatch。
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
                irq_action_dispatch_deferred(action);
            }
        }
    }

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
                    irq_handler_registry_dispatch_deferred(IrqHandlerRegistry);
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
            irq_handler_registry_dispatch_deferred(IrqHandlerRegistry);
        }
    }
}

/*
 * IrqAction 表示 request_irq() 创建的一条 logical IRQ -> handler 记录。
 * 它可以要求 hardirq context，但本步骤不执行 handler、不 enable source。
 */
object IrqAction: InterruptObject {
    initial_state: State::Base;
    parent: IrqHandlerRegistry;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
                    irq_action_dispatch_deferred(IrqAction);
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
            irq_action_dispatch_deferred(IrqAction);
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
        events {
            on Event::Setup -> State::Ready {
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
                    uart_irq_chain_kunit_observer_reads_deferred_route(
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
            uart_irq_chain_kunit_observer_reads_deferred_route(
                UartIrqChainKunitObserver,
                Plic,
                IrqHandlerRegistry
            );
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
        events {
            on Event::Preset -> State::Prepared {
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

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Plic.state == State::Ready;
                    IrqController.state == State::Ready;
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
        }
    }
}

/*
 * PlicIrqMapping 是 PlicIrqDomain 内的一条 source -> logical IRQ 记录。
 * Setup 建立映射记录并保持 source disabled / handler unregistered。
 */
object PlicIrqMapping: InterruptObject {
    initial_state: State::Base;
    parent: PlicIrqDomain;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PlicIrqDomain.state == State::Ready;
                    Plic.state == State::Ready;
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
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                drives {
                    TickBroadcast.Event::Preset;
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

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RiscvTimerProvider.state == State::Ready;
                }

                drives {
                    TickBroadcast.Event::Setup;
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
        events {
            on Event::Preset -> State::Prepared {
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

        events {
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Softirq.state == State::Prepared;
                    BootInitTask.state == State::Online;
                }

                ensures {
                    timer_wheel_ready(TimerWheel, CpuGroup);
                    cpu_timer_bases_ready(TimerWheel, PerCpuStorage);
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
            boot_init_task_posix_cpu_timer_work_ready(BootInitTask);
            timer_softirq_registered(Softirq, TimerWheel);
        }
    }
}

/*
 * HrtimerCore 表示 hrtimers_init() 建立的高精度 timer 基础。
 */
object HrtimerCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PerCpuStorage.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    Softirq.state == State::Prepared;
                }

                ensures {
                    hrtimer_core_ready(HrtimerCore, CpuGroup);
                    boot_cpu_hrtimer_base_ready(HrtimerCore);
                    hrtimer_softirq_registered(Softirq, HrtimerCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            hrtimer_core_ready(HrtimerCore, CpuGroup);
            boot_cpu_hrtimer_base_ready(HrtimerCore);
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Tick.state == State::Prepared;
                    StaticBranch.state == State::Ready;
                }

                drives {
                    ClocksourceCore.Event::Preset;
                    JiffiesClocksource.Event::Preset;
                }

                ensures {
                    timekeeper_ready(Timekeeper);
                    wall_time_basis_ready(Timekeeper);
                    monotonic_time_basis_ready(Timekeeper);
                    raw_time_basis_ready(Timekeeper);
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
            current_clocksource_is_jiffies(Timekeeper, JiffiesClocksource);
        }
    }
}

object ClocksourceCore: KernelObject {
    initial_state: State::Base;
    parent: Timekeeper;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
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
        events {
            on Event::Preset -> State::Prepared {
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
        events {
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
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
 * PlicDriver 表示 Linux-like IRQCHIP_DECLARE()/irqchip init entry 层。
 * 它不是普通 PlatformBus driver；其 init callback 由 irqchip_init()
 * 经 of_irq_init() 遍历 LDS section 并根据 DeviceTree compatible 匹配后调度。
 */
object PlicDriver: InterruptObject {
    initial_state: State::Base;
    parent: IrqChipInitTable;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
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
 * external 输入连接。external IRQ 运行期路由、claim/complete 和 handler
 * dispatch 仍后续展开。
 */
object Plic: InterruptObject {
    initial_state: State::Base;
    parent: RiscvIntc;

    state State::Base {
        events {
            on Event::Preset -> State::Ready {
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
                    plic_external_irq_route_deferred(Plic);
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
            plic_external_irq_route_deferred(Plic);
        }
    }
}

/*
 * SmpCallFunction 表示 call_function_init() 建立的 SMP function-call 基础。
 */
object SmpCallFunction: TaskObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    IpiMux.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    smp_call_function_ready(SmpCallFunction, CpuGroup);
                    call_single_queue_ready(SmpCallFunction, PerCpuStorage);
                    smp_call_function_ipi_route_ready(SmpCallFunction, IpiMux);
                    smp_call_function_ipi_mux_ready(SmpCallFunction, IpiMux);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            smp_call_function_ready(SmpCallFunction, CpuGroup);
            call_single_queue_ready(SmpCallFunction, PerCpuStorage);
            smp_call_function_ipi_route_ready(SmpCallFunction, IpiMux);
            smp_call_function_ipi_mux_ready(SmpCallFunction, IpiMux);
        }
    }
}

/*
 * IrqTimeInitPhase 表示 InterruptPhase 的第一个子阶段。它在阶段末尾打开
 * boot CPU 本地中断总入口，使后续 smoke 可以真实验收 timer interrupt。
 */
object IrqTimeInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SchedInitPhase.state == State::Ready;
                    Scheduler.state == State::Online;
                    RcuCore.state == State::Ready;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Prepared;
                    Randomness.state == State::Prepared;
                    InterruptStream.state == State::Ready;
                }

                drives {
                    IrqController.Event::Setup;
                    RiscvIntc.Event::Setup;
                    IrqChipInitTable.Event::Preset;
                    PlicDriver.Event::Preset;
                    IrqChipInitTable.Event::Setup;
                    PlicIrqDomain.Event::Preset;
                    PlicIrqDomain.Event::Setup;
                    IrqHandlerRegistry.Event::Setup;
                    IrqDispatchTree.Event::Setup;
                    Tick.Event::Preset;
                    TimerWheel.Event::Setup;
                    HrtimerCore.Event::Setup;
                    Timekeeper.Event::Setup;
                    RiscvTimerProvider.Event::Setup;
                    Tick.Event::Setup;
                    Softirq.Event::Setup;
                    Randomness.Event::Setup;
                    SbiIpi.Event::Setup;
                    IpiMux.Event::Setup;
                    SmpCallFunction.Event::Setup;
                    InterruptStream.Event::Enable;
                }

                ensures {
                    irq_time_init_ready(IrqTimeInitPhase);
                    interrupt_concurrency_open_for_boot_cpu();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    time_read_smoke_available(RiscvTimerProvider);
                    clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
                }

                deferred {
                    "early_irq_init() 的完整 irq_desc allocator 细节暂缓；当前只要求 descriptor/domain 壳和 timer IRQ mapping。";
                    "perf_event_init() 暂缓：perf core 和 PMU registry 不属于当前最小 IRQ/timer 验收闭环。";
                    "profile_init() 暂缓：profile buffer 和 proc export 后续再建模。";
                    "late_time_init hook 不在本阶段执行；当前 RISC-V 路径无 hook。";
                    "RiscvTimerProvider.enable() 暂缓：正式周期 tick 服务属于中断打开后的运行期推进。";
                    "PLIC external IRQ route 暂缓：当前只保留 provider discovery/parent reserved，不开放外部中断 handler。";
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
            HrtimerCore.state == State::Ready;
            Timekeeper.state == State::Ready;
            ClocksourceCore.state == State::Prepared;
            JiffiesClocksource.state == State::Prepared;
            RiscvTimerProvider.state == State::Ready;
            Softirq.state == State::Ready;
            Randomness.state == State::Ready;
            IpiMux.state == State::Ready;
            SbiIpi.state == State::Ready;
            SmpCallFunction.state == State::Ready;
            InterruptStream.state == State::Online;
            interrupt_concurrency_open_for_boot_cpu();
            task_concurrency_closed();
            smp_concurrency_closed();
            time_read_smoke_available(RiscvTimerProvider);
            clockevent_callback_smoke_available(RiscvTimerProvider, IrqDispatchTree);
        }
    }
}
