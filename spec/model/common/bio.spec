/*
 * Minimal synchronous bio / buffer_head model.
 *
 * This is the first Linux-like block I/O adapter above BlockDeviceRegistry.
 * It models the traditional ext2-facing path only far enough for one
 * synchronous read:
 *
 *   sb_bread() / __bread_gfp()
 *     -> submit_bio_wait()
 *       -> blk_mq_submit_bio()
 *         -> BlockDeviceRegistry default or dev_t lookup
 *           -> BlockDevice.Action::Read
 *
 * Full request_queue ownership, tag sets, schedulers, page cache, writeback,
 * multi-page bios, async completion callbacks, write/flush/discard, and buffer
 * cache lifetime management remain deferred. BufferHead owns its data buffer,
 * but the data storage must not be a large inline stack object; 4KiB filesystem
 * blocks are carried by heap-backed or equivalent exclusive dynamic storage.
 */

enum BioOp {
    Read,
}

enum BioSubmitPath {
    DefaultBlockDevice,
    MajorMinorLookup,
}

predicate bio_allocated<T>(bio: T) -> bool;
predicate bio_op_is_read<T>(bio: T) -> bool;
predicate bio_targets_block_device<T, D>(bio: T, device: D) -> bool;
predicate bio_sector_bound<T>(bio: T) -> bool;
predicate bio_buffer_bound<T>(bio: T) -> bool;
predicate bio_submit_path_bound<T>(bio: T, path: BioSubmitPath) -> bool;
predicate bio_submit_bio_wait_called<T>(bio: T) -> bool;
predicate bio_blk_mq_submit_bio_entered<T>(bio: T) -> bool;
predicate bio_submitted<T>(bio: T) -> bool;
predicate bio_completion_observed<T>(bio: T) -> bool;
predicate bio_status_ok<T>(bio: T) -> bool;
predicate bio_copies_to_caller<T>(bio: T) -> bool;

predicate buffer_head_allocated<T>(bh: T) -> bool;
predicate buffer_head_targets_block_device<T, D>(bh: T, device: D) -> bool;
predicate buffer_head_sector_bound<T>(bh: T) -> bool;
predicate buffer_head_size_bound<T>(bh: T) -> bool;
predicate buffer_head_data_buffer_owned<T>(bh: T) -> bool;
predicate buffer_head_data_buffer_dynamic_storage<T, R>(bh: T, alloc_ref: R) -> bool;
predicate buffer_head_no_large_inline_stack_data<T>(bh: T) -> bool;
predicate buffer_head_sb_bread_called<T>(bh: T) -> bool;
predicate buffer_head_bread_gfp_called<T>(bh: T) -> bool;
predicate buffer_head_uses_submit_bio_wait<T, B>(bh: T, bio: B) -> bool;
predicate buffer_head_uptodate<T>(bh: T) -> bool;
predicate buffer_head_data_ready<T>(bh: T) -> bool;
predicate buffer_head_data_nonzero<T>(bh: T) -> bool;
predicate buffer_head_ext2_superblock_sector_read<T>(bh: T) -> bool;

object Bio: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    BlockDevice.state == State::Online;
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                    block_device_read_callback_bound(BlockDevice);
                }

                ensures {
                    bio_allocated(self);
                    bio_op_is_read(self);
                    bio_targets_block_device(self, BlockDevice);
                    bio_sector_bound(self);
                    bio_buffer_bound(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            bio_allocated(self);
            bio_op_is_read(self);
            bio_targets_block_device(self, BlockDevice);
            bio_sector_bound(self);
            bio_buffer_bound(self);
        }

        actions {
            Action::SubmitBioWaitDefault {
                state_effect: StateEffect::None;
                depends_on {
                    Bio.state == State::Ready;
                    BlockDeviceRegistry.state == State::Ready;
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                }
                drives {
                    BlockDeviceRegistry.Action::ReadDefault;
                }
                ensures {
                    bio_submit_path_bound(self, BioSubmitPath::DefaultBlockDevice);
                    bio_submit_bio_wait_called(self);
                    bio_blk_mq_submit_bio_entered(self);
                    bio_submitted(self);
                    bio_completion_observed(self);
                    bio_status_ok(self);
                    bio_copies_to_caller(self);
                }
            }

            Action::SubmitBioWaitByMajorMinor {
                state_effect: StateEffect::None;
                depends_on {
                    Bio.state == State::Ready;
                    BlockDeviceRegistry.state == State::Ready;
                    block_core_major_minor_lookup_returns(BlockDeviceRegistry, BlockDevice);
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                }
                drives {
                    BlockDeviceRegistry.Action::ReadByMajorMinor;
                }
                ensures {
                    bio_submit_path_bound(self, BioSubmitPath::MajorMinorLookup);
                    bio_submit_bio_wait_called(self);
                    bio_blk_mq_submit_bio_entered(self);
                    bio_submitted(self);
                    bio_completion_observed(self);
                    bio_status_ok(self);
                    bio_copies_to_caller(self);
                }
            }
        }
    }
}

object BufferHead: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    BlockDeviceRegistry.state == State::Ready;
                    BlockDevice.state == State::Online;
                    block_device_registered(BlockDevice, BlockDeviceRegistry);
                    Bio.state == State::Ready;
                    KernelGlobalAllocator.state == State::Ready;
                    kernel_global_allocator_alloc_zeroed_api_ready(KernelGlobalAllocator);
                }

                ensures {
                    buffer_head_allocated(self);
                    buffer_head_targets_block_device(self, BlockDevice);
                    buffer_head_sector_bound(self);
                    buffer_head_size_bound(self);
                    buffer_head_data_buffer_owned(self);
                    buffer_head_data_buffer_dynamic_storage(self, HeapAllocRef);
                    buffer_head_no_large_inline_stack_data(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            buffer_head_allocated(self);
            buffer_head_targets_block_device(self, BlockDevice);
            buffer_head_sector_bound(self);
            buffer_head_size_bound(self);
            buffer_head_data_buffer_owned(self);
            buffer_head_data_buffer_dynamic_storage(self, HeapAllocRef);
            buffer_head_no_large_inline_stack_data(self);
        }

        actions {
            Action::SbBreadDefault {
                state_effect: StateEffect::None;
                depends_on {
                    BufferHead.state == State::Ready;
                    Bio.state == State::Ready;
                    block_device_default(BlockDevice, BlockDeviceRegistry);
                }
                drives {
                    Bio.Action::SubmitBioWaitDefault;
                }
                ensures {
                    buffer_head_sb_bread_called(self);
                    buffer_head_bread_gfp_called(self);
                    buffer_head_uses_submit_bio_wait(self, Bio);
                    buffer_head_uptodate(self);
                    buffer_head_data_ready(self);
                }
            }

            Action::SbBreadByMajorMinor {
                state_effect: StateEffect::None;
                depends_on {
                    BufferHead.state == State::Ready;
                    Bio.state == State::Ready;
                    block_core_major_minor_lookup_returns(BlockDeviceRegistry, BlockDevice);
                }
                drives {
                    Bio.Action::SubmitBioWaitByMajorMinor;
                }
                ensures {
                    buffer_head_sb_bread_called(self);
                    buffer_head_bread_gfp_called(self);
                    buffer_head_uses_submit_bio_wait(self, Bio);
                    buffer_head_uptodate(self);
                    buffer_head_data_ready(self);
                }
            }
        }
    }
}
