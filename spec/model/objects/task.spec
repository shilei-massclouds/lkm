/* Fixed Task carrier and lifetime TaskFlow association. */

enum TaskRuntimeState { New, Running }
enum TaskExecutionAuthority { None, Reserved, Live }
enum TaskBreakpointState { Invalid, Prepared, Valid }
enum TaskTerminalReason { OutOfMemory, SegmentationFault }
enum UserSegvCode { Maperr, Accerr }
enum UserProcessSlotState { Empty, Reserved, Published, Zombie, Reaping }

type TaskRef {
    processes {
        Action::Bind(task: Task) {
            state_effect: StateEffect::None;
            ensures { task_ref_targets(self, task); task_ref_ready(self); }
        }
    }
}

type TaskRefSet { }
type TaskSet: ResourceObject { }
type UserProcessGeneration { }
type UserProcessLease { }
type UserProcessAggregate: ResourceObject { }
type RegisterValue { }
type ContextEpoch { }
type DispatchRecord { }
type ContextCoordinate { }

type TaskThreadContext {
    ra: RegisterValue;
    sp: RegisterValue;
    s0: RegisterValue;
    s1: RegisterValue;
    s2: RegisterValue;
    s3: RegisterValue;
    s4: RegisterValue;
    s5: RegisterValue;
    s6: RegisterValue;
    s7: RegisterValue;
    s8: RegisterValue;
    s9: RegisterValue;
    s10: RegisterValue;
    s11: RegisterValue;
    coordinate: ContextCoordinate;
    breakpoint_state: TaskBreakpointState;
    flow_ref: TaskFlowRef;
    root_trap_flow_ref: OptionalTrapFlowRef;
    context_epoch: ContextEpoch;
    dispatch_record: DispatchRecord;
    save_count: usize;
    restore_count: usize;
}

/* Stack is Task-owned value metadata, not an object. */
type Stack { }

predicate task_ref_targets<R: TaskRef, T: Task>(task_ref: R, task: T) -> bool;
predicate task_ref_ready<R: TaskRef>(task_ref: R) -> bool;
predicate task_ref_targets_online_task<R: TaskRef>(task_ref: R) -> bool;
predicate task_fresh_identity<T: Task>(task: T) -> bool;
predicate task_fixed_flow_is<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_fixed_flow_binding_complete<T: Task>(task: T) -> bool;
predicate task_fixed_flow_binding_consistent<T: Task>(task: T) -> bool;
predicate task_flow_ref_ready<T: Task>(task: T) -> bool;
predicate task_clone_spec_ready<T: Task>(task: T) -> bool;
predicate task_clone_args_ready<T: Task>(task: T) -> bool;
predicate task_pid_allocated<T: Task, P>(task: T, pid_ns: P) -> bool;
predicate task_thread_context_ready<T: Task>(task: T) -> bool;
predicate task_thread_context_owned<T: Task, C: TaskThreadContext>(task: T, context: C) -> bool;
predicate task_thread_context_core_register_set<C: TaskThreadContext>(context: C) -> bool;
predicate task_thread_context_core_saved<C: TaskThreadContext>(context: C) -> bool;
predicate task_thread_context_core_restored<C: TaskThreadContext>(context: C) -> bool;
predicate task_context_coordinate_ready<C: TaskThreadContext>(context: C) -> bool;
predicate task_context_coordinate_saved<T: Task, C: TaskThreadContext>(task: T, context: C) -> bool;
predicate task_context_flow_ref_is_fixed<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_context_epoch_advanced<T: Task>(task: T) -> bool;
predicate task_dispatch_record_committed<T: Task>(task: T) -> bool;
predicate task_sched_entity_initialized<T: Task, S: Scheduler>(task: T, scheduler: S) -> bool;
predicate task_scheduler_publication_committed<T: Task>(task: T) -> bool;
predicate task_state_new<T: Task>(task: T) -> bool;
predicate task_state_running<T: Task>(task: T) -> bool;
predicate task_not_enqueued<T: Task>(task: T) -> bool;
predicate task_not_on_cpu<T: Task>(task: T) -> bool;
predicate task_on_cpu<T: Task>(task: T) -> bool;
predicate task_on_cpu_matches_current_task<T: Task>(task: T) -> bool;
predicate task_execution_authority_is<T: Task>(task: T, authority: TaskExecutionAuthority) -> bool;
predicate task_breakpoint_state_is<T: Task>(task: T, state: TaskBreakpointState) -> bool;
predicate task_breakpoint_bound_to_flow_ref<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_breakpoint_flow_ref_generation_valid<T: Task>(task: T) -> bool;
predicate task_breakpoint_published_on_enable<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_breakpoint_published_on_suspend<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_breakpoint_consumed_on_dispatch<T: Task>(task: T) -> bool;
predicate task_dispatch_sent_only_by_scheduler<T: Task>(task: T) -> bool;
predicate task_initial_context_coordinate_bound<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_initial_context_complete<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_dispatch_enter_proof_created<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_context_optional_root_generation_checked<T: Task>(task: T) -> bool;
predicate task_context_active_trap_leaf_checked<T: Task>(task: T) -> bool;
predicate task_suspend_sent_only_by_scheduler<T: Task>(task: T) -> bool;
predicate task_online_has_recoverable_context<T: Task>(task: T) -> bool;
predicate task_online_eligibility_is_scheduler_owned<T: Task>(task: T) -> bool;
predicate task_online_does_not_imply_dispatched<T: Task>(task: T) -> bool;
predicate task_terminal_disable_keeps_breakpoint_invalid<T: Task>(task: T) -> bool;
predicate task_terminal_reason_is<T: Task>(task: T, reason: TaskTerminalReason) -> bool;
predicate task_oom_terminal_wait_signal_nine<T: Task>(task: T) -> bool;
predicate task_oom_terminal_generates_sigchld<T: Task>(task: T) -> bool;
predicate task_oom_terminal_has_no_user_signal_frame<T: Task>(task: T) -> bool;
predicate task_oom_terminal_has_no_oom_killer_selection<T: Task>(task: T) -> bool;
predicate task_oom_terminal_releases_mm_once<T: Task>(task: T) -> bool;
predicate task_segv_terminal_info_valid<T: Task>(task: T) -> bool;
predicate task_segv_terminal_fault_address_exact<T: Task>(task: T) -> bool;
predicate task_segv_terminal_wait_signal_eleven<T: Task>(task: T) -> bool;
predicate task_segv_terminal_generates_sigchld<T: Task>(task: T) -> bool;
predicate task_segv_terminal_has_no_user_signal_frame<T: Task>(task: T) -> bool;
predicate task_segv_terminal_releases_mm_once<T: Task>(task: T) -> bool;
predicate task_fixed_flow_offline<T: Task>(task: T) -> bool;
predicate task_fixed_flow_destroyed<T: Task>(task: T) -> bool;
predicate task_destroyed_only_after_flow_cleanup<T: Task>(task: T) -> bool;
predicate task_runtime_state_transition_allowed<T: Task>(task: T, state: TaskRuntimeState) -> bool;
predicate task_runtime_state_is<T: Task>(task: T, state: TaskRuntimeState) -> bool;
predicate task_flag_no_setaffinity<T: Task>(task: T) -> bool;
predicate task_cpumask_is<T: Task, C: CpuRef>(task: T, cpu_ref: C) -> bool;
predicate task_stack_guard_ready<T: Task, S: Stack>(task: T, stack: S) -> bool;
predicate task_stack_range_valid<T: Task, S: Stack>(task: T, stack: S) -> bool;
predicate task_stack_is_static_initial_property<T: Task, S: Stack>(task: T, stack: S) -> bool;
predicate task_stack_range_is<T: Task, S: Stack, A, B>(task: T, stack: S, start: A, end: B) -> bool;
predicate task_has_unique_stack_attribute<T: Task>(task: T) -> bool;
predicate task_authority_activated_by_hsm<T: Task>(task: T) -> bool;
predicate task_ap_idle_reserved_for_cpu<T: Task>(task: T) -> bool;
predicate task_creation_copy_process_committed<C, P: Task, T: Task>(core: C, parent: P, task: T) -> bool;
predicate task_creation_bound_flow<C, T: Task, F: TaskFlow>(core: C, task: T, flow: F) -> bool;
predicate kernel_task_generation_fresh<T: Task>(task: T) -> bool;
predicate kernel_task_target_cpu_explicit<T: Task, C: CpuRef>(task: T, cpu_ref: C) -> bool;
predicate kernel_task_target_cpu_immutable_after_publish<T: Task>(task: T) -> bool;
predicate kernel_task_initial_entry_and_stack_complete<T: Task>(task: T) -> bool;
predicate kernel_task_activation_uses_target_mailbox<T: Task>(task: T) -> bool;
predicate kernel_task_wake_uses_target_mailbox<T: Task>(task: T) -> bool;
predicate current_stack_binding_matches_task<C, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_task_ref_derived_from_selector<R: TaskRef, T: Task>(task_ref: R, task: T) -> bool;
predicate boot_task_preemption_is_static_initial_property<T: Task>(task: T) -> bool;

predicate user_process_registry_capacity_is_32<S: TaskSet>(registry: S) -> bool;
predicate user_process_registry_pid1_stable<S: TaskSet>(registry: S) -> bool;
predicate user_process_registry_allows_independent_aggregates<S: TaskSet>(registry: S) -> bool;
predicate user_process_registry_contains<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_stores_ref<S: TaskSet, R: TaskRef>(registry: S, task_ref: R) -> bool;
predicate user_process_registry_ref_targets_member<S: TaskSet, R: TaskRef, T: Task>(registry: S, task_ref: R, task: T) -> bool;
predicate user_process_registry_members_fresh_and_independent<S: TaskSet>(registry: S) -> bool;
predicate user_process_registry_slot_state_is<S: TaskSet>(registry: S, state: UserProcessSlotState) -> bool;
predicate user_process_registry_slot_generation_is<S: TaskSet, G: UserProcessGeneration>(registry: S, generation: G) -> bool;
predicate user_process_registry_reservation_unpublished<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_fork_resources_complete<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_inbox_reservation_complete<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_fork_failure_has_no_residue<S: TaskSet>(registry: S) -> bool;
predicate user_process_registry_generation_reuse_checked<S: TaskSet>(registry: S) -> bool;
predicate user_process_registry_aggregate_owns_process_resources<S: TaskSet, A: UserProcessAggregate>(registry: S, aggregate: A) -> bool;
predicate user_process_registry_lease_generation_valid<S: TaskSet, L: UserProcessLease>(registry: S, lease: L) -> bool;
predicate user_process_registry_lease_cpu_owner_valid<S: TaskSet, L: UserProcessLease, C: CpuRef>(registry: S, lease: L, cpu_ref: C) -> bool;
predicate user_process_registry_lease_acquire_checks_generation_and_cpu<S: TaskSet, R: TaskRef, C: CpuRef>(registry: S, task_ref: R, cpu_ref: C) -> bool;
predicate user_process_registry_reap_excludes_scheduler_and_lease_refs<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_exit_status_published_release<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_parent_wake_target_cpu_fixed<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_process_registry_parent_child_identity_bound<S: TaskSet, P: Task, C: Task>(registry: S, parent: P, child: C) -> bool;
predicate user_process_registry_wait_uses_current_parent_identity<S: TaskSet, P: Task, C: Task>(registry: S, parent: P, child: C) -> bool;
predicate user_process_registry_reap_preserves_unselected_parent_state<S: TaskSet, P: Task, C: Task>(registry: S, parent: P, child: C) -> bool;
predicate user_process_registry_exec_preserves_identity<S: TaskSet, T: Task>(registry: S, task: T) -> bool;
predicate user_task_instance_fresh<T: Task>(task: T) -> bool;
predicate user_task_pid_and_lifecycle_independent<T: Task>(task: T) -> bool;

type Task: ResourceObject {
    initial_state: State::Base;
    ext_state: TaskRuntimeState;
    execution_authority: TaskExecutionAuthority;

    attrs { stack: Stack; }
    associations { flow: TaskFlow; }
    owned { thread_context: TaskThreadContext; }

    state State::Base {
        transitions {
            on Transition::Preset(parent_task: Task, task_ref: TaskRef, flow: TaskFlow)
                -> State::Prepared {
                depends_on { parent_task.state == State::OnCpu; }
                ensures {
                    task_fresh_identity(self);
                    task_ref_targets(task_ref, self);
                    task_ref_ready(task_ref);
                    task_fixed_flow_is(self, flow);
                    task_fixed_flow_binding_complete(self);
                    task_fixed_flow_binding_consistent(self);
                    task_flow_ref_ready(self);
                    task_flow_owner_is(flow, self);
                    task_flow_parent_is(flow, self);
                    task_flow_owner_exclusive(flow);
                    task_clone_spec_ready(self);
                    task_clone_args_ready(self);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup(
                parent_task: Task,
                pid_ns: RootPidNamespace,
                scheduler: Scheduler,
                flow: TaskFlow
            ) -> State::Ready {
                depends_on {
                    parent_task.state == State::OnCpu;
                    task_fixed_flow_is(self, flow);
                    task_creation_copy_process_committed(TaskCreationCore, parent_task, self);
                    task_creation_bound_flow(TaskCreationCore, self, flow);
                }
                ensures {
                    task_pid_allocated(self, pid_ns);
                    task_thread_context_ready(self);
                    task_thread_context_owned(self, self.thread_context);
                    task_thread_context_core_register_set(self.thread_context);
                    task_context_coordinate_ready(self.thread_context);
                    task_context_flow_ref_is_fixed(self, flow);
                    task_initial_context_coordinate_bound(self, flow);
                    task_initial_context_complete(self, flow);
                    task_breakpoint_state_is(self, TaskBreakpointState::Prepared);
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_sched_entity_initialized(self, scheduler);
                    task_state_new(self);
                    task_not_enqueued(self);
                    task_has_unique_stack_attribute(self);
                    task_stack_range_valid(self, self.stack);
                }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    task_state_running(self);
                    task_scheduler_publication_committed(self);
                    self.flow.state == State::Online;
                    task_breakpoint_state_is(self, TaskBreakpointState::Prepared);
                }
                ensures {
                    task_online_has_recoverable_context(self);
                    task_online_eligibility_is_scheduler_owned(self);
                    task_online_does_not_imply_dispatched(self);
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(self, TaskBreakpointState::Valid);
                    task_breakpoint_bound_to_flow_ref(self, self.flow);
                    task_breakpoint_flow_ref_generation_valid(self);
                    task_breakpoint_published_on_enable(self, self.flow);
                }
            }
        }
    }

    state State::Online {
        invariant {
            task_fixed_flow_binding_consistent(self);
            task_online_has_recoverable_context(self);
            task_execution_authority_is(self, TaskExecutionAuthority::None);
            task_breakpoint_state_is(self, TaskBreakpointState::Valid);
            task_breakpoint_bound_to_flow_ref(self, self.flow);
            task_breakpoint_flow_ref_generation_valid(self);
        }
        transitions {
            on Transition::Dispatch -> State::OnCpu {
                depends_on {
                    self.flow.state == State::Online;
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(self, TaskBreakpointState::Valid);
                    task_breakpoint_flow_ref_generation_valid(self);
                    task_breakpoint_bound_to_flow_ref(self, self.flow);
                }
                ensures {
                    task_on_cpu(self);
                    task_on_cpu_matches_current_task(self);
                    task_dispatch_sent_only_by_scheduler(self);
                    task_execution_authority_is(self, TaskExecutionAuthority::Live);
                    task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
                    task_breakpoint_consumed_on_dispatch(self);
                    task_dispatch_enter_proof_created(self, self.flow);
                }
            }
        }
    }

    state State::OnCpu {
        invariant {
            task_fixed_flow_binding_consistent(self);
            task_execution_authority_is(self, TaskExecutionAuthority::Live);
            task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
        }
        transitions {
            on Transition::Suspend -> State::Online {
                depends_on {
                    self.flow.state == State::Online;
                    task_execution_authority_is(self, TaskExecutionAuthority::Live);
                    task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
                }
                ensures {
                    task_not_on_cpu(self);
                    task_suspend_sent_only_by_scheduler(self);
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(self, TaskBreakpointState::Valid);
                    task_breakpoint_bound_to_flow_ref(self, self.flow);
                    task_breakpoint_flow_ref_generation_valid(self);
                    task_breakpoint_published_on_suspend(self, self.flow);
                    task_context_optional_root_generation_checked(self);
                    task_context_active_trap_leaf_checked(self);
                }
            }
            on Transition::Disable -> State::Offline {
                depends_on { task_fixed_flow_offline(self); }
                ensures {
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
                    task_terminal_disable_keeps_breakpoint_invalid(self);
                }
            }
        }
    }

    state State::Offline {
        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on { task_fixed_flow_destroyed(self); }
                ensures { task_destroyed_only_after_flow_cleanup(self); }
            }
        }
    }
    state State::Destroyed { invariant { task_destroyed_only_after_flow_cleanup(self); } }

    processes {
        Action::TerminateForSegmentationFault {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::OnCpu;
                task_execution_authority_is(self, TaskExecutionAuthority::Live);
            }
            ensures {
                task_terminal_reason_is(self, TaskTerminalReason::SegmentationFault);
                task_segv_terminal_info_valid(self);
                task_segv_terminal_fault_address_exact(self);
                task_segv_terminal_wait_signal_eleven(self);
                task_segv_terminal_generates_sigchld(self);
                task_segv_terminal_has_no_user_signal_frame(self);
                task_segv_terminal_releases_mm_once(self);
            }
        }

        Action::TerminateForOutOfMemory {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::OnCpu;
                task_execution_authority_is(self, TaskExecutionAuthority::Live);
            }
            ensures {
                task_terminal_reason_is(self, TaskTerminalReason::OutOfMemory);
                task_oom_terminal_wait_signal_nine(self);
                task_oom_terminal_generates_sigchld(self);
                task_oom_terminal_has_no_user_signal_frame(self);
                task_oom_terminal_has_no_oom_killer_selection(self);
                task_oom_terminal_releases_mm_once(self);
            }
        }

        Action::EnableStackGuard {
            state_effect: StateEffect::None;
            depends_on {
                task_has_unique_stack_attribute(self);
                task_stack_range_valid(self, self.stack);
            }
            ensures { task_stack_guard_ready(self, self.stack); }
        }

        Action::ConfirmCurrentTaskRef(task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::OnCpu;
                task_ref_ready(task_ref);
                task_ref_targets(task_ref, self);
            }
            ensures {
                task_on_cpu_matches_current_task(self);
                current_task_ref_derived_from_selector(task_ref, self);
            }
        }

        Transition::SetRuntimeState(state: TaskRuntimeState) {
            state_effect: StateEffect::Conditional;
            depends_on { task_runtime_state_transition_allowed(self, state); }
            transitions {
                TaskRuntimeState::New -> TaskRuntimeState::Running;
                TaskRuntimeState::Running -> TaskRuntimeState::Running;
            }
            ensures { task_runtime_state_is(self, state); }
            result {
                Allowed: Success(runtime_state_set);
                Disallowed: Failed(invalid_runtime_state_transition);
            }
        }

        Action::PinToBootCpu(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on { cpu_ref_ready(cpu_ref); }
            ensures { task_flag_no_setaffinity(self); task_cpumask_is(self, cpu_ref); }
        }

        Action::SaveCoreContext {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::OnCpu;
                task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
                task_context_flow_ref_is_fixed(self, self.flow);
            }
            ensures {
                task_thread_context_core_saved(self.thread_context);
                task_context_coordinate_saved(self, self.thread_context);
                task_context_epoch_advanced(self);
                task_dispatch_record_committed(self);
            }
        }

        Action::RestoreCoreContext {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                task_breakpoint_state_is(self, TaskBreakpointState::Valid);
                task_breakpoint_bound_to_flow_ref(self, self.flow);
                task_breakpoint_flow_ref_generation_valid(self);
            }
            ensures { task_thread_context_core_restored(self.thread_context); }
        }
    }
}

object UserProcessRegistry: TaskSet {
    initial_state: State::Base;
    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    user_process_registry_capacity_is_32(self);
                    user_process_registry_pid1_stable(self);
                    user_process_registry_allows_independent_aggregates(self);
                    user_process_registry_members_fresh_and_independent(self);
                    user_process_registry_generation_reuse_checked(self);
                }
            }
        }
    }
    state State::Ready {
        invariant {
            user_process_registry_capacity_is_32(self);
            user_process_registry_pid1_stable(self);
            user_process_registry_generation_reuse_checked(self);
        }
    }
    actions {
        Action::ReserveFork(
            task: Task,
            aggregate: UserProcessAggregate,
            generation: UserProcessGeneration
        ) {
            state_effect: StateEffect::None;
            depends_on {
                task.state == State::Online;
                user_task_instance_fresh(task);
                user_task_pid_and_lifecycle_independent(task);
            }
            ensures {
                user_process_registry_slot_state_is(self, UserProcessSlotState::Reserved);
                user_process_registry_slot_generation_is(self, generation);
                user_process_registry_reservation_unpublished(self, task);
                user_process_registry_aggregate_owns_process_resources(self, aggregate);
            }
        }

        Action::PublishFork(task: Task, task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task.state == State::Online;
                task_ref_targets(task_ref, task);
                user_process_registry_reservation_unpublished(self, task);
                user_process_registry_fork_resources_complete(self, task);
                user_process_registry_inbox_reservation_complete(self, task);
            }
            ensures {
                user_process_registry_slot_state_is(self, UserProcessSlotState::Published);
                user_process_registry_contains(self, task);
                user_process_registry_stores_ref(self, task_ref);
                user_process_registry_ref_targets_member(self, task_ref, task);
            }
        }

        Action::AcquireLease(
            task_ref: TaskRef,
            cpu_ref: CpuRef,
            generation: UserProcessGeneration
        ) -> UserProcessLease {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(task_ref);
                user_process_registry_slot_generation_is(self, generation);
            }
            ensures {
                user_process_registry_lease_acquire_checks_generation_and_cpu(self, task_ref, cpu_ref);
            }
            result {
                Acquired: Success(lease_acquired);
                StaleGeneration: Failed(stale_generation);
                WrongCpu: Failed(wrong_cpu);
            }
        }

        Action::PublishZombie(task: Task) {
            state_effect: StateEffect::None;
            depends_on { user_process_registry_contains(self, task); }
            ensures {
                user_process_registry_slot_state_is(self, UserProcessSlotState::Zombie);
                user_process_registry_exit_status_published_release(self, task);
                user_process_registry_parent_wake_target_cpu_fixed(self, task);
            }
        }

        Action::Reap(task: Task) {
            state_effect: StateEffect::None;
            depends_on {
                user_process_registry_slot_state_is(self, UserProcessSlotState::Zombie);
                user_process_registry_reap_excludes_scheduler_and_lease_refs(self, task);
            }
            ensures {
                user_process_registry_slot_state_is(self, UserProcessSlotState::Empty);
            }
        }
    }
}

object BootTask: Task {
    lifecycle_override: true;
    initial_state: State::OnCpu;
    parent: Kernel;
    source: static::linux_6_12;
    associations { flow = BootInitFlow; }
    attrs {
        storage: ObjectStorage<BootTask>;
        pid: Derived<usize, 0>;
        canonical_ref: Derived<TaskRef, BootTaskRef>;
        stack: Derived<Stack, stack(Lds.init_stack_start, Lds.init_stack_end)>;
    }
    reference linux_6_12 { storage = symbol("init_task"); stack = symbol("init_stack"); }

    state State::OnCpu {
        invariant {
            task_fixed_flow_is(self, BootInitFlow);
            task_execution_authority_is(self, TaskExecutionAuthority::Live);
            task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
            boot_task_preemption_is_static_initial_property(self);
            task_stack_is_static_initial_property(self, self.stack);
            task_has_unique_stack_attribute(self);
            task_stack_range_valid(self, self.stack);
        }
        transitions {
            on Transition::Suspend -> State::Online {
                ensures {
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(self, TaskBreakpointState::Valid);
                    task_breakpoint_bound_to_flow_ref(self, BootInitFlow);
                    task_breakpoint_flow_ref_generation_valid(self);
                    task_breakpoint_published_on_suspend(self, BootInitFlow);
                }
            }
        }
    }
    state State::Online {
        invariant {
            task_fixed_flow_is(self, BootInitFlow);
            task_breakpoint_bound_to_flow_ref(self, BootInitFlow);
        }
        transitions {
            on Transition::Dispatch -> State::OnCpu {
                ensures {
                    task_execution_authority_is(self, TaskExecutionAuthority::Live);
                    task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
                    task_breakpoint_consumed_on_dispatch(self);
                }
            }
        }
    }
}

object KernelInitTask: Task { associations { flow = KernelInitFlow; } }
object KthreaddTask: Task { associations { flow = KthreaddFlow; } }

object KernelTask: Task { associations { flow = KernelTaskFlow; } }

object ApIdleTask: Task {
    lifecycle_override: true;
    initial_state: State::OnCpu;
    parent: CpuGroup;
    associations { flow = ApIdleFlow; }
    state State::OnCpu {
        transitions {
            on Transition::Suspend -> State::Online {
                depends_on { task_execution_authority_is(self, TaskExecutionAuthority::Live); }
                ensures {
                    task_execution_authority_is(self, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(self, TaskBreakpointState::Valid);
                    task_breakpoint_bound_to_flow_ref(self, ApIdleFlow);
                }
            }
        }
    }
    state State::Online {
        transitions {
            on Transition::Dispatch -> State::OnCpu {
                depends_on { task_breakpoint_flow_ref_generation_valid(self); }
                ensures {
                    task_execution_authority_is(self, TaskExecutionAuthority::Live);
                    task_breakpoint_state_is(self, TaskBreakpointState::Invalid);
                    task_breakpoint_consumed_on_dispatch(self);
                }
            }
        }
    }
    actions {
        Action::ActivateHsmAuthority {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::OnCpu;
                task_ap_idle_reserved_for_cpu(self);
            }
            ensures {
                task_execution_authority_is(self, TaskExecutionAuthority::Live);
                task_authority_activated_by_hsm(self);
            }
        }
    }
}
