/*
 * Virtio block driver/device model, first read-request slice.
 *
 * This follows the early shape of Linux drivers/block/virtio_blk.c through one
 * real read request and a minimal block-core registration surface: a virtio
 * driver matches VIRTIO_ID_BLOCK, probes a generic VirtioDevice, reads the
 * block capacity, sets up one request virtqueue, reaches DRIVER_OK, submits a
 * virtio_blk_outhdr + data buffer + status byte descriptor chain, notifies the
 * queue, consumes the device used-buffer completion, registers a BlockDevice so
 * the disk is discoverable by major/minor or default lookup, and serves a
 * minimal block-level read through that registered BlockDevice. The default
 * test disk may be formatted as ext2 so the read buffer has deterministic
 * nonzero bytes, but this spec does not introduce an ext2/filesystem object.
 * Full blk-mq tag sets, request_queue, bio/page cache, partition scan,
 * flush/discard/write-zeroes, multi-queue and reset/remove remain deferred.
 * checkpoint/KUnit handlers are read-only observers and must not drive blk,
 * ring, transport, block registry or bus actions.
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
predicate virtio_blk_read_header_prepared<T>(device: T) -> bool;
predicate virtio_blk_read_data_buffer_prepared<T>(device: T) -> bool;
predicate virtio_blk_read_status_buffer_prepared<T>(device: T) -> bool;
predicate virtio_blk_read_request_submitted<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_blk_read_request_notifies_mmio<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_blk_read_request_pending<T>(device: T) -> bool;
predicate virtio_blk_mmio_irq_acknowledged<T>(device: T) -> bool;
predicate virtio_blk_irq_callback_invoked<T>(device: T) -> bool;
predicate virtio_blk_complete_gets_used_buffer<T, Q>(device: T, queue: Q) -> bool;
predicate virtio_blk_complete_status_ok<T>(device: T) -> bool;
predicate virtio_blk_complete_data_nonzero<T>(device: T) -> bool;
predicate virtio_blk_complete_ext2_magic_observed<T>(device: T) -> bool;
predicate virtio_blk_completion_count_incremented<T>(device: T) -> bool;
predicate virtio_blk_read_request_done<T>(device: T) -> bool;
predicate virtio_blk_block_device_embedded<T, D>(device: T, block_device: D) -> bool;
predicate virtio_blk_registers_block_device<T, D>(device: T, block_device: D) -> bool;
predicate virtio_blk_block_device_registered<T, D, C>(device: T, block_device: D, core: C) -> bool;
predicate virtio_blk_serves_block_read<T, D>(device: T, block_device: D) -> bool;
predicate virtio_blk_block_read_copies_to_caller<T, D>(device: T, block_device: D) -> bool;
predicate virtio_blk_filesystem_parse_deferred<T>(device: T) -> bool;
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
                    virtio_blk_block_device_embedded(self, BlockDevice);
                    virtio_blk_filesystem_parse_deferred(self);
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
            virtio_blk_block_device_embedded(self, BlockDevice);
            virtio_blk_filesystem_parse_deferred(self);
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
                    VirtioMmioTransportDevice.Action::EnableIrqSourceGate;
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
                    virtio_mmio_transport_irq_source_gate_open(VirtioMmioTransportDevice);
                    virtio_blk_device_capacity_read(self);
                    virtio_blk_device_capacity_nonzero(self);
                    virtio_blk_device_queue_setup_done(self);
                    virtio_blk_device_driver_ok(self);
                    virtio_blk_device_ready(self);
                }
            }

            Action::SubmitReadRequest {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_blk_device_ready(self);
                    virtio_device_status_driver_ok(VirtioDevice);
                    VirtQueue.state == State::Ready;
                }
                drives {
                    VirtQueue.Action::AddDescriptorChain;
                    VirtQueue.Action::Kick;
                    VirtioDevice.Action::NotifyQueue;
                }
                ensures {
                    virtqueue_out_descriptor_added(VirtQueue);
                    virtqueue_in_descriptor_added(VirtQueue);
                    virtqueue_chain_head_published(VirtQueue);
                    virtio_device_queue_notify_done(VirtioDevice);
                    virtio_blk_read_header_prepared(self);
                    virtio_blk_read_data_buffer_prepared(self);
                    virtio_blk_read_status_buffer_prepared(self);
                    virtio_blk_read_request_submitted(self, VirtQueue);
                    virtio_blk_read_request_notifies_mmio(self, VirtQueue);
                    virtio_blk_read_request_pending(self);
                }
            }

            Action::CompleteReadRequest {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_blk_read_request_pending(self);
                    virtqueue_real_used_completion_observed(VirtQueue);
                }
                drives {
                    VirtQueue.Action::GetBuf;
                }
                ensures {
                    virtio_blk_mmio_irq_acknowledged(self);
                    virtio_blk_irq_callback_invoked(self);
                    virtio_blk_complete_gets_used_buffer(self, VirtQueue);
                    virtio_blk_complete_status_ok(self);
                    virtio_blk_complete_data_nonzero(self);
                    virtio_blk_complete_ext2_magic_observed(self);
                    virtio_blk_completion_count_incremented(self);
                    virtio_blk_read_request_done(self);
                }
            }

            Action::RegisterBlockDevice {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_blk_device_ready(self);
                    virtio_blk_device_capacity_nonzero(self);
                    BlockDeviceRegistry.state == State::Ready;
                    BlockDevice.state == State::Ready;
                }
                drives {
                    BlockDeviceRegistry.Action::Register(BlockDevice);
                }
                ensures {
                    virtio_blk_registers_block_device(self, BlockDevice);
                    block_device_provider_is_virtio_blk(BlockDevice, self);
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                    block_core_major_minor_lookup_returns(BlockDeviceRegistry, BlockDevice);
                    virtio_blk_block_device_registered(self, BlockDevice, BlockDeviceRegistry);
                }
            }

            Action::ServeBlockRead {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_blk_device_ready(self);
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                    block_device_provider_is_virtio_blk(BlockDevice, self);
                    VirtQueue.state == State::Ready;
                }
                drives {
                    self.Action::SubmitReadRequest;
                    self.Action::CompleteReadRequest;
                }
                ensures {
                    virtio_blk_serves_block_read(self, BlockDevice);
                    virtio_blk_read_request_done(self);
                    virtio_blk_complete_status_ok(self);
                    virtio_blk_complete_data_nonzero(self);
                    virtio_blk_block_read_copies_to_caller(self, BlockDevice);
                }
            }
        }
    }
}
