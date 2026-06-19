/*
 * Virtio block driver/device model, first discovery/config slice.
 *
 * This follows the early shape of Linux drivers/block/virtio_blk.c without
 * entering the block layer or submitting I/O: a virtio driver matches
 * VIRTIO_ID_BLOCK, probes a generic VirtioDevice, reads the block capacity
 * from virtio config space, sets up one request virtqueue, and reaches
 * DRIVER_OK. Request construction, interrupts/completion, tag sets, gendisk,
 * partitions, filesystem use, flush/discard/write-zeroes, multi-queue and
 * reset/remove remain deferred. checkpoint/KUnit handlers are read-only
 * observers and must not drive blk, ring, transport or bus actions.
 */

predicate virtio_blk_driver_declared<T>(driver: T) -> bool;
predicate virtio_blk_driver_name_bound<T>(driver: T) -> bool;
predicate virtio_blk_driver_id_table_contains_block<T>(driver: T) -> bool;
predicate virtio_blk_driver_matches_device<T, D>(driver: T, device: D) -> bool;
predicate virtio_blk_driver_probe_called<T, D>(driver: T, device: D) -> bool;
predicate virtio_blk_driver_probe_return_zero<T, D>(driver: T, device: D) -> bool;

predicate virtio_blk_device_allocated<T>(device: T) -> bool;
predicate virtio_blk_device_bound_to_virtio_device<T, D>(device: T, virtio_device: D) -> bool;
predicate virtio_blk_device_single_request_queue<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_blk_device_capacity_read<T>(device: T) -> bool;
predicate virtio_blk_device_capacity_nonzero<T>(device: T) -> bool;
predicate virtio_blk_device_queue_setup_done<T>(device: T) -> bool;
predicate virtio_blk_device_driver_ok<T>(device: T) -> bool;
predicate virtio_blk_device_ready<T>(device: T) -> bool;
predicate virtio_blk_request_io_deferred<T>(device: T) -> bool;
predicate virtio_blk_block_layer_deferred<T>(device: T) -> bool;
predicate virtio_blk_multi_queue_deferred<T>(device: T) -> bool;
predicate virtio_blk_reset_remove_deferred<T>(device: T) -> bool;

object VirtioBlkDriver: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioBus.state == State::Ready;
                }

                ensures {
                    virtio_blk_driver_declared(self);
                    virtio_blk_driver_name_bound(self);
                    virtio_blk_driver_id_table_contains_block(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_blk_driver_declared(self);
            virtio_blk_driver_name_bound(self);
            virtio_blk_driver_id_table_contains_block(self);
        }
    }
}

object VirtioBlkDevice: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioBlkDriver.state == State::Ready;
                    VirtioDevice.state == State::Ready;
                    virtio_device_block_id(VirtioDevice);
                }

                drives {
                    VirtioSplitRing.Event::Setup;
                    VirtQueue.Event::Setup;
                }

                ensures {
                    virtio_blk_driver_matches_device(VirtioBlkDriver, VirtioDevice);
                    virtio_blk_driver_probe_called(VirtioBlkDriver, VirtioDevice);
                    virtio_blk_driver_probe_return_zero(VirtioBlkDriver, VirtioDevice);
                    virtio_blk_device_allocated(self);
                    virtio_blk_device_bound_to_virtio_device(self, VirtioDevice);
                    virtio_blk_device_single_request_queue(self, VirtQueue);
                    virtio_blk_request_io_deferred(self);
                    virtio_blk_block_layer_deferred(self);
                    virtio_blk_multi_queue_deferred(self);
                    virtio_blk_reset_remove_deferred(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_blk_device_allocated(self);
            virtio_blk_device_bound_to_virtio_device(self, VirtioDevice);
            virtio_blk_device_single_request_queue(self, VirtQueue);
            virtio_blk_request_io_deferred(self);
            virtio_blk_block_layer_deferred(self);
            virtio_blk_multi_queue_deferred(self);
            virtio_blk_reset_remove_deferred(self);
        }

        processes {
            Action::SetupRealTransport {
                state_effect: StateEffect::None;
                depends_on {
                    VirtioDevice.state == State::Ready;
                    VirtioMmioTransportDevice.state == State::Ready;
                    VirtQueue.state == State::Ready;
                }
                drives {
                    VirtioDevice.Action::ResetStatus;
                    VirtioDevice.Action::SetupDriverStatus;
                    VirtioDevice.Action::NegotiateFeatures;
                    VirtioDevice.Action::ReadConfig;
                    VirtioDevice.Action::SetupQueue;
                    VirtioDevice.Action::SetDriverOk;
                }
                ensures {
                    virtio_device_status_reset(VirtioDevice);
                    virtio_device_status_acknowledged(VirtioDevice);
                    virtio_device_status_driver_seen(VirtioDevice);
                    virtio_device_features_read(VirtioDevice);
                    virtio_device_driver_features_written(VirtioDevice);
                    virtio_device_feature_negotiation_done(VirtioDevice);
                    virtio_device_config_capacity_read(VirtioDevice);
                    virtio_device_queue_setup_done(VirtioDevice);
                    virtio_device_status_driver_ok(VirtioDevice);
                    virtio_blk_device_capacity_read(self);
                    virtio_blk_device_capacity_nonzero(self);
                    virtio_blk_device_queue_setup_done(self);
                    virtio_blk_device_driver_ok(self);
                    virtio_blk_device_ready(self);
                }
            }
        }
    }
}
