/* Linux-like sparse, downward-growing user stack. */

predicate user_stack_allocated<T>(stack: T) -> bool;
predicate user_stack_config_linux_defaults_bound<T, C>(stack: T, config: C) -> bool;
predicate user_stack_sparse_backing_unique_owner<T>(stack: T) -> bool;
predicate user_stack_sparse_pages_sorted<T>(stack: T) -> bool;
predicate user_stack_initial_vma_expanded<T>(stack: T) -> bool;
predicate user_stack_initial_pages_demand_allocated<T>(stack: T) -> bool;
predicate user_stack_rw_nx<T>(stack: T) -> bool;
predicate user_stack_mapped_into_address_space<T, A>(stack: T, space: A) -> bool;
predicate user_stack_initial_sp_bound<T>(stack: T) -> bool;
predicate user_stack_initial_argc_argv_envp_auxv_bound<T>(stack: T) -> bool;
predicate user_stack_at_random_bound<T>(stack: T) -> bool;
predicate user_stack_at_random_hwrng_exact<T>(stack: T) -> bool;
predicate user_stack_rlimit_bound<T>(stack: T) -> bool;
predicate user_stack_guard_gap_bound<T>(stack: T) -> bool;
predicate user_stack_fault_range_resolver_bound<T, A>(stack: T, space: A) -> bool;
predicate user_stack_fault_rollback_atomic<T>(stack: T) -> bool;
predicate user_stack_targeted_tlb_flush<T>(stack: T) -> bool;
predicate user_stack_usercopy_growth_bound<T>(stack: T) -> bool;
predicate user_stack_move_swap_ownership<T>(stack: T) -> bool;
predicate user_stack_growth_checkpoint_bound<T>(stack: T) -> bool;

object UserStack: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    UserAddressSpace.state == State::Prepared;
                    PageAllocator.state == State::Ready;
                    HwRngCore.state == State::Ready;
                }

                ensures {
                    user_stack_allocated(self);
                    user_stack_config_linux_defaults_bound(self, Config);
                    user_stack_sparse_backing_unique_owner(self);
                    user_stack_sparse_pages_sorted(self);
                    user_stack_initial_vma_expanded(self);
                    user_stack_initial_pages_demand_allocated(self);
                    user_stack_rw_nx(self);
                    user_stack_mapped_into_address_space(self, UserAddressSpace);
                    user_stack_initial_sp_bound(self);
                    user_stack_initial_argc_argv_envp_auxv_bound(self);
                    user_stack_at_random_bound(self);
                    user_stack_at_random_hwrng_exact(self);
                    user_stack_rlimit_bound(self);
                    user_stack_guard_gap_bound(self);
                    user_stack_fault_range_resolver_bound(self, UserAddressSpace);
                    user_stack_fault_rollback_atomic(self);
                    user_stack_targeted_tlb_flush(self);
                    user_stack_usercopy_growth_bound(self);
                    user_stack_move_swap_ownership(self);
                    user_stack_growth_checkpoint_bound(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_stack_allocated(self);
            user_stack_config_linux_defaults_bound(self, Config);
            user_stack_sparse_backing_unique_owner(self);
            user_stack_sparse_pages_sorted(self);
            user_stack_rw_nx(self);
            user_stack_rlimit_bound(self);
            user_stack_guard_gap_bound(self);
            user_stack_fault_rollback_atomic(self);
            user_stack_move_swap_ownership(self);
        }

        actions {
            on Action::Grow {
                ensures {
                    user_stack_sparse_pages_sorted(self);
                    user_stack_rlimit_bound(self);
                    user_stack_guard_gap_bound(self);
                    user_stack_targeted_tlb_flush(self);
                    user_stack_growth_checkpoint_bound(self);
                }
            }

            on Action::GrowRejected {
                ensures {
                    user_stack_fault_rollback_atomic(self);
                    user_stack_growth_checkpoint_bound(self);
                }
            }
        }
    }
}
