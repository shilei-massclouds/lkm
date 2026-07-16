/* Reusable Context-owned transaction shared by kernel_execve and execve(221). */

enum ExecOwnerKind {
    Boot,
    Runtime,
}

enum ExecFailureKind {
    Fault,
    ArgumentsTooBig,
    NotFound,
    NoExecutableFormat,
    NoMemory,
    EntropyUnavailable,
}

predicate exec_transaction_context_owned_single_slot<T>(transaction: T) -> bool;
predicate exec_transaction_reusable<T>(transaction: T) -> bool;
predicate exec_transaction_inactive<T>(transaction: T) -> bool;
predicate exec_transaction_active<T>(transaction: T) -> bool;
predicate exec_transaction_owner_bound<T, O>(transaction: T, owner: O) -> bool;
predicate exec_transaction_absolute_filename_bound<T>(transaction: T) -> bool;
predicate exec_transaction_arguments_normalized<T>(transaction: T) -> bool;
predicate exec_transaction_argument_limits_config_bound<T, C>(transaction: T, config: C) -> bool;
predicate exec_transaction_linux_default_argument_limits<T, C>(transaction: T, config: C) -> bool;
predicate exec_transaction_argument_pointer_bytes_accounted<T>(transaction: T) -> bool;
predicate exec_transaction_argument_allocation_fallible<T>(transaction: T) -> bool;
predicate exec_transaction_boot_default_args_bound<T>(transaction: T) -> bool;
predicate exec_transaction_runtime_usercopy_precedes_begin<T>(transaction: T) -> bool;
predicate exec_transaction_registry_dispatch_bound<T, R>(transaction: T, registry: R) -> bool;
predicate exec_transaction_staging_image_ready<T>(transaction: T) -> bool;
predicate exec_transaction_all_fallible_checks_precommit<T>(transaction: T) -> bool;
predicate exec_transaction_point_of_no_return<T>(transaction: T) -> bool;
predicate exec_transaction_current_staging_swapped<T>(transaction: T) -> bool;
predicate exec_transaction_cloexec_prechecked_then_applied<T>(transaction: T) -> bool;
predicate exec_transaction_owner_handoff_complete<T>(transaction: T) -> bool;
predicate exec_transaction_retired_mm_released_or_parent_owned<T>(transaction: T) -> bool;
predicate exec_transaction_abort_releases_staging<T>(transaction: T) -> bool;
predicate exec_transaction_abort_preserves_current_state<T>(transaction: T) -> bool;
predicate exec_transaction_failure_errno_bound<T, E>(transaction: T, error: E) -> bool;
predicate exec_transaction_no_errno_after_point_of_no_return<T>(transaction: T) -> bool;
predicate exec_transaction_checkpoint_namespace_owner_scoped<T>(transaction: T) -> bool;
predicate exec_transaction_hwrng_random_precommit<T>(transaction: T) -> bool;
predicate exec_transaction_entropy_failure_preserves_current<T>(transaction: T) -> bool;
predicate exec_transaction_boot_entropy_failure_terminal<T>(transaction: T) -> bool;

object ExecTransaction: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    BinaryFormatRegistry.state == State::Ready;
                    ExecSyncBoundaries.state == State::Ready;
                }

                ensures {
                    exec_transaction_context_owned_single_slot(self);
                    exec_transaction_reusable(self);
                    exec_transaction_inactive(self);
                    exec_transaction_argument_limits_config_bound(self, Config);
                    exec_transaction_linux_default_argument_limits(self, Config);
                    exec_transaction_argument_pointer_bytes_accounted(self);
                    exec_transaction_argument_allocation_fallible(self);
                    exec_transaction_registry_dispatch_bound(self, BinaryFormatRegistry);
                    exec_transaction_all_fallible_checks_precommit(self);
                    exec_transaction_no_errno_after_point_of_no_return(self);
                    exec_transaction_checkpoint_namespace_owner_scoped(self);
                    exec_transaction_hwrng_random_precommit(self);
                    exec_transaction_entropy_failure_preserves_current(self);
                    exec_transaction_boot_entropy_failure_terminal(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            exec_transaction_context_owned_single_slot(self);
            exec_transaction_reusable(self);
            exec_transaction_argument_limits_config_bound(self, Config);
            exec_transaction_linux_default_argument_limits(self, Config);
            exec_transaction_argument_pointer_bytes_accounted(self);
            exec_transaction_argument_allocation_fallible(self);
            exec_transaction_registry_dispatch_bound(self, BinaryFormatRegistry);
            exec_transaction_all_fallible_checks_precommit(self);
            exec_transaction_no_errno_after_point_of_no_return(self);
            exec_transaction_checkpoint_namespace_owner_scoped(self);
            exec_transaction_hwrng_random_precommit(self);
            exec_transaction_entropy_failure_preserves_current(self);
            exec_transaction_boot_entropy_failure_terminal(self);
        }

        actions {
            on Action::BeginBoot {
                ensures {
                    exec_transaction_active(self);
                    exec_transaction_owner_bound(self, ExecOwnerKind::Boot);
                    exec_transaction_absolute_filename_bound(self);
                    exec_transaction_arguments_normalized(self);
                    exec_transaction_boot_default_args_bound(self);
                }
            }

            on Action::BeginRuntime {
                ensures {
                    exec_transaction_active(self);
                    exec_transaction_owner_bound(self, ExecOwnerKind::Runtime);
                    exec_transaction_absolute_filename_bound(self);
                    exec_transaction_arguments_normalized(self);
                    exec_transaction_runtime_usercopy_precedes_begin(self);
                }
            }

            on Action::Prepare {
                ensures {
                    exec_transaction_staging_image_ready(self);
                    exec_transaction_all_fallible_checks_precommit(self);
                }
            }

            on Action::Commit {
                ensures {
                    exec_transaction_point_of_no_return(self);
                    exec_transaction_current_staging_swapped(self);
                    exec_transaction_cloexec_prechecked_then_applied(self);
                    exec_transaction_owner_handoff_complete(self);
                    exec_transaction_retired_mm_released_or_parent_owned(self);
                    exec_transaction_inactive(self);
                }
            }

            on Action::Abort {
                ensures {
                    exec_transaction_abort_releases_staging(self);
                    exec_transaction_abort_preserves_current_state(self);
                    exec_transaction_inactive(self);
                }
            }
        }
    }
}
