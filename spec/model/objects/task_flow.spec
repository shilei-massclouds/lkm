/*
 * Task Flow Object Model
 *
 * TaskFlow models executable behavior/continuation, not task_struct-like
 * carrier state. It is a PhaseObject subtype: every TaskFlow instance is a
 * phase carried by exactly one owner Task. Changing the active flow never
 * allocates or replaces that Task.
 */

type TaskFlow: PhaseObject {
    /* Every TaskFlow parent is structurally constrained to Task. */
    parent: Task;

    processes {
        Action::Continue {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
                task_active_flow_is(self.parent, self);
            }
            ensures {
                task_flow_resume_active_continuation(self);
            }
        }
    }

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Base;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
                task_flow_start_binding_consistent(self);
            }
            ensures {
                task_flow_started(self);
            }
            emits {
                Transition::Setup;
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Prepared;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            emits {
                Transition::Enable;
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Ready;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                task_flow_active_binding_committed(self);
                task_flow_online_on_cpu(self);
                task_has_unique_active_flow(self.parent);
            }
        }
    }
}

type KthreaddFlowType: TaskFlow {
    lifecycle {
        Transition::Disable {
            state_effect: StateEffect::Always;
            depends_on {
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                task_flow_no_longer_active(self);
            }
        }

        Transition::Cleanup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Offline;
                task_flow_no_longer_active(self);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                task_flow_instance_released(self);
                task_flow_not_active_after_cleanup(self);
            }
        }
    }
}

enum UserAppFlowEntrySource {
    Pid1Exec,
    ForkContinuation,
    ChildExec,
}

type UserAppFlow: TaskFlow {
    lifecycle_override: true;

    processes {
        /* Structural binding is part of declaration, not Flow execution. */
        Action::Bind(
            owner_task: Task,
            entry_source: UserAppFlowEntrySource
        ) {
            state_effect: StateEffect::None;
            structural_binding: true;
            depends_on {
                self.state == State::Base;
            }
            ensures {
                task_owns_flow(owner_task, self);
                task_flow_owner_is(self, owner_task);
                task_flow_parent_is(self, owner_task);
                task_flow_owner_exclusive(self);
                user_app_flow_owner_bound(self);
                user_app_flow_entry_source_bound(self);
                user_app_flow_entry_source_is(self, entry_source);
                user_app_flow_instance_fresh(self);
                user_app_flow_owner_exclusive(self);
                task_flow_start_binding_consistent(self);
            }
        }
    }

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Base;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(
                    self.parent,
                    TaskExecutionAuthority::Live
                );
                task_flow_start_binding_consistent(self);
                user_app_flow_owner_bound(self);
                user_app_flow_entry_source_bound(self);
            }
            ensures {
                task_flow_started(self);
            }
            emits {
                Transition::Setup;
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Prepared;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(
                    self.parent,
                    TaskExecutionAuthority::Live
                );
                user_app_flow_owner_bound(self);
                user_app_flow_entry_source_bound(self);
            }
            ensures {
                user_app_flow_execution_context_ready(self);
                user_app_flow_exec_image_or_fork_continuation_ready(self);
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Ready;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(
                    self.parent,
                    TaskExecutionAuthority::Live
                );
                user_app_flow_execution_context_ready(self);
                task_flow_active_binding_committed(self);
            }
            ensures {
                task_flow_active_binding_committed(self);
                task_flow_online_on_cpu(self);
                task_has_unique_active_flow(self.parent);
                user_app_flow_online(self);
                user_app_flow_is_owner_unique_online_flow(self);
                user_application_black_box_entered(self);
            }
        }

        Transition::Disable {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Online;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                user_app_flow_disable_reason_is_exit_exit_group_or_successful_exec(self);
                task_flow_no_longer_active(self);
                user_app_flow_execution_stopped(self);
            }
        }

        Transition::Cleanup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Offline;
                task_flow_no_longer_active(self);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                task_flow_instance_released(self);
                task_flow_not_active_after_cleanup(self);
                user_app_flow_resources_released(self);
            }
        }
    }
}

predicate task_owns_flow<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_flow_owner_is<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate task_flow_parent_is<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate task_flow_owner_exclusive<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_initial_binding_consistent<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_start_binding_consistent<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_started<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_online_on_cpu<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_resume_active_continuation<F: TaskFlow>(flow: F) -> bool;
predicate task_active_flow_is<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate task_at_most_one_flow_online<T: Task>(task: T) -> bool;
predicate task_flow_no_longer_active<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_not_active_after_cleanup<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_instance_released<F: TaskFlow>(flow: F) -> bool;
predicate task_flow_handoff<
    T: Task,
    From: TaskFlow,
    To: TaskFlow
>(task: T, from: From, to: To) -> bool;
predicate task_flow_handoff_old_inactive<T: Task, From: TaskFlow>(
    task: T,
    from: From
) -> bool;
predicate task_flow_handoff_new_active<T: Task, To: TaskFlow>(
    task: T,
    to: To
) -> bool;
predicate task_flow_instances_distinct<From: TaskFlow, To: TaskFlow>(
    from: From,
    to: To
) -> bool;
predicate task_creation_bound_flow<C, T: Task, F: TaskFlow>(
    core: C,
    task: T,
    flow: F
) -> bool;
predicate user_app_flow_owner_bound<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_entry_source_bound<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_entry_source_is<F: UserAppFlow>(
    flow: F,
    source: UserAppFlowEntrySource
) -> bool;
predicate user_app_flow_instance_fresh<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_owner_exclusive<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_execution_context_ready<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_exec_image_or_fork_continuation_ready<F: UserAppFlow>(flow: F) -> bool;
predicate task_flow_active_binding_committed<F: TaskFlow>(flow: F) -> bool;
predicate user_app_flow_online<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_is_owner_unique_online_flow<F: UserAppFlow>(flow: F) -> bool;
predicate user_application_black_box_entered<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_disable_reason_is_exit_exit_group_or_successful_exec<F: UserAppFlow>(
    flow: F
) -> bool;
predicate user_app_flow_execution_stopped<F: UserAppFlow>(flow: F) -> bool;
predicate user_app_flow_resources_released<F: UserAppFlow>(flow: F) -> bool;
predicate boot_task_idle_flow_entered<T: Task, F: BootIdleFlowType>(
    task: T,
    flow: F
) -> bool;
predicate kthreadd_schedule_loop_deferred_until_on_cpu<
    T: Task,
    F: TaskFlow
>(task: T, flow: F) -> bool;
predicate ap_idle_flow_key_matches_task<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate ap_idle_flow_hsm_startup_keyed<F: TaskFlow>(flow: F) -> bool;
predicate ap_idle_flow_task_ref_and_flow_ref_preserved<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;
predicate ap_idle_flow_pointwise_phases_complete<F: TaskFlow>(flow: F) -> bool;
predicate ap_idle_flow_no_task_enable_or_continue<F: TaskFlow>(flow: F) -> bool;

object KernelInitFlow: TaskFlow {
    initial_state: State::Base;
    parent: KernelInitTask;

    state State::Base {
        invariant {
            task_initial_flow_is(KernelInitTask, self);
            task_owns_flow(KernelInitTask, self);
            task_flow_owner_is(self, KernelInitTask);
            task_flow_parent_is(self, KernelInitTask);
        }

        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    KernelInitTask.state == State::OnCpu;
                    BootInitFlow.state == State::Online;
                    kernel_init_entry_reaches_kernel_init_flow(
                        KernelInitTask,
                        KernelInitFlow
                    );
                }

                drives {
                    PreSmpInitPhase.Transition::Preset;
                    SmpBringupPhase.Transition::Preset;
                }

                ensures {
                    PreSmpInitPhase.state == State::Online;
                    kernel_init_flow_first_leaf(self, PreSmpInitPhase);
                    kernel_init_flow_preset_runs_on_verified_stack(self, KernelInitTask);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            task_flow_started(self);
            PreSmpInitPhase.state == State::Online;
            kernel_init_flow_preset_runs_on_verified_stack(self, KernelInitTask);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SmpBringupPhase.state == State::Online;
                }

                drives {
                    RuntimeCorePhase.Transition::Preset;
                    InitcallPhase.Transition::Preset;
                    RootfsPhase.Transition::Preset;
                    FinalizePhase.Transition::Preset;
                    PayloadPreparePhase.Transition::Preset;
                }

                ensures {
                    RuntimeCorePhase.state == State::Online;
                    InitcallPhase.state == State::Online;
                    RootfsPhase.state == State::Online;
                    FinalizePhase.state == State::Online;
                    PayloadPreparePhase.state == State::Online;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            task_flow_started(self);
            PreSmpInitPhase.state == State::Online;
            SmpBringupPhase.state == State::Online;
            RuntimeCorePhase.state == State::Online;
            InitcallPhase.state == State::Online;
            RootfsPhase.state == State::Online;
            FinalizePhase.state == State::Online;
            PayloadPreparePhase.state == State::Online;
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    PayloadHandoffPreparePhase.Transition::Preset;
                }

                ensures {
                    PayloadHandoffPreparePhase.state == State::Online;
                    kernel_init_flow_survives_payload_precommit(self);
                    task_active_flow_is(KernelInitTask, self);
                }

                emits {
                    self.Action::CommitPayloadHandoff;
                }
            }
        }
    }

    state State::Online {
        invariant {
            task_flow_started(self);
            PreSmpInitPhase.state == State::Online;
            SmpBringupPhase.state == State::Online;
            RuntimeCorePhase.state == State::Online;
            InitcallPhase.state == State::Online;
            RootfsPhase.state == State::Online;
            FinalizePhase.state == State::Online;
            PayloadPreparePhase.state == State::Online;
            PayloadHandoffPreparePhase.state == State::Online;
            task_active_flow_is(KernelInitTask, self);
            task_flow_online_on_cpu(self);
        }

        transitions {
            on Transition::Disable -> State::Offline {
                depends_on {
                    self.parent.state == State::OnCpu;
                    task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
                }
                ensures {
                    task_flow_no_longer_active(self);
                }
            }
        }
    }

    state State::Offline {
        invariant {
            task_owns_flow(KernelInitTask, self);
            task_flow_owner_is(self, KernelInitTask);
            task_flow_parent_is(self, KernelInitTask);
            task_flow_no_longer_active(self);
        }

        transitions {
            on Transition::Cleanup -> State::Destroyed {
                depends_on {
                    task_flow_no_longer_active(self);
                    self.parent.state == State::OnCpu;
                    task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
                }
                ensures {
                    task_flow_instance_released(self);
                    task_flow_not_active_after_cleanup(self);
                }
            }
        }
    }

    state State::Destroyed {
        invariant {
            task_owns_flow(KernelInitTask, self);
            task_flow_owner_is(self, KernelInitTask);
            task_flow_parent_is(self, KernelInitTask);
            task_flow_instance_released(self);
            task_flow_not_active_after_cleanup(self);
        }
    }

    actions {
        /*
         * PID 1 observes kthreadd_done on the kernel_init execution line.
         */
        Action::ObserveKthreaddDoneRelease {
            state_effect: StateEffect::None;
            depends_on {
                KthreaddReadyGate.state == State::Online;
                completion_complete_committed(KthreaddReadyGate);
                completion_token_available(KthreaddReadyGate);
                kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }

            drives {
                KthreaddReadyGate.Transition::Wait;
            }

            ensures {
                completion_waiter_enqueued(KthreaddReadyGate);
                completion_waiter_finished(KthreaddReadyGate);
                kernel_init_observed_kthreadd_done_release(KernelInitTask, KthreaddReadyGate);
                kernel_init_released_for_pre_smp_init(KernelInitTask);
            }
        }

        Action::CommitPayloadHandoff {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                PayloadHandoffPreparePhase.state == State::Online;
                SelectedPayloadHandoff.state == State::Online;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }

            drives {
                SelectedPayloadHandoff.Action::CommitSelectedVariant;
            }

            ensures {
                selected_payload_no_return_handoff();
                kernel_init_flow_payload_handoff_committed(self);
                selected_payload_user_boot_replacement_ordered(
                    SelectedPayloadHandoff,
                    self,
                    KernelInitTask
                );
                selected_payload_kernel_mode_keeps_kernel_init_flow(
                    SelectedPayloadHandoff,
                    self
                );
            }
        }
    }
}

/*
 * The first PID 1 exec uses a fresh UserAppFlow instance backed by the
 * implementation's dedicated first-user-flow slot. It is structurally bound
 * and prepared while KernelInitFlow is still Ready; only the later payload
 * commit makes it active and Online.
 */
object Pid1UserAppFlow: UserAppFlow {
    initial_state: State::Base;
    parent: KernelInitTask;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    user_app_flow_instance_fresh(self);
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
            }
        }
    }

    state State::Ready {
        invariant {
            user_app_flow_instance_fresh(self);
            user_app_flow_execution_context_ready(self);
            user_app_flow_exec_image_or_fork_continuation_ready(self);
        }

        transitions {
            on Transition::Enable -> State::Online {
            }
        }
    }

    state State::Online {
        invariant {
            user_app_flow_instance_fresh(self);
            user_app_flow_online(self);
            task_active_flow_is(KernelInitTask, self);
        }
    }

    state State::Offline { }
    state State::Destroyed { }
}

predicate kernel_init_entry_stack_verified<T: Task>(task: T) -> bool;
predicate kernel_init_flow_first_leaf<F: TaskFlow, P>(flow: F, phase: P) -> bool;
predicate kernel_init_entry_reaches_kernel_init_flow<T: Task, F: TaskFlow>(task: T, flow: F) -> bool;
predicate kernel_init_flow_preset_runs_on_verified_stack<F: TaskFlow, T: Task>(flow: F, task: T) -> bool;

object KthreaddFlow: KthreaddFlowType {
    initial_state: State::Base;
    parent: KthreaddTask;

    state State::Base {
        invariant {
            task_initial_flow_is(KthreaddTask, self);
            task_owns_flow(KthreaddTask, self);
            task_flow_owner_is(self, KthreaddTask);
            task_flow_parent_is(self, KthreaddTask);
        }

        transitions {
            on Transition::Preset -> State::Prepared {
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    task_active_flow_is(KthreaddTask, self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            task_flow_started(self);
            task_active_flow_is(KthreaddTask, self);
            task_flow_online_on_cpu(self);
        }
    }

    state State::Offline {
        invariant {
            task_owns_flow(KthreaddTask, self);
            task_flow_owner_is(self, KthreaddTask);
            task_flow_parent_is(self, KthreaddTask);
            task_flow_no_longer_active(self);
        }
    }

    state State::Destroyed {
        invariant {
            task_owns_flow(KthreaddTask, self);
            task_flow_owner_is(self, KthreaddTask);
            task_flow_parent_is(self, KthreaddTask);
            task_flow_instance_released(self);
            task_flow_not_active_after_cleanup(self);
        }
    }

    actions {
        /*
         * Minimal kthreadd service loop; the carrier stays KthreaddTask.
         */
        Action::RunScheduleLoop {
            state_effect: StateEffect::None;
            depends_on {
                task_owns_flow(KthreaddTask, KthreaddFlow);
                task_state_running(KthreaddTask);
                kthreadd_provider_ready(KthreaddTask);
                Scheduler.state == State::Online;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                kthreadd_entry_reaches_schedule_loop(KthreaddTask, Scheduler);
                kthreadd_schedule_loop_ready(KthreaddTask, Scheduler);
                kthreadd_schedule_loop_active(KthreaddTask, Scheduler);
            }
        }
    }
}

/*
 * Historical/deferred application appendix
 *
 * User application instructions, BusyBox shell/login sequencing and LTP
 * command behavior are historical observations only. They are intentionally
 * outside the formal UserAppFlow state machine. Syscall, trap, files and other
 * kernel resource operations remain modeled by their corresponding objects.
 */

/*
 * Pointwise representative of the logical-id indexed AP idle Flow family.
 * The HSM delivery key is resolved from boot-data task_ptr.initial_flow after
 * the AP carrier authority has changed from Reserved to Live.
 */
type ApIdleFlowType: TaskFlow {
}

object ApIdleFlow: ApIdleFlowType {
    initial_state: State::Base;
    parent: ApIdleTask;

    state State::Base {
        invariant {
            task_initial_flow_is(ApIdleTask, self);
            task_owns_flow(ApIdleTask, self);
            task_flow_owner_is(self, ApIdleTask);
            task_flow_parent_is(self, ApIdleTask);
        }

        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    task_authority_activated_by_hsm(ApIdleTask);
                    ap_idle_flow_hsm_startup_keyed(self);
                    ap_idle_flow_key_matches_task(self, ApIdleTask);
                }
                drives {
                    ApEntryPreludePhase.Transition::Preset;
                    ApSmpCallinPhase.Transition::Preset;
                    ApOnlineIdlePhase.Transition::Preset;
                }
                ensures {
                    ap_idle_flow_task_ref_and_flow_ref_preserved(self, ApIdleTask);
                    ap_idle_flow_pointwise_phases_complete(self);
                    ap_idle_flow_no_task_enable_or_continue(self);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ap_idle_flow_pointwise_phases_complete(self);
            ap_idle_flow_no_task_enable_or_continue(self);
        }
        transitions {
            on Transition::Setup -> State::Ready {
            }
        }
    }

    state State::Ready {
        invariant {
            ap_idle_flow_pointwise_phases_complete(self);
        }
        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    task_active_flow_is(ApIdleTask, self);
                    ap_idle_flow_no_task_enable_or_continue(self);
                }
                emits {
                    SmpBringupPhase.Action::ObserveApCompletion;
                }
            }
        }
    }

    state State::Online {
        invariant {
            task_active_flow_is(ApIdleTask, self);
            task_flow_online_on_cpu(self);
            ap_idle_flow_pointwise_phases_complete(self);
            ap_idle_flow_no_task_enable_or_continue(self);
        }
    }
}

/*
 * Idle flow behavior and its boot-CPU instance.
 */
type BootIdleFlowType: TaskFlow {
    processes {
        Action::PrepareIdleEntry {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                scheduler_first_schedule_committed(Scheduler);
                task_ref_ready(CurrentTaskRef);
                task_ref_targets(CurrentTaskRef, BootTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                boot_idle_entry_prepared(self, BootTask);
                task_flow_resume_active_continuation(self);
                boot_task_idle_flow_entered(BootTask, BootIdleFlow);
                boot_idle_task_pf_idle(BootTask);
                boot_idle_arch_cpu_idle_prepare_done(self, BootCPU);
                boot_idle_cpuhp_online_state_confirmed(self, BootCPU);
                boot_cpu_hotplug_state_online(BootCPU);
                boot_idle_need_resched_clear_before_wait(BootTask);
            }
        }

        Action::RunIdleLoop {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_entry_prepared(self, BootTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            drives {
                self.Action::DoIdleCycle;
            }
            ensures {
                boot_idle_runtime_loop_entered(self, BootTask);
                boot_idle_loop_continues(self);
            }
        }

        Action::DoIdleCycle {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_entry_prepared(self, BootTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            drives {
                self.Action::WaitWhileNoNeedResched;
                self.Action::ObserveNeedResched;
                self.Action::ScheduleIfNeedResched;
            }
            ensures {
                boot_idle_runtime_cycle_started(self, BootTask);
                boot_idle_nohz_run_idle_balance_done(self, BootCPU);
                boot_idle_loop_cycle_committed(self);
                boot_idle_loop_continues(self);
            }
        }

        Action::WaitWhileNoNeedResched {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_entry_prepared(self, BootTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            within BootIdleWaitLocalInterruptContext {
                ensures {
                    boot_idle_local_irq_disabled_for_sleep(self, BootCpuLocalInterrupt);
                    boot_idle_arch_cpu_idle_enter_done(self, BootCPU);
                    boot_idle_rcu_nocb_deferred_wakeup_flushed(self);
                    boot_idle_cpu_offline_dead_path_not_taken(self, BootCPU);
                    boot_idle_poll_or_cpuidle_path_deferred(self);
                    boot_idle_arch_cpu_idle_exit_done(self, BootCPU);
                }
            }
            ensures {
                boot_idle_runtime_cycle_started(self, BootTask);
                boot_idle_need_resched_clear_before_wait(BootTask);
                boot_idle_runtime_observed_no_need_resched(self, BootTask);
                boot_idle_polling_set(BootTask);
                boot_idle_polling_rmb_before_sleep_check(BootTask);
                boot_idle_nohz_entered(self);
                boot_idle_local_irq_disabled_for_sleep(self, BootCpuLocalInterrupt);
                boot_idle_arch_cpu_idle_enter_done(self, BootCPU);
                boot_idle_rcu_nocb_deferred_wakeup_flushed(self);
                boot_idle_cpu_offline_dead_path_not_taken(self, BootCPU);
                boot_idle_poll_or_cpuidle_path_deferred(self);
                boot_idle_arch_cpu_idle_exit_done(self, BootCPU);
                boot_idle_runtime_waiting(self, BootTask);
                boot_idle_wait_path_deferred(self);
            }
        }

        Action::ObserveNeedResched {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_runtime_waiting(self, BootTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            ensures {
                boot_idle_need_resched_set_for_schedule(BootTask);
                boot_idle_runtime_observed_need_resched(self, BootTask);
                boot_idle_polling_cleared(BootTask);
                boot_idle_preempt_need_resched_set(BootTask);
                boot_idle_nohz_exited(self);
                boot_idle_polling_clear_mb_before_flush(BootTask);
                boot_idle_smp_call_function_queue_flushed(self);
            }
        }

        Action::ScheduleIfNeedResched {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_need_resched_set_for_schedule(BootTask);
                task_ref_ready(CurrentTaskRef);
                task_ref_targets(CurrentTaskRef, BootTask);
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
            }
            drives {
                Scheduler.Action::ScheduleIdle;
            }
            ensures {
                boot_idle_schedule_requested(self, Scheduler);
                boot_idle_schedule_returned(self, Scheduler);
                scheduler_idle_schedule_returned_to_idle(Scheduler, CurrentTaskRef);
                boot_idle_need_resched_drained_after_schedule(BootTask);
                boot_idle_loop_continues(self);
                boot_idle_livepatch_state_update_deferred(self);
                task_ref_targets(CurrentTaskRef, BootTask);
            }
        }
    }
}

object BootIdleFlow: BootIdleFlowType {
    lifecycle_override: true;
    initial_state: State::Base;
    parent: BootTask;

    /*
     * Base 表示 boot idle 任务已存在，但尚未进入 cpu_startup_entry() 运行期入口。
     */
    state State::Base {
        transitions {
            /*
             * Setup 在首次不可逆 task switch 前把 active continuation 从
             * initial BootInitFlow 交给后继 BootIdleFlow。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    BootIdleSetup.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                    KthreaddReadyGate.state == State::Online;
                    CpuGroup.state == State::Ready;
                    BootTask.state == State::OnCpu;
                    task_execution_authority_is(BootTask, TaskExecutionAuthority::Live);
                }

                ensures {
                    boot_idle_runtime_ready(BootIdleFlow, BootTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
                    task_owns_flow(BootTask, BootIdleFlow);
                    task_flow_owner_is(BootIdleFlow, BootTask);
                    task_flow_parent_is(BootIdleFlow, BootTask);
                    task_flow_owner_exclusive(BootIdleFlow);
                    task_active_flow_is(BootTask, BootIdleFlow);
                    task_has_unique_active_flow(BootTask);
                    secondary_cpus_not_started(CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示 BootTask 的 idle successor binding 已预先建立；idle entry 尚未执行。
     */
    state State::Ready {
        invariant {
            boot_idle_runtime_ready(BootIdleFlow, BootTask);
            boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
            task_owns_flow(BootTask, BootIdleFlow);
            task_flow_owner_is(BootIdleFlow, BootTask);
            task_flow_parent_is(BootIdleFlow, BootTask);
            task_flow_owner_exclusive(BootIdleFlow);
            task_active_flow_is(BootTask, BootIdleFlow);
            task_has_unique_active_flow(BootTask);
            secondary_cpus_not_started(CpuGroup);
        }
    }

    actions {
        Action::Continue {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                self.parent.state == State::OnCpu;
                task_execution_authority_is(self.parent, TaskExecutionAuthority::Live);
                task_active_flow_is(BootTask, self);
            }
            drives {
                self.Action::PrepareIdleEntry;
                self.Action::RunIdleLoop;
            }
            ensures {
                task_flow_resume_active_continuation(self);
                boot_task_idle_flow_entered(BootTask, self);
            }
        }
    }

}

/*
 * Appendix: extracted flow mapping and deferred decisions
 *
 * - KernelInitFlow spans kernel_init, pre-SMP initialization, initcalls and the
 *   exec handoff. A fresh declared UserAppFlow becomes active after exec
 *   without replacing KernelInitTask.
 * - KthreaddFlow owns the kthreadd service loop; BootIdleFlow owns
 *   cpu_startup_entry()/do_idle()/schedule_idle behavior.
 * - User application images do not create per-program flow types. Every exec
 *   creates a fresh UserAppFlow instance owned by the unchanged Task.
 * - Dynamic Task/Flow declaration, owned-flow facts and generic Type lifecycle
 *   invocation are formal model capabilities. Runtime instances are not added
 *   to the static object inventory.
 */
