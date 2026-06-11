/*
 * Console registration and early/real console handoff model.
 *
 * Linux reference shape:
 * - earlycon registers a CON_BOOT console through register_console().
 * - 8250 console registration creates a real ttyS console.
 * - When the real console becomes CON_CONSDEV, printk unregisters boot
 *   consoles unless keep_bootcon was requested.
 */

predicate boot_console_registered<T, E>(boot_console: T, earlycon: E) -> bool;
predicate boot_console_con_boot_flag_set<T>(boot_console: T) -> bool;
predicate boot_console_printbuffer_flag_set<T>(boot_console: T) -> bool;
predicate boot_console_write_routes_to_earlycon<T, E>(boot_console: T, earlycon: E) -> bool;
predicate boot_console_kept_by_policy<T>(boot_console: T) -> bool;
predicate boot_console_unregistered<T>(boot_console: T) -> bool;
predicate boot_console_removed_from_registry<T, R>(boot_console: T, registry: R) -> bool;

predicate console_registry_ready<T>(registry: T) -> bool;
predicate console_registry_register_console_api_ready<T>(registry: T) -> bool;
predicate console_registry_has_boot_console<T, B>(registry: T, boot_console: B) -> bool;
predicate console_registry_has_real_console<T, C>(registry: T, console: C) -> bool;
predicate console_registry_preferred_console_from_stdout_path<T, D>(registry: T, device_tree: D) -> bool;
predicate console_registry_keep_bootcon_policy_ready<T>(registry: T) -> bool;
predicate console_registry_keep_bootcon_disabled<T>(registry: T) -> bool;
predicate console_registry_keep_bootcon_enabled<T>(registry: T) -> bool;
predicate console_registry_printk_route_real_console<T, C>(registry: T, console: C) -> bool;

predicate uart8250_port_resources_ready<T, D, R>(port: T, device: D, device_tree: R) -> bool;
predicate uart8250_port_mmio_resource_bound<T>(port: T) -> bool;
predicate uart8250_port_mapbase_bound<T>(port: T) -> bool;
predicate uart8250_port_membase_ioremapped<T, I, M>(port: T, ioremap: I, mapping: M) -> bool;
predicate uart8250_port_uses_ioremap<T>(port: T) -> bool;
predicate uart8250_port_reg_shift_ready<T>(port: T) -> bool;
predicate uart8250_port_reg_io_width_ready<T>(port: T) -> bool;
predicate uart8250_port_clock_ready<T>(port: T) -> bool;
predicate uart8250_port_line_assigned<T>(port: T) -> bool;
predicate uart8250_port_registered<T>(port: T) -> bool;
predicate uart8250_port_registration_returned_line<T>(port: T) -> bool;
predicate uart8250_port_bound_to_stdout_node<T, D>(port: T, device_tree: D) -> bool;

predicate serial8250_console_registered<T, P>(console: T, port: P) -> bool;
predicate serial8250_console_real_console<T>(console: T) -> bool;
predicate serial8250_console_consdev<T>(console: T) -> bool;
predicate serial8250_console_printbuffer_suppressed_for_boot_handoff<T>(console: T) -> bool;
predicate serial8250_console_matches_stdout_path<T, D>(console: T, device_tree: D) -> bool;
predicate serial8250_console_write_backend_ready<T, P>(console: T, port: P) -> bool;

predicate console_handoff_ready<T, B, S>(handoff: T, boot_console: B, serial_console: S) -> bool;
predicate console_handoff_triggered_by_register_console<T, R>(handoff: T, registry: R) -> bool;
predicate console_handoff_boot_console_unregistered<T, B>(handoff: T, boot_console: B) -> bool;
predicate console_handoff_boot_console_retained_by_keep_bootcon<T, B>(handoff: T, boot_console: B) -> bool;
predicate console_handoff_printk_route_switched<T, R, S>(handoff: T, registry: R, serial_console: S) -> bool;

object BootConsole: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    boot_console_con_boot_flag_set(BootConsole);
                    boot_console_printbuffer_flag_set(BootConsole);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            boot_console_con_boot_flag_set(BootConsole);
            boot_console_printbuffer_flag_set(BootConsole);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    boot_console_registered(BootConsole, EarlyCon);
                    boot_console_con_boot_flag_set(BootConsole);
                    boot_console_printbuffer_flag_set(BootConsole);
                    boot_console_write_routes_to_earlycon(BootConsole, EarlyCon);
                }
            }
        }
    }

    state State::Online {
        invariant {
            boot_console_registered(BootConsole, EarlyCon);
            boot_console_con_boot_flag_set(BootConsole);
            boot_console_printbuffer_flag_set(BootConsole);
            boot_console_write_routes_to_earlycon(BootConsole, EarlyCon);
        }

        events {
            on Event::Disable -> State::Offline {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    Serial8250Console.state == State::Ready;
                    console_registry_keep_bootcon_disabled(ConsoleRegistry);
                }

                ensures {
                    boot_console_unregistered(BootConsole);
                    boot_console_removed_from_registry(BootConsole, ConsoleRegistry);
                }
            }
        }
    }

    state State::Offline {
        invariant {
            boot_console_unregistered(BootConsole);
            boot_console_removed_from_registry(BootConsole, ConsoleRegistry);
        }
    }
}

object Uart8250Port: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DeviceTree.state == State::Ready;
                    Ioremap.state == State::Ready;
                    platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
                    device_tree_stdout_path_resolves_to_node(DeviceTree, DeviceNodeRef::Ns16550aSerial);
                }

                ensures {
                    uart8250_port_resources_ready(Uart8250Port, DeviceRef::Ns16550aSerial, DeviceTree);
                    uart8250_port_mmio_resource_bound(Uart8250Port);
                    uart8250_port_mapbase_bound(Uart8250Port);
                    uart8250_port_membase_ioremapped(Uart8250Port, Ioremap, IoMemoryMappingRef::Ns16550aSerial);
                    uart8250_port_uses_ioremap(Uart8250Port);
                    uart8250_port_reg_shift_ready(Uart8250Port);
                    uart8250_port_reg_io_width_ready(Uart8250Port);
                    uart8250_port_clock_ready(Uart8250Port);
                    uart8250_port_line_assigned(Uart8250Port);
                    uart8250_port_registered(Uart8250Port);
                    uart8250_port_registration_returned_line(Uart8250Port);
                    uart8250_port_bound_to_stdout_node(Uart8250Port, DeviceTree);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uart8250_port_resources_ready(Uart8250Port, DeviceRef::Ns16550aSerial, DeviceTree);
            uart8250_port_mmio_resource_bound(Uart8250Port);
            uart8250_port_mapbase_bound(Uart8250Port);
            uart8250_port_membase_ioremapped(Uart8250Port, Ioremap, IoMemoryMappingRef::Ns16550aSerial);
            uart8250_port_uses_ioremap(Uart8250Port);
            uart8250_port_reg_shift_ready(Uart8250Port);
            uart8250_port_reg_io_width_ready(Uart8250Port);
            uart8250_port_clock_ready(Uart8250Port);
            uart8250_port_line_assigned(Uart8250Port);
            uart8250_port_registered(Uart8250Port);
            uart8250_port_registration_returned_line(Uart8250Port);
            uart8250_port_bound_to_stdout_node(Uart8250Port, DeviceTree);
        }
    }
}

object Serial8250Console: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    Uart8250Port.state == State::Ready;
                    Console.state == State::Prepared;
                    ConsoleDriverSet.state == State::Prepared;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    serial8250_console_registered(Serial8250Console, Uart8250Port);
                    serial8250_console_real_console(Serial8250Console);
                    serial8250_console_consdev(Serial8250Console);
                    serial8250_console_printbuffer_suppressed_for_boot_handoff(Serial8250Console);
                    serial8250_console_matches_stdout_path(Serial8250Console, DeviceTree);
                    serial8250_console_write_backend_ready(Serial8250Console, Uart8250Port);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            serial8250_console_registered(Serial8250Console, Uart8250Port);
            serial8250_console_real_console(Serial8250Console);
            serial8250_console_consdev(Serial8250Console);
            serial8250_console_printbuffer_suppressed_for_boot_handoff(Serial8250Console);
            serial8250_console_matches_stdout_path(Serial8250Console, DeviceTree);
            serial8250_console_write_backend_ready(Serial8250Console, Uart8250Port);
        }
    }
}

object ConsoleRegistry: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    BootConsole.state == State::Online;
                    Serial8250Console.state == State::Ready;
                    PrintkBuffer.state == State::Ready;
                    DeviceTree.state == State::Ready;
                }

                ensures {
                    console_registry_ready(ConsoleRegistry);
                    console_registry_register_console_api_ready(ConsoleRegistry);
                    console_registry_has_boot_console(ConsoleRegistry, BootConsole);
                    console_registry_has_real_console(ConsoleRegistry, Serial8250Console);
                    console_registry_preferred_console_from_stdout_path(ConsoleRegistry, DeviceTree);
                    console_registry_keep_bootcon_policy_ready(ConsoleRegistry);
                    console_registry_keep_bootcon_disabled(ConsoleRegistry);
                    console_registry_printk_route_real_console(ConsoleRegistry, Serial8250Console);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            console_registry_ready(ConsoleRegistry);
            console_registry_register_console_api_ready(ConsoleRegistry);
            console_registry_has_real_console(ConsoleRegistry, Serial8250Console);
            console_registry_preferred_console_from_stdout_path(ConsoleRegistry, DeviceTree);
            console_registry_keep_bootcon_policy_ready(ConsoleRegistry);
            console_registry_keep_bootcon_disabled(ConsoleRegistry);
            console_registry_printk_route_real_console(ConsoleRegistry, Serial8250Console);
        }
    }
}

object ConsoleHandoff: ConsoleObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    BootConsole.state == State::Online;
                    Serial8250Console.state == State::Ready;
                    console_registry_keep_bootcon_disabled(ConsoleRegistry);
                }

                drives {
                    BootConsole.Event::Disable;
                }

                ensures {
                    console_handoff_ready(ConsoleHandoff, BootConsole, Serial8250Console);
                    console_handoff_triggered_by_register_console(ConsoleHandoff, ConsoleRegistry);
                    console_handoff_boot_console_unregistered(ConsoleHandoff, BootConsole);
                    console_handoff_printk_route_switched(ConsoleHandoff, ConsoleRegistry, Serial8250Console);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootConsole.state == State::Offline;
            Serial8250Console.state == State::Ready;
            ConsoleRegistry.state == State::Ready;
            console_handoff_ready(ConsoleHandoff, BootConsole, Serial8250Console);
            console_handoff_triggered_by_register_console(ConsoleHandoff, ConsoleRegistry);
            console_handoff_boot_console_unregistered(ConsoleHandoff, BootConsole);
            console_handoff_printk_route_switched(ConsoleHandoff, ConsoleRegistry, Serial8250Console);
        }
    }
}
