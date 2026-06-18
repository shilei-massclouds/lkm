/*
 * Virtio MMIO transport model, first slice.
 *
 * This models Linux drivers/virtio/virtio_mmio.c only up to the platform
 * driver and transport/header boundary. Virtio core registration and virtqueue
 * setup are intentionally left to the following virtio core/ring steps.
 */

predicate virtio_mmio_driver_storage_ready<T>(storage: T) -> bool;
predicate virtio_mmio_driver_storage_pinned<T>(storage: T) -> bool;
predicate virtio_mmio_driver_storage_contains_ref<T, R>(storage: T, driver_ref: R) -> bool;
predicate virtio_mmio_driver_name_bound<T>(driver: T) -> bool;
predicate virtio_mmio_of_match_table_ready<T>(driver: T) -> bool;
predicate virtio_mmio_of_match_table_contains_compatible<T, C>(driver: T, compatible: C) -> bool;
predicate virtio_mmio_platform_driver_registered<T, B>(driver: T, bus: B) -> bool;
predicate virtio_mmio_platform_driver_probe_called<T, D>(driver: T, device: D) -> bool;
predicate virtio_mmio_platform_driver_probe_return_zero<T, D>(driver: T, device: D) -> bool;
predicate virtio_mmio_platform_driver_bound_device<T, D>(driver: T, device: D) -> bool;

predicate virtio_mmio_platform_device_compatible<T, C>(device: T, compatible: C) -> bool;
predicate virtio_mmio_platform_device_reg_resource_ready<T>(device: T) -> bool;
predicate virtio_mmio_platform_device_irq_resource_ready<T>(device: T) -> bool;
predicate virtio_mmio_platform_device_from_of_node<T, N>(device: T, node: N) -> bool;

predicate virtio_mmio_transport_allocated<T, D>(transport: T, device: D) -> bool;
predicate virtio_mmio_transport_platform_device_bound<T, D>(transport: T, device: D) -> bool;
predicate virtio_mmio_transport_ioremapped<T, I>(transport: T, ioremap: I) -> bool;
predicate virtio_mmio_transport_mmio_base_bound<T>(transport: T) -> bool;
predicate virtio_mmio_transport_irq_source_bound<T>(transport: T) -> bool;
predicate virtio_mmio_transport_header_read<T>(transport: T) -> bool;
predicate virtio_mmio_transport_magic_valid<T>(transport: T) -> bool;
predicate virtio_mmio_transport_version_supported<T>(transport: T) -> bool;
predicate virtio_mmio_transport_device_id_read<T>(transport: T) -> bool;
predicate virtio_mmio_transport_vendor_id_read<T>(transport: T) -> bool;
predicate virtio_mmio_transport_placeholder_device<T>(transport: T) -> bool;
predicate virtio_mmio_transport_rng_candidate<T>(transport: T) -> bool;
predicate virtio_mmio_transport_ready_for_virtio_core<T>(transport: T) -> bool;

object VirtioMmioPlatformDriverStorage: DeviceDriverStorage {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                ensures {
                    virtio_mmio_driver_storage_ready(VirtioMmioPlatformDriverStorage);
                    virtio_mmio_driver_storage_pinned(VirtioMmioPlatformDriverStorage);
                    virtio_mmio_driver_storage_contains_ref(VirtioMmioPlatformDriverStorage, DeviceDriverRef::VirtioMmioPlatformDriver);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_mmio_driver_storage_ready(VirtioMmioPlatformDriverStorage);
            virtio_mmio_driver_storage_pinned(VirtioMmioPlatformDriverStorage);
            virtio_mmio_driver_storage_contains_ref(VirtioMmioPlatformDriverStorage, DeviceDriverRef::VirtioMmioPlatformDriver);
        }
    }

}

object VirtioMmioTransportDevice: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PlatformBus.state == State::Ready;
                    Ioremap.state == State::Ready;
                }
                ensures {
                    virtio_mmio_platform_device_compatible(DeviceRef::VirtioMmioPlatformDevice, CompatibleString::VirtioMmio);
                    virtio_mmio_platform_device_reg_resource_ready(DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_platform_device_irq_resource_ready(DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_platform_device_from_of_node(DeviceRef::VirtioMmioPlatformDevice, DeviceNodeRef::VirtioMmio);
                    virtio_mmio_transport_allocated(VirtioMmioTransportDevice, DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_transport_platform_device_bound(VirtioMmioTransportDevice, DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_transport_ioremapped(VirtioMmioTransportDevice, Ioremap);
                    virtio_mmio_transport_mmio_base_bound(VirtioMmioTransportDevice);
                    virtio_mmio_transport_irq_source_bound(VirtioMmioTransportDevice);
                    virtio_mmio_transport_header_read(VirtioMmioTransportDevice);
                    virtio_mmio_transport_magic_valid(VirtioMmioTransportDevice);
                    virtio_mmio_transport_version_supported(VirtioMmioTransportDevice);
                    virtio_mmio_transport_device_id_read(VirtioMmioTransportDevice);
                    virtio_mmio_transport_vendor_id_read(VirtioMmioTransportDevice);
                    virtio_mmio_transport_rng_candidate(VirtioMmioTransportDevice);
                    virtio_mmio_transport_ready_for_virtio_core(VirtioMmioTransportDevice);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_mmio_platform_device_compatible(DeviceRef::VirtioMmioPlatformDevice, CompatibleString::VirtioMmio);
            virtio_mmio_platform_device_reg_resource_ready(DeviceRef::VirtioMmioPlatformDevice);
            virtio_mmio_platform_device_irq_resource_ready(DeviceRef::VirtioMmioPlatformDevice);
            virtio_mmio_platform_device_from_of_node(DeviceRef::VirtioMmioPlatformDevice, DeviceNodeRef::VirtioMmio);
            virtio_mmio_transport_allocated(VirtioMmioTransportDevice, DeviceRef::VirtioMmioPlatformDevice);
            virtio_mmio_transport_platform_device_bound(VirtioMmioTransportDevice, DeviceRef::VirtioMmioPlatformDevice);
            virtio_mmio_transport_ioremapped(VirtioMmioTransportDevice, Ioremap);
            virtio_mmio_transport_mmio_base_bound(VirtioMmioTransportDevice);
            virtio_mmio_transport_irq_source_bound(VirtioMmioTransportDevice);
            virtio_mmio_transport_header_read(VirtioMmioTransportDevice);
            virtio_mmio_transport_magic_valid(VirtioMmioTransportDevice);
            virtio_mmio_transport_version_supported(VirtioMmioTransportDevice);
            virtio_mmio_transport_device_id_read(VirtioMmioTransportDevice);
            virtio_mmio_transport_vendor_id_read(VirtioMmioTransportDevice);
            virtio_mmio_transport_rng_candidate(VirtioMmioTransportDevice);
            virtio_mmio_transport_ready_for_virtio_core(VirtioMmioTransportDevice);
        }
    }

}

object VirtioMmioPlatformDriver: PlatformDriverType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    StaticObjects.state == State::Online;
                    VirtioMmioPlatformDriverStorage.state == State::Ready;
                }

                drives {
                    InitcallTable.Action::Register(
                        level: InitcallLevel::Device,
                        entry: InitcallEntry::VirtioMmioPlatformDriver
                    );
                }

                ensures {
                    device_driver_core_storage_bound(VirtioMmioPlatformDriver);
                    device_driver_name_bound(VirtioMmioPlatformDriver);
                    virtio_mmio_driver_name_bound(VirtioMmioPlatformDriver);
                    device_driver_of_match_table_bound(VirtioMmioPlatformDriver, VirtioMmioPlatformDriver.of_match_table);
                    of_match_table_ready(VirtioMmioPlatformDriver.of_match_table);
                    of_match_table_contains(VirtioMmioPlatformDriver.of_match_table, CompatibleString::VirtioMmio);
                    virtio_mmio_of_match_table_ready(VirtioMmioPlatformDriver);
                    virtio_mmio_of_match_table_contains_compatible(VirtioMmioPlatformDriver, CompatibleString::VirtioMmio);
                    platform_driver_extends_device_driver(VirtioMmioPlatformDriver);
                    device_driver_ref_targets(DeviceDriverRef::VirtioMmioPlatformDriver, VirtioMmioPlatformDriver);
                    device_driver_ref_storage_bound(DeviceDriverRef::VirtioMmioPlatformDriver, VirtioMmioPlatformDriverStorage);
                    device_driver_ref_lifetime_stable(DeviceDriverRef::VirtioMmioPlatformDriver);
                    device_driver_ref_ready(DeviceDriverRef::VirtioMmioPlatformDriver);
                    initcall_entry_declared(InitcallEntry::VirtioMmioPlatformDriver);
                    initcall_entry_has_prototype(InitcallEntry::VirtioMmioPlatformDriver, InitcallEntryPrototype);
                    initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
                    initcall_entry_prototype_returns_result(InitcallEntryPrototype);
                    initcall_entry_owner_bound(InitcallEntry::VirtioMmioPlatformDriver, VirtioMmioPlatformDriver);
                    initcall_entry_operation_bound(InitcallEntry::VirtioMmioPlatformDriver, PlatformBus.Action::RegisterVirtioMmioPlatformDriver);
                    initcall_table_registration_committed(InitcallTable, InitcallLevel::Device, InitcallEntry::VirtioMmioPlatformDriver);
                    initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::VirtioMmioPlatformDriver, VirtioMmioPlatformDriver);
                    initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::VirtioMmioPlatformDriver);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            device_driver_core_storage_bound(VirtioMmioPlatformDriver);
            device_driver_name_bound(VirtioMmioPlatformDriver);
            virtio_mmio_driver_name_bound(VirtioMmioPlatformDriver);
            device_driver_of_match_table_bound(VirtioMmioPlatformDriver, VirtioMmioPlatformDriver.of_match_table);
            of_match_table_ready(VirtioMmioPlatformDriver.of_match_table);
            of_match_table_contains(VirtioMmioPlatformDriver.of_match_table, CompatibleString::VirtioMmio);
            virtio_mmio_of_match_table_ready(VirtioMmioPlatformDriver);
            virtio_mmio_of_match_table_contains_compatible(VirtioMmioPlatformDriver, CompatibleString::VirtioMmio);
            platform_driver_extends_device_driver(VirtioMmioPlatformDriver);
            device_driver_ref_targets(DeviceDriverRef::VirtioMmioPlatformDriver, VirtioMmioPlatformDriver);
            device_driver_ref_storage_bound(DeviceDriverRef::VirtioMmioPlatformDriver, VirtioMmioPlatformDriverStorage);
            device_driver_ref_lifetime_stable(DeviceDriverRef::VirtioMmioPlatformDriver);
            device_driver_ref_ready(DeviceDriverRef::VirtioMmioPlatformDriver);
            initcall_entry_declared(InitcallEntry::VirtioMmioPlatformDriver);
            initcall_entry_has_prototype(InitcallEntry::VirtioMmioPlatformDriver, InitcallEntryPrototype);
            initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
            initcall_entry_prototype_returns_result(InitcallEntryPrototype);
            initcall_entry_owner_bound(InitcallEntry::VirtioMmioPlatformDriver, VirtioMmioPlatformDriver);
            initcall_entry_operation_bound(InitcallEntry::VirtioMmioPlatformDriver, PlatformBus.Action::RegisterVirtioMmioPlatformDriver);
            initcall_table_registration_committed(InitcallTable, InitcallLevel::Device, InitcallEntry::VirtioMmioPlatformDriver);
            initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::VirtioMmioPlatformDriver, VirtioMmioPlatformDriver);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::VirtioMmioPlatformDriver);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    InitcallTable.state == State::Ready;
                    PlatformBus.state == State::Ready;
                    Ioremap.state == State::Ready;
                }

                drives {
                    Ioremap.Action::MapDeviceMmio(
                        device: DeviceRef::VirtioMmioPlatformDevice,
                        mapping: IoMemoryMappingRef::VirtioMmio
                    );
                    VirtioMmioTransportDevice.Event::Setup;
                }

                ensures {
                    device_driver_bus_bound(VirtioMmioPlatformDriver, PlatformBus);
                    device_driver_registered(VirtioMmioPlatformDriver);
                    device_driver_register_return_zero(VirtioMmioPlatformDriver);
                    platform_driver_platform_bus_bound(VirtioMmioPlatformDriver, PlatformBus);
                    virtio_mmio_platform_driver_registered(VirtioMmioPlatformDriver, PlatformBus);
                    platform_driver_matches_device_node(VirtioMmioPlatformDriver, DeviceNodeRef::VirtioMmio);
                    platform_driver_probe_called(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_driver_probe_return_zero(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_driver_bound_device(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_platform_driver_probe_called(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_platform_driver_probe_return_zero(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    virtio_mmio_platform_driver_bound_device(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_bus_driver_registered(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver);
                    platform_bus_device_discovered(PlatformBus, DeviceRef::VirtioMmioPlatformDevice);
                    platform_bus_match_attempted(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_bus_probe_called(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    platform_bus_device_bound(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
                    ioremap_mapping_created(Ioremap, IoMemoryMappingRef::VirtioMmio);
                    ioremap_mapping_owner_bound(Ioremap, IoMemoryMappingRef::VirtioMmio, DeviceRef::VirtioMmioPlatformDevice);
                    ioremap_mapping_phys_range_bound(Ioremap, IoMemoryMappingRef::VirtioMmio);
                    ioremap_mapping_phys_range_from_device_resource(Ioremap, IoMemoryMappingRef::VirtioMmio, DeviceRef::VirtioMmioPlatformDevice);
                    ioremap_mapping_uses_vm_ioremap_flag(Ioremap, IoMemoryMappingRef::VirtioMmio);
                    ioremap_mapping_uses_io_page_protection(Ioremap, IoMemoryMappingRef::VirtioMmio);
                    ioremap_mapping_page_aligned(Ioremap, IoMemoryMappingRef::VirtioMmio);
                    ioremap_mapping_membase_cookie_ready(Ioremap, IoMemoryMappingRef::VirtioMmio);
                    ioremap_mapping_not_linear_direct_map(Ioremap, IoMemoryMappingRef::VirtioMmio);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            device_driver_bus_bound(VirtioMmioPlatformDriver, PlatformBus);
            device_driver_registered(VirtioMmioPlatformDriver);
            device_driver_register_return_zero(VirtioMmioPlatformDriver);
            platform_driver_platform_bus_bound(VirtioMmioPlatformDriver, PlatformBus);
            virtio_mmio_platform_driver_registered(VirtioMmioPlatformDriver, PlatformBus);
            platform_driver_matches_device_node(VirtioMmioPlatformDriver, DeviceNodeRef::VirtioMmio);
            platform_driver_probe_called(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_driver_probe_return_zero(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_driver_bound_device(VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_bus_driver_registered(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver);
            platform_bus_device_discovered(PlatformBus, DeviceRef::VirtioMmioPlatformDevice);
            platform_bus_match_attempted(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_bus_probe_called(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            platform_bus_device_bound(PlatformBus, DeviceDriverRef::VirtioMmioPlatformDriver, DeviceRef::VirtioMmioPlatformDevice);
            VirtioMmioTransportDevice.state == State::Ready;
            virtio_mmio_transport_ready_for_virtio_core(VirtioMmioTransportDevice);
        }
    }
}
