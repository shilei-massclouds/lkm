/*
 * Virtio core bus/device model, first slice.
 *
 * VirtioBus is a global core bus instance owned by Context. It is not owned by
 * PlatformBus. PlatformBus only matches and calls the virtio-mmio platform
 * driver's probe; the virtio-mmio driver then registers a generic VirtioDevice
 * with VirtioBus after a valid transport header has been observed. Device
 * type specific drivers, such as virtio-rng and virtio-blk, refine the
 * generic device id facts in their own specs.
 *
 * The device-side API is intentionally transport-neutral. Drivers such as
 * virtio-rng and virtio-blk should request status setup, feature negotiation,
 * config reads, queue setup, and queue notify through VirtioDevice/VirtQueue
 * facts instead of directly depending on virtio-mmio helper details.
 */

predicate virtio_bus_context_owned<T, C>(bus: T, context: C) -> bool;
predicate virtio_bus_registered<T>(bus: T) -> bool;
predicate virtio_bus_platform_independent<T>(bus: T) -> bool;
predicate virtio_bus_device_set_bound<T, S>(bus: T, devices: S) -> bool;
predicate virtio_bus_device_added<T, D>(bus: T, device: D) -> bool;
predicate virtio_bus_device_count_nonzero<T>(bus: T) -> bool;
predicate virtio_bus_rng_device_count_nonzero<T>(bus: T) -> bool;
predicate virtio_bus_block_device_count_nonzero<T>(bus: T) -> bool;
predicate virtio_bus_supported_device_count_nonzero<T>(bus: T) -> bool;
predicate virtio_bus_mmio_transport_count_nonzero<T>(bus: T) -> bool;

predicate virtio_device_allocated<T>(device: T) -> bool;
predicate virtio_device_registered_on_bus<T, B>(device: T, bus: B) -> bool;
predicate virtio_device_id_bound<T, I>(device: T, id: I) -> bool;
predicate virtio_device_vendor_id_bound<T>(device: T) -> bool;
predicate virtio_device_status_registered<T>(device: T) -> bool;
predicate virtio_device_status_reset<T>(device: T) -> bool;
predicate virtio_device_status_acknowledged<T>(device: T) -> bool;
predicate virtio_device_status_driver_seen<T>(device: T) -> bool;
predicate virtio_device_status_features_ok<T>(device: T) -> bool;
predicate virtio_device_status_driver_ok<T>(device: T) -> bool;
predicate virtio_device_transport_bound<T, R>(device: T, transport: R) -> bool;
predicate virtio_device_transport_is_mmio<T, R>(device: T, transport: R) -> bool;
predicate virtio_device_platform_device_ref_bound<T, R>(device: T, platform_device: R) -> bool;
predicate virtio_device_supported_id<T>(device: T) -> bool;
predicate virtio_device_rng_id<T>(device: T) -> bool;
predicate virtio_device_block_id<T>(device: T) -> bool;
predicate virtio_device_features_read<T>(device: T) -> bool;
predicate virtio_device_driver_features_written<T>(device: T) -> bool;
predicate virtio_device_feature_negotiation_done<T>(device: T) -> bool;
predicate virtio_device_config_access_ready<T>(device: T) -> bool;
predicate virtio_device_config_capacity_read<T>(device: T) -> bool;
predicate virtio_device_config_read_deferred<T>(device: T) -> bool;
predicate virtio_device_single_queue_discovered<T>(device: T) -> bool;
predicate virtio_device_queue_setup_done<T>(device: T) -> bool;
predicate virtio_device_queue_notify_done<T>(device: T) -> bool;
predicate virtio_device_multi_queue_deferred<T>(device: T) -> bool;
predicate virtio_device_reset_remove_deferred<T>(device: T) -> bool;

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

        processes {
            Action::AddDevice(device: VirtioDevice) {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_allocated(device);
                    virtio_device_transport_bound(device, VirtioMmioTransportDevice);
                }
                ensures {
                    virtio_bus_device_added(self, device);
                    virtio_bus_device_count_nonzero(self);
                    virtio_bus_mmio_transport_count_nonzero(self);
                    virtio_device_registered_on_bus(device, self);
                }
            }
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
                    virtio_bus_supported_device_count_nonzero(VirtioBus);
                    virtio_bus_mmio_transport_count_nonzero(VirtioBus);
                    virtio_device_supported_id(VirtioDevice);
                    virtio_device_vendor_id_bound(VirtioDevice);
                    virtio_device_status_registered(VirtioDevice);
                    virtio_device_transport_bound(VirtioDevice, VirtioMmioTransportDevice);
                    virtio_device_transport_is_mmio(VirtioDevice, VirtioMmioTransportDevice);
                    virtio_device_platform_device_ref_bound(VirtioDevice, DeviceRef::VirtioMmioPlatformDevice);
                    virtio_device_config_access_ready(VirtioDevice);
                    virtio_device_config_read_deferred(VirtioDevice);
                    virtio_device_multi_queue_deferred(VirtioDevice);
                    virtio_device_reset_remove_deferred(VirtioDevice);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_device_allocated(VirtioDevice);
            virtio_device_registered_on_bus(VirtioDevice, VirtioBus);
            virtio_device_supported_id(VirtioDevice);
            virtio_device_vendor_id_bound(VirtioDevice);
            virtio_device_status_registered(VirtioDevice);
            virtio_device_transport_bound(VirtioDevice, VirtioMmioTransportDevice);
            virtio_device_transport_is_mmio(VirtioDevice, VirtioMmioTransportDevice);
            virtio_device_platform_device_ref_bound(VirtioDevice, DeviceRef::VirtioMmioPlatformDevice);
            virtio_device_config_access_ready(VirtioDevice);
            virtio_device_config_read_deferred(VirtioDevice);
            virtio_device_multi_queue_deferred(VirtioDevice);
            virtio_device_reset_remove_deferred(VirtioDevice);
        }

        processes {
            Action::ResetStatus {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_transport_bound(self, VirtioMmioTransportDevice);
                }
                ensures {
                    virtio_device_status_reset(self);
                }
            }

            Action::SetupDriverStatus {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_status_reset(self);
                }
                ensures {
                    virtio_device_status_acknowledged(self);
                    virtio_device_status_driver_seen(self);
                }
            }

            Action::NegotiateFeatures {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_status_driver_seen(self);
                }
                ensures {
                    virtio_device_features_read(self);
                    virtio_device_driver_features_written(self);
                    virtio_device_status_features_ok(self);
                    virtio_device_feature_negotiation_done(self);
                }
            }

            Action::ReadConfig {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_config_access_ready(self);
                    virtio_device_feature_negotiation_done(self);
                }
                ensures {
                    virtio_device_config_capacity_read(self);
                    virtio_device_config_read_deferred(self);
                }
            }

            Action::SetupQueue {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_feature_negotiation_done(self);
                    VirtQueue.state == State::Ready;
                }
                ensures {
                    virtio_device_single_queue_discovered(self);
                    virtio_device_queue_setup_done(self);
                    virtio_device_multi_queue_deferred(self);
                }
            }

            Action::SetDriverOk {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_queue_setup_done(self);
                }
                ensures {
                    virtio_device_status_driver_ok(self);
                }
            }

            Action::NotifyQueue {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_device_status_driver_ok(self);
                    virtqueue_avail_index_advanced(VirtQueue);
                }
                ensures {
                    virtio_device_queue_notify_done(self);
                }
            }
        }
    }
}
