/*
 * Task Flow Object Model
 *
 * TaskFlow models executable behavior/continuation, not task_struct-like
 * carrier state. Flow types define reusable behavior while every object below
 * is a distinct runtime instance with its own lifecycle state and exactly one
 * owner Task. Changing the active flow never allocates or replaces that Task.
 */

type TaskFlow: FlowObject {
}

type KernelInitFlowType: TaskFlow {
    lifecycle {
        Transition::Disable {
            state_effect: StateEffect::Always;
            ensures {
                task_flow_no_longer_active(self);
            }
        }

        Transition::Cleanup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Offline;
                task_flow_no_longer_active(self);
            }
            ensures {
                task_flow_instance_released(self);
                task_flow_not_active_after_cleanup(self);
            }
        }
    }
}

type KthreaddFlowType: TaskFlow {
    lifecycle {
        Transition::Disable {
            state_effect: StateEffect::Always;
            ensures {
                task_flow_no_longer_active(self);
            }
        }

        Transition::Cleanup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Offline;
                task_flow_no_longer_active(self);
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
    lifecycle {
        Transition::Preset(
            owner_task: Task,
            entry_source: UserAppFlowEntrySource
        ) {
            state_effect: StateEffect::Always;
            ensures {
                task_owns_flow(owner_task, self);
                task_flow_owner_is(self, owner_task);
                task_flow_owner_exclusive(self);
                user_app_flow_owner_bound(self);
                user_app_flow_entry_source_bound(self);
                user_app_flow_entry_source_is(self, entry_source);
                user_app_flow_instance_fresh(self);
                user_app_flow_owner_exclusive(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Prepared;
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
                user_app_flow_execution_context_ready(self);
                task_flow_active_binding_committed(self);
            }
            ensures {
                user_app_flow_online(self);
                user_app_flow_is_owner_unique_online_flow(self);
                user_application_black_box_entered(self);
            }
        }

        Transition::Disable {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Online;
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
predicate task_flow_owner_exclusive<F: TaskFlow>(flow: F) -> bool;
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

object KernelInitFlow: KernelInitFlowType {
    initial_state: State::Online;
    parent: KernelInitTask;

    state State::Online {
    }

    state State::Offline {
        invariant {
            task_owns_flow(KernelInitTask, self);
            task_flow_owner_is(self, KernelInitTask);
            task_flow_no_longer_active(self);
        }
    }

    state State::Destroyed {
        invariant {
            task_owns_flow(KernelInitTask, self);
            task_flow_owner_is(self, KernelInitTask);
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
    }
}

object KthreaddFlow: KthreaddFlowType {
    initial_state: State::Online;
    parent: KthreaddTask;

    state State::Online {
    }

    state State::Offline {
        invariant {
            task_owns_flow(KthreaddTask, self);
            task_flow_owner_is(self, KthreaddTask);
            task_flow_no_longer_active(self);
        }
    }

    state State::Destroyed {
        invariant {
            task_owns_flow(KthreaddTask, self);
            task_flow_owner_is(self, KthreaddTask);
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
            }
            ensures {
                boot_idle_entry_prepared(self, BootTask);
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
    initial_state: State::Base;
    parent: BootTask;

    /*
     * Base 表示 boot idle 任务已存在，但尚未进入 cpu_startup_entry() 运行期入口。
     */
    state State::Base {
        transitions {
            /*
             * Setup 在首次不可逆 task switch 前建立 BootTask 的首个 TaskFlow binding。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    BootIdleSetup.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                    KthreaddReadyGate.state == State::Online;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    boot_idle_runtime_ready(BootIdleFlow, BootTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
                    task_owns_flow(BootTask, BootIdleFlow);
                    task_flow_owner_is(BootIdleFlow, BootTask);
                    task_flow_owner_exclusive(BootIdleFlow);
                    task_active_flow_is(BootTask, BootIdleFlow);
                    secondary_cpus_not_started(CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示 BootTask 的首个 TaskFlow binding 已预先建立；idle entry 尚未执行。
     */
    state State::Ready {
        invariant {
            boot_idle_runtime_ready(BootIdleFlow, BootTask);
            boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
            task_owns_flow(BootTask, BootIdleFlow);
            task_flow_owner_is(BootIdleFlow, BootTask);
            task_flow_owner_exclusive(BootIdleFlow);
            task_active_flow_is(BootTask, BootIdleFlow);
            secondary_cpus_not_started(CpuGroup);
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
