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
predicate bus_driver_ref_ready<T>(driver: T) -> bool;
predicate bus_type_device_added<T, U>(bus: T, device: U) -> bool;
predicate bus_type_driver_added<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_devices_klist_nonempty<T>(bus: T) -> bool;
predicate bus_type_drivers_klist_nonempty<T>(bus: T) -> bool;
predicate bus_type_probe_driver_deferred<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_probe_device_deferred<T, U>(bus: T, device: U) -> bool;
predicate bus_type_probe_driver_scans_devices<T, U>(bus: T, driver: U) -> bool;
predicate bus_type_probe_device_scans_drivers<T, U>(bus: T, device: U) -> bool;

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

type BusDriverRef {
}

/*
 * BusSubsysPrivate models Linux struct subsys_private as created by
 * bus_register(). Preset corresponds to kzalloc() and static field binding;
 * Setup registers the bus kobject/kset shape; Enable publishes the runtime
 * device/driver containers and probe-control files.
 */
type BusSubsysPrivate {
    owned {
        klist_devices: DeviceRefSet;
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
        Action::AddDriver(driver: BusDriverRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_drivers_ready(self.subsys);
                bus_driver_ref_ready(driver);
            }
            ensures {
                bus_type_driver_added(self, driver);
                bus_type_drivers_klist_nonempty(self);
                bus_subsys_klist_drivers_contains(self.subsys, driver);
            }
        }

        /*
         * ProbeDriver is only the driver-side probe boundary for now: it proves
         * a registered driver can scan klist_devices, but Device/Driver
         * matching and binding are deferred until those types exist.
         */
        Action::ProbeDriver(driver: BusDriverRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_devices_ready(self.subsys);
                bus_type_devices_klist_nonempty(self);
                device_ref_set_nonempty(self.subsys.klist_devices);
                bus_driver_ref_ready(driver);
            }
            ensures {
                bus_type_probe_driver_deferred(self, driver);
                bus_type_probe_driver_scans_devices(self, driver);
            }
            deferred {
                "ProbeDriver 当前只建立 driver_attach()/bus_for_each_dev() 边界；真实 match/probe/bind 依赖 Device 和 Driver 类型建模后展开。";
            }
        }

        /*
         * ProbeDevice is the symmetric device-side probe boundary: it proves a
         * registered device can scan klist_drivers, while real binding remains
         * deferred.
         */
        Action::ProbeDevice(device: DeviceRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                bus_type_subsys_private_online(self, self.subsys);
                bus_subsys_klist_drivers_ready(self.subsys);
                bus_type_drivers_klist_nonempty(self);
                device_ref_ready(device);
            }
            ensures {
                bus_type_probe_device_deferred(self, device);
                bus_type_probe_device_scans_drivers(self, device);
            }
            deferred {
                "ProbeDevice 当前只建立 bus_probe_device()/device_initial_probe() 边界；真实 match/probe/bind 依赖 Device 和 Driver 类型建模后展开。";
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
    }
}
