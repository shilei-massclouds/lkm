/*
 * Payload Phase Specification
 *
 * PayloadPhase prepares one build-time selected payload and returns to the
 * Kernel.Enable continuation before the selected no-return entry is invoked.
 * PayloadPhase.Online, Kernel.Online and the no-return entry are distinct.
 */

/*
 * SelectedPayloadHandoff is the variant-neutral handoff object. Its actions
 * dispatch on Config.selected_payload_kind. SetupSelectedVariant drives
 * UserBootPayload.Setup only for SelectedPayloadKind::UserBoot; Hello and
 * Smoke bind their kernel-mode entries without advancing UserBootPayload.
 * PrepareSelectedVariant likewise prepares only the selected variant.
 */
object SelectedPayloadHandoff: ResourceObject {
    initial_state: State::Base;
    parent: PayloadPhase;

    state State::Base {
        actions {
            on Action::SetupSelectedVariant {
                depends_on {
                    Config.state == State::Online;
                    ExecSyncBoundaries.state == State::Ready;
                    UserCloneDeferredBoundaries.state == State::Ready;
                }

                ensures {
                    selected_payload_variant_setup_ready(
                        self,
                        Config,
                        UserBootPayload
                    );
                }
            }
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                }

                drives {
                    SelectedPayloadHandoff.Action::SetupSelectedVariant;
                }

                ensures {
                    selected_payload_handoff_kind_bound(self, Config);
                    selected_payload_variant_setup_ready(self, Config, UserBootPayload);
                    selected_payload_ready();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            selected_payload_handoff_kind_bound(self, Config);
            selected_payload_variant_setup_ready(self, Config, UserBootPayload);
        }

        actions {
            on Action::PrepareSelectedVariant {
                ensures {
                    selected_payload_variant_prepare_ready(
                        self,
                        Config,
                        UserBootPayload
                    );
                    selected_payload_no_return_entry_bound(self);
                }
            }
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    SelectedPayloadHandoff.Action::PrepareSelectedVariant;
                }

                ensures {
                    selected_payload_variant_prepare_ready(self, Config, UserBootPayload);
                    selected_payload_no_return_entry_bound(self);
                    selected_payload_no_return_handoff();
                }
            }
        }
    }

    state State::Online {
        invariant {
            selected_payload_handoff_kind_bound(self, Config);
            selected_payload_variant_setup_ready(self, Config, UserBootPayload);
            selected_payload_variant_prepare_ready(self, Config, UserBootPayload);
            selected_payload_no_return_entry_bound(self);
        }
    }
}

/* KernelInitTask owns this phase from SmpRuntimePhase.Online to Kernel. */
object PayloadPhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SmpRuntimePhase.state == State::Online;
                    FinalizePhase.state == State::Online;
                    FinalizeBoundary.state == State::Ready;
                    SystemState.state == State::Online;
                    KernelInitTask.state == State::Online;
                    system_state_running(SystemState);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    payload_phase_next_boundary();
                    BinaryFormatRegistry.state == State::Ready;
                }

                drives {
                    ExecSyncBoundaries.Transition::Setup;
                    ExecTransaction.Transition::Setup;
                    UserCloneDeferredBoundaries.Transition::Setup;
                }

                ensures {
                    ExecSyncBoundaries.state == State::Ready;
                    ExecTransaction.state == State::Ready;
                    UserCloneDeferredBoundaries.state == State::Ready;
                    payload_execution_owned_by_kernel_init_task(
                        PayloadPhase,
                        KernelInitTask
                    );
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ExecSyncBoundaries.state == State::Ready;
            ExecTransaction.state == State::Ready;
            UserCloneDeferredBoundaries.state == State::Ready;
            KernelInitTask.state == State::Online;
            payload_execution_owned_by_kernel_init_task(PayloadPhase, KernelInitTask);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    ExecSyncBoundaries.state == State::Ready;
                    ExecTransaction.state == State::Ready;
                    UserCloneDeferredBoundaries.state == State::Ready;
                }

                drives {
                    SelectedPayloadHandoff.Transition::Setup;
                }

                ensures {
                    SelectedPayloadHandoff.state == State::Ready;
                    selected_payload_ready();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootPhase.state == State::Online;
            InterruptPhase.state == State::Online;
            BootInitFlow.state == State::Online;
            SmpRuntimePhase.state == State::Online;
            FinalizePhase.state == State::Online;
            FinalizeBoundary.state == State::Ready;
            SystemState.state == State::Online;
            KernelInitTask.state == State::Online;
            ExecSyncBoundaries.state == State::Ready;
            ExecTransaction.state == State::Ready;
            UserCloneDeferredBoundaries.state == State::Ready;
            SelectedPayloadHandoff.state == State::Ready;
            selected_payload_ready();
            payload_execution_owned_by_kernel_init_task(PayloadPhase, KernelInitTask);
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    SelectedPayloadHandoff.Transition::Enable;
                }

                ensures {
                    SelectedPayloadHandoff.state == State::Online;
                    selected_payload_no_return_entry_bound(SelectedPayloadHandoff);
                    selected_payload_no_return_handoff();
                }
            }
        }
    }

    state State::Online {
        invariant {
            BootPhase.state == State::Online;
            InterruptPhase.state == State::Online;
            BootInitFlow.state == State::Online;
            SmpRuntimePhase.state == State::Online;
            KernelInitTask.state == State::Online;
            ExecSyncBoundaries.state == State::Ready;
            ExecTransaction.state == State::Ready;
            UserCloneDeferredBoundaries.state == State::Ready;
            SelectedPayloadHandoff.state == State::Online;
            selected_payload_no_return_entry_bound(SelectedPayloadHandoff);
            payload_execution_owned_by_kernel_init_task(PayloadPhase, KernelInitTask);
        }
    }
}
