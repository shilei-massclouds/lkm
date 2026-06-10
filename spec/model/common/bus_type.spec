/*
 * Generic driver-core bus type model.
 *
 * BusType corresponds to Linux struct bus_type plus its driver-core
 * registration state. Preset captures the static descriptor/name/ops binding;
 * Setup captures bus_register() and drives the private subsystem record through
 * allocation, sysfs registration, and runtime publication.
 */

predicate bus_type_descriptor_bound<T>(bus: T) -> bool;
predicate bus_type_name_bound<T>(bus: T) -> bool;
predicate bus_type_ops_bound<T>(bus: T) -> bool;
predicate bus_type_subsys_private_ready<T, U>(bus: T, subsys: U) -> bool;
predicate bus_type_subsys_private_online<T, U>(bus: T, subsys: U) -> bool;
predicate bus_type_registered<T>(bus: T) -> bool;
predicate bus_type_devices_kset_ready<T>(bus: T) -> bool;
predicate bus_type_drivers_kset_ready<T>(bus: T) -> bool;
predicate bus_type_autoprobe_enabled<T>(bus: T) -> bool;
predicate bus_type_register_return_zero<T>(bus: T) -> bool;
predicate bus_type_device_added<T, U>(bus: T, device: U) -> bool;
predicate bus_type_driver_added<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_devices_klist_nonempty<T>(bus: T) -> bool;
predicate bus_type_drivers_klist_nonempty<T>(bus: T) -> bool;
predicate bus_type_probe_driver_deferred<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_probe_device_deferred<T, U>(bus: T, device: U) -> bool;
predicate bus_type_probe_driver_scans_devices<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_probe_device_scans_drivers<T, U>(bus: T, device: U) -> bool;
predicate bus_type_probe_driver_match_attempted<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_probe_device_match_attempted<T, U>(bus: T, device: U) -> bool;
predicate bus_type_driver_matches_device<T, D, R>(bus: T, driver: D, device: R) -> bool;
predicate bus_type_device_matches_driver<T, R, D>(bus: T, device: R, driver: D) -> bool;
predicate bus_type_driver_probe_bound_device<T, D, R>(bus: T, driver: D, device: R) -> bool;
predicate bus_type_device_probe_bound_driver<T, R, D>(bus: T, device: R, driver: D) -> bool;

predicate bus_subsys_private_allocated<T>(subsys: T) -> bool;
predicate bus_subsys_private_bound_to_bus<T, U>(subsys: T, bus: U) -> bool;
predicate bus_subsys_notifier_ready<T>(subsys: T) -> bool;
predicate bus_subsys_autoprobe_enabled<T>(subsys: T) -> bool;
predicate bus_subsys_kobject_named<T>(subsys: T) -> bool;
predicate bus_subsys_kobject_attached_to_bus_kset<T>(subsys: T) -> bool;
predicate bus_subsys_registered<T>(subsys: T) -> bool;
predicate bus_subsys_uevent_file_ready<T>(subsys: T) -> bool;
predicate bus_subsys_devices_kset_ready<T>(subsys: T) -> bool;
predicate bus_subsys_drivers_kset_ready<T>(subsys: T) -> bool;
predicate bus_subsys_interfaces_ready<T>(subsys: T) -> bool;
predicate bus_subsys_mutex_ready<T>(subsys: T) -> bool;
predicate bus_subsys_klist_devices_ready<T>(subsys: T) -> bool;
predicate bus_subsys_klist_drivers_ready<T>(subsys: T) -> bool;
predicate bus_subsys_device_ref_set_bound<T, S>(subsys: T, device_refs: S) -> bool;
predicate bus_subsys_driver_ref_set_bound<T, S>(subsys: T, driver_refs: S) -> bool;
predicate bus_subsys_klist_devices_contains<T, U>(subsys: T, device: U) -> bool;
predicate bus_subsys_klist_drivers_contains<T, U>(subsys: T, driver: U) -> bool;
predicate bus_subsys_probe_files_ready<T>(subsys: T) -> bool;
predicate bus_subsys_groups_ready<T>(subsys: T) -> bool;

predicate early_platform_cleanup_deferred() -> bool;
predicate platform_bus_static_device_registered<T>(device: T) -> bool;
predicate platform_bus_device_name_bound<T>(device: T) -> bool;
predicate platform_bus_device_register_return_zero<T>(device: T) -> bool;
predicate platform_bus_type_ops_bound<T>(bus: T) -> bool;
predicate platform_bus_type_registered<T>(bus: T) -> bool;
predicate platform_bus_type_devices_kset_ready<T>(bus: T) -> bool;
predicate platform_bus_type_drivers_kset_ready<T>(bus: T) -> bool;
predicate platform_bus_type_autoprobe_enabled<T>(bus: T) -> bool;
predicate platform_bus_register_return_zero<T>(bus: T) -> bool;
predicate of_platform_default_populate_source_tree_ready<T, D>(bus: T, device_tree: D) -> bool;
predicate of_platform_default_populate_root_children_scanned<T, D>(bus: T, device_tree: D) -> bool;
predicate of_platform_default_populate_strict_compatible_required<T>(bus: T) -> bool;
predicate of_platform_default_populate_default_bus_match_table_used<T>(bus: T) -> bool;
predicate of_platform_default_populate_bus_nodes_recurse<T>(bus: T) -> bool;
predicate of_platform_default_populate_candidates_identified<T>(bus: T) -> bool;
predicate of_platform_default_populate_candidates_are_available<T>(bus: T) -> bool;
predicate of_platform_default_populate_candidate_names_printed<T>(bus: T) -> bool;
predicate of_platform_default_populate_candidate_compatibles_printed<T>(bus: T) -> bool;
predicate of_platform_default_populate_scan_complete_checkpoint<T>(bus: T) -> bool;
predicate of_platform_default_populate_node_refs_bound<T>(bus: T) -> bool;
predicate of_platform_default_populate_platform_devices_created<T>(bus: T) -> bool;
predicate of_platform_default_populate_device_refs_bound<T>(bus: T) -> bool;
predicate of_platform_default_populate_devices_added_to_bus<T>(bus: T) -> bool;
predicate platform_bus_platform_device_set_bound<T, S>(bus: T, platform_devices: S) -> bool;
predicate platform_bus_platform_device_owner_ready<T, S>(bus: T, platform_devices: S) -> bool;
predicate platform_bus_devices_added_from_platform_device_set<T, S>(bus: T, platform_devices: S) -> bool;
predicate of_platform_default_populate_device_node_ids_bound<T>(bus: T) -> bool;
predicate of_platform_default_populate_platform_devices_owned<T, S>(bus: T, platform_devices: S) -> bool;
predicate platform_bus_mock_ns16550a_driver_registered<T>(bus: T) -> bool;
predicate platform_bus_mock_ns16550a_driver_match_table_ready<T>(bus: T) -> bool;
predicate platform_bus_mock_ns16550a_driver_probe_called<T>(bus: T) -> bool;
predicate platform_bus_mock_ns16550a_driver_probe_return_zero<T>(bus: T) -> bool;
predicate platform_bus_mock_ns16550a_device_matched<T>(bus: T) -> bool;
predicate platform_bus_mock_ns16550a_device_bound<T>(bus: T) -> bool;

/*
 * BusSubsysPrivate models Linux struct subsys_private as created by
 * bus_register(). Preset corresponds to kzalloc() and static field binding;
 * Setup registers the bus kobject/kset shape; Enable publishes the runtime
 * device/driver containers and probe-control files.
 */
type BusSubsysPrivate {
    owned {
        klist_devices: DeviceRefSet;
        klist_drivers: DeviceDriverRefSet;
    }

    lifecycle {
        Event::Preset {
            state_effect: StateEffect::Always;
            ensures {
                bus_subsys_private_allocated(self);
                bus_subsys_private_bound_to_bus(self, owner);
                bus_subsys_notifier_ready(self);
                bus_subsys_autoprobe_enabled(self);
            }
        }

        Event::Setup {
            state_effect: StateEffect::Always;
            ensures {
                bus_subsys_kobject_named(self);
                bus_subsys_kobject_attached_to_bus_kset(self);
                bus_subsys_registered(self);
                bus_subsys_uevent_file_ready(self);
            }
        }

        Event::Enable {
            state_effect: StateEffect::Always;
            ensures {
                bus_subsys_devices_kset_ready(self);
                bus_subsys_drivers_kset_ready(self);
                bus_subsys_interfaces_ready(self);
                bus_subsys_mutex_ready(self);
                bus_subsys_klist_devices_ready(self);
                bus_subsys_device_ref_set_bound(self, self.klist_devices);
                device_ref_set_ready(self.klist_devices);
                bus_subsys_klist_drivers_ready(self);
                bus_subsys_driver_ref_set_bound(self, self.klist_drivers);
                device_driver_ref_set_ready(self.klist_drivers);
                bus_subsys_probe_files_ready(self);
                bus_subsys_groups_ready(self);
            }
        }
    }
}

type BusType {
    owned {
        subsys: BusSubsysPrivate;
    }

    lifecycle {
        Event::Preset {
            state_effect: StateEffect::Always;
            ensures {
                bus_type_descriptor_bound(self);
                bus_type_name_bound(self);
                bus_type_ops_bound(self);
            }
        }

        Event::Setup {
            state_effect: StateEffect::Always;
            drives {
                self.subsys.Event::Preset;
                self.subsys.Event::Setup;
                self.subsys.Event::Enable;
            }
            ensures {
                bus_type_subsys_private_ready(self, self.subsys);
                bus_type_subsys_private_online(self, self.subsys);
                bus_type_registered(self);
                bus_type_devices_kset_ready(self);
                bus_type_drivers_kset_ready(self);
                bus_type_autoprobe_enabled(self);
                bus_type_register_return_zero(self);
            }
        }
    }

    processes {
        /*
         * AddDevice records the bus-side device membership in
         * subsys_private.klist_devices. It is the reusable BusType boundary
         * that later Device modeling can call from device_add()/bus_add_device().
         */
        Action::AddDevice(device: DeviceRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_devices_ready(self.subsys);
                device_ref_set_ready(self.subsys.klist_devices);
                device_ref_ready(device);
            }
            ensures {
                bus_type_device_added(self, device);
                bus_type_devices_klist_nonempty(self);
                device_ref_set_contains(self.subsys.klist_devices, device);
                device_ref_set_nonempty(self.subsys.klist_devices);
                bus_subsys_klist_devices_contains(self.subsys, device);
            }
        }

        /*
         * AddDriver records the bus-side driver membership in
         * subsys_private.klist_drivers. It is the reusable BusType boundary
         * that later Driver modeling can call from driver_register()/bus_add_driver().
         */
        Action::AddDriver(driver: DeviceDriverRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_drivers_ready(self.subsys);
                device_driver_ref_set_ready(self.subsys.klist_drivers);
                device_driver_ref_ready(driver);
            }
            ensures {
                bus_type_driver_added(self, driver);
                bus_type_drivers_klist_nonempty(self);
                device_driver_ref_set_contains(self.subsys.klist_drivers, driver);
                device_driver_ref_set_nonempty(self.subsys.klist_drivers);
                bus_subsys_klist_drivers_contains(self.subsys, driver);
            }
        }

        /*
         * ProbeDriver is the driver-side attach boundary: a registered driver
         * scans klist_devices and attempts bus-specific match/probe/bind.
         */
        Action::ProbeDriver(driver: DeviceDriverRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_devices_ready(self.subsys);
                bus_type_devices_klist_nonempty(self);
                device_ref_set_nonempty(self.subsys.klist_devices);
                device_driver_ref_ready(driver);
            }
            ensures {
                bus_type_probe_driver_scans_devices(self, driver);
                bus_type_probe_driver_match_attempted(self, driver);
            }
        }

        /*
         * ProbeDevice is the symmetric device-side boundary: a device that has
         * just reached the bus scans klist_drivers and attempts match/probe.
         */
        Action::ProbeDevice(device: DeviceRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_drivers_ready(self.subsys);
                bus_type_drivers_klist_nonempty(self);
                device_driver_ref_set_nonempty(self.subsys.klist_drivers);
                device_ref_ready(device);
            }
            ensures {
                bus_type_probe_device_scans_drivers(self, device);
                bus_type_probe_device_match_attempted(self, device);
            }
        }
    }
}

/*
 * PlatformBusType extends the generic BusType with platform-specific
 * initcall behavior. Instances still carry the BusType lifecycle through
 * object wrapper events while this type defines reusable platform actions.
 */
type PlatformBusType: BusType {
    owned {
        platform_devices: PlatformDeviceSet;
    }

    processes {
        Action::OfPlatformDefaultPopulateInit {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                DeviceTree.state == State::Ready;
                InitcallTable.state == State::Prepared;
            }
            ensures {
                initcall_entry_invoked(InitcallEntry::OfPlatformDefaultPopulate);
                initcall_entry_return_recorded(InitcallEntry::OfPlatformDefaultPopulate);
                initcall_entry_run_context_checked(InitcallEntry::OfPlatformDefaultPopulate);
                of_platform_default_populate_source_tree_ready(self, DeviceTree);
                of_platform_default_populate_root_children_scanned(self, DeviceTree);
                of_platform_default_populate_strict_compatible_required(self);
                of_platform_default_populate_default_bus_match_table_used(self);
                of_platform_default_populate_bus_nodes_recurse(self);
                of_platform_default_populate_candidates_identified(self);
                of_platform_default_populate_candidates_are_available(self);
                of_platform_default_populate_candidate_names_printed(self);
                of_platform_default_populate_candidate_compatibles_printed(self);
                of_platform_default_populate_scan_complete_checkpoint(self);
                of_platform_default_populate_device_node_ids_bound(self);
                of_platform_default_populate_node_refs_bound(self);
                of_platform_default_populate_platform_devices_created(self);
                of_platform_default_populate_platform_devices_owned(self, self.platform_devices);
                of_platform_default_populate_device_refs_bound(self);
                of_platform_default_populate_devices_added_to_bus(self);
                platform_bus_platform_device_set_bound(self, self.platform_devices);
                platform_bus_platform_device_owner_ready(self, self.platform_devices);
                platform_bus_devices_added_from_platform_device_set(self, self.platform_devices);
                platform_device_set_ready(self.platform_devices);
                platform_device_set_nonempty(self.platform_devices);
                bus_type_devices_klist_nonempty(self);
                device_ref_set_nonempty(self.subsys.klist_devices);
            }
        }

        Action::RegisterMockNs16550aPlatformDriver {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                DeviceTree.state == State::Ready;
                InitcallTable.state == State::Prepared;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_drivers_ready(self.subsys);
                bus_type_devices_klist_nonempty(self);
                device_ref_set_nonempty(self.subsys.klist_devices);
                platform_device_set_nonempty(self.platform_devices);
            }
            drives {
                self.Action::AddDriver(DeviceDriverRef::MockNs16550aPlatformDriver);
                self.Action::ProbeDriver(DeviceDriverRef::MockNs16550aPlatformDriver);
            }
            ensures {
                initcall_entry_invoked(InitcallEntry::MockNs16550aPlatformDriver);
                initcall_entry_return_recorded(InitcallEntry::MockNs16550aPlatformDriver);
                initcall_entry_run_context_checked(InitcallEntry::MockNs16550aPlatformDriver);
                platform_bus_mock_ns16550a_driver_registered(self);
                platform_bus_mock_ns16550a_driver_match_table_ready(self);
                platform_bus_mock_ns16550a_device_matched(self);
                platform_bus_mock_ns16550a_driver_probe_called(self);
                platform_bus_mock_ns16550a_driver_probe_return_zero(self);
                platform_bus_mock_ns16550a_device_bound(self);
                bus_type_driver_added(self, DeviceDriverRef::MockNs16550aPlatformDriver);
                bus_type_drivers_klist_nonempty(self);
                bus_subsys_klist_drivers_contains(self.subsys, DeviceDriverRef::MockNs16550aPlatformDriver);
                bus_type_probe_driver_scans_devices(self, DeviceDriverRef::MockNs16550aPlatformDriver);
                bus_type_probe_driver_match_attempted(self, DeviceDriverRef::MockNs16550aPlatformDriver);
                bus_type_driver_probe_bound_device(self, DeviceDriverRef::MockNs16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            }
        }
    }
}
