/*
 * Hardware RNG core model.
 *
 * This follows the Linux drivers/char/hw_random/core.c surface needed by
 * virtio-rng: hwrng_register() adds a struct hwrng to rng_list and may make it
 * current_rng; users read through the current hwrng, not by reaching into a
 * provider-private driver object. The random pool fill thread, /dev/hwrng file
 * plumbing, sysfs selection, user-selected current_rng, cleanup refcounting and
 * quality arbitration beyond the single-device first slice stay deferred.
 */

predicate hwrng_core_initialized<T>(core: T) -> bool;
predicate hwrng_core_registry_ready<T>(core: T) -> bool;
predicate hwrng_core_current_slot_ready<T>(core: T) -> bool;
predicate hwrng_core_miscdev_deferred<T>(core: T) -> bool;
predicate hwrng_core_random_pool_deferred<T>(core: T) -> bool;
predicate hwrng_core_user_selection_deferred<T>(core: T) -> bool;
predicate hwrng_core_quality_policy_deferred<T>(core: T) -> bool;

predicate hwrng_device_allocated<T>(device: T) -> bool;
predicate hwrng_device_name_bound<T>(device: T) -> bool;
predicate hwrng_device_provider_is_virtio_rng<T, V>(device: T, provider: V) -> bool;
predicate hwrng_device_read_callback_bound<T>(device: T) -> bool;
predicate hwrng_device_cleanup_callback_bound<T>(device: T) -> bool;
predicate hwrng_device_priv_points_to_provider<T, V>(device: T, provider: V) -> bool;
predicate hwrng_device_quality_defaulted<T>(device: T) -> bool;
predicate hwrng_device_registered<T, C>(device: T, core: C) -> bool;
predicate hwrng_device_current<T, C>(device: T, core: C) -> bool;
predicate hwrng_device_duplicate_name_rejected<T, C>(device: T, core: C) -> bool;
predicate hwrng_device_unregistered<T, C>(device: T, core: C) -> bool;

predicate hwrng_core_register_called<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_register_return_zero<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_rng_list_contains<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_rng_list_nonempty<T>(core: T) -> bool;
predicate hwrng_core_current_rng_set<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_current_rng_ref_acquired<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_current_rng_read_invoked<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_read_copies_from_current<T, D>(core: T, device: D) -> bool;
predicate hwrng_core_read_returns_nonzero<T>(core: T) -> bool;
predicate hwrng_core_read_nonblocking<T>(core: T) -> bool;

object HwRngCore: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            /*
             * Setup represents the hwrng core being available for built-in
             * drivers. Linux registers the misc device from hwrng_modinit();
             * this first slice models only the in-kernel registry and current
             * hwrng selection surface used by virtio-rng.
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    DriverCoreBase.state == State::Ready;
                }

                ensures {
                    hwrng_core_initialized(HwRngCore);
                    hwrng_core_registry_ready(HwRngCore);
                    hwrng_core_current_slot_ready(HwRngCore);
                    hwrng_core_miscdev_deferred(HwRngCore);
                    hwrng_core_random_pool_deferred(HwRngCore);
                    hwrng_core_user_selection_deferred(HwRngCore);
                    hwrng_core_quality_policy_deferred(HwRngCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            hwrng_core_initialized(HwRngCore);
            hwrng_core_registry_ready(HwRngCore);
            hwrng_core_current_slot_ready(HwRngCore);
            hwrng_core_miscdev_deferred(HwRngCore);
            hwrng_core_random_pool_deferred(HwRngCore);
            hwrng_core_user_selection_deferred(HwRngCore);
            hwrng_core_quality_policy_deferred(HwRngCore);
        }

        actions {
            Action::Register(device: HwRngDevice) {
                state_effect: StateEffect::None;
                depends_on {
                    HwRngCore.state == State::Ready;
                    HwRngDevice.state == State::Ready;
                    hwrng_device_read_callback_bound(device);
                    hwrng_device_name_bound(device);
                }

                ensures {
                    hwrng_core_register_called(HwRngCore, device);
                    hwrng_core_register_return_zero(HwRngCore, device);
                    hwrng_device_registered(device, HwRngCore);
                    hwrng_core_rng_list_contains(HwRngCore, device);
                    hwrng_core_rng_list_nonempty(HwRngCore);
                    hwrng_device_quality_defaulted(device);
                    hwrng_core_current_rng_set(HwRngCore, device);
                    hwrng_device_current(device, HwRngCore);
                    hwrng_device_duplicate_name_rejected(device, HwRngCore);
                }
            }

            Action::ReadCurrent {
                state_effect: StateEffect::None;
                depends_on {
                    HwRngCore.state == State::Ready;
                    HwRngDevice.state == State::Ready;
                    hwrng_device_current(HwRngDevice, HwRngCore);
                    hwrng_device_read_callback_bound(HwRngDevice);
                }
                drives {
                    HwRngDevice.Action::Read;
                }
                ensures {
                    hwrng_core_current_rng_ref_acquired(HwRngCore, HwRngDevice);
                    hwrng_core_current_rng_read_invoked(HwRngCore, HwRngDevice);
                    hwrng_core_read_copies_from_current(HwRngCore, HwRngDevice);
                    hwrng_core_read_returns_nonzero(HwRngCore);
                    hwrng_core_read_nonblocking(HwRngCore);
                }
            }

            Action::Unregister(device: HwRngDevice) {
                state_effect: StateEffect::None;
                depends_on {
                    HwRngCore.state == State::Ready;
                    HwRngDevice.state == State::Ready;
                    hwrng_device_registered(device, HwRngCore);
                }

                ensures {
                    hwrng_device_unregistered(device, HwRngCore);
                }
            }
        }
    }
}

object HwRngDevice: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioRngDevice.state == State::Ready;
                }

                ensures {
                    hwrng_device_allocated(HwRngDevice);
                    hwrng_device_name_bound(HwRngDevice);
                    hwrng_device_provider_is_virtio_rng(HwRngDevice, VirtioRngDevice);
                    hwrng_device_read_callback_bound(HwRngDevice);
                    hwrng_device_cleanup_callback_bound(HwRngDevice);
                    hwrng_device_priv_points_to_provider(HwRngDevice, VirtioRngDevice);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            hwrng_device_allocated(HwRngDevice);
            hwrng_device_name_bound(HwRngDevice);
            hwrng_device_provider_is_virtio_rng(HwRngDevice, VirtioRngDevice);
            hwrng_device_read_callback_bound(HwRngDevice);
            hwrng_device_cleanup_callback_bound(HwRngDevice);
            hwrng_device_priv_points_to_provider(HwRngDevice, VirtioRngDevice);
        }

        actions {
            Action::Read {
                state_effect: StateEffect::None;
                depends_on {
                    HwRngDevice.state == State::Ready;
                    VirtioRngDevice.state == State::Ready;
                    hwrng_device_provider_is_virtio_rng(HwRngDevice, VirtioRngDevice);
                }
                drives {
                    VirtioRngDevice.Action::ReadEntropy;
                }
                ensures {
                    hwrng_core_current_rng_read_invoked(HwRngCore, HwRngDevice);
                }
            }
        }
    }
}
