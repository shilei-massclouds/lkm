/*
 * Virtio split virtqueue model, first slice.
 *
 * This models the reusable virtio_ring.c split-ring boundary needed by
 * virtio-rng and the virtio-blk prework. The object-level slice uses
 * Vec-backed direct descriptors; smoke fixtures may construct a ready
 * used-ring entry as test data, but that construction is not a VirtQueue
 * action/API. The real QEMU slice adds one device-visible split ring backing,
 * MMIO notify, and used-ring get_buf after a virtio-mmio interrupt. Direct
 * descriptor chains are supported so a request can publish out/in descriptors
 * under one avail head. Packed rings, indirect descriptors, event idx,
 * multi-queue devices, a general DMA allocator, cache maintenance,
 * reset/remove, suspend/resume, and filesystem users stay deferred.
 */

predicate virtio_split_ring_allocated<T>(ring: T) -> bool;
predicate virtio_split_ring_queue_size_bound<T>(ring: T) -> bool;
predicate virtio_split_ring_backing_sized_by_queue<T>(ring: T) -> bool;
predicate virtio_split_ring_descriptor_table_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_avail_ring_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_used_ring_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_free_list_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_single_queue<T>(ring: T) -> bool;
predicate virtio_split_ring_direct_descriptor_chain_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_indirect_descriptors_deferred<T>(ring: T) -> bool;
predicate virtio_split_ring_event_idx_deferred<T>(ring: T) -> bool;
predicate virtio_split_ring_dma_cache_deferred<T>(ring: T) -> bool;
predicate virtio_split_ring_static_coherent_backing_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_desc_avail_used_layout_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_device_visible_phys_addr_ready<T>(ring: T) -> bool;

predicate virtqueue_split_ring_bound<T, R>(queue: T, ring: R) -> bool;
predicate virtqueue_index_bound<T>(queue: T) -> bool;
predicate virtqueue_num_max_observed<T>(queue: T) -> bool;
predicate virtqueue_legacy_mmio_queue_pfn_written<T>(queue: T) -> bool;
predicate virtqueue_modern_mmio_queue_addrs_written<T>(queue: T) -> bool;
predicate virtqueue_mmio_queue_ready_written<T>(queue: T) -> bool;
predicate virtqueue_input_buffer_added<T>(queue: T) -> bool;
predicate virtqueue_descriptor_chain_allocated<T>(queue: T) -> bool;
predicate virtqueue_descriptor_chain_direct<T>(queue: T) -> bool;
predicate virtqueue_out_descriptor_added<T>(queue: T) -> bool;
predicate virtqueue_in_descriptor_added<T>(queue: T) -> bool;
predicate virtqueue_chain_head_published<T>(queue: T) -> bool;
predicate virtqueue_chain_free_descriptor_count_matched<T>(queue: T) -> bool;
predicate virtqueue_chain_allocation_atomic<T>(queue: T) -> bool;
predicate virtqueue_free_descriptors_consumed<T>(queue: T) -> bool;
predicate virtqueue_avail_index_advanced<T>(queue: T) -> bool;
predicate virtqueue_kick_recorded<T>(queue: T) -> bool;
predicate virtqueue_mmio_notify_written<T>(queue: T) -> bool;
predicate virtqueue_descriptor_exhaustion_rejected<T>(queue: T) -> bool;
predicate virtqueue_fixture_used_entry_ready<T>(queue: T) -> bool;
predicate virtqueue_real_used_completion_observed<T>(queue: T) -> bool;
predicate virtqueue_used_index_advanced<T>(queue: T) -> bool;
predicate virtqueue_get_buf_returns_len<T>(queue: T) -> bool;
predicate virtqueue_get_buf_empty_rejected<T>(queue: T) -> bool;
predicate virtqueue_descriptor_chain_released<T>(queue: T) -> bool;
predicate virtqueue_buffer_ownership_released<T>(queue: T) -> bool;

object VirtioSplitRing: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VirtioDevice.state == State::Ready;
                }

                ensures {
                    virtio_split_ring_allocated(self);
                    virtio_split_ring_queue_size_bound(self);
                    virtio_split_ring_backing_sized_by_queue(self);
                    virtio_split_ring_descriptor_table_ready(self);
                    virtio_split_ring_avail_ring_ready(self);
                    virtio_split_ring_used_ring_ready(self);
                    virtio_split_ring_free_list_ready(self);
                    virtio_split_ring_single_queue(self);
                    virtio_split_ring_direct_descriptor_chain_ready(self);
                    virtio_split_ring_indirect_descriptors_deferred(self);
                    virtio_split_ring_event_idx_deferred(self);
                    virtio_split_ring_dma_cache_deferred(self);
                    virtio_split_ring_static_coherent_backing_ready(self);
                    virtio_split_ring_desc_avail_used_layout_ready(self);
                    virtio_split_ring_device_visible_phys_addr_ready(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtio_split_ring_allocated(self);
            virtio_split_ring_queue_size_bound(self);
            virtio_split_ring_backing_sized_by_queue(self);
            virtio_split_ring_descriptor_table_ready(self);
            virtio_split_ring_avail_ring_ready(self);
            virtio_split_ring_used_ring_ready(self);
            virtio_split_ring_free_list_ready(self);
            virtio_split_ring_single_queue(self);
            virtio_split_ring_direct_descriptor_chain_ready(self);
            virtio_split_ring_indirect_descriptors_deferred(self);
            virtio_split_ring_event_idx_deferred(self);
            virtio_split_ring_dma_cache_deferred(self);
            virtio_split_ring_static_coherent_backing_ready(self);
            virtio_split_ring_desc_avail_used_layout_ready(self);
            virtio_split_ring_device_visible_phys_addr_ready(self);
        }
    }
}

object VirtQueue: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VirtioSplitRing.state == State::Ready;
                }

                ensures {
                    virtqueue_split_ring_bound(self, VirtioSplitRing);
                    virtqueue_index_bound(self);
                    virtqueue_num_max_observed(self);
                    virtqueue_legacy_mmio_queue_pfn_written(self);
                    virtqueue_modern_mmio_queue_addrs_written(self);
                    virtqueue_mmio_queue_ready_written(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtqueue_split_ring_bound(self, VirtioSplitRing);
            virtqueue_index_bound(self);
        }

        processes {
            Action::AddChain {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_split_ring_free_list_ready(VirtioSplitRing);
                    virtio_split_ring_direct_descriptor_chain_ready(VirtioSplitRing);
                }
                ensures {
                    virtqueue_descriptor_chain_allocated(self);
                    virtqueue_descriptor_chain_direct(self);
                    virtqueue_out_descriptor_added(self);
                    virtqueue_in_descriptor_added(self);
                    virtqueue_chain_head_published(self);
                    virtqueue_chain_free_descriptor_count_matched(self);
                    virtqueue_chain_allocation_atomic(self);
                    virtqueue_free_descriptors_consumed(self);
                    virtqueue_avail_index_advanced(self);
                    virtqueue_descriptor_exhaustion_rejected(self);
                }
            }

            Action::AddInbuf {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_split_ring_free_list_ready(VirtioSplitRing);
                    virtio_split_ring_direct_descriptor_chain_ready(VirtioSplitRing);
                }
                ensures {
                    virtqueue_input_buffer_added(self);
                    virtqueue_descriptor_chain_allocated(self);
                    virtqueue_descriptor_chain_direct(self);
                    virtqueue_in_descriptor_added(self);
                    virtqueue_chain_head_published(self);
                    virtqueue_chain_free_descriptor_count_matched(self);
                    virtqueue_chain_allocation_atomic(self);
                    virtqueue_free_descriptors_consumed(self);
                    virtqueue_avail_index_advanced(self);
                    virtqueue_descriptor_exhaustion_rejected(self);
                }
            }

            Action::Kick {
                state_effect: StateEffect::None;
                depends_on {
                    virtqueue_chain_head_published(self);
                }
                ensures {
                    virtqueue_kick_recorded(self);
                    virtqueue_mmio_notify_written(self);
                }
            }

            Action::GetBuf {
                state_effect: StateEffect::None;
                depends_on {
                    virtqueue_fixture_used_entry_ready(self) || virtqueue_real_used_completion_observed(self);
                }
                ensures {
                    virtqueue_get_buf_returns_len(self);
                    virtqueue_get_buf_empty_rejected(self);
                    virtqueue_descriptor_chain_released(self);
                    virtqueue_buffer_ownership_released(self);
                }
            }
        }
    }
}
