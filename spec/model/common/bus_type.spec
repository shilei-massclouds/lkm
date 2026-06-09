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
predicate bus_subsys_probe_files_ready<T>(subsys: T) -> bool;
predicate bus_subsys_groups_ready<T>(subsys: T) -> bool;

/*
 * BusSubsysPrivate models Linux struct subsys_private as created by
 * bus_register(). Preset corresponds to kzalloc() and static field binding;
 * Setup registers the bus kobject/kset shape; Enable publishes the runtime
 * device/driver containers and probe-control files.
 */
type BusSubsysPrivate: DeviceObject {
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
                bus_subsys_klist_drivers_ready(self);
                bus_subsys_probe_files_ready(self);
                bus_subsys_groups_ready(self);
            }
        }
    }
}

type BusType: DeviceObject {
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
}
