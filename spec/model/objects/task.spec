/*
 * Task Object Model
 *
 * Task is the single task-structure carrier abstraction, corresponding
 * approximately to Linux task_struct. The former empty task bucket is
 * intentionally abolished:
 * subsystem services and resources are not tasks, and concrete task carriers
 * do not need a parallel task-kind wrapper.
 *
 * TaskFlow is defined separately in task_flow.spec. A Task may change its
 * active TaskFlow without changing task identity; the scheduler schedules Task
 * instances and resumes the continuation of the Task's active flow.
 */

enum TaskRuntimeState {
    New,
    Running,
}

type TaskRef {
    processes {
        Action::SetCurrent(task: Task) {
            state_effect: StateEffect::None;
            ensures {
                task_ref_targets(self, task);
            }
        }
    }
}

type TaskRefSet {
}

/*
 * TaskSet is an aggregate of Task instances, not itself a task carrier.
 */
type TaskSet: ResourceObject {
}

predicate task_all_owned_flows_inactive<T: Task>(task: T) -> bool;
predicate task_all_owned_flows_destroyed<T: Task>(task: T) -> bool;
predicate task_no_owned_flow_online<T: Task>(task: T) -> bool;
predicate task_exit_flow_disable_cleanup_ordered<T: Task>(task: T) -> bool;
predicate task_destroyed_only_after_flow_cleanup<T: Task>(task: T) -> bool;
predicate user_task_set_allows_multiple_independent_tasks<S: TaskSet>(set: S) -> bool;
predicate user_task_set_contains<S: TaskSet, T: Task>(set: S, task: T) -> bool;
predicate user_task_set_stores_ref<S: TaskSet, R: TaskRef>(set: S, task_ref: R) -> bool;
predicate user_task_set_ref_targets_member<S: TaskSet, R: TaskRef, T: Task>(
    set: S,
    task_ref: R,
    task: T
) -> bool;
predicate user_task_set_members_fresh_and_independent<S: TaskSet>(set: S) -> bool;
predicate user_task_instance_fresh<T: Task>(task: T) -> bool;
predicate user_task_pid_and_lifecycle_independent<T: Task>(task: T) -> bool;
predicate task_has_no_prior_active_flow<T: Task>(task: T) -> bool;
predicate task_all_prior_owned_flows_destroyed<T: Task>(task: T) -> bool;
predicate task_retired_flow_destroyed<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_fresh_identity<T: Task>(task: T) -> bool;
predicate task_initial_ref_ready<T: Task>(task: T) -> bool;
predicate task_initial_flow_owned<T: Task>(task: T) -> bool;
predicate task_initial_flow_is<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_initial_flow_binding_complete<T: Task>(task: T) -> bool;
predicate task_initial_flow_binding_consistent<T: Task>(task: T) -> bool;
predicate task_clone_spec_ready<T: Task>(task: T) -> bool;
predicate task_runqueue_publication_committed<T: Task>(task: T) -> bool;
predicate task_has_unique_active_flow<T: Task>(task: T) -> bool;
predicate task_online_schedulable<T: Task>(task: T) -> bool;
predicate task_online_does_not_imply_dispatched<T: Task>(task: T) -> bool;
predicate task_ref_targets_online_task<R: TaskRef>(task_ref: R) -> bool;
predicate task_on_cpu<T: Task>(task: T) -> bool;
predicate task_not_on_cpu<T: Task>(task: T) -> bool;
predicate task_dispatch_continuation_pending<T: Task>(task: T) -> bool;
predicate task_dispatch_continuation_consumed<T: Task>(task: T) -> bool;
predicate task_continue_sent_only_by_scheduler<T: Task>(task: T) -> bool;
predicate task_suspend_sent_only_by_scheduler<T: Task>(task: T) -> bool;
predicate task_on_cpu_matches_current_slot<T: Task>(task: T) -> bool;
predicate current_task_slot_matches_on_cpu_task<S, T: Task>(slot: S, task: T) -> bool;
predicate scheduler_continue_signal_pending<S, R: TaskRef>(scheduler: S, task_ref: R) -> bool;

predicate boot_task_idle_role_ready<T: Task, R>(
    task: T,
    runqueue: R
) -> bool;
predicate boot_task_identity_preserved_for_idle<T: Task>(task: T) -> bool;

type RegisterValue {
}

/*
 * TaskThreadContext is the architecture-specific core switch context owned by
 * every Task. On RISC-V this is the task_struct.thread register subset saved
 * and restored by __switch_to: ra, sp and s0..s11. Floating-point/vector
 * state, prev_cpu, icache policy and memory-context switching remain separate
 * or deferred concerns.
 */
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
}

/*
 * Task is the only reusable carrier type. TaskRuntimeState is an extended
 * runtime state, not lifecycle state. Concrete carrier instances below all
 * have kind Task; behavior-specific distinctions belong to TaskFlow.
 */
type Task: ResourceObject {
    initial_state: State::Base;

    ext_state: TaskRuntimeState;
    cpu_ref: CpuRef;

    associations {
        initial_flow: TaskFlow;
        mutable active_flow: TaskFlow;
    }

    owned {
        thread_context: TaskThreadContext;
    }

    state State::Base {
        transitions {
            on Transition::Preset(
                parent_task: Task,
                task_ref: TaskRef,
                initial_flow: TaskFlow
            ) -> State::Prepared {
                depends_on {
                    parent_task.state == State::OnCpu;
                }
                ensures {
                    task_fresh_identity(self);
                    task_initial_ref_ready(self);
                    task_initial_flow_owned(self);
                    task_initial_flow_is(self, initial_flow);
                    task_initial_flow_binding_complete(self);
                    task_initial_flow_binding_consistent(self);
                    task_clone_spec_ready(self);
                    task_clone_args_ready(self);
                    task_ref_targets(task_ref, self);
                    task_ref_ready(task_ref);
                    task_owns_flow(self, initial_flow);
                    task_flow_owner_is(initial_flow, self);
                    task_flow_parent_is(initial_flow, self);
                    task_flow_owner_exclusive(initial_flow);
                }
                updates {
                    self.active_flow = initial_flow;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            task_fresh_identity(self);
            task_initial_ref_ready(self);
            task_initial_flow_owned(self);
            task_initial_flow_binding_complete(self);
            task_initial_flow_binding_consistent(self);
            task_clone_spec_ready(self);
            task_clone_args_ready(self);
        }

        transitions {
            on Transition::Setup(
                parent_task: Task,
                pid_ns: RootPidNamespace,
                scheduler: Scheduler,
                initial_flow: TaskFlow
            ) -> State::Ready {
                depends_on {
                    parent_task.state == State::OnCpu;
                    task_creation_copy_process_committed(
                        TaskCreationCore,
                        parent_task,
                        self
                    );
                    task_creation_bound_flow(TaskCreationCore, self, initial_flow);
                }
                ensures {
                    task_fresh_identity(self);
                    task_initial_ref_ready(self);
                    task_initial_flow_owned(self);
                    task_initial_flow_is(self, initial_flow);
                    task_initial_flow_binding_complete(self);
                    task_initial_flow_binding_consistent(self);
                    task_clone_spec_ready(self);
                    task_pid_allocated(self, pid_ns);
                    task_thread_context_ready(self);
                    task_thread_context_owned(self, self.thread_context);
                    task_thread_context_core_register_set(self.thread_context);
                    task_sched_entity_initialized(self, scheduler);
                    task_state_new(self);
                    task_not_enqueued(self);
                    task_has_no_prior_active_flow(self);
                    task_all_prior_owned_flows_destroyed(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            task_fresh_identity(self);
            task_initial_ref_ready(self);
            task_initial_flow_owned(self);
            task_initial_flow_binding_complete(self);
            task_initial_flow_binding_consistent(self);
            task_clone_spec_ready(self);
            task_thread_context_ready(self);
            task_thread_context_owned(self, self.thread_context);
            task_thread_context_core_register_set(self.thread_context);
            task_state_new(self);
            task_not_enqueued(self);
            task_has_no_prior_active_flow(self);
            task_all_prior_owned_flows_destroyed(self);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    task_state_running(self);
                    task_runqueue_publication_committed(self);
                    task_initial_flow_binding_complete(self);
                    task_initial_flow_binding_consistent(self);
                    task_owns_flow(self, self.initial_flow);
                    task_flow_owner_is(self.initial_flow, self);
                    task_flow_parent_is(self.initial_flow, self);
                }
                ensures {
                    task_fresh_identity(self);
                    task_initial_ref_ready(self);
                    task_initial_flow_owned(self);
                    task_initial_flow_binding_complete(self);
                    task_initial_flow_binding_consistent(self);
                    task_clone_spec_ready(self);
                    task_thread_context_ready(self);
                    task_thread_context_owned(self, self.thread_context);
                    task_thread_context_core_register_set(self.thread_context);
                    task_state_running(self);
                    task_runqueue_publication_committed(self);
                    task_owns_flow(self, self.initial_flow);
                    task_flow_owner_is(self.initial_flow, self);
                    task_flow_parent_is(self.initial_flow, self);
                    task_at_most_one_flow_online(self);
                    task_online_schedulable(self);
                    task_online_does_not_imply_dispatched(self);
                }

            }
        }
    }

    state State::Online {
        invariant {
            task_fresh_identity(self);
            task_initial_ref_ready(self);
            task_initial_flow_owned(self);
            task_initial_flow_binding_complete(self);
            task_initial_flow_binding_consistent(self);
            task_clone_spec_ready(self);
            task_thread_context_ready(self);
            task_thread_context_owned(self, self.thread_context);
            task_thread_context_core_register_set(self.thread_context);
            task_state_running(self);
            task_runqueue_publication_committed(self);
            task_at_most_one_flow_online(self);
            task_online_schedulable(self);
            task_online_does_not_imply_dispatched(self);
        }

        transitions {
            on Transition::Continue -> State::OnCpu {
                ensures {
                    task_on_cpu(self);
                    task_on_cpu_matches_current_slot(self);
                    task_dispatch_continuation_pending(self);
                    task_continue_sent_only_by_scheduler(self);
                }

                emits {
                    self.Action::DispatchContinuation;
                }
            }

            on Transition::Disable -> State::Offline {
                depends_on {
                    task_all_owned_flows_inactive(self);
                    task_no_owned_flow_online(self);
                }
                ensures {
                    task_all_owned_flows_inactive(self);
                    task_exit_flow_disable_cleanup_ordered(self);
                    task_no_owned_flow_online(self);
                }
            }
        }
    }

    state State::OnCpu {
        invariant {
            task_fresh_identity(self);
            task_initial_ref_ready(self);
            task_initial_flow_owned(self);
            task_initial_flow_binding_complete(self);
            task_initial_flow_binding_consistent(self);
            task_clone_spec_ready(self);
            task_thread_context_ready(self);
            task_thread_context_owned(self, self.thread_context);
            task_thread_context_core_register_set(self.thread_context);
            task_state_running(self);
            task_runqueue_publication_committed(self);
            task_at_most_one_flow_online(self);
        }

        transitions {
            on Transition::Suspend -> State::Online {
                ensures {
                    task_not_on_cpu(self);
                    task_suspend_sent_only_by_scheduler(self);
                }
            }
        }
    }

    state State::Offline {
        invariant {
            task_all_owned_flows_inactive(self);
            task_exit_flow_disable_cleanup_ordered(self);
            task_no_owned_flow_online(self);
        }

        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on {
                    task_all_owned_flows_destroyed(self);
                    task_no_owned_flow_online(self);
                }
                ensures {
                    task_all_owned_flows_destroyed(self);
                    task_no_owned_flow_online(self);
                    task_destroyed_only_after_flow_cleanup(self);
                }
            }
        }
    }

    state State::Destroyed {
        invariant {
            task_all_owned_flows_destroyed(self);
            task_no_owned_flow_online(self);
            task_destroyed_only_after_flow_cleanup(self);
        }
    }

    processes {
        Action::DispatchContinuation {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::OnCpu;
                task_on_cpu(self);
                task_dispatch_continuation_pending(self);
            }
            drives {
                self.initial_flow.Transition::Preset || self.active_flow.Action::Continue;
            }
            ensures {
                task_dispatch_continuation_consumed(self);
            }
        }

        Transition::SetRuntimeState(state: TaskRuntimeState) {
            state_effect: StateEffect::Conditional;
            depends_on {
                task_runtime_state_transition_allowed(self, state);
            }
            transitions {
                TaskRuntimeState::New -> TaskRuntimeState::Running;
                TaskRuntimeState::Running -> TaskRuntimeState::Running;
            }
            ensures {
                task_runtime_state_is(self, state);
            }
            result {
                Allowed: Success(runtime_state_set);
                Disallowed: Failed(invalid_runtime_state_transition);
            }
        }

        Action::PinToBootCpu(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                cpu_ref_ready(cpu_ref);
            }
            ensures {
                task_flag_no_setaffinity(self);
                task_cpumask_is(self, cpu_ref);
            }
        }

        Action::SetTaskCpu(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                cpu_ref_ready(cpu_ref);
            }
            ensures {
                task_cpu_ref_is(self, cpu_ref);
            }
        }

        Action::SaveCoreContext {
            state_effect: StateEffect::None;
            depends_on {
                task_thread_context_owned(self, self.thread_context);
                task_thread_context_core_register_set(self.thread_context);
            }
            ensures {
                task_thread_context_core_saved(self.thread_context);
            }
        }

        Action::RestoreCoreContext {
            state_effect: StateEffect::None;
            depends_on {
                task_thread_context_owned(self, self.thread_context);
                task_thread_context_core_register_set(self.thread_context);
            }
            ensures {
                task_thread_context_core_restored(self.thread_context);
            }
        }

        Action::CommitFlowHandoff(from_flow: TaskFlow, to_flow: TaskFlow) {
            state_effect: StateEffect::None;
            depends_on {
                task_owns_flow(self, from_flow);
                task_owns_flow(self, to_flow);
                task_flow_owner_is(from_flow, self);
                task_flow_owner_is(to_flow, self);
                from_flow.state == State::Offline;
                to_flow.state == State::Ready;
                task_flow_no_longer_active(from_flow);
            }
            ensures {
                task_flow_handoff(self, from_flow, to_flow);
                task_flow_handoff_old_inactive(self, from_flow);
                task_flow_handoff_new_active(self, to_flow);
                task_active_flow_is(self, to_flow);
                task_at_most_one_flow_online(self);
                task_flow_active_binding_committed(to_flow);
                task_flow_instances_distinct(from_flow, to_flow);
            }
            updates {
                self.active_flow = to_flow;
            }
        }

        Action::RecordRetiredFlowDestroyed(flow: TaskFlow) {
            state_effect: StateEffect::None;
            depends_on {
                task_owns_flow(self, flow);
                flow.state == State::Destroyed;
                task_flow_not_active_after_cleanup(flow);
            }
            ensures {
                task_retired_flow_destroyed(self, flow);
                task_all_prior_owned_flows_destroyed(self);
            }
        }

        Action::ConfirmOwnedFlowSetDestroyed(current_flow: TaskFlow) {
            state_effect: StateEffect::None;
            depends_on {
                task_owns_flow(self, current_flow);
                task_all_prior_owned_flows_destroyed(self);
                current_flow.state == State::Destroyed;
                task_flow_not_active_after_cleanup(current_flow);
            }
            ensures {
                task_all_owned_flows_inactive(self);
                task_all_owned_flows_destroyed(self);
                task_no_owned_flow_online(self);
                task_exit_flow_disable_cleanup_ordered(self);
            }
        }
    }
}

/*
 * General aggregate for all user Tasks. It does not impose a single active
 * child slot: every fork/clone adds a fresh TaskRef/Task pair with an
 * independent PID and lifecycle.
 */
object UserTaskSet: TaskSet {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    user_task_set_allows_multiple_independent_tasks(self);
                    user_task_set_members_fresh_and_independent(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_task_set_allows_multiple_independent_tasks(self);
            user_task_set_members_fresh_and_independent(self);
        }
    }

    actions {
        Action::Insert(task: Task, task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                task.state == State::Prepared;
                task_ref_targets(task_ref, task);
                task_ref_ready(task_ref);
                user_task_instance_fresh(task);
                user_task_pid_and_lifecycle_independent(task);
            }
            ensures {
                user_task_set_contains(self, task);
                user_task_set_stores_ref(self, task_ref);
                user_task_set_ref_targets_member(self, task_ref, task);
                user_task_set_members_fresh_and_independent(self);
            }
        }
    }
}

/*
 * Concrete Task instances
 *
 * BootTask is the statically allocated init_task carrier. KernelInitTask and
 * KthreaddTask are copy_process-created static model objects. User children
 * are runtime Task instances declared by the clone action.
 */

/*
 * Static init_task carrier. Firmware/architecture entry has already granted
 * the boot CPU to this Task before the model begins. Its initial execution is
 * therefore OnCpu and does not pass through Scheduler.Continue.
 */
object BootTask: Task {
    lifecycle_override: true;
    initial_state: State::OnCpu;
    parent: Kernel;
    source: static::linux_6_12;

    associations {
        initial_flow = BootInitFlow;
        active_flow = BootIdleFlow;
    }

    attrs {
        storage: ObjectStorage<BootTask>;
        pid: Derived<usize, 0>;
        canonical_ref: Derived<TaskRef, BootTaskRef>;
    }

    reference linux_6_12 {
        storage = symbol("init_task");
    }

    state State::OnCpu {
        invariant {
            attrs_accessible(self);
            valid_object_storage(storage);
            valid_task_storage(storage);
            task_initial_flow_is(self, BootInitFlow);
            task_initial_flow_binding_complete(self);
            task_initial_flow_binding_consistent(self);
            task_owns_flow(self, BootInitFlow);
            task_flow_owner_is(BootInitFlow, self);
            task_flow_parent_is(BootInitFlow, self);
        }

        transitions {
            on Transition::Suspend -> State::Online {
                ensures {
                    attrs_accessible(self);
                    valid_object_storage(storage);
                    valid_task_storage(storage);
                    task_initial_flow_is(self, BootInitFlow);
                    task_initial_flow_binding_complete(self);
                    task_initial_flow_binding_consistent(self);
                    task_owns_flow(self, BootInitFlow);
                    task_flow_owner_is(BootInitFlow, self);
                    task_flow_parent_is(BootInitFlow, self);
                    task_not_on_cpu(self);
                    task_suspend_sent_only_by_scheduler(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_object_storage(storage);
            valid_task_storage(storage);
            task_initial_flow_is(self, BootInitFlow);
            task_initial_flow_binding_complete(self);
            task_initial_flow_binding_consistent(self);
            task_owns_flow(self, BootInitFlow);
            task_flow_owner_is(BootInitFlow, self);
            task_flow_parent_is(BootInitFlow, self);
            task_online_schedulable(self);
            task_online_does_not_imply_dispatched(self);
            task_not_on_cpu(self);
        }

        transitions {
            on Transition::Continue -> State::OnCpu {
                ensures {
                    task_on_cpu(self);
                    task_on_cpu_matches_current_slot(self);
                    task_dispatch_continuation_pending(self);
                    task_continue_sent_only_by_scheduler(self);
                }
                emits {
                    self.Action::DispatchContinuation;
                }
            }
        }
    }
}

/*
 * PID 1 carrier created by copy_process. KernelInitFlow is its initial flow;
 * exec later hands the same carrier to a declared UserAppFlow instance.
 */
object KernelInitTask: Task {
    associations {
        initial_flow = KernelInitFlow;
        active_flow = KernelInitFlow;
    }
}

/*
 * kthreadd task_struct carrier. Provider-reference publication stays on the
 * carrier; the service loop belongs to KthreaddFlow.
 */
object KthreaddTask: Task {
    associations {
        initial_flow = KthreaddFlow;
        active_flow = KthreaddFlow;
    }
}

/*
 * Appendix: current boundaries and deferred refinements
 *
 * - Boot idle is not a second Task instance: BootTask retains identity when
 *   its flow changes to the idle flow. The sched-init
 *   coordination wrapper remains phase-owned and is not a task carrier.
 * - PID 1 user resources and role attach directly to KernelInitTask; there is
 *   no separate persona wrapper.
 * - SecondaryIdleTaskSet is an aggregate resource. Individuating one Task per
 *   possible non-boot CPU is deferred until the SMP task topology is expanded.
 * - UserTaskSet admits multiple independent runtime child Tasks and stores
 *   typed TaskRefs for later process dispatch; no static child witness exists.
 */
