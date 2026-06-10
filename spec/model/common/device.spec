/*
 * Generic driver-core device model.
 *
 * DeviceType is the formal model type for Linux struct device. It is not
 * Linux struct device_type; if that descriptor is needed later, model it as a
 * separate DeviceTypeDescriptor/DeviceKind object.
 */

predicate device_ref_targets<T, D>(device_ref: T, device: D) -> bool;
predicate device_ref_ready<T>(device_ref: T) -> bool;
predicate device_ref_set_ready<T>(device_refs: T) -> bool;
predicate device_ref_set_contains<T, R>(device_refs: T, device_ref: R) -> bool;
predicate device_ref_set_nonempty<T>(device_refs: T) -> bool;
predicate device_node_id_ready<T>(node_id: T) -> bool;
predicate device_node_id_stable<T>(node_id: T) -> bool;
predicate device_node_id_in_tree<T, D>(node_id: T, device_tree: D) -> bool;
predicate device_node_ref_id_bound<T, I>(node_ref: T, node_id: I) -> bool;
predicate device_node_ref_in_tree<T, D>(node_ref: T, device_tree: D) -> bool;
predicate device_node_ref_identity_stable<T>(node_ref: T) -> bool;
predicate device_node_ref_properties_queryable<T>(node_ref: T) -> bool;
predicate device_node_ref_compatible_queryable<T>(node_ref: T) -> bool;
predicate device_node_ref_lifetime_static<T>(node_ref: T) -> bool;
predicate device_node_ref_ready<T>(node_ref: T) -> bool;
predicate device_core_storage_bound<T>(device: T) -> bool;
predicate device_core_name_bound<T>(device: T) -> bool;
predicate device_core_bus_bound<T, B>(device: T, bus: B) -> bool;
predicate device_core_registered<T>(device: T) -> bool;
predicate device_core_register_return_zero<T>(device: T) -> bool;
predicate device_of_node_bound<T, N>(device: T, node_ref: N) -> bool;
predicate device_fwnode_bound<T, N>(device: T, node_ref: N) -> bool;

predicate platform_device_embeds_device<T, D>(platform_device: T, device: D) -> bool;
predicate platform_device_name_bound<T>(platform_device: T) -> bool;
predicate platform_device_id_bound<T>(platform_device: T) -> bool;
predicate platform_device_resources_bound<T>(platform_device: T) -> bool;
predicate platform_device_allocated_from_node<T, N>(platform_device: T, node_ref: N) -> bool;
predicate platform_device_node_bound<T, N>(platform_device: T, node_ref: N) -> bool;
predicate platform_device_platform_bus_bound<T>(platform_device: T) -> bool;
predicate platform_device_added<T>(platform_device: T) -> bool;
predicate platform_device_add_return_zero<T>(platform_device: T) -> bool;
predicate platform_device_ref_targets<R, T>(platform_device_ref: R, platform_device: T) -> bool;
predicate platform_device_ref_ready<R>(platform_device_ref: R) -> bool;
predicate platform_device_core_ref_ready<T, R>(platform_device: T, device_ref: R) -> bool;
predicate platform_device_from_device_ref_ready<R, T>(device_ref: R, platform_device: T) -> bool;
predicate platform_device_ref_from_device_ref_ready<D, P>(device_ref: D, platform_device_ref: P) -> bool;
predicate platform_device_set_ready<T>(platform_devices: T) -> bool;
predicate platform_device_set_contains<T, P>(platform_devices: T, platform_device: P) -> bool;
predicate platform_device_set_contains_ref<T, R>(platform_devices: T, platform_device_ref: R) -> bool;
predicate platform_device_set_nonempty<T>(platform_devices: T) -> bool;

/*
 * DeviceNodeId is the stable identity of a DeviceTree node in the expanded
 * tree. Coding may use a node index/path handle; long-lived Context objects
 * should store this id and resolve temporary DeviceNodeRef views when needed.
 */
type DeviceNodeId {
}

/*
 * DeviceNodeRef is an abstract reference to a DeviceTree node. The compatible
 * property remains owned by the DeviceTree node; DeviceType only holds a
 * reference to it through SetNode.
 */
type DeviceNodeRef {
    id: DeviceNodeId;

    processes {
        Action::BindNode(node_id: DeviceNodeId, device_tree: ResourceObject) {
            state_effect: StateEffect::None;
            depends_on {
                device_tree.state == State::Ready;
                device_node_id_ready(node_id);
                device_node_id_in_tree(node_id, device_tree);
                device_tree_properties_queryable(device_tree);
            }
            ensures {
                device_node_ref_id_bound(self, node_id);
                device_node_ref_in_tree(self, device_tree);
                device_node_ref_identity_stable(self);
                device_node_ref_properties_queryable(self);
                device_node_ref_compatible_queryable(self);
                device_node_ref_lifetime_static(self);
                device_node_ref_ready(self);
            }
        }
    }
}

/*
 * DeviceType models Linux struct device: the common driver-core object that
 * can be registered globally, linked into a bus klist through DeviceRef, and
 * associated with a firmware node through device_set_node().
 */
type DeviceType {
    of_node: DeviceNodeRef;

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

        /*
         * SetNode corresponds to Linux device_set_node()/dev.of_node binding.
         * It does not copy OF compatible data into struct device; later probe
         * obtains compatible through the bound DeviceNodeRef.
         */
        Action::SetNode(node_ref: DeviceNodeRef) {
            state_effect: StateEffect::None;
            depends_on {
                device_node_ref_ready(node_ref);
            }
            ensures {
                device_of_node_bound(self, node_ref);
                device_fwnode_bound(self, node_ref);
            }
        }
    }
}

type DeviceRefSet {
}

type PlatformDeviceSet {
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

type PlatformDeviceRef {
    processes {
        Action::BindPlatformDevice(platform_device: PlatformDeviceType) {
            state_effect: StateEffect::None;
            ensures {
                platform_device_ref_targets(self, platform_device);
                platform_device_ref_ready(self);
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
type PlatformDeviceType {
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

        /*
         * CreateFromNode corresponds to the OF path
         * of_platform_device_create_pdata() -> of_device_alloc() ->
         * device_set_node() -> platform_device_add()/device_add().
         */
        Action::CreateFromNode(
            node_ref: DeviceNodeRef,
            device_ref: DeviceRef,
            platform_device_ref: PlatformDeviceRef
        ) {
            state_effect: StateEffect::None;
            depends_on {
                device_node_ref_ready(node_ref);
            }
            ensures {
                platform_device_allocated_from_node(self, node_ref);
                platform_device_embeds_device(self, self.dev);
                platform_device_name_bound(self);
                platform_device_id_bound(self);
                platform_device_resources_bound(self);
                platform_device_node_bound(self, node_ref);
                platform_device_platform_bus_bound(self);
                device_core_storage_bound(self.dev);
                device_core_name_bound(self.dev);
                device_of_node_bound(self.dev, node_ref);
                device_fwnode_bound(self.dev, node_ref);
                device_core_registered(self.dev);
                device_core_register_return_zero(self.dev);
                device_ref_targets(device_ref, self.dev);
                device_ref_ready(device_ref);
                platform_device_ref_targets(platform_device_ref, self);
                platform_device_ref_ready(platform_device_ref);
                platform_device_core_ref_ready(self, device_ref);
                platform_device_from_device_ref_ready(device_ref, self);
                platform_device_ref_from_device_ref_ready(device_ref, platform_device_ref);
                platform_device_added(self);
                platform_device_add_return_zero(self);
            }
        }
    }
}
