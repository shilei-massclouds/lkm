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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    StaticObjects.state == State::Online;
                }

                ensures {
                    device_driver_storage_ready(Ns16550aPlatformDriverStorage);
                    device_driver_storage_pinned(Ns16550aPlatformDriverStorage);
                    device_driver_storage_contains_ref(
                        Ns16550aPlatformDriverStorage,
                        DeviceDriverRef::Ns16550aPlatformDriver
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            device_driver_storage_ready(Ns16550aPlatformDriverStorage);
            device_driver_storage_pinned(Ns16550aPlatformDriverStorage);
            device_driver_storage_contains_ref(
                Ns16550aPlatformDriverStorage,
                DeviceDriverRef::Ns16550aPlatformDriver
            );
        }
    }
}

object Ns16550aPlatformDriver: PlatformDriverType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    StaticObjects.state == State::Online;
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
                    device_driver_ref_storage_bound(
                        DeviceDriverRef::Ns16550aPlatformDriver,
                        Ns16550aPlatformDriverStorage
                    );
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
            device_driver_ref_storage_bound(
                DeviceDriverRef::Ns16550aPlatformDriver,
                Ns16550aPlatformDriverStorage
            );
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

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    InitcallTable.state == State::Ready;
                    PlatformBus.state == State::Ready;
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
                    platform_bus_ns16550a_driver_registered(PlatformBus);
                    platform_bus_ns16550a_driver_match_table_ready(PlatformBus);
                    platform_bus_ns16550a_device_matched(PlatformBus);
                    platform_bus_ns16550a_driver_probe_called(PlatformBus);
                    platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
                    platform_bus_ns16550a_device_bound(PlatformBus);
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
            device_driver_ref_storage_bound(
                DeviceDriverRef::Ns16550aPlatformDriver,
                Ns16550aPlatformDriverStorage
            );
            device_driver_ref_lifetime_stable(DeviceDriverRef::Ns16550aPlatformDriver);
            device_driver_ref_ready(DeviceDriverRef::Ns16550aPlatformDriver);
            platform_driver_matches_device_node(Ns16550aPlatformDriver, DeviceNodeRef::Ns16550aSerial);
            platform_driver_probe_called(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_driver_probe_return_zero(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_driver_bound_device(Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_ns16550a_driver_registered(PlatformBus);
            platform_bus_ns16550a_driver_match_table_ready(PlatformBus);
            platform_bus_ns16550a_device_matched(PlatformBus);
            platform_bus_ns16550a_driver_probe_called(PlatformBus);
            platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
            platform_bus_ns16550a_device_bound(PlatformBus);
        }
    }
}
