/*
 * NS16550A platform serial driver model.
 *
 * Linux reference: drivers/tty/serial/8250/8250_of.c defines
 * of_platform_serial_driver with of_platform_serial_table, including
 * compatible "ns16550a". Its module_platform_driver()/builtin equivalent
 * registers through platform_driver_register(), which reaches the platform
 * bus klist_drivers through the generic driver-core path.
 */

object Ns16550aPlatformDriverStorage: DeviceDriverStorage {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    device_driver_static_storage_ready(Ns16550aPlatformDriverStorage);
                    device_driver_storage_ready(Ns16550aPlatformDriverStorage);
                    device_driver_storage_pinned(Ns16550aPlatformDriverStorage);
                    device_driver_storage_contains_ref(Ns16550aPlatformDriverStorage, DeviceDriverRef::Ns16550aPlatformDriver);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            device_driver_static_storage_ready(Ns16550aPlatformDriverStorage);
            device_driver_storage_ready(Ns16550aPlatformDriverStorage);
            device_driver_storage_pinned(Ns16550aPlatformDriverStorage);
            device_driver_storage_contains_ref(Ns16550aPlatformDriverStorage, DeviceDriverRef::Ns16550aPlatformDriver);
        }
    }
}

/*
 * PlatformIrqResource represents the IRQ resource parsed from the platform
 * device's OF node. For ns16550a on QEMU virt this is the UART interrupt
 * specifier whose interrupt parent is the PLIC. It is not a handler and not
 * the PLIC domain mapping itself.
 */
object PlatformIrqResource: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    DeviceTree.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                }

                ensures {
                    platform_irq_resource_device_bound(PlatformIrqResource, DeviceRef::Ns16550aSerial);
                    platform_irq_resource_node_bound(PlatformIrqResource, DeviceNodeRef::Ns16550aSerial);
                    platform_irq_resource_interrupt_parent_is_plic(PlatformIrqResource, Plic);
                    platform_irq_resource_raw_specifier_ready(PlatformIrqResource);
                    platform_irq_resource_raw_specifier_ready(PlatformIrqResourceRef::Ns16550aSerial);
                    platform_irq_resource_specifier_cells_ready(PlatformIrqResource);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            platform_irq_resource_device_bound(PlatformIrqResource, DeviceRef::Ns16550aSerial);
            platform_irq_resource_node_bound(PlatformIrqResource, DeviceNodeRef::Ns16550aSerial);
            platform_irq_resource_interrupt_parent_is_plic(PlatformIrqResource, Plic);
            platform_irq_resource_raw_specifier_ready(PlatformIrqResource);
            platform_irq_resource_raw_specifier_ready(PlatformIrqResourceRef::Ns16550aSerial);
            platform_irq_resource_specifier_cells_ready(PlatformIrqResource);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PlicIrqDomain.state == State::Ready;
                    PlatformIrqResource.state == State::Prepared;
                }

                drives {
                    PlicIrqDomain.Action::TranslateIrqSpecifier(PlatformIrqResourceRef::Ns16550aSerial);
                    PlicIrqDomain.Action::MapHwirq(HwirqRef::PlicUart0, PlicIrqMappingRef::Uart0);
                    PlicIrqMapping.Transition::Setup;
                }

                ensures {
                    platform_irq_resource_translated_by_domain(PlatformIrqResource, PlicIrqDomain);
                    platform_irq_resource_hwirq_bound(PlatformIrqResource, HwirqRef::PlicUart0);
                    platform_irq_resource_logical_irq_bound(PlatformIrqResource, LogicalIrqRef::Uart0);
                    platform_irq_resource_ready(PlatformIrqResource);
                    platform_irq_resource_not_handler_registration(PlatformIrqResource);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            platform_irq_resource_device_bound(PlatformIrqResource, DeviceRef::Ns16550aSerial);
            platform_irq_resource_node_bound(PlatformIrqResource, DeviceNodeRef::Ns16550aSerial);
            platform_irq_resource_interrupt_parent_is_plic(PlatformIrqResource, Plic);
            platform_irq_resource_raw_specifier_ready(PlatformIrqResource);
            platform_irq_resource_raw_specifier_ready(PlatformIrqResourceRef::Ns16550aSerial);
            platform_irq_resource_specifier_cells_ready(PlatformIrqResource);
            platform_irq_resource_translated_by_domain(PlatformIrqResource, PlicIrqDomain);
            platform_irq_resource_hwirq_bound(PlatformIrqResource, HwirqRef::PlicUart0);
            platform_irq_resource_logical_irq_bound(PlatformIrqResource, LogicalIrqRef::Uart0);
            platform_irq_resource_ready(PlatformIrqResource);
            platform_irq_resource_not_handler_registration(PlatformIrqResource);
        }
    }
}

predicate platform_irq_resource_device_bound<T, D>(resource: T, device: D) -> bool;
predicate platform_irq_resource_node_bound<T, N>(resource: T, node: N) -> bool;
predicate platform_irq_resource_interrupt_parent_is_plic<T, P>(resource: T, plic: P) -> bool;
predicate platform_irq_resource_raw_specifier_ready<T>(resource: T) -> bool;
predicate platform_irq_resource_specifier_cells_ready<T>(resource: T) -> bool;
predicate platform_irq_resource_translated_by_domain<T, D>(resource: T, domain: D) -> bool;
predicate platform_irq_resource_hwirq_bound<T, H>(resource: T, hwirq: H) -> bool;
predicate platform_irq_resource_logical_irq_bound<T, L>(resource: T, logical_irq: L) -> bool;
predicate platform_irq_resource_ready<T>(resource: T) -> bool;
predicate platform_irq_resource_not_handler_registration<T>(resource: T) -> bool;

object Ns16550aPlatformDriver: PlatformDriverType {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Ns16550aPlatformDriverStorage.state == State::Ready;
                }

                drives {
                    InitcallTable.Action::Register(
                        level: InitcallLevel::Device,
                        entry: InitcallEntry::Ns16550aPlatformDriver
                    );
                }

                ensures {
                    device_driver_core_storage_bound(Ns16550aPlatformDriver);
                    device_driver_name_bound(Ns16550aPlatformDriver);
                    device_driver_of_match_table_bound(Ns16550aPlatformDriver, Ns16550aPlatformDriver.of_match_table);
                    of_match_table_ready(Ns16550aPlatformDriver.of_match_table);
                    of_match_table_contains(Ns16550aPlatformDriver.of_match_table, CompatibleString::Ns16550a);
                    platform_driver_extends_device_driver(Ns16550aPlatformDriver);
                    device_driver_ref_targets(DeviceDriverRef::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
                    device_driver_ref_storage_bound(DeviceDriverRef::Ns16550aPlatformDriver, Ns16550aPlatformDriverStorage);
                    device_driver_ref_lifetime_stable(DeviceDriverRef::Ns16550aPlatformDriver);
                    device_driver_ref_ready(DeviceDriverRef::Ns16550aPlatformDriver);
                    initcall_entry_declared(InitcallEntry::Ns16550aPlatformDriver);
                    initcall_entry_has_prototype(InitcallEntry::Ns16550aPlatformDriver, InitcallEntryPrototype);
                    initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
                    initcall_entry_prototype_returns_result(InitcallEntryPrototype);
                    initcall_entry_owner_bound(InitcallEntry::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
                    initcall_entry_operation_bound(InitcallEntry::Ns16550aPlatformDriver, PlatformBus.Action::RegisterNs16550aPlatformDriver);
                    initcall_table_registration_committed(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
                    initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
                    initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            device_driver_core_storage_bound(Ns16550aPlatformDriver);
            device_driver_name_bound(Ns16550aPlatformDriver);
            device_driver_of_match_table_bound(Ns16550aPlatformDriver, Ns16550aPlatformDriver.of_match_table);
            of_match_table_ready(Ns16550aPlatformDriver.of_match_table);
            of_match_table_contains(Ns16550aPlatformDriver.of_match_table, CompatibleString::Ns16550a);
            platform_driver_extends_device_driver(Ns16550aPlatformDriver);
            device_driver_ref_targets(DeviceDriverRef::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
            device_driver_ref_storage_bound(DeviceDriverRef::Ns16550aPlatformDriver, Ns16550aPlatformDriverStorage);
            device_driver_ref_lifetime_stable(DeviceDriverRef::Ns16550aPlatformDriver);
            device_driver_ref_ready(DeviceDriverRef::Ns16550aPlatformDriver);
            initcall_entry_declared(InitcallEntry::Ns16550aPlatformDriver);
            initcall_entry_has_prototype(InitcallEntry::Ns16550aPlatformDriver, InitcallEntryPrototype);
            initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
            initcall_entry_prototype_returns_result(InitcallEntryPrototype);
            initcall_entry_owner_bound(InitcallEntry::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
            initcall_entry_operation_bound(InitcallEntry::Ns16550aPlatformDriver, PlatformBus.Action::RegisterNs16550aPlatformDriver);
            initcall_table_registration_committed(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
            initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    InitcallTable.state == State::Ready;
                    PlatformBus.state == State::Ready;
                    Ioremap.state == State::Ready;
                    PlicIrqDomain.state == State::Ready;
                    IrqHandlerRegistry.state == State::Ready;
                }

                drives {
                    Ioremap.Action::MapDeviceMmio(
                        device: DeviceRef::Ns16550aSerial,
                        mapping: IoMemoryMappingRef::Ns16550aSerial
                    );
                    PlatformIrqResource.Transition::Preset;
                    PlatformIrqResource.Transition::Setup;
                    IrqAction.Transition::Setup;
                    Uart8250Port.Transition::Setup;
                    Serial8250Console.Transition::Setup;
                    ConsoleRegistry.Transition::Setup;
                    ConsoleHandoff.Transition::Setup;
                }

                ensures {
                    device_driver_bus_bound(Ns16550aPlatformDriver, PlatformBus);
                    device_driver_registered(Ns16550aPlatformDriver);
                    device_driver_register_return_zero(Ns16550aPlatformDriver);
                    platform_driver_platform_bus_bound(Ns16550aPlatformDriver, PlatformBus);
                    platform_driver_matches_device_node(Ns16550aPlatformDriver, DeviceNodeRef::Ns16550aSerial);
                    platform_driver_probe_called(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_driver_probe_return_zero(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_driver_bound_device(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_driver_registered(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver);
                    platform_bus_device_discovered(PlatformBus, DeviceRef::Ns16550aSerial);
                    platform_bus_match_attempted(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_probe_called(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_device_bound(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    ioremap_mapping_created(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    ioremap_mapping_owner_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial, DeviceRef::Ns16550aSerial);
                    ioremap_mapping_phys_range_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    ioremap_mapping_phys_range_from_device_resource(Ioremap, IoMemoryMappingRef::Ns16550aSerial, DeviceRef::Ns16550aSerial);
                    ioremap_mapping_vmap_area_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial, VmapAreaRef::IoremapDeviceMmio);
                    ioremap_mapping_vmalloc_mapping_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial, VmapMappingRef::IoremapDeviceMmio);
                    ioremap_mapping_uses_vm_ioremap_flag(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    ioremap_mapping_uses_io_page_protection(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    ioremap_mapping_page_aligned(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    ioremap_mapping_membase_cookie_ready(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    ioremap_mapping_not_linear_direct_map(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    uart8250_port_resources_ready(Uart8250Port, DeviceRef::Ns16550aSerial, DeviceTree);
                    uart8250_port_mapbase_bound(Uart8250Port);
                    uart8250_port_membase_ioremapped(Uart8250Port, Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    uart8250_port_uses_ioremap(Uart8250Port);
                    platform_irq_resource_ready(PlatformIrqResource);
                    platform_irq_resource_logical_irq_bound(PlatformIrqResource, LogicalIrqRef::Uart0);
                    plic_irq_mapping_logical_irq_assigned(PlicIrqMapping, LogicalIrqRef::Uart0);
                    irq_handler_registry_registered_action(IrqHandlerRegistry, IrqActionRef::Ns16550aUart);
                    irq_action_logical_irq_bound(IrqAction, LogicalIrqRef::Uart0);
                    irq_action_device_bound(IrqAction, DeviceRef::Ns16550aSerial);
                    irq_action_handler_bound(IrqAction);
                    irq_action_hardirq_context_required(IrqAction);
                    irq_action_does_not_enable_source(IrqAction);
                    irq_action_dispatch_ready(IrqAction);
                    irq_action_handler_runs_after_plic_claim(IrqAction, Plic);
                    irq_action_handler_runs_before_plic_complete(IrqAction, Plic);
                    uart8250_port_irq_resource_ready(Uart8250Port, PlatformIrqResource);
                    uart8250_port_logical_irq_bound(Uart8250Port, LogicalIrqRef::Uart0);
                    uart8250_port_interrupt_output_still_deferred(Uart8250Port);
                    uart8250_port_registered(Uart8250Port);
                    serial8250_console_registered(Serial8250Console, Uart8250Port);
                    console_handoff_ready(ConsoleHandoff, BootConsole, Serial8250Console);
                    console_handoff_printk_route_switched(ConsoleHandoff, ConsoleRegistry, Serial8250Console);
                    platform_bus_ns16550a_driver_registered(PlatformBus);
                    platform_bus_ns16550a_driver_match_table_ready(PlatformBus);
                    platform_bus_ns16550a_device_matched(PlatformBus);
                    platform_bus_ns16550a_driver_probe_called(PlatformBus);
                    platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
                    platform_bus_ns16550a_device_bound(PlatformBus);
                    platform_bus_ns16550a_probe_ioremaps_uart8250_port(PlatformBus, Ioremap, Uart8250Port);
                    platform_bus_ns16550a_probe_registers_uart8250_port(PlatformBus, Uart8250Port);
                    platform_bus_ns16550a_probe_registers_serial_console(PlatformBus, Serial8250Console);
                    platform_bus_ns16550a_probe_triggers_console_handoff(PlatformBus, ConsoleHandoff);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            device_driver_core_storage_bound(Ns16550aPlatformDriver);
            device_driver_name_bound(Ns16550aPlatformDriver);
            device_driver_bus_bound(Ns16550aPlatformDriver, PlatformBus);
            device_driver_registered(Ns16550aPlatformDriver);
            device_driver_register_return_zero(Ns16550aPlatformDriver);
            platform_driver_extends_device_driver(Ns16550aPlatformDriver);
            platform_driver_platform_bus_bound(Ns16550aPlatformDriver, PlatformBus);
            device_driver_of_match_table_bound(Ns16550aPlatformDriver, Ns16550aPlatformDriver.of_match_table);
            of_match_table_ready(Ns16550aPlatformDriver.of_match_table);
            of_match_table_contains(Ns16550aPlatformDriver.of_match_table, CompatibleString::Ns16550a);
            device_driver_ref_targets(DeviceDriverRef::Ns16550aPlatformDriver, Ns16550aPlatformDriver);
            device_driver_ref_storage_bound(DeviceDriverRef::Ns16550aPlatformDriver, Ns16550aPlatformDriverStorage);
            device_driver_ref_lifetime_stable(DeviceDriverRef::Ns16550aPlatformDriver);
            device_driver_ref_ready(DeviceDriverRef::Ns16550aPlatformDriver);
            Uart8250Port.state == State::Ready;
            PlatformIrqResource.state == State::Ready;
            PlicIrqMapping.state == State::Ready;
            IrqAction.state == State::Ready;
            Serial8250Console.state == State::Ready;
            ConsoleRegistry.state == State::Ready;
            ConsoleHandoff.state == State::Ready;
            BootConsole.state == State::Offline;
            platform_driver_matches_device_node(Ns16550aPlatformDriver, DeviceNodeRef::Ns16550aSerial);
            platform_driver_probe_called(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_driver_probe_return_zero(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_driver_bound_device(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_driver_registered(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver);
            platform_bus_device_discovered(PlatformBus, DeviceRef::Ns16550aSerial);
            platform_bus_match_attempted(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_probe_called(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_device_bound(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            ioremap_mapping_created(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            ioremap_mapping_owner_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial, DeviceRef::Ns16550aSerial);
            ioremap_mapping_phys_range_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            ioremap_mapping_phys_range_from_device_resource(Ioremap, IoMemoryMappingRef::Ns16550aSerial, DeviceRef::Ns16550aSerial);
            ioremap_mapping_vmap_area_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial, VmapAreaRef::IoremapDeviceMmio);
            ioremap_mapping_vmalloc_mapping_bound(Ioremap, IoMemoryMappingRef::Ns16550aSerial, VmapMappingRef::IoremapDeviceMmio);
            ioremap_mapping_uses_vm_ioremap_flag(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            ioremap_mapping_uses_io_page_protection(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            ioremap_mapping_page_aligned(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            ioremap_mapping_membase_cookie_ready(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            ioremap_mapping_not_linear_direct_map(Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            uart8250_port_resources_ready(Uart8250Port, DeviceRef::Ns16550aSerial, DeviceTree);
            uart8250_port_mapbase_bound(Uart8250Port);
            uart8250_port_membase_ioremapped(Uart8250Port, Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            uart8250_port_uses_ioremap(Uart8250Port);
            platform_irq_resource_ready(PlatformIrqResource);
            platform_irq_resource_logical_irq_bound(PlatformIrqResource, LogicalIrqRef::Uart0);
            plic_irq_mapping_logical_irq_assigned(PlicIrqMapping, LogicalIrqRef::Uart0);
            irq_handler_registry_registered_action(IrqHandlerRegistry, IrqActionRef::Ns16550aUart);
            irq_action_logical_irq_bound(IrqAction, LogicalIrqRef::Uart0);
            irq_action_device_bound(IrqAction, DeviceRef::Ns16550aSerial);
            irq_action_handler_bound(IrqAction);
            irq_action_hardirq_context_required(IrqAction);
            irq_action_does_not_enable_source(IrqAction);
            irq_action_dispatch_ready(IrqAction);
            irq_action_handler_runs_after_plic_claim(IrqAction, Plic);
            irq_action_handler_runs_before_plic_complete(IrqAction, Plic);
            uart8250_port_irq_resource_ready(Uart8250Port, PlatformIrqResource);
            uart8250_port_logical_irq_bound(Uart8250Port, LogicalIrqRef::Uart0);
            uart8250_port_interrupt_output_still_deferred(Uart8250Port);
            uart8250_port_registered(Uart8250Port);
            serial8250_console_registered(Serial8250Console, Uart8250Port);
            console_handoff_ready(ConsoleHandoff, BootConsole, Serial8250Console);
            console_handoff_printk_route_switched(ConsoleHandoff, ConsoleRegistry, Serial8250Console);
            platform_bus_ns16550a_driver_registered(PlatformBus);
            platform_bus_ns16550a_driver_match_table_ready(PlatformBus);
            platform_bus_ns16550a_device_matched(PlatformBus);
            platform_bus_ns16550a_driver_probe_called(PlatformBus);
            platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
            platform_bus_ns16550a_device_bound(PlatformBus);
            platform_bus_ns16550a_probe_ioremaps_uart8250_port(PlatformBus, Ioremap, Uart8250Port);
            platform_bus_ns16550a_probe_registers_uart8250_port(PlatformBus, Uart8250Port);
            platform_bus_ns16550a_probe_registers_serial_console(PlatformBus, Serial8250Console);
            platform_bus_ns16550a_probe_triggers_console_handoff(PlatformBus, ConsoleHandoff);
        }
    }
}
