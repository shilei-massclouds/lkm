/* Lifetime TaskFlow, contextual dispatch, and Flow-owned user runtime. */

type TaskFlowRef { }

type TaskFlow: PhaseObject {
    initial_context: Action::RunUserContinuation;
    initial_state: State::Base;
    parent: Task;
    associations {
        mutable cpu_ref: CpuRef;
        flow_ref: TaskFlowRef;
    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    self.parent.state == State::Ready;
                    task_fixed_flow_is(self.parent, self);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on { self.parent.state == State::Ready; }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    self.parent.state == State::Ready;
                    task_initial_context_complete(self.parent, self);
                }
                ensures { task_flow_published_with_task(self, self.parent); }
            }
        }
    }

    state State::Online {
        invariant {
            task_fixed_flow_is(self.parent, self);
            task_flow_owner_is(self, self.parent);
            task_flow_parent_is(self, self.parent);
            task_flow_owner_exclusive(self);
            task_flow_ref_generation_valid(self);
        }
        transitions {
            on Transition::Disable -> State::Offline {
                depends_on {
                    task_flow_terminal_runtime_quiesced(self);
                    task_flow_no_pending_yield(self);
                }
            }
        }
    }

    state State::Offline {
        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on {
                    task_flow_terminal_runtime_quiesced(self);
                    task_flow_no_pending_yield(self);
                }
            }
        }
    }
    state State::Destroyed { }

    processes {
        Action::BindOwner(owner_task: Task, flow_ref: TaskFlowRef) {
            state_effect: StateEffect::None;
            structural_binding: true;
            ensures {
                task_fixed_flow_is(owner_task, self);
                task_flow_owner_is(self, owner_task);
                task_flow_parent_is(self, owner_task);
                task_flow_owner_exclusive(self);
                task_flow_ref_generation_valid(self);
                task_flow_ref_targets(flow_ref, self);
            }
            updates {
                owner_task.flow = self;
                self.flow_ref = flow_ref;
            }
        }

        Action::RunUserContinuation {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                self.parent.state == State::OnCpu;
            }
            ensures { task_flow_user_continuation_coordinate_resumed(self); }
        }

        Action::BindTask(task_ref: TaskRef, dispatch_flow: TaskFlow) {
            state_effect: StateEffect::None;
            depends_on {
                current_task_bind_scheduler_commit_boundary_valid(self, task_ref, dispatch_flow);
                task_ref_ready(task_ref);
                task_ref_targets_online_task(task_ref);
                scheduler_preflight_dispatch_flow_is(task_ref, dispatch_flow);
            }
            ensures {
                current_task_binding_committed(CurrentCPU, task_ref, dispatch_flow);
                current_task_binding_address_refreshed(CurrentCPU, task_ref);
                current_task_binding_is_cpu_local(CurrentCPU);
                current_task_binding_replaced_at_scheduler_commit(CurrentCPU, task_ref);
                current_stack_binding_committed(CurrentCPU, task_ref, task_ref.stack);
                current_stack_binding_matches_task(CurrentCPU, task_ref, task_ref.stack);
            }
        }

        Action::BindTaskStack(task: Task, stack: Stack) {
            state_effect: StateEffect::None;
            depends_on {
                task.state == State::OnCpu;
                task.flow == self;
                current_task_stack_pair_unbound(CurrentCPU);
            }
            ensures {
                boot_task_bind_task_stack_boundary_valid(self, task, stack);
                task_flow_owner_is(self, task);
                current_task_binding_committed(CurrentCPU, task, self);
                current_stack_binding_committed(CurrentCPU, task, task.stack);
                current_task_stack_binding_pair_consistent(CurrentCPU, task, task.stack);
                boot_task_bind_task_stack_atomic(CurrentCPU, task, task.stack);
            }
        }

        Action::RefreshTaskStack(task: Task, stack: Stack) {
            state_effect: StateEffect::None;
            depends_on {
                task.state == State::OnCpu;
                task.flow == self;
                current_task_binding_committed(CurrentCPU, task, self);
                current_stack_binding_committed(CurrentCPU, task, stack);
            }
            ensures {
                boot_task_refresh_task_stack_boundary_valid(self, task, stack);
                current_task_binding_address_refreshed(CurrentCPU, task);
                current_stack_binding_address_refreshed(CurrentCPU, stack);
                current_task_stack_binding_pair_consistent(CurrentCPU, task, stack);
                boot_task_stack_current_binding_refreshed_for_active_controller(CurrentCPU, task, stack);
            }
        }

        Action::AssignCpuRef(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                cpu_ref_ready(cpu_ref);
            }
            ensures {
                task_flow_cpu_ref_is(self, cpu_ref);
                task_flow_cpu_ref_write_boundary_valid(self, cpu_ref);
                task_flow_cpu_ref_write_at_entry_or_scheduler_commit(self);
            }
            updates { self.cpu_ref = cpu_ref; }
        }

        Action::Enter {
            state_effect: StateEffect::None;
            contextual_entry: true;
            depends_on {
                self.state == State::Online;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
                task_fixed_flow_is(self.parent, self);
                task_flow_effective_execution_guard(self);
                task_context_flow_ref_is_fixed(self.parent, self);
            }
            ensures {
                task_flow_ref_generation_valid(self);
                task_flow_context_epoch_matches_dispatch(self);
                task_flow_contextual_enter_received(self);
                task_flow_dispatch_proof_consumed_exactly_once(self);
                task_flow_current_coordinate_consumed(self);
                task_flow_machine_entry_comes_from_context(self);
                task_flow_pending_yield_resumed_exactly_once_or_absent(self);
                task_flow_optional_root_preflight_valid(self.parent, self);
                task_flow_active_trap_leaf_resumed_exactly_once_or_absent(self);
                task_flow_enter_does_not_select_machine_coordinate(self);
            }
        }
    }
}

predicate task_flow_owner_is<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate task_flow_parent_is<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate task_flow_owner_exclusive<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_ref_generation_valid<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_ref_targets<R: TaskFlowRef, F: TaskFlow>(flow_ref: R, flow: F) -> bool;
predicate task_flow_cpu_ref_is<F: TaskFlow, R: CpuRef>(flow: F, cpu_ref: R) -> bool;
predicate task_flow_cpu_ref_write_boundary_valid<F: TaskFlow, R: CpuRef>(flow: F, cpu_ref: R) -> bool;
predicate task_flow_cpu_ref_write_at_entry_or_scheduler_commit<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_cpu_ref_read_only_while_executing<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_effective_execution_guard<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_context_epoch_matches_dispatch<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_contextual_enter_received<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_dispatch_proof_consumed_exactly_once<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_current_coordinate_consumed<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_machine_entry_comes_from_context<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_pending_yield_resumed_exactly_once_or_absent<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_optional_root_preflight_valid<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_flow_active_trap_leaf_resumed_exactly_once_or_absent<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_enter_does_not_select_machine_coordinate<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_published_with_task<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate task_flow_terminal_runtime_quiesced<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_no_pending_yield<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_user_continuation_coordinate_resumed<F: TaskFlow>(flow: F) -> bool;

predicate current_task_bind_scheduler_commit_boundary_valid<F: TaskFlow, R: TaskRef, D: TaskFlow>(flow: F, task_ref: R, dispatch_flow: D) -> bool;
predicate scheduler_preflight_dispatch_flow_is<R: TaskRef, F: TaskFlow>(task_ref: R, flow: F) -> bool;
predicate current_task_binding_committed<C, T, F: TaskFlow>(cpu: C, task: T, flow: F) -> bool;
predicate current_task_binding_address_refreshed<C, T>(cpu: C, task: T) -> bool;
predicate current_task_binding_is_cpu_local<C>(cpu: C) -> bool;
predicate current_task_binding_replaced_at_scheduler_commit<C, T>(cpu: C, task: T) -> bool;
predicate current_stack_binding_committed<C, T, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_stack_binding_address_refreshed<C, S: Stack>(cpu: C, stack: S) -> bool;
predicate current_task_stack_binding_pair_consistent<C, T, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_task_stack_pair_unbound<C>(cpu: C) -> bool;
predicate boot_task_bind_task_stack_boundary_valid<F: TaskFlow, T: Task, S: Stack>(flow: F, task: T, stack: S) -> bool;
predicate boot_task_refresh_task_stack_boundary_valid<F: TaskFlow, T: Task, S: Stack>(flow: F, task: T, stack: S) -> bool;
predicate boot_task_bind_task_stack_atomic<C, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate boot_task_stack_current_binding_refreshed_for_active_controller<C, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;

/* Stable, private application runtime owned by a user-capable lifetime Flow. */
type ApplicationInstance: ResourceObject { }

type UserAppRuntime: ResourceObject {
    initial_state: State::Base;
    parent: TaskFlow;
    associations { mutable application: ApplicationInstance; }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                emits { Transition::Setup; }
            }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                emits { Transition::Enable; }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    user_app_runtime_owned_by_flow(self, self.parent);
                    user_app_runtime_unique_for_flow(self, self.parent);
                    user_app_runtime_not_shared(self);
                    user_app_runtime_application_continuation_online(self, self.application);
                    user_app_runtime_passive_lifecycle(self);
                    application_instance_owned_by_runtime(self.application, self);
                }
            }
        }
    }
    state State::Online {
        invariant {
            user_app_runtime_owned_by_flow(self, self.parent);
            user_app_runtime_unique_for_flow(self, self.parent);
            user_app_runtime_not_shared(self);
            user_app_runtime_application_continuation_online(self, self.application);
            user_app_runtime_passive_lifecycle(self);
        }
        transitions {
            on Transition::Disable -> State::Offline {
                depends_on { user_app_runtime_terminal_quiesced(self); }
            }
        }
    }
    state State::Offline {
        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on { user_app_runtime_terminal_quiesced(self); }
            }
        }
    }
    state State::Destroyed { }

    processes {
        Action::Bind(flow: TaskFlow, application: ApplicationInstance) {
            state_effect: StateEffect::None;
            structural_binding: true;
            depends_on { application_instance_fresh(application); }
            ensures {
                user_app_runtime_owned_by_flow(self, flow);
                user_app_runtime_unique_for_flow(self, flow);
                user_app_runtime_not_shared(self);
                user_app_runtime_application_continuation_online(self, application);
                user_app_runtime_passive_lifecycle(self);
                application_instance_owned_by_runtime(application, self);
            }
            updates { self.application = application; }
        }

        Action::ReplaceApplication(next: ApplicationInstance) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                user_app_runtime_exec_precommit_valid(self, next);
                application_instance_fresh(next);
            }
            ensures {
                user_app_runtime_identity_preserved(self);
                user_app_runtime_application_replaced(self, next);
                user_app_runtime_exec_committed(self, next);
            }
            updates { self.application = next; }
        }
    }
}

predicate user_app_runtime_exec_precommit_valid<R: UserAppRuntime, A: ApplicationInstance>(runtime: R, app: A) -> bool;
predicate application_instance_fresh<A: ApplicationInstance>(app: A) -> bool;
predicate user_app_runtime_identity_preserved<R: UserAppRuntime>(runtime: R) -> bool;
predicate user_app_runtime_application_replaced<R: UserAppRuntime, A: ApplicationInstance>(runtime: R, app: A) -> bool;
predicate user_app_runtime_exec_committed<R: UserAppRuntime, A: ApplicationInstance>(runtime: R, app: A) -> bool;
predicate user_app_runtime_owned_by_flow<R: UserAppRuntime, F: TaskFlow>(runtime: R, flow: F) -> bool;
predicate user_app_runtime_unique_for_flow<R: UserAppRuntime, F: TaskFlow>(runtime: R, flow: F) -> bool;
predicate user_app_runtime_not_shared<R: UserAppRuntime>(runtime: R) -> bool;
predicate user_app_runtime_application_continuation_online<R: UserAppRuntime, A: ApplicationInstance>(runtime: R, app: A) -> bool;
predicate user_app_runtime_passive_lifecycle<R: UserAppRuntime>(runtime: R) -> bool;
predicate user_app_runtime_terminal_quiesced<R: UserAppRuntime>(runtime: R) -> bool;
predicate application_instance_owned_by_runtime<A: ApplicationInstance, R: UserAppRuntime>(app: A, runtime: R) -> bool;

object KernelInitUserAppRuntime: UserAppRuntime {
    parent: KernelInitFlow;
    associations { application = KernelInitApplicationInstance; }
}

object KernelInitApplicationInstance: ApplicationInstance {
    parent: KernelInitUserAppRuntime;
    initial_state: State::Base;
    state State::Base { }
}

predicate kernel_init_entry_stack_verified<T: Task>(task: T) -> bool;
predicate kernel_init_flow_first_leaf<F: TaskFlow, P>(flow: F, phase: P) -> bool;
predicate kernel_init_entry_reaches_kernel_init_flow<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate kernel_init_flow_run_kernel_init_on_verified_stack<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate kernel_init_flow_payload_handoff_committed<F: TaskFlow>(flow: F) -> bool;
predicate kthreadd_entry_reaches_schedule_loop<T: Task, S: Scheduler>(task: T, scheduler: S) -> bool;
predicate kthreadd_schedule_loop_ready<T: Task, S: Scheduler>(task: T, scheduler: S) -> bool;
predicate kthreadd_schedule_loop_active<T: Task, S: Scheduler>(task: T, scheduler: S) -> bool;
predicate ap_idle_flow_key_matches_task<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate ap_idle_flow_pointwise_phases_complete<F: TaskFlow>(flow: F) -> bool;

object KernelInitFlow: TaskFlow {
    lifecycle_override: true;
    initial_context: Action::RunKernelInit;
    initial_state: State::Base;
    parent: KernelInitTask;
    owned { runtime: UserAppRuntime; }
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on { KernelInitTask.state == State::Prepared; }
                ensures {
                    task_flow_owner_is(self, KernelInitTask);
                    task_flow_parent_is(self, KernelInitTask);
                    task_fixed_flow_is(KernelInitTask, self);
                }
            }
        }
    }
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on { KernelInitTask.state == State::Ready; }
            }
        }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    KernelInitTask.state == State::Ready;
                    task_initial_context_complete(KernelInitTask, self);
                }
                ensures { task_flow_published_with_task(self, KernelInitTask); }
            }
        }
    }
    state State::Online {
        actions {
            on Action::RunKernelInit {
                state_effect: StateEffect::None;
                depends_on {
                    KernelInitTask.state == State::OnCpu;
                    kernel_init_entry_stack_verified(KernelInitTask);
                }
                drives {
                    PreSmpInitPhase.Transition::Preset;
                    SmpBringupPhase.Transition::Preset;
                    RuntimeCorePhase.Transition::Preset;
                    InitcallPhase.Transition::Preset;
                    RootfsPhase.Transition::Preset;
                    FinalizePhase.Transition::Preset;
                    PayloadPreparePhase.Transition::Preset;
                    KernelInitUserAppRuntime.Transition::Preset;
                    PayloadHandoffPreparePhase.Transition::Preset;
                }
                ensures {
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, self);
                    kernel_init_flow_run_kernel_init_on_verified_stack(self, KernelInitTask);
                }
            }

            on Action::CommitPayloadHandoff {
                state_effect: StateEffect::None;
                depends_on {
                    Kernel.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    PayloadHandoffPreparePhase.state == State::Online;
                    KernelInitUserAppRuntime.state == State::Online;
                }
                drives {
                    UserBootPayload.Transition::Enable;
                    Cpu0Scheduler.Action::SwitchTo(
                        KernelInitTaskRef,
                        BootTaskRef
                    );
                }
                ensures { kernel_init_flow_payload_handoff_committed(self); }
            }

            on Action::ObserveKthreaddDoneRelease {
                state_effect: StateEffect::None;
                depends_on {
                    KthreaddReadyGate.state == State::Online;
                    completion_complete_committed(KthreaddReadyGate);
                    completion_token_available(KthreaddReadyGate);
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                    KernelInitTask.state == State::OnCpu;
                }
                drives { KthreaddReadyGate.Transition::Wait; }
                ensures {
                    completion_waiter_enqueued(KthreaddReadyGate);
                    completion_waiter_finished(KthreaddReadyGate);
                    kernel_init_observed_kthreadd_done_release(KernelInitTask, KthreaddReadyGate);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
                }
            }
        }
    }
}

object KthreaddFlow: TaskFlow {
    lifecycle_override: true;
    initial_context: Action::RunScheduleLoop;
    initial_state: State::Base;
    parent: KthreaddTask;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on { KthreaddTask.state == State::Prepared; }
                ensures { task_fixed_flow_is(KthreaddTask, self); }
            }
        }
    }
    state State::Prepared {
        transitions { on Transition::Setup -> State::Ready { } }
    }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on { task_initial_context_complete(KthreaddTask, self); }
                ensures { task_flow_published_with_task(self, KthreaddTask); }
            }
        }
    }
    state State::Online {
        actions {
            on Action::RunScheduleLoop {
                state_effect: StateEffect::None;
                depends_on { KthreaddTask.state == State::OnCpu; }
                ensures {
                    kthreadd_entry_reaches_schedule_loop(KthreaddTask, Cpu0Scheduler);
                    kthreadd_schedule_loop_ready(KthreaddTask, Cpu0Scheduler);
                    kthreadd_schedule_loop_active(KthreaddTask, Cpu0Scheduler);
                }
            }
        }
    }
}

object ApIdleFlow: TaskFlow {
    lifecycle_override: true;
    initial_context: Action::RunIdle;
    initial_state: State::Base;
    parent: ApIdleTask;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    task_fixed_flow_is(ApIdleTask, self);
                    ap_idle_flow_key_matches_task(self, ApIdleTask);
                }
            }
        }
    }
    state State::Prepared { transitions { on Transition::Setup -> State::Ready { } } }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on { task_initial_context_complete(ApIdleTask, self); }
                ensures { task_flow_published_with_task(self, ApIdleTask); }
            }
        }
    }
    state State::Online {
        actions {
            on Action::RunIdle {
                state_effect: StateEffect::None;
                depends_on { ApIdleTask.state == State::OnCpu; }
                drives {
                    ApEntryPreludePhase.Transition::Preset;
                    ApSmpCallinPhase.Transition::Preset;
                    ApOnlineIdlePhase.Transition::Preset;
                }
                ensures { ap_idle_flow_pointwise_phases_complete(self); }
                emits { SmpBringupPhase.Action::ObserveApCompletion; }
            }
        }
    }
}

object KernelTaskFlow: TaskFlow {
    lifecycle_override: true;
    initial_context: Action::RunKernelTask;
    initial_state: State::Base;
    parent: KernelTask;
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures { task_fixed_flow_is(KernelTask, self); }
            }
        }
    }
    state State::Prepared { transitions { on Transition::Setup -> State::Ready { } } }
    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on { task_initial_context_complete(KernelTask, self); }
                ensures { task_flow_published_with_task(self, KernelTask); }
            }
        }
    }
    state State::Online {
        actions {
            on Action::RunKernelTask {
                state_effect: StateEffect::None;
                depends_on { KernelTask.state == State::OnCpu; }
            }
        }
    }
}
