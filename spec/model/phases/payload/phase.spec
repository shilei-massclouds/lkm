/*
 * Payload leaves directly owned by KernelInitFlow.
 *
 * PayloadPreparePhase closes the former PayloadPhase.Ready boundary.
 * PayloadHandoffPreparePhase is reversible precommit only.  The actual
 * replacement/no-return decision is KernelInitFlow.CommitPayloadHandoff.
 */

object SelectedPayloadHandoff: ResourceObject {
    initial_state: State::Base;
    parent: KernelInitFlow;

    state State::Base {
        actions {
            on Action::SetupSelectedVariant {
                depends_on {
                    Config.state == State::Online;
                    ExecSyncBoundaries.state == State::Ready;
                    UserCloneDeferredBoundaries.state == State::Ready;
                }

                drives {
                    UserBootPayload.Transition::Setup;
                }

                ensures {
                    selected_payload_variant_setup_ready(self, Config, UserBootPayload);
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
                depends_on {
                    KernelInitFlow.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                }

                drives {
                    UserBootPayload.Action::PrepareHandoff;
                }

                ensures {
                    selected_payload_variant_prepare_ready(self, Config, UserBootPayload);
                    selected_payload_no_return_entry_bound(self);
                    selected_payload_replacement_precheck_complete(self, Config);
                    selected_payload_user_flow_preset_setup_ready(self, Config, KernelInitTask);
                    KernelInitUserAppRuntime.state == State::Online;
                    kernel_init_flow_survives_payload_precommit(KernelInitFlow);
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
                    selected_payload_replacement_precheck_complete(self, Config);
                    selected_payload_user_flow_preset_setup_ready(self, Config, KernelInitTask);
                    kernel_init_flow_survives_payload_precommit(KernelInitFlow);
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
            selected_payload_replacement_precheck_complete(self, Config);
            kernel_init_flow_survives_payload_precommit(KernelInitFlow);
        }

        actions {
            on Action::CommitSelectedVariant {
                depends_on {
                    KernelInitFlow.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    selected_payload_replacement_precheck_complete(self, Config);
                }

                drives {
                    UserBootPayload.Transition::Enable;
                }

                ensures {
                    selected_payload_user_boot_replacement_ordered(
                        self,
                        KernelInitFlow,
                        KernelInitTask
                    );
                    selected_payload_kernel_mode_keeps_kernel_init_flow(
                        self,
                        KernelInitFlow
                    );
                    selected_payload_no_return_handoff();
                    kernel_init_flow_payload_handoff_committed(KernelInitFlow);
                }
            }
        }
    }
}

object PayloadPreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: KernelInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    FinalizePhase.state == State::Online;
                    FinalizeBoundary.state == State::Ready;
                    SystemState.state == State::Online;
                    system_state_running(SystemState);
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
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
                    payload_execution_owned_by_kernel_init_task(self, KernelInitTask);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::OnCpu;
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
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    KernelInitTask.state == State::OnCpu;
                    SelectedPayloadHandoff.state == State::Ready;
                }
                ensures {
                    SelectedPayloadHandoff.state == State::Ready;
                    selected_payload_ready();
                    payload_execution_owned_by_kernel_init_task(self, KernelInitTask);
                }
            }
        }
    }

    state State::Online {
        invariant {
            FinalizePhase.state == State::Online;
            ExecSyncBoundaries.state == State::Ready;
            ExecTransaction.state == State::Ready;
            UserCloneDeferredBoundaries.state == State::Ready;
            SelectedPayloadHandoff.state == State::Ready;
            KernelInitTask.state == State::OnCpu;
            task_execution_authority_is(
                KernelInitTask,
                TaskExecutionAuthority::Live
            );
            payload_execution_owned_by_kernel_init_task(self, KernelInitTask);
        }
    }
}

object PayloadHandoffPreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: KernelInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PayloadPreparePhase.state == State::Online;
                    KernelInitFlow.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                }

                drives {
                    SelectedPayloadHandoff.Transition::Enable;
                }

                ensures {
                    SelectedPayloadHandoff.state == State::Online;
                    selected_payload_replacement_precheck_complete(
                        SelectedPayloadHandoff,
                        Config
                    );
                    kernel_init_flow_survives_payload_precommit(KernelInitFlow);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::OnCpu;
                }
                ensures {
                    KernelInitFlow.state == State::Online;
                    kernel_init_flow_survives_payload_precommit(KernelInitFlow);
                }
                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    KernelInitTask.state == State::OnCpu;
                }
                ensures {
                    KernelInitFlow.state == State::Online;
                    SelectedPayloadHandoff.state == State::Online;
                    kernel_init_flow_survives_payload_precommit(KernelInitFlow);
                }
            }
        }
    }

    state State::Online {
        invariant {
            PayloadPreparePhase.state == State::Online;
            SelectedPayloadHandoff.state == State::Online;
            kernel_init_flow_survives_payload_precommit(KernelInitFlow);
        }
    }
}

predicate selected_payload_replacement_precheck_complete<H, C>(handoff: H, config: C) -> bool;
predicate selected_payload_user_flow_preset_setup_ready<H, C, T>(handoff: H, config: C, task: T) -> bool;
predicate kernel_init_flow_survives_payload_precommit<F>(flow: F) -> bool;
predicate selected_payload_user_boot_replacement_ordered<H, F, T>(handoff: H, flow: F, task: T) -> bool;
predicate selected_payload_kernel_mode_keeps_kernel_init_flow<H, F>(handoff: H, flow: F) -> bool;
predicate kernel_init_flow_payload_handoff_committed<F>(flow: F) -> bool;
