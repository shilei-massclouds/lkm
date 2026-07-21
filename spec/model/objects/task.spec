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
    ext_state: TaskRuntimeState;
    cpu_ref: CpuRef;

    owned {
        thread_context: TaskThreadContext;
    }

    lifecycle {
        Transition::Preset(
            parent_task: Task,
            task_ref: TaskRef,
            initial_flow: UserAppFlow
        ) {
            state_effect: StateEffect::Always;
            depends_on {
                parent_task.state == State::Online;
                initial_flow.state == State::Base;
            }
            ensures {
                user_child_process_prepared(self);
                task_clone_args_ready(self);
                user_task_instance_fresh(self);
                user_task_pid_and_lifecycle_independent(self);
                task_ref_targets(task_ref, self);
                task_ref_ready(task_ref);
                task_owns_flow(self, initial_flow);
                task_flow_owner_is(initial_flow, self);
                task_flow_owner_exclusive(initial_flow);
            }
        }

        Transition::Setup(
            parent_task: Task,
            initial_flow: UserAppFlow
        ) {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Prepared;
                parent_task.state == State::Online;
                initial_flow.state == State::Ready;
                task_creation_copy_process_committed(
                    TaskCreationCore,
                    parent_task,
                    self
                );
            }
            ensures {
                user_child_process_parent_pid1_or_current_child(self, parent_task);
                user_child_process_pid_allocated(self, RootPidNamespace);
                user_child_process_tgid_equals_pid(self);
                user_child_process_process_group_visible_to_parent(self, parent_task);
                user_child_process_exit_signal_sigchld(self);
                user_child_process_files_struct_copied(self, FilesStruct);
                user_child_process_fs_struct_copied(self, FsStruct);
                user_child_process_parent_fd_snapshot_saved(self, FilesStruct);
                user_child_process_credentials_copied(self, parent_task);
                user_child_process_signal_state_copied(self, parent_task);
                user_child_process_user_address_space_snapshot(self, UserAddressSpace);
                user_child_process_user_stack_snapshot_copied(self, UserAddressSpace);
                user_child_process_trap_frame_copied(self, UserTrapFrame);
                user_child_process_trap_frame_child_return_zero(self);
                user_child_process_tls_inherited(self);
                user_child_process_enqueued(self, Scheduler);
                task_has_no_prior_active_flow(self);
                task_all_prior_owned_flows_destroyed(self);
            }
        }

        Transition::Enable(initial_flow: UserAppFlow) {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Ready;
                initial_flow.state == State::Ready;
                task_active_flow_is(self, initial_flow);
                task_flow_active_binding_committed(initial_flow);
            }
            ensures {
                user_task_instance_fresh(self);
                user_task_pid_and_lifecycle_independent(self);
                task_owns_flow(self, initial_flow);
                task_flow_owner_is(initial_flow, self);
                task_active_flow_is(self, initial_flow);
            }
        }

        Transition::Disable {
            state_effect: StateEffect::Always;
            depends_on {
                task_all_owned_flows_inactive(self);
                task_no_owned_flow_online(self);
            }
            ensures {
                task_exit_flow_disable_cleanup_ordered(self);
                task_no_owned_flow_online(self);
            }
        }

        Transition::Cleanup {
            state_effect: StateEffect::Always;
            depends_on {
                self.state == State::Offline;
                task_all_owned_flows_destroyed(self);
                task_no_owned_flow_online(self);
            }
            ensures {
                task_destroyed_only_after_flow_cleanup(self);
            }
        }
    }

    processes {
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
        }

        Action::ActivateInitialFlow(flow: TaskFlow) {
            state_effect: StateEffect::None;
            depends_on {
                task_owns_flow(self, flow);
                task_flow_owner_is(flow, self);
                flow.state == State::Ready;
                task_has_no_prior_active_flow(self);
            }
            ensures {
                task_active_flow_is(self, flow);
                task_at_most_one_flow_online(self);
                task_flow_handoff_new_active(self, flow);
                task_flow_active_binding_committed(flow);
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
 * Static init_task carrier. It remains the same Task when sched_init assigns
 * the boot-idle role; RootStream and BootIdleFlow distinguish its behavior.
 */
object BootTask: Task {
    initial_state: State::Base;
    source: static::linux_6_12;

    attrs {
        storage: ObjectStorage<BootTask>;
    }

    reference linux_6_12 {
        storage = symbol("init_task");
    }

    /*
     * Base 表示根任务对象尚未绑定到当前执行 hart 的任务指针。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立物理地址阶段的根任务指针。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.tp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_object_storage(storage);
                    valid_task_storage(storage);
                    Riscv64.tp == phys_addr(BootTask.storage);
                    valid_task_ref(Riscv64.tp);
                    task_preemption_control_ready(BootTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootTask);
                    task_preemption_disabled(BootTask);
                    task_ref_targets(BootTaskRef, BootTask);
                    task_ref_ready(BootTaskRef);
                    task_owns_flow(BootTask, RootStream);
                    task_flow_owner_is(RootStream, BootTask);
                    task_flow_owner_exclusive(RootStream);
                }
            }
        }
    }

    /*
     * Prepared 表示 tp 已经指向 init_task 的物理地址，可支撑物理地址阶段继续执行。
     * init_task.thread_info.preempt_count 仍保持 INIT_PREEMPT_COUNT，使调度器运行前
     * 内核抢占关闭。
     */
    state State::Prepared {
        invariant {
            attrs_accessible(self);
            valid_object_storage(storage);
            valid_task_storage(storage);
            Riscv64.tp == phys_addr(BootTask.storage);
            valid_task_ref(Riscv64.tp);
            task_preemption_control_ready(BootTask);
            task_preempt_count_initialized_to_init_preempt_count(BootTask);
            task_preemption_disabled(BootTask);
            task_ref_targets(BootTaskRef, BootTask);
            task_ref_ready(BootTaskRef);
            task_owns_flow(BootTask, RootStream);
            task_flow_owner_is(RootStream, BootTask);
            task_flow_owner_exclusive(RootStream);
        }

        transitions {
            /*
             * Enable 在早期虚拟地址空间可用后，将根任务指针切换为虚拟地址。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                may_change {
                    Riscv64.tp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_object_storage(storage);
                    valid_task_storage(storage);
                    Riscv64.tp == virt_addr(BootTask.storage, EarlyVm, KernelImageMap);
                    valid_task_ref(Riscv64.tp);
                    task_preemption_control_ready(BootTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootTask);
                    task_preemption_disabled(BootTask);
                    task_ref_targets(BootTaskRef, BootTask);
                    task_ref_ready(BootTaskRef);
                    task_owns_flow(BootTask, RootStream);
                    task_flow_owner_is(RootStream, BootTask);
                    task_flow_owner_exclusive(RootStream);
                }
            }
        }
    }

    /*
     * Online 表示根任务指针已经使用 EarlyVm 中的内核映像虚拟区域地址，且调度器运行前的
     * 初始抢占关闭状态仍被保留。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_object_storage(storage);
            valid_task_storage(storage);
            Riscv64.tp == virt_addr(BootTask.storage, EarlyVm, KernelImageMap);
            valid_task_ref(Riscv64.tp);
            task_preemption_control_ready(BootTask);
            task_preempt_count_initialized_to_init_preempt_count(BootTask);
            task_preemption_disabled(BootTask);
            task_ref_targets(BootTaskRef, BootTask);
            task_ref_ready(BootTaskRef);
            task_owns_flow(BootTask, RootStream);
            task_flow_owner_is(RootStream, BootTask);
            task_flow_owner_exclusive(RootStream);
        }
    }
}

/*
 * PID 1 carrier created by copy_process. KernelInitFlow is its initial flow;
 * exec later hands the same carrier to a declared UserAppFlow instance.
 */
object KernelInitTask: Task {
    initial_state: State::Base;

    /*
     * Base 表示 PID 1 的创建规格尚未建立。
     */
    state State::Base {
        transitions {
            /*
             * Preset 选择 kernel_init 入口并记录 CLONE_FS 创建约束。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    BootTask.state == State::Online;
                }

                ensures {
                    kernel_init_spawn_spec_ready(KernelInitTask);
                    kernel_init_entry_selected(KernelInitTask);
                    task_clone_args_ready(KernelInitTask);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_owner_is(KernelInitFlow, KernelInitTask);
                    task_flow_owner_exclusive(KernelInitFlow);
                    kernel_init_clone_fs_flag_set(KernelInitTask);
                    kernel_init_not_user_mm_yet(KernelInitTask);
                }
            }
        }
    }

    /*
     * Prepared 表示 kernel_init 的创建规格已准备好，但任务实体尚未进入调度体系。
     */
    state State::Prepared {
        invariant {
            kernel_init_spawn_spec_ready(KernelInitTask);
            kernel_init_entry_selected(KernelInitTask);
            task_clone_args_ready(KernelInitTask);
            task_owns_flow(KernelInitTask, KernelInitFlow);
            task_flow_owner_is(KernelInitFlow, KernelInitTask);
            task_flow_owner_exclusive(KernelInitFlow);
            kernel_init_clone_fs_flag_set(KernelInitTask);
        }

        transitions {
            /*
             * Setup 对应 user_mode_thread(kernel_init, NULL, CLONE_FS) 创建 PID 1。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                drives {
                    TaskCreationCore.Action::CopyProcess(
                        src_task: BootTask,
                        dst_task: KernelInitTask,
                        pid_ns: RootPidNamespace,
                        creds: CredentialCore,
                        signal: SignalCore,
                        files: TaskFileContext,
                        security: SecurityCore,
                        scheduler: Scheduler,
                        flow: KernelInitFlow
                    );
                }

                ensures {
                    task_creation_bound_flow(TaskCreationCore, KernelInitTask, KernelInitFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_owner_is(KernelInitFlow, KernelInitTask);
                    task_flow_owner_exclusive(KernelInitFlow);
                    kernel_init_task_ready(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_thread_context_ready(KernelInitTask);
                    kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
                    task_ref_targets(KernelInitTaskRef, KernelInitTask);
                    task_ref_ready(KernelInitTaskRef);
                    task_state_new(KernelInitTask);
                    task_not_enqueued(KernelInitTask);
                    kernel_init_waits_for_kthreadd_done(KernelInitTask);
                }
            }
        }
    }

    /*
     * Ready 表示 PID 1 任务实体已建立，并等待 kthreadd_done completion。
     */
    state State::Ready {
        invariant {
            kernel_init_task_ready(KernelInitTask);
            kernel_init_task_pid_is_one(KernelInitTask);
            task_owns_flow(KernelInitTask, KernelInitFlow);
            task_flow_owner_is(KernelInitFlow, KernelInitTask);
            task_flow_owner_exclusive(KernelInitFlow);
            kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
            task_ref_targets(KernelInitTaskRef, KernelInitTask);
            task_ref_ready(KernelInitTaskRef);
            task_state_new(KernelInitTask);
            task_not_enqueued(KernelInitTask);
            kernel_init_waits_for_kthreadd_done(KernelInitTask);
        }

        transitions {
            /*
             * Enable 把 kernel_init 放入可调度集合，但仍等待 kthreadd_done 释放。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                    BootCurrentCPU.state == State::Online;
                    BootCpuLocalInterrupt.state == State::Ready;
                    BootCpuCurrentTask.state == State::Ready;
                    current_task_slot_current(BootCpuCurrentTask, BootTask);
                    task_preemption_control_ready(BootTask);
                    task_ref_ready(KernelInitTaskRef);
                    runqueue_ref_ready(BootRunQueueRef);
                    task_state_new(KernelInitTask);
                    task_not_enqueued(KernelInitTask);
                }

                within WakeUpNewTaskContext {
                    depends_on {
                        task_ref_ready(KernelInitTaskRef);
                        runqueue_ref_ready(BootRunQueueRef);
                        task_state_new(KernelInitTask);
                        task_not_enqueued(KernelInitTask);
                        current_task_slot_current(BootCpuCurrentTask, BootTask);
                        BootRunQueue.state == State::Ready;
                    }

                    drives {
                        KernelInitTask.Transition::SetRuntimeState(TaskRuntimeState::Running);
                        let selected_rq: RunQueueRef <-
                            Scheduler.Action::SelectRunQueue(KernelInitTaskRef);
                        KernelInitTask.Action::SetTaskCpu(BootCPURef);
                    }

                    within EnqueueSelectedRunQueueContext {
                        depends_on {
                            runqueue_ref_targets(selected_rq, BootRunQueue);
                            runqueue_ref_cpu_is(selected_rq, BootCPURef);
                            task_cpu_ref_is(KernelInitTask, BootCPURef);
                        }

                        drives {
                            selected_rq.Transition::EnqueueTask(KernelInitTaskRef);
                        }

                        ensures {
                            raw_spinlock_irqsave_entered(BootRunQueueLock, BootCurrentCPU);
                            raw_spinlock_irqrestore_exited(BootRunQueueLock, BootCurrentCPU);
                            runqueue_contains_task(BootRunQueue, KernelInitTaskRef);
                        }
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(KernelInitTaskPiLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(KernelInitTaskPiLock, BootCurrentCPU);
                        scheduler_select_runqueue_returns(Scheduler, KernelInitTaskRef, BootRunQueueRef);
                        task_runqueue_selected(Scheduler, KernelInitTaskRef, BootRunQueueRef);
                        task_cpu_ref_is(KernelInitTask, BootCPURef);
                        task_enqueued_on_runqueue(KernelInitTaskRef, BootRunQueueRef);
                        task_wakeup_new_rq_clock_updated(KernelInitTask, BootRunQueue);
                        task_wakeup_new_initial_util_avg_posted(KernelInitTask, BootRunQueue);
                        task_wakeup_new_trace_emitted(KernelInitTask);
                        task_wakeup_new_preempt_check_done(KernelInitTask, BootRunQueue);
                        task_wakeup_new_task_woken_hook_deferred(KernelInitTask);
                    }
                }

                ensures {
                    kernel_init_task_online(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                    task_active_flow_is(KernelInitTask, KernelInitFlow);
                    task_at_most_one_flow_online(KernelInitTask);
                }
            }
        }
    }

    /*
     * Online 表示 PID 1 已成为可调度任务，后续由 completion 释放进入 PreSmpInitPhase。
     */
    state State::Online {
        invariant {
            kernel_init_task_online(KernelInitTask);
            kernel_init_task_pid_is_one(KernelInitTask);
            task_owns_flow(KernelInitTask, KernelInitFlow);
            task_flow_owner_is(KernelInitFlow, KernelInitTask);
            task_flow_owner_exclusive(KernelInitFlow);
            task_active_flow_is(KernelInitTask, KernelInitFlow);
            task_at_most_one_flow_online(KernelInitTask);
            kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
            task_state_running(KernelInitTask);
            task_cpu_ref_is(KernelInitTask, BootCPURef);
            task_enqueued_on_runqueue(KernelInitTaskRef, BootRunQueueRef);
        }
    }

    state State::Offline {
        invariant {
            task_all_owned_flows_inactive(self);
            task_no_owned_flow_online(self);
            task_exit_flow_disable_cleanup_ordered(self);
        }
    }

    state State::Destroyed {
        invariant {
            task_all_owned_flows_destroyed(self);
            task_no_owned_flow_online(self);
            task_destroyed_only_after_flow_cleanup(self);
        }
    }

    actions {
        /*
         * PinToBootCpu 设置 PF_NO_SETAFFINITY 并把 PID 1 临时固定到 boot CPU。
         * 源码通过 RCU 读侧保护下的 pid lookup 取回 task 指针；规格层不把
         * pid lookup 提升为正式 drives，因为 receiver 已经是 KernelInitTask。
         */
        Action::PinToBootCpu(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                cpu_ref_ready(cpu_ref);
                RootPidNamespace.state == State::Ready;
                BootIdleRcuReadSide.state == State::Prepared;
            }

            within KernelInitPidLookupRcuReadSideContext {
                ensures {
                    rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                    rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                    task_pid_lookup_under_rcu_read(
                        KernelInitTask,
                        RootPidNamespace,
                        BootIdleRcuReadSide
                    );
                    task_pid_lookup_rcu_guard_used(KernelInitTask, BootIdleRcuReadSide);
                    task_flag_no_setaffinity(KernelInitTask);
                    task_cpumask_is(KernelInitTask, cpu_ref);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                }
            }

            ensures {
                rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                task_pid_lookup_under_rcu_read(
                    KernelInitTask,
                    RootPidNamespace,
                    BootIdleRcuReadSide
                );
                task_pid_lookup_rcu_guard_used(KernelInitTask, BootIdleRcuReadSide);
                task_flag_no_setaffinity(KernelInitTask);
                task_cpumask_is(KernelInitTask, cpu_ref);
                kernel_init_pf_no_setaffinity(KernelInitTask);
                kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
            }
        }

    }
}

/*
 * kthreadd task_struct carrier. Provider-reference publication stays on the
 * carrier; the service loop belongs to KthreaddFlow.
 */
object KthreaddTask: Task {
    initial_state: State::Base;

    /*
     * Base 表示 kthreadd 的创建规格尚未建立。
     */
    state State::Base {
        transitions {
            /*
             * Preset 选择 kthreadd 入口并记录 kernel_thread 创建约束。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    BootTask.state == State::Online;
                }

                ensures {
                    kthreadd_spawn_spec_ready(KthreaddTask);
                    kthreadd_entry_selected(KthreaddTask);
                    task_clone_args_ready(KthreaddTask);
                    task_owns_flow(KthreaddTask, KthreaddFlow);
                    task_flow_owner_is(KthreaddFlow, KthreaddTask);
                    task_flow_owner_exclusive(KthreaddFlow);
                    kthreadd_clone_fs_files_flags_set(KthreaddTask);
                    kthreadd_clone_vm_flag_set(KthreaddTask);
                    kthreadd_clone_untraced_flag_set(KthreaddTask);
                    kthreadd_kernel_thread_flag_set(KthreaddTask);
                    kthreadd_is_kernel_thread_provider(KthreaddTask);
                }
            }
        }
    }

    /*
     * Prepared 表示 kthreadd 创建规格已准备好，但全局管理线程尚未建立。
     */
    state State::Prepared {
        invariant {
            kthreadd_spawn_spec_ready(KthreaddTask);
            kthreadd_entry_selected(KthreaddTask);
            task_clone_args_ready(KthreaddTask);
            task_owns_flow(KthreaddTask, KthreaddFlow);
            task_flow_owner_is(KthreaddFlow, KthreaddTask);
            task_flow_owner_exclusive(KthreaddFlow);
            kthreadd_clone_fs_files_flags_set(KthreaddTask);
            kthreadd_clone_vm_flag_set(KthreaddTask);
            kthreadd_clone_untraced_flag_set(KthreaddTask);
            kthreadd_kernel_thread_flag_set(KthreaddTask);
            kthreadd_is_kernel_thread_provider(KthreaddTask);
        }

        transitions {
            /*
             * Setup 对应 kernel_thread(kthreadd, NULL, CLONE_FS | CLONE_FILES)。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                drives {
                    TaskCreationCore.Action::CopyProcess(
                        src_task: BootTask,
                        dst_task: KthreaddTask,
                        pid_ns: RootPidNamespace,
                        creds: CredentialCore,
                        signal: SignalCore,
                        files: TaskFileContext,
                        security: SecurityCore,
                        scheduler: Scheduler,
                        flow: KthreaddFlow
                    );
                }

                ensures {
                    task_creation_bound_flow(TaskCreationCore, KthreaddTask, KthreaddFlow);
                    task_owns_flow(KthreaddTask, KthreaddFlow);
                    task_flow_owner_is(KthreaddFlow, KthreaddTask);
                    task_flow_owner_exclusive(KthreaddFlow);
                    kthreadd_task_ready(KthreaddTask);
                    kthreadd_task_pid_allocated(KthreaddTask, RootPidNamespace);
                    kthreadd_thread_context_ready(KthreaddTask);
                    kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
                    task_ref_targets(KthreaddTaskRef, KthreaddTask);
                    task_ref_ready(KthreaddTaskRef);
                    task_state_new(KthreaddTask);
                    task_not_enqueued(KthreaddTask);
                }
            }
        }
    }

    /*
     * Ready 表示 kthreadd 任务实体已建立，并绑定为全局内核线程管理者。
     */
    state State::Ready {
        invariant {
            kthreadd_task_ready(KthreaddTask);
            task_owns_flow(KthreaddTask, KthreaddFlow);
            task_flow_owner_is(KthreaddFlow, KthreaddTask);
            task_flow_owner_exclusive(KthreaddFlow);
            kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
            task_ref_targets(KthreaddTaskRef, KthreaddTask);
            task_ref_ready(KthreaddTaskRef);
            task_state_new(KthreaddTask);
            task_not_enqueued(KthreaddTask);
        }

        transitions {
            /*
             * Enable 把 kthreadd 放入可调度集合。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                    BootCurrentCPU.state == State::Online;
                    BootCpuLocalInterrupt.state == State::Ready;
                    BootCpuCurrentTask.state == State::Ready;
                    current_task_slot_current(BootCpuCurrentTask, BootTask);
                    task_preemption_control_ready(BootTask);
                    task_ref_ready(KthreaddTaskRef);
                    runqueue_ref_ready(BootRunQueueRef);
                    task_state_new(KthreaddTask);
                    task_not_enqueued(KthreaddTask);
                }

                within WakeUpKthreaddTaskContext {
                    depends_on {
                        task_ref_ready(KthreaddTaskRef);
                        runqueue_ref_ready(BootRunQueueRef);
                        task_state_new(KthreaddTask);
                        task_not_enqueued(KthreaddTask);
                        current_task_slot_current(BootCpuCurrentTask, BootTask);
                        BootRunQueue.state == State::Ready;
                    }

                    drives {
                        KthreaddTask.Transition::SetRuntimeState(TaskRuntimeState::Running);
                        let selected_rq: RunQueueRef <-
                            Scheduler.Action::SelectRunQueue(KthreaddTaskRef);
                        KthreaddTask.Action::SetTaskCpu(BootCPURef);
                    }

                    within EnqueueSelectedRunQueueContext {
                        depends_on {
                            runqueue_ref_targets(selected_rq, BootRunQueue);
                            runqueue_ref_cpu_is(selected_rq, BootCPURef);
                            task_cpu_ref_is(KthreaddTask, BootCPURef);
                        }

                        drives {
                            selected_rq.Transition::EnqueueTask(KthreaddTaskRef);
                        }

                        ensures {
                            raw_spinlock_irqsave_entered(BootRunQueueLock, BootCurrentCPU);
                            raw_spinlock_irqrestore_exited(BootRunQueueLock, BootCurrentCPU);
                            runqueue_contains_task(BootRunQueue, KthreaddTaskRef);
                        }
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(KthreaddTaskPiLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(KthreaddTaskPiLock, BootCurrentCPU);
                        scheduler_select_runqueue_returns(Scheduler, KthreaddTaskRef, BootRunQueueRef);
                        task_runqueue_selected(Scheduler, KthreaddTaskRef, BootRunQueueRef);
                        task_cpu_ref_is(KthreaddTask, BootCPURef);
                        task_enqueued_on_runqueue(KthreaddTaskRef, BootRunQueueRef);
                        task_wakeup_new_rq_clock_updated(KthreaddTask, BootRunQueue);
                        task_wakeup_new_initial_util_avg_posted(KthreaddTask, BootRunQueue);
                        task_wakeup_new_trace_emitted(KthreaddTask);
                        task_wakeup_new_preempt_check_done(KthreaddTask, BootRunQueue);
                        task_wakeup_new_task_woken_hook_deferred(KthreaddTask);
                    }
                }

                ensures {
                    kthreadd_task_online(KthreaddTask);
                    kthreadd_task_enqueued(KthreaddTask, BootRunQueue);
                    task_active_flow_is(KthreaddTask, KthreaddFlow);
                    task_at_most_one_flow_online(KthreaddTask);
                }
            }
        }
    }

    /*
     * Online 表示 kthreadd 已可调度，并可作为后续 kthread 服务提供者。
     */
    state State::Online {
        invariant {
            kthreadd_task_online(KthreaddTask);
            task_owns_flow(KthreaddTask, KthreaddFlow);
            task_flow_owner_is(KthreaddFlow, KthreaddTask);
            task_flow_owner_exclusive(KthreaddFlow);
            task_active_flow_is(KthreaddTask, KthreaddFlow);
            task_at_most_one_flow_online(KthreaddTask);
            kthreadd_task_enqueued(KthreaddTask, BootRunQueue);
            task_state_running(KthreaddTask);
            task_cpu_ref_is(KthreaddTask, BootCPURef);
            task_enqueued_on_runqueue(KthreaddTaskRef, BootRunQueueRef);
        }
    }

    actions {
        /*
         * BindGlobalRef 对应 rest_init() 中 kernel_thread() 返回后的
         * kthreadd_task = find_task_by_pid_ns(pid, &init_pid_ns)。规格层已经
         * 持有 KthreaddTask receiver 和 KthreaddTaskRef；源码 pid lookup 只
         * 是实现路径，不作为获取规格 task 引用的必要步骤。
         */
        Action::BindGlobalRef {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(KthreaddTaskRef);
                task_state_running(KthreaddTask);
                task_enqueued_on_runqueue(KthreaddTaskRef, BootRunQueueRef);
                RootPidNamespace.state == State::Ready;
                BootIdleRcuReadSide.state == State::Prepared;
            }

            within KthreaddPidLookupRcuReadSideContext {
                ensures {
                    rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                    rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                    task_pid_lookup_under_rcu_read(
                        KthreaddTask,
                        RootPidNamespace,
                        BootIdleRcuReadSide
                    );
                    task_pid_lookup_rcu_guard_used(KthreaddTask, BootIdleRcuReadSide);
                    kthreadd_global_ref_bound(KthreaddTask);
                    kthreadd_provider_ref_targets(KthreaddTaskRef, KthreaddTask);
                    kthreadd_provider_ready(KthreaddTask);
                }
            }

            ensures {
                rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                task_pid_lookup_under_rcu_read(
                    KthreaddTask,
                    RootPidNamespace,
                    BootIdleRcuReadSide
                );
                task_pid_lookup_rcu_guard_used(KthreaddTask, BootIdleRcuReadSide);
                kthreadd_global_ref_bound(KthreaddTask);
                kthreadd_provider_ref_targets(KthreaddTaskRef, KthreaddTask);
                kthreadd_provider_ready(KthreaddTask);
            }
        }

    }
}

/*
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
