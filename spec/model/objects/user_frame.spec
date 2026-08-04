/* Checked shared references for resident private user backing frames. */

type UserFrame: ResourceObject { }

type UserFrameRef {
    processes {
        Action::Acquire -> UserFrameRef {
            state_effect: StateEffect::None;
            depends_on {
                user_frame_ref_ready(self);
                user_frame_ref_targets_user_backing(self);
                user_frame_ref_metadata_in_map(self, PageMetadataMap);
                user_frame_refcount_nonzero(self, PageMetadataMap);
                user_frame_refcount_increment_fits(self, PageMetadataMap);
            }
            ensures {
                user_frame_ref_acquire_committed(self, PageMetadataMap);
                user_frame_refcount_conserved(PageMetadataMap);
            }
        }

        Action::Release {
            state_effect: StateEffect::None;
            depends_on {
                user_frame_ref_ready(self);
                user_frame_ref_targets_user_backing(self);
                user_frame_ref_metadata_in_map(self, PageMetadataMap);
                user_frame_ref_owner_live(self);
                user_frame_refcount_nonzero(self, PageMetadataMap);
            }
            ensures {
                user_frame_ref_release_committed(self, PageMetadataMap);
                user_frame_ref_zero_freed_once(self, PageAllocator, PageMetadataMap);
                user_frame_refcount_conserved(PageMetadataMap);
            }
        }
    }
}

predicate user_frame_ref_ready<R: UserFrameRef>(frame_ref: R) -> bool;
predicate user_frame_ref_targets_user_backing<R: UserFrameRef>(frame_ref: R) -> bool;
predicate user_frame_ref_excludes_page_table_and_kernel_pages<R: UserFrameRef>(frame_ref: R) -> bool;
predicate user_frame_ref_metadata_in_map<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_initial_count_one<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_owner_live<R: UserFrameRef>(frame_ref: R) -> bool;
predicate user_frame_refcount_nonzero<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_refcount_increment_fits<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_acquire_committed<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_release_committed<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_zero_freed_once<R: UserFrameRef, A, M>(frame_ref: R, allocator: A, metadata_map: M) -> bool;
predicate user_frame_refcount_conserved<M>(metadata_map: M) -> bool;
predicate user_frame_refcount_one_is_unique<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_refcount_many_is_shared<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_stale_underflow_double_release_rejected<R: UserFrameRef, M>(frame_ref: R, metadata_map: M) -> bool;
predicate user_frame_ref_transaction_rollback_atomic<M>(metadata_map: M) -> bool;
