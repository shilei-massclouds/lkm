/* PhysicalDirect model specification. */

/* PhysicalDirect 是 satp=0 的共享 translation controller。 */
object PhysicalDirect: PrepareObject {
    initial_state: State::Ready;
    parent: Vm;

    state State::Ready {
        actions {
            on Action::ActivateOnCpu(cpu_ref: CpuRef) {
                depends_on {
                    cpu_ref_dereference_requires_published_element(cpu_ref);
                    cpu_active_translation_controller_absent_for_ref(cpu_ref);
                    translation_initial_activation_entry_satp_for_ref_is(cpu_ref, 0);
                }

                ensures {
                    translation_live_satp_for_ref_is(cpu_ref, 0);
                    cpu_active_translation_controller_for_ref_is(
                        cpu_ref,
                        TranslationControllerKind::PhysicalDirect
                    );
                    translation_activation_kind_is(
                        cpu_ref,
                        TranslationActivationKind::InitialActivation
                    );
                    translation_initial_activation_recorded(
                        cpu_ref,
                        TranslationControllerKind::PhysicalDirect,
                        0
                    );
                    translation_activation_fence_complete(cpu_ref);
                    translation_activation_committed_atomically(cpu_ref);
                    cpu_translation_controller_matches_live_satp_for_ref(cpu_ref);
                }
            }
        }
    }
}
