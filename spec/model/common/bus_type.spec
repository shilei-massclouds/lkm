/*
 * Generic driver-core bus type model.
 *
 * BusType corresponds to Linux struct bus_type plus its driver-core
 * registration state. Preset captures the static descriptor/name/ops binding;
 * Setup captures bus_register(): subsystem private state is allocated, the bus
 * kobject is registered under the bus registry, devices/drivers ksets are
 * created, and driver autoprobe becomes enabled.
 */

predicate bus_type_descriptor_bound<T>(bus: T) -> bool;
predicate bus_type_name_bound<T>(bus: T) -> bool;
predicate bus_type_ops_bound<T>(bus: T) -> bool;
predicate bus_type_registered<T>(bus: T) -> bool;
predicate bus_type_devices_kset_ready<T>(bus: T) -> bool;
predicate bus_type_drivers_kset_ready<T>(bus: T) -> bool;
predicate bus_type_autoprobe_enabled<T>(bus: T) -> bool;
predicate bus_type_register_return_zero<T>(bus: T) -> bool;

type BusType: DeviceObject {
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
            ensures {
                bus_type_registered(self);
                bus_type_devices_kset_ready(self);
                bus_type_drivers_kset_ready(self);
                bus_type_autoprobe_enabled(self);
                bus_type_register_return_zero(self);
            }
        }
    }
}
