/*
 * Virtio core bus/device model, first slice.
 *
 * VirtioBus is a global core bus instance owned by Context. It is not owned by
 * PlatformBus. PlatformBus only matches and calls the virtio-mmio platform
 * driver's probe; the virtio-mmio driver then registers a generic VirtioDevice
 * with VirtioBus after a valid transport header has been observed.
 */

predicate virtio_bus_context_owned<T, C>(bus: T, context: C) -> bool;
predicate virtio_bus_registered<T>(bus: T) -> bool;
predicate virtio_bus_platform_independent<T>(bus: T) -> bool;
predicate virtio_bus_device_set_bound<T, S>(bus: T, devices: S) -> bool;
predicate virtio_bus_device_added<T, D>(bus: T, device: D) -> bool;
predicate virtio_bus_device_count_nonzero<T>(bus: T) -> bool;
predicate virtio_bus_rng_device_count_nonzero<T>(bus: T) -> bool;
predicate virtio_bus_mmio_transport_count_nonzero<T>(bus: T) -> bool;

predicate virtio_device_allocated<T>(device: T) -> bool;
predicate virtio_device_registered_on_bus<T, B>(device: T, bus: B) -> bool;
predicate virtio_device_id_bound<T, I>(device: T, id: I) -> bool;
predicate virtio_device_vendor_id_bound<T>(device: T) -> bool;
predicate virtio_device_status_registered<T>(device: T) -> bool;
predicate virtio_device_transport_bound<T, R>(device: T, transport: R) -> bool;
predicate virtio_device_transport_is_mmio<T, R>(device: T, transport: R) -> bool;
predicate virtio_device_platform_device_ref_bound<T, R>(device: T, platform_device: R) -> bool;
predicate virtio_device_rng_id<T>(device: T) -> bool;

object VirtioBus: BusType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PlatformBus.state == State::Ready;
                }

                ensures {
                    virtio_bus_context_owned(VirtioBus, Context);
                    virtio_bus_registered(VirtioBus);
                    virtio_bus_platform_independent(VirtioBus);
                    virtio_bus_device_set_bound(VirtioBus, VirtioDeviceSet);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_bus_context_owned(VirtioBus, Context);
            virtio_bus_registered(VirtioBus);
            virtio_bus_platform_independent(VirtioBus);
            virtio_bus_device_set_bound(VirtioBus, VirtioDeviceSet);
        }
    }
}

object VirtioDevice: Device {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioBus.state == State::Ready;
                    VirtioMmioTransportDevice.state == State::Ready;
                    virtio_mmio_transport_ready_for_virtio_core(VirtioMmioTransportDevice);
                }

                drives {
                    VirtioBus.Action::AddDevice(VirtioDevice);
                }

                ensures {
                    virtio_device_allocated(VirtioDevice);
                    virtio_device_registered_on_bus(VirtioDevice, VirtioBus);
                    virtio_bus_device_added(VirtioBus, VirtioDevice);
                    virtio_bus_device_count_nonzero(VirtioBus);
                    virtio_bus_mmio_transport_count_nonzero(VirtioBus);
                    virtio_device_id_bound(VirtioDevice, VirtioDeviceId::Rng);
                    virtio_device_rng_id(VirtioDevice);
                    virtio_bus_rng_device_count_nonzero(VirtioBus);
                    virtio_device_vendor_id_bound(VirtioDevice);
                    virtio_device_status_registered(VirtioDevice);
                    virtio_device_transport_bound(VirtioDevice, VirtioMmioTransportDevice);
                    virtio_device_transport_is_mmio(VirtioDevice, VirtioMmioTransportDevice);
                    virtio_device_platform_device_ref_bound(VirtioDevice, DeviceRef::VirtioMmioPlatformDevice);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_device_allocated(VirtioDevice);
            virtio_device_registered_on_bus(VirtioDevice, VirtioBus);
            virtio_device_id_bound(VirtioDevice, VirtioDeviceId::Rng);
            virtio_device_rng_id(VirtioDevice);
            virtio_device_vendor_id_bound(VirtioDevice);
            virtio_device_status_registered(VirtioDevice);
            virtio_device_transport_bound(VirtioDevice, VirtioMmioTransportDevice);
            virtio_device_transport_is_mmio(VirtioDevice, VirtioMmioTransportDevice);
            virtio_device_platform_device_ref_bound(VirtioDevice, DeviceRef::VirtioMmioPlatformDevice);
        }
    }
}
