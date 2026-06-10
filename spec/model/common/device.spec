/*
 * Generic driver-core device model.
 *
 * DeviceType is the formal model type for Linux struct device. It is not
 * Linux struct device_type; if that descriptor is needed later, model it as a
 * separate DeviceTypeDescriptor/DeviceKind object.
 */

predicate device_ref_targets<T, D>(device_ref: T, device: D) -> bool;
predicate device_ref_ready<T>(device_ref: T) -> bool;
predicate device_core_storage_bound<T>(device: T) -> bool;
predicate device_core_name_bound<T>(device: T) -> bool;
predicate device_core_bus_bound<T, B>(device: T, bus: B) -> bool;
predicate device_core_registered<T>(device: T) -> bool;
predicate device_core_register_return_zero<T>(device: T) -> bool;

predicate platform_device_embeds_device<T, D>(platform_device: T, device: D) -> bool;
predicate platform_device_name_bound<T>(platform_device: T) -> bool;
predicate platform_device_id_bound<T>(platform_device: T) -> bool;
predicate platform_device_resources_bound<T>(platform_device: T) -> bool;
predicate platform_device_core_ref_ready<T, R>(platform_device: T, device_ref: R) -> bool;
predicate platform_device_from_device_ref_ready<R, T>(device_ref: R, platform_device: T) -> bool;

/*
 * DeviceType models Linux struct device: the common driver-core object that
 * can be registered globally and linked into a bus klist through DeviceRef.
 */
type DeviceType: DeviceObject {
    processes {
        Action::Register {
            state_effect: StateEffect::None;
            depends_on {
                device_core_storage_bound(self);
                device_core_name_bound(self);
            }
            ensures {
                device_core_registered(self);
                device_core_register_return_zero(self);
            }
        }
    }
}

type DeviceRef {
    processes {
        Action::BindDevice(device: DeviceType) {
            state_effect: StateEffect::None;
            depends_on {
                device_core_registered(device);
            }
            ensures {
                device_ref_targets(self, device);
                device_ref_ready(self);
            }
        }
    }
}

/*
 * PlatformDeviceType models Linux struct platform_device. It embeds a core
 * DeviceType member and adds platform-specific identity/resources. Coding is
 * expected to provide a container_of-like conversion from DeviceRef back to the
 * owning PlatformDeviceType when platform bus matching/probe needs it.
 */
type PlatformDeviceType: DeviceType {
    owned {
        dev: DeviceType;
    }

    processes {
        Action::BindCoreDevice(device_ref: DeviceRef) {
            state_effect: StateEffect::None;
            ensures {
                platform_device_embeds_device(self, self.dev);
                platform_device_name_bound(self);
                platform_device_id_bound(self);
                platform_device_resources_bound(self);
                device_ref_targets(device_ref, self.dev);
                device_ref_ready(device_ref);
                platform_device_core_ref_ready(self, device_ref);
                platform_device_from_device_ref_ready(device_ref, self);
            }
        }
    }
}
