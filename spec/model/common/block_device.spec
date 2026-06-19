/*
 * Minimal block device core registry model.
 *
 * This follows the Linux block surface only far enough for virtio-blk to
 * publish a system-visible disk identity. Linux virtio_blk allocates a gendisk,
 * binds a major/first_minor, sets capacity, and calls device_add_disk(). This
 * first slice keeps those observable registry facts and exposes a minimal
 * block-level read entry, but defers blk-mq tag sets, request_queue, bio/page
 * cache, partition scan, uevents, sysfs, and /dev node plumbing. Users must
 * discover the registered device through BlockDeviceRegistry rather than
 * reaching directly into VirtioBlkDevice for system-level lookup.
 */

predicate block_core_initialized<T>(core: T) -> bool;
predicate block_core_registry_ready<T>(core: T) -> bool;
predicate block_core_major_allocator_ready<T>(core: T) -> bool;
predicate block_core_default_device_slot_ready<T>(core: T) -> bool;
predicate block_core_request_queue_deferred<T>(core: T) -> bool;
predicate block_core_bio_page_cache_deferred<T>(core: T) -> bool;
predicate block_core_partition_scan_deferred<T>(core: T) -> bool;
predicate block_core_dev_node_deferred<T>(core: T) -> bool;

predicate block_device_allocated<T>(device: T) -> bool;
predicate block_device_name_bound<T>(device: T) -> bool;
predicate block_device_provider_is_virtio_blk<T, V>(device: T, provider: V) -> bool;
predicate block_device_read_callback_bound<T>(device: T) -> bool;
predicate block_device_capacity_bound<T>(device: T) -> bool;
predicate block_device_major_minor_bound<T>(device: T) -> bool;
predicate block_device_registered<T, C>(device: T, core: C) -> bool;
predicate block_device_default<T, C>(device: T, core: C) -> bool;
predicate block_device_read_submitted<T>(device: T) -> bool;
predicate block_device_read_completion_observed<T>(device: T) -> bool;
predicate block_device_read_copies_to_caller<T>(device: T) -> bool;
predicate block_device_read_returns_nonzero<T>(device: T) -> bool;
predicate block_device_read_count_incremented<T>(device: T) -> bool;

predicate block_core_register_blkdev_called<T>(core: T) -> bool;
predicate block_core_register_blkdev_returned_major<T>(core: T) -> bool;
predicate block_core_device_add_disk_called<T, D>(core: T, device: D) -> bool;
predicate block_core_device_add_disk_return_zero<T, D>(core: T, device: D) -> bool;
predicate block_core_gendisk_list_contains<T, D>(core: T, device: D) -> bool;
predicate block_core_major_minor_lookup_ready<T>(core: T) -> bool;
predicate block_core_major_minor_lookup_returns<T, D>(core: T, device: D) -> bool;
predicate block_core_default_device_set<T, D>(core: T, device: D) -> bool;
predicate block_core_default_device_ref_acquired<T, D>(core: T, device: D) -> bool;
predicate block_core_major_minor_ref_acquired<T, D>(core: T, device: D) -> bool;
predicate block_core_read_invokes_provider<T, D>(core: T, device: D) -> bool;
predicate block_core_read_completion_observed<T, D>(core: T, device: D) -> bool;
predicate block_core_read_copies_to_caller<T, D>(core: T, device: D) -> bool;
predicate block_core_read_returns_nonzero<T>(core: T) -> bool;

object BlockDeviceRegistry: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DriverCoreBase.state == State::Ready;
                }

                ensures {
                    block_core_initialized(BlockDeviceRegistry);
                    block_core_registry_ready(BlockDeviceRegistry);
                    block_core_major_allocator_ready(BlockDeviceRegistry);
                    block_core_default_device_slot_ready(BlockDeviceRegistry);
                    block_core_request_queue_deferred(BlockDeviceRegistry);
                    block_core_bio_page_cache_deferred(BlockDeviceRegistry);
                    block_core_partition_scan_deferred(BlockDeviceRegistry);
                    block_core_dev_node_deferred(BlockDeviceRegistry);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            block_core_initialized(BlockDeviceRegistry);
            block_core_registry_ready(BlockDeviceRegistry);
            block_core_major_allocator_ready(BlockDeviceRegistry);
            block_core_default_device_slot_ready(BlockDeviceRegistry);
            block_core_request_queue_deferred(BlockDeviceRegistry);
            block_core_bio_page_cache_deferred(BlockDeviceRegistry);
            block_core_partition_scan_deferred(BlockDeviceRegistry);
            block_core_dev_node_deferred(BlockDeviceRegistry);
        }

        actions {
            Action::Register(device: BlockDevice) {
                state_effect: StateEffect::None;
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    BlockDevice.state == State::Ready;
                    block_device_provider_is_virtio_blk(device, VirtioBlkDevice);
                    block_device_read_callback_bound(device);
                    block_device_capacity_bound(device);
                    block_device_name_bound(device);
                }

                ensures {
                    block_core_register_blkdev_called(BlockDeviceRegistry);
                    block_core_register_blkdev_returned_major(BlockDeviceRegistry);
                    block_core_device_add_disk_called(BlockDeviceRegistry, device);
                    block_core_device_add_disk_return_zero(BlockDeviceRegistry, device);
                    block_device_major_minor_bound(device);
                    block_device_registered(device, BlockDeviceRegistry);
                    block_core_gendisk_list_contains(BlockDeviceRegistry, device);
                    block_core_major_minor_lookup_ready(BlockDeviceRegistry);
                    block_core_major_minor_lookup_returns(BlockDeviceRegistry, device);
                    block_core_default_device_set(BlockDeviceRegistry, device);
                    block_device_default(device, BlockDeviceRegistry);
                }
            }

            Action::ReadDefault {
                state_effect: StateEffect::None;
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    BlockDevice.state == State::Ready;
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                    block_device_read_callback_bound(BlockDevice);
                }
                drives {
                    BlockDevice.Action::Read;
                }
                ensures {
                    block_core_default_device_ref_acquired(BlockDeviceRegistry, BlockDevice);
                    block_core_read_invokes_provider(BlockDeviceRegistry, BlockDevice);
                    block_core_read_completion_observed(BlockDeviceRegistry, BlockDevice);
                    block_core_read_copies_to_caller(BlockDeviceRegistry, BlockDevice);
                    block_core_read_returns_nonzero(BlockDeviceRegistry);
                }
            }

            Action::ReadByMajorMinor {
                state_effect: StateEffect::None;
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    BlockDevice.state == State::Ready;
                    block_core_major_minor_lookup_returns(BlockDeviceRegistry, BlockDevice);
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                    block_device_read_callback_bound(BlockDevice);
                }
                drives {
                    BlockDevice.Action::Read;
                }
                ensures {
                    block_core_major_minor_ref_acquired(BlockDeviceRegistry, BlockDevice);
                    block_core_read_invokes_provider(BlockDeviceRegistry, BlockDevice);
                    block_core_read_completion_observed(BlockDeviceRegistry, BlockDevice);
                    block_core_read_copies_to_caller(BlockDeviceRegistry, BlockDevice);
                    block_core_read_returns_nonzero(BlockDeviceRegistry);
                }
            }
        }
    }
}

object BlockDevice: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioBlkDevice.state == State::Ready;
                    virtio_blk_device_capacity_nonzero(VirtioBlkDevice);
                }

                ensures {
                    block_device_allocated(BlockDevice);
                    block_device_name_bound(BlockDevice);
                    block_device_provider_is_virtio_blk(BlockDevice, VirtioBlkDevice);
                    block_device_read_callback_bound(BlockDevice);
                    block_device_capacity_bound(BlockDevice);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            block_device_allocated(BlockDevice);
            block_device_name_bound(BlockDevice);
            block_device_provider_is_virtio_blk(BlockDevice, VirtioBlkDevice);
            block_device_read_callback_bound(BlockDevice);
            block_device_capacity_bound(BlockDevice);
        }

        actions {
            Action::Read {
                state_effect: StateEffect::None;
                depends_on {
                    BlockDevice.state == State::Ready;
                    VirtioBlkDevice.state == State::Ready;
                    block_device_provider_is_virtio_blk(BlockDevice, VirtioBlkDevice);
                    block_device_read_callback_bound(BlockDevice);
                }
                drives {
                    VirtioBlkDevice.Action::ServeBlockRead;
                }
                ensures {
                    block_device_read_submitted(BlockDevice);
                    block_device_read_completion_observed(BlockDevice);
                    block_device_read_copies_to_caller(BlockDevice);
                    block_device_read_returns_nonzero(BlockDevice);
                    block_device_read_count_incremented(BlockDevice);
                }
            }
        }
    }
}
