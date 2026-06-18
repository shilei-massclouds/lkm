/*
 * Virtio RNG driver/device model.
 *
 * This follows the small core of Linux drivers/char/hw_random/virtio-rng.c:
 * a virtio driver matches VIRTIO_ID_RNG, probes a generic VirtioDevice, owns a
 * single input VirtQueue, submits one input buffer during probe_common,
 * receives a used-buffer completion through the virtio-mmio IRQ callback, and
 * updates data_avail/data_idx. hwrng registration, random pool integration,
 * user-visible reads, freeze/restore, and full remove/reset teardown stay
 * deferred. Fake completion remains an object/smoke fixture path and must not
 * be driven by checkpoint/KUnit handlers.
 */

predicate virtio_rng_driver_declared<T>(driver: T) -> bool;
predicate virtio_rng_driver_name_bound<T>(driver: T) -> bool;
predicate virtio_rng_driver_id_table_contains_rng<T>(driver: T) -> bool;
predicate virtio_rng_driver_matches_device<T, D>(driver: T, device: D) -> bool;
predicate virtio_rng_driver_probe_called<T, D>(driver: T, device: D) -> bool;
predicate virtio_rng_driver_probe_return_zero<T, D>(driver: T, device: D) -> bool;

predicate virtio_rng_device_allocated<T>(device: T) -> bool;
predicate virtio_rng_device_bound_to_virtio_device<T, D>(device: T, virtio_device: D) -> bool;
predicate virtio_rng_device_single_input_queue<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_rng_device_ready<T>(device: T) -> bool;
predicate virtio_rng_hwrng_registration_deferred<T>(device: T) -> bool;
predicate virtio_rng_random_pool_deferred<T>(device: T) -> bool;
predicate virtio_rng_user_api_deferred<T>(device: T) -> bool;
predicate virtio_rng_real_notify_irq_ready<T>(device: T) -> bool;
predicate virtio_rng_probe_common_requests_entropy<T>(device: T) -> bool;

predicate virtio_rng_request_pending<T>(device: T) -> bool;
predicate virtio_rng_request_submits_inbuf<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_rng_request_kicks_queue<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_rng_request_notifies_mmio<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_rng_request_count_incremented<T>(device: T) -> bool;
predicate virtio_rng_repeat_request_rejected<T>(device: T) -> bool;
predicate virtio_rng_zero_len_completion_rejected<T>(device: T) -> bool;
predicate virtio_rng_fake_transport_completion_recorded<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_rng_mmio_irq_acknowledged<T>(device: T) -> bool;
predicate virtio_rng_irq_callback_invoked<T>(device: T) -> bool;
predicate virtio_rng_complete_gets_used_buffer<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_rng_complete_updates_data_avail<T>(device: T) -> bool;
predicate virtio_rng_complete_resets_data_idx<T>(device: T) -> bool;
predicate virtio_rng_completion_count_incremented<T>(device: T) -> bool;
predicate virtio_rng_device_removed<T>(device: T) -> bool;
predicate virtio_rng_removed_rejects_io<T>(device: T) -> bool;

object VirtioRngDriver: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioBus.state == State::Ready;
                }

                ensures {
                    virtio_rng_driver_declared(self);
                    virtio_rng_driver_name_bound(self);
                    virtio_rng_driver_id_table_contains_rng(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_rng_driver_declared(self);
            virtio_rng_driver_name_bound(self);
            virtio_rng_driver_id_table_contains_rng(self);
        }
    }
}

object VirtioRngDevice: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioRngDriver.state == State::Ready;
                    VirtioDevice.state == State::Ready;
                    virtio_device_rng_id(VirtioDevice);
                }

                drives {
                    VirtioSplitRing.Event::Setup;
                    VirtQueue.Event::Setup;
                }

                ensures {
                    virtio_rng_driver_matches_device(VirtioRngDriver, VirtioDevice);
                    virtio_rng_driver_probe_called(VirtioRngDriver, VirtioDevice);
                    virtio_rng_driver_probe_return_zero(VirtioRngDriver, VirtioDevice);
                    virtio_rng_device_allocated(self);
                    virtio_rng_device_bound_to_virtio_device(self, VirtioDevice);
                    virtio_rng_device_single_input_queue(self, VirtQueue);
                    virtio_rng_device_ready(self);
                    virtio_rng_hwrng_registration_deferred(self);
                    virtio_rng_random_pool_deferred(self);
                    virtio_rng_user_api_deferred(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_rng_device_allocated(self);
            virtio_rng_device_bound_to_virtio_device(self, VirtioDevice);
            virtio_rng_device_single_input_queue(self, VirtQueue);
            virtio_rng_device_ready(self);
            virtio_rng_hwrng_registration_deferred(self);
            virtio_rng_random_pool_deferred(self);
            virtio_rng_user_api_deferred(self);
        }

        events {
            on Event::Cleanup -> State::Destroyed {
                ensures {
                    virtio_rng_device_removed(self);
                    virtio_rng_removed_rejects_io(self);
                }
            }
        }

        processes {
            Action::SetupRealTransport {
                state_effect: StateEffect::None;
                depends_on {
                    VirtioMmioTransportDevice.state == State::Ready;
                    VirtQueue.state == State::Ready;
                }
                ensures {
                    virtio_rng_real_notify_irq_ready(self);
                    virtio_rng_probe_common_requests_entropy(self);
                    virtio_rng_request_pending(self);
                }
            }

            Action::RequestEntropy {
                state_effect: StateEffect::None;
                depends_on {
                    VirtQueue.state == State::Ready;
                }
                drives {
                    VirtQueue.Action::AddInbuf;
                    VirtQueue.Action::Kick;
                }
                ensures {
                    virtio_rng_request_pending(self);
                    virtio_rng_request_submits_inbuf(self, VirtQueue);
                    virtio_rng_request_kicks_queue(self, VirtQueue);
                    virtio_rng_request_notifies_mmio(self, VirtQueue);
                    virtio_rng_request_count_incremented(self);
                    virtio_rng_repeat_request_rejected(self);
                }
            }

            Action::FakeTransportComplete {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_rng_request_pending(self);
                }
                drives {
                    VirtQueue.Action::FakeCompleteUsed;
                }
                ensures {
                    virtio_rng_fake_transport_completion_recorded(self, VirtQueue);
                    virtio_rng_zero_len_completion_rejected(self);
                }
            }

            Action::CompleteEntropy {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_rng_fake_transport_completion_recorded(self, VirtQueue) || virtio_rng_irq_callback_invoked(self);
                }
                drives {
                    VirtQueue.Action::GetBuf;
                }
                ensures {
                    virtio_rng_complete_gets_used_buffer(self, VirtQueue);
                    virtio_rng_complete_updates_data_avail(self);
                    virtio_rng_complete_resets_data_idx(self);
                    virtio_rng_completion_count_incremented(self);
                }
            }

            Action::VirtioMmioIrqComplete {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_rng_request_pending(self);
                    virtqueue_real_used_completion_observed(VirtQueue);
                }
                drives {
                    VirtQueue.Action::GetBuf;
                }
                ensures {
                    virtio_rng_mmio_irq_acknowledged(self);
                    virtio_rng_irq_callback_invoked(self);
                    virtio_rng_complete_gets_used_buffer(self, VirtQueue);
                    virtio_rng_complete_updates_data_avail(self);
                    virtio_rng_complete_resets_data_idx(self);
                    virtio_rng_completion_count_incremented(self);
                }
            }
        }
    }

    state State::Destroyed {
        invariant {
            virtio_rng_device_removed(self);
            virtio_rng_removed_rejects_io(self);
        }
    }
}
