/*
 * Virtio split virtqueue model, first slice.
 *
 * This models the reusable virtio_ring.c split-ring boundary needed before
 * virtio-rng fake integration. The first slice is deliberately small:
 * one queue, direct descriptors, add_inbuf, fake used completion, and get_buf.
 * Packed rings, indirect descriptors, event idx, real MMIO notify/IRQ, DMA API
 * and cache maintenance stay deferred.
 */

predicate virtio_split_ring_allocated<T>(ring: T) -> bool;
predicate virtio_split_ring_queue_size_bound<T>(ring: T) -> bool;
predicate virtio_split_ring_backing_sized_by_queue<T>(ring: T) -> bool;
predicate virtio_split_ring_descriptor_table_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_avail_ring_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_used_ring_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_free_list_ready<T>(ring: T) -> bool;
predicate virtio_split_ring_single_queue<T>(ring: T) -> bool;
predicate virtio_split_ring_direct_descriptors_only<T>(ring: T) -> bool;
predicate virtio_split_ring_indirect_descriptors_deferred<T>(ring: T) -> bool;
predicate virtio_split_ring_event_idx_deferred<T>(ring: T) -> bool;
predicate virtio_split_ring_dma_cache_deferred<T>(ring: T) -> bool;

predicate virtqueue_split_ring_bound<T, R>(queue: T, ring: R) -> bool;
predicate virtqueue_input_buffer_added<T>(queue: T) -> bool;
predicate virtqueue_free_descriptor_consumed<T>(queue: T) -> bool;
predicate virtqueue_avail_index_advanced<T>(queue: T) -> bool;
predicate virtqueue_kick_recorded<T>(queue: T) -> bool;
predicate virtqueue_descriptor_exhaustion_rejected<T>(queue: T) -> bool;
predicate virtqueue_fake_completion_recorded<T>(queue: T) -> bool;
predicate virtqueue_used_index_advanced<T>(queue: T) -> bool;
predicate virtqueue_get_buf_returns_len<T>(queue: T) -> bool;
predicate virtqueue_get_buf_empty_rejected<T>(queue: T) -> bool;
predicate virtqueue_buffer_ownership_released<T>(queue: T) -> bool;

object VirtioSplitRing: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
                    virtio_split_ring_direct_descriptors_only(self);
                    virtio_split_ring_indirect_descriptors_deferred(self);
                    virtio_split_ring_event_idx_deferred(self);
                    virtio_split_ring_dma_cache_deferred(self);
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
            virtio_split_ring_direct_descriptors_only(self);
            virtio_split_ring_indirect_descriptors_deferred(self);
            virtio_split_ring_event_idx_deferred(self);
            virtio_split_ring_dma_cache_deferred(self);
        }
    }
}

object VirtQueue: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    VirtioSplitRing.state == State::Ready;
                }

                ensures {
                    virtqueue_split_ring_bound(self, VirtioSplitRing);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            virtqueue_split_ring_bound(self, VirtioSplitRing);
        }

        processes {
            Action::AddInbuf {
                state_effect: StateEffect::None;
                depends_on {
                    virtio_split_ring_free_list_ready(VirtioSplitRing);
                }
                ensures {
                    virtqueue_input_buffer_added(self);
                    virtqueue_free_descriptor_consumed(self);
                    virtqueue_avail_index_advanced(self);
                    virtqueue_descriptor_exhaustion_rejected(self);
                }
            }

            Action::Kick {
                state_effect: StateEffect::None;
                depends_on {
                    virtqueue_input_buffer_added(self);
                }
                ensures {
                    virtqueue_kick_recorded(self);
                }
            }

            Action::FakeCompleteUsed {
                state_effect: StateEffect::None;
                depends_on {
                    virtqueue_input_buffer_added(self);
                }
                ensures {
                    virtqueue_fake_completion_recorded(self);
                    virtqueue_used_index_advanced(self);
                }
            }

            Action::GetBuf {
                state_effect: StateEffect::None;
                depends_on {
                    virtqueue_fake_completion_recorded(self);
                }
                ensures {
                    virtqueue_get_buf_returns_len(self);
                    virtqueue_get_buf_empty_rejected(self);
                    virtqueue_buffer_ownership_released(self);
                }
            }
        }
    }
}
