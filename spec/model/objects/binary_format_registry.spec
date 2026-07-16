/* Fixed boot/runtime exec format registry. */

enum BinaryFormatHandlerKind {
    ElfBinaryFormat,
}

predicate binary_format_registry_fixed_capacity<T>(registry: T) -> bool;
predicate binary_format_registry_ready<T>(registry: T) -> bool;
predicate binary_format_registry_handler_registered<T, H>(registry: T, handler: H) -> bool;
predicate binary_format_registry_only_elf_handler<T>(registry: T) -> bool;
predicate binary_format_registry_dispatches_kernel_bytes<T>(registry: T) -> bool;
predicate binary_format_registry_never_copies_user_pointers<T>(registry: T) -> bool;
predicate binary_format_registry_unknown_format_returns_enoexec<T>(registry: T) -> bool;
predicate binary_format_registry_script_deferred<T>(registry: T) -> bool;
predicate binary_format_registry_misc_deferred<T>(registry: T) -> bool;
predicate binary_format_registry_dynamic_registration_deferred<T>(registry: T) -> bool;
predicate binary_format_registry_module_retry_trimmed<T>(registry: T) -> bool;

object BinaryFormatRegistry: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    InitcallTable.state == State::Ready;
                }

                ensures {
                    binary_format_registry_fixed_capacity(self);
                    binary_format_registry_ready(self);
                    binary_format_registry_handler_registered(
                        self,
                        BinaryFormatHandlerKind::ElfBinaryFormat
                    );
                    binary_format_registry_only_elf_handler(self);
                    binary_format_registry_dispatches_kernel_bytes(self);
                    binary_format_registry_never_copies_user_pointers(self);
                    binary_format_registry_unknown_format_returns_enoexec(self);
                    binary_format_registry_script_deferred(self);
                    binary_format_registry_misc_deferred(self);
                    binary_format_registry_dynamic_registration_deferred(self);
                    binary_format_registry_module_retry_trimmed(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            binary_format_registry_fixed_capacity(self);
            binary_format_registry_ready(self);
            binary_format_registry_handler_registered(
                self,
                BinaryFormatHandlerKind::ElfBinaryFormat
            );
            binary_format_registry_only_elf_handler(self);
            binary_format_registry_dispatches_kernel_bytes(self);
            binary_format_registry_never_copies_user_pointers(self);
            binary_format_registry_unknown_format_returns_enoexec(self);
            binary_format_registry_script_deferred(self);
            binary_format_registry_misc_deferred(self);
            binary_format_registry_dynamic_registration_deferred(self);
            binary_format_registry_module_retry_trimmed(self);
        }
    }
}
