/*
 * Process Prepare Phase Specification
 *
 * This is the fourth interrupt-oriented leaf directly driven by
 * BootInitFlow.Setup. It starts after IrqOpenPreparePhase has
 * prepared the interrupt-open late core/platform boundary and covers the Linux
 * start_kernel() segment from pid_idr_init() through kcsan_init(), stopping
 * before rest_init() creates PID 1 and kthreadd.
 */

/*
 * RootPidNamespace 表示 pid_idr_init() 后 init_pid_ns 的 PID 分配基础。
 */
object RootPidNamespace: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    root_pid_namespace_ready(RootPidNamespace);
                    init_pid_ns_idr_ready(RootPidNamespace);
                    pid_cache_level0_ready(RootPidNamespace, SlubSubsystem);
                    pid_allocator_limits_configured(RootPidNamespace, CpuGroup);
                    pid_max_limit_compiletime_checked();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            root_pid_namespace_ready(RootPidNamespace);
            init_pid_ns_idr_ready(RootPidNamespace);
            pid_cache_level0_ready(RootPidNamespace, SlubSubsystem);
            pid_allocator_limits_configured(RootPidNamespace, CpuGroup);
        }
    }
}

/*
 * AnonVmaCore 表示 anon_vma_init() 建立的匿名内存 rmap graph 分配基础。
 */
object AnonVmaCore: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    anon_vma_core_ready(AnonVmaCore);
                    anon_vma_cache_ready(AnonVmaCore, SlubSubsystem);
                    anon_vma_chain_cache_ready(AnonVmaCore, SlubSubsystem);
                    anon_vma_runtime_graph_deferred(AnonVmaCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            anon_vma_core_ready(AnonVmaCore);
            anon_vma_cache_ready(AnonVmaCore, SlubSubsystem);
            anon_vma_chain_cache_ready(AnonVmaCore, SlubSubsystem);
            anon_vma_runtime_graph_deferred(AnonVmaCore);
        }
    }
}

/*
 * CredentialCore 表示 cred_init() 后 cred cache 的准备边界。
 */
object CredentialCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    credential_core_prepared(CredentialCore);
                    cred_cache_ready(CredentialCore, SlubSubsystem);
                    init_cred_static_root_not_created_here(CredentialCore);
                    credential_runtime_relations_deferred(CredentialCore);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            credential_core_prepared(CredentialCore);
            cred_cache_ready(CredentialCore, SlubSubsystem);
            credential_runtime_relations_deferred(CredentialCore);
        }
    }
}

/*
 * VectorContext 表示 RISC-V arch_task_cache_init()/riscv_v_setup_ctx_cache()
 * 后的 vector context cache 准备边界。
 */
object VectorContext: HardwareObject {
    initial_state: State::Base;
    parent: TaskCreationCore;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    CpuCapabilities.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                }

                ensures {
                    vector_context_prepared(VectorContext, CpuCapabilities);
                    user_vector_context_cache_ready_if_supported(VectorContext, CpuCapabilities);
                    kernel_vector_context_cache_ready_if_preemptive(VectorContext, CpuCapabilities);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            vector_context_prepared(VectorContext, CpuCapabilities);
            user_vector_context_cache_ready_if_supported(VectorContext, CpuCapabilities);
            kernel_vector_context_cache_ready_if_preemptive(VectorContext, CpuCapabilities);
        }
    }
}

/*
 * UprobeCore 表示 uprobes_init() 后 uprobes 调试/插桩基础进入 Ready。
 */
object UprobeCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CurrentCPU.trap.exception.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                }

                ensures {
                    uprobe_core_ready(UprobeCore);
                    uprobes_hash_mutex_ready(UprobeCore);
                    uprobes_die_notifier_registered(UprobeCore, CurrentCPU.trap.exception);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uprobe_core_ready(UprobeCore);
            uprobes_hash_mutex_ready(UprobeCore);
            uprobes_die_notifier_registered(UprobeCore, CurrentCPU.trap.exception);
        }
    }
}

/*
 * TaskCreationCore 表示 thread_stack_cache_init() 与 fork_init() 后的 task
 * 创建基础。它不创建 PID 1/kthreadd；这些属于 rest_init()。它提供
 * copy_process() 的通用创建契约：rest_init() 传入的 flow 是新任务的
 * 初始 TaskFlow，并由其具体类型决定任务之后进入 kernel_init 线还是
 * kthreadd 入口循环。
 */
object TaskCreationCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    task_creation_core_prepared(TaskCreationCore);
                    thread_stack_cache_ready(TaskCreationCore, SlubSubsystem);
                    vmap_stack_path_selected(TaskCreationCore);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            task_creation_core_prepared(TaskCreationCore);
            thread_stack_cache_ready(TaskCreationCore, SlubSubsystem);
            vmap_stack_path_selected(TaskCreationCore);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    CpuGroup.state == State::Ready;
                    CpuCapabilities.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    BootTask.state == State::OnCpu;
                }

                drives {
                    VectorContext.Transition::Preset;
                    UprobeCore.Transition::Setup;
                }

                ensures {
                    task_creation_core_ready(TaskCreationCore);
                    task_struct_cache_ready(TaskCreationCore, SlubSubsystem);
                    max_threads_configured(TaskCreationCore, CpuGroup);
                    init_task_rlimits_ready(TaskCreationCore, BootTask);
                    init_user_namespace_ucounts_ready(TaskCreationCore);
                    fork_vm_stack_cpuhp_registered(TaskCreationCore);
                    task_creation_flow_contract_ready(TaskCreationCore);
                    rest_init_task_creation_inputs_ready(TaskCreationCore, RootPidNamespace, CredentialCore);
                    task_creation_copy_process_sighand_siglock_deferred(TaskCreationCore);
                    task_creation_copy_process_tasklist_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_pidmap_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_sched_fork_locks_deferred(TaskCreationCore);
                    task_creation_copy_process_reference_sync_deferred(TaskCreationCore);
                    task_creation_copy_process_failure_rollback_deferred(TaskCreationCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            VectorContext.state == State::Prepared;
            UprobeCore.state == State::Ready;
            task_creation_core_ready(TaskCreationCore);
            task_struct_cache_ready(TaskCreationCore, SlubSubsystem);
            max_threads_configured(TaskCreationCore, CpuGroup);
            init_task_rlimits_ready(TaskCreationCore, BootTask);
            init_user_namespace_ucounts_ready(TaskCreationCore);
            fork_vm_stack_cpuhp_registered(TaskCreationCore);
            task_creation_flow_contract_ready(TaskCreationCore);
            rest_init_task_creation_inputs_ready(TaskCreationCore, RootPidNamespace, CredentialCore);
            task_creation_copy_process_sighand_siglock_deferred(TaskCreationCore);
            task_creation_copy_process_tasklist_lock_deferred(TaskCreationCore);
            task_creation_copy_process_pidmap_lock_deferred(TaskCreationCore);
            task_creation_copy_process_sched_fork_locks_deferred(TaskCreationCore);
            task_creation_copy_process_reference_sync_deferred(TaskCreationCore);
            task_creation_copy_process_failure_rollback_deferred(TaskCreationCore);
        }

        deferred task_creation.001 {
            category: DeferredCategory::Protocol;
            summary: "Implement sighand siglock ordering inside copy_process.";
            evidence { task_creation_copy_process_sighand_siglock_deferred(TaskCreationCore); }
            close_when: "Sighand sharing/copying and signal-lock ordering tests pass for clone and fork.";
        }
        deferred task_creation.002 {
            category: DeferredCategory::Protocol;
            summary: "Implement tasklist lock ordering inside task creation.";
            evidence { task_creation_copy_process_tasklist_lock_deferred(TaskCreationCore); }
            close_when: "Task publication, lookup and concurrent exit/fork lock-order tests pass.";
        }
        deferred task_creation.003 {
            category: DeferredCategory::Protocol;
            summary: "Implement PID allocator and pidmap synchronization.";
            evidence { task_creation_copy_process_pidmap_lock_deferred(TaskCreationCore); }
            close_when: "Concurrent PID allocation, wraparound and release tests pass.";
        }
        deferred task_creation.004 {
            category: DeferredCategory::Protocol;
            summary: "Implement sched_fork and priority-inheritance initialization locking.";
            evidence { task_creation_copy_process_sched_fork_locks_deferred(TaskCreationCore); }
            close_when: "Scheduler fork initialization, PI state and publication ordering tests pass.";
        }
        deferred task_creation.005 {
            category: DeferredCategory::Protocol;
            summary: "Implement reference synchronization for copy_creds, files, fs, signal and mm.";
            evidence { task_creation_copy_process_reference_sync_deferred(TaskCreationCore); }
            close_when: "Reference acquisition, sharing, copy and concurrent-release tests pass for every clone mode.";
        }
        deferred task_creation.006 {
            category: DeferredCategory::Protocol;
            summary: "Implement complete copy_process failure rollback.";
            evidence { task_creation_copy_process_failure_rollback_deferred(TaskCreationCore); }
            close_when: "Every allocation and hook failure unwinds resources, counters and publication in reverse order under tests.";
        }

        actions {
            /*
             * CopyProcess 对应 kernel_clone()/copy_process() 的成功路径。
             * flow 是调用者传入的执行流程，不是创建后的普通属性补丁：
             * TaskCreationCore 必须把它绑定到新 task 的初始 thread context，
             * 后续任务第一次被调度时就从该 TaskFlow 的 continuation 开始。
             */
            Action::CopyProcess<Src: Task, New: Task>(
                src_task: Src,
                src_task_ref: TaskRef,
                current_task_ref: TaskRef,
                dst_task: New,
                pid_ns: RootPidNamespace,
                creds: CredentialCore,
                signal: SignalCore,
                files: TaskFileContext,
                security: SecurityCore,
                scheduler: Scheduler,
                flow: TaskFlow
            ) {
                state_effect: StateEffect::None;
                depends_on {
                    src_task.state == State::OnCpu;
                    task_execution_authority_is(
                        src_task,
                        TaskExecutionAuthority::Live
                    );
                    task_ref_ready(src_task_ref);
                    task_ref_targets(src_task_ref, src_task);
                    task_ref_ready(current_task_ref);
                    task_ref_targets(current_task_ref, src_task);
                    dst_task.state == State::Prepared;
                    pid_ns.state == State::Ready;
                    creds.state == State::Prepared;
                    signal.state == State::Prepared;
                    files.state == State::Prepared;
                    security.state == State::Ready;
                    scheduler.state == State::Online;
                    task_creation_flow_contract_ready(TaskCreationCore);
                    task_clone_args_ready(dst_task);
                    task_fixed_flow_is(dst_task, flow);
                }

                ensures {
                    current_task_ref_derived_from_selector(current_task_ref, src_task);
                    task_creation_copy_process_committed(TaskCreationCore, src_task, dst_task);
                    task_creation_copy_process_used_current_source(
                        TaskCreationCore,
                        src_task,
                        src_task_ref
                    );
                    task_creation_used_clone_args(TaskCreationCore, dst_task);
                    task_creation_bound_flow(TaskCreationCore, dst_task, flow);
                    task_creation_copy_process_sighand_siglock_deferred(TaskCreationCore);
                    task_creation_copy_process_tasklist_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_pidmap_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_sched_fork_locks_deferred(TaskCreationCore);
                    task_struct_allocated(dst_task);
                    task_duplicated_from(dst_task, src_task);
                    task_pid_allocated(dst_task, pid_ns);
                    task_creds_copied(dst_task, creds);
                    task_file_context_copied_or_shared(dst_task, files);
                    task_signal_context_ready(dst_task, signal);
                    task_security_context_allocated(dst_task, security);
                    task_thread_context_ready(dst_task);
                    task_sched_entity_initialized(dst_task, scheduler);
                    task_state_new(dst_task);
                    task_not_enqueued(dst_task);
                }

            }

            /*
             * CopyUserProcess is the Task/TaskRef-parameterized user fork/clone
             * creation boundary. Each invocation receives a fresh destination
             * Task with an independent PID and lifecycle and completes the
             * caller's atomic fork snapshot for a fresh ordinary TaskFlow.
             */
            Action::CopyUserProcess(
                src_process: Task,
                src_ref: TaskRef,
                dst_process: Task,
                dst_ref: TaskRef,
                flow: TaskFlow,
                flow_ref: TaskFlowRef,
                runtime: UserAppRuntime,
                application: ApplicationInstance,
                pid_ns: RootPidNamespace,
                scheduler: Scheduler,
                fs: FsStruct,
                files: FilesStruct,
                address_space: UserAddressSpace,
                trap_frame: UserTrapFrame,
                boundaries: UserCloneDeferredBoundaries
            ) {
                state_effect: StateEffect::None;
                depends_on {
                    src_process.state == State::OnCpu;
                    task_execution_authority_is(
                        src_process,
                        TaskExecutionAuthority::Live
                    );
                    dst_process.state == State::Online;
                    task_ref_targets(src_ref, src_process);
                    task_ref_targets(dst_ref, dst_process);
                    pid_ns.state == State::Ready;
                    scheduler.state == State::Online;
                    fs.state == State::Ready;
                    files.state == State::Ready;
                    address_space.state == State::Online;
                    trap_frame.state == State::Ready;
                    boundaries.state == State::Ready;
                    flow.state == State::Online;
                    runtime.state == State::Online;
                    task_fixed_flow_is(dst_process, flow);
                    task_flow_owner_is(flow, dst_process);
                    task_flow_parent_is(flow, dst_process);
                    task_flow_owner_exclusive(flow);
                    task_flow_ref_targets(flow_ref, flow);
                    user_app_runtime_owned_by_flow(runtime, flow);
                    user_app_runtime_unique_for_flow(runtime, flow);
                    application_instance_owned_by_runtime(application, runtime);
                    user_clone_plain_fork_first_slice_bound(boundaries);
                    user_clone_fresh_task_per_child_bound(boundaries);
                    user_clone_multiple_independent_tasks_bound(boundaries);
                    user_task_set_allows_multiple_independent_tasks(UserTaskSet);
                }

                drives {
                    let runqueue: SchedulerRef <- scheduler.Action::SelectScheduler(dst_ref);
                    flow.Action::AssignCpuRef(BootCPURef);
                    Cpu0Scheduler.Action::EnqueueTask(runqueue, dst_ref);
                }

                ensures {
                    task_creation_copy_process_committed(TaskCreationCore, src_process, dst_process);
                    task_creation_used_clone_args(TaskCreationCore, dst_process);
                    task_creation_bound_flow(TaskCreationCore, dst_process, flow);
                    task_struct_allocated(dst_process);
                    task_duplicated_from(dst_process, src_process);
                    task_pid_allocated(dst_process, pid_ns);
                    task_thread_context_ready(dst_process);
                    task_context_coordinate_ready(dst_process.thread_context);
                    task_context_flow_ref_is_fixed(dst_process, flow);
                    task_initial_context_coordinate_bound(dst_process, flow);
                    task_initial_context_complete(dst_process, flow);
                    task_sched_entity_initialized(dst_process, scheduler);
                    task_state_running(dst_process);
                    task_scheduler_publication_committed(dst_process);
                    task_fixed_flow_binding_complete(dst_process);
                    task_fixed_flow_binding_consistent(dst_process);
                    task_online_has_recoverable_context(dst_process);
                    task_online_eligibility_is_scheduler_owned(dst_process);
                    task_online_does_not_imply_dispatched(dst_process);
                    task_execution_authority_is(dst_process, TaskExecutionAuthority::None);
                    task_breakpoint_state_is(dst_process, TaskBreakpointState::Valid);
                    task_breakpoint_bound_to_flow_ref(dst_process, flow);
                    task_breakpoint_flow_ref_generation_valid(dst_process);
                    task_flow_ref_generation_valid(flow);
                    fork_snapshot_post_fork_continuation_ready(dst_process);
                    fork_snapshot_child_has_no_parent_overlay_authority(dst_process);
                    user_child_process_parent_pid1_or_current_child(dst_process, src_process);
                    user_child_process_pid_allocated(dst_process, pid_ns);
                    user_child_process_tgid_equals_pid(dst_process);
                    user_child_process_exit_signal_sigchld(dst_process);
                    user_child_process_files_struct_copied(dst_process, files);
                    user_child_process_fs_struct_copied(dst_process, fs);
                    user_child_process_credentials_copied(dst_process, src_process);
                    user_child_process_signal_state_copied(dst_process, src_process);
                    user_child_process_independent_mm_owned(dst_process, address_space);
                    user_child_process_root_and_satp_distinct(dst_process, src_process);
                    user_child_process_cow_private_pages_shared(dst_process, address_space);
                    user_child_process_dup_mm_failure_atomic(dst_process, src_process);
                    user_child_process_trap_frame_copied(dst_process, trap_frame);
                    user_child_process_trap_frame_child_return_zero(dst_process);
                    user_child_process_tls_inherited(dst_process);
                    user_child_process_enqueued(dst_process, Cpu0Scheduler);
                    task_enqueued_on_scheduler(dst_ref, Cpu0Scheduler);
                    task_fixed_flow_is(dst_process, flow);
                    task_flow_owner_is(flow, dst_process);
                    task_flow_parent_is(flow, dst_process);
                    task_flow_owner_exclusive(flow);
                    user_task_instance_fresh(dst_process);
                    user_task_pid_and_lifecycle_independent(dst_process);
                    user_app_runtime_owned_by_flow(runtime, flow);
                    user_app_runtime_unique_for_flow(runtime, flow);
                    user_app_runtime_not_shared(runtime);
                    user_app_runtime_application_continuation_online(runtime, application);
                    user_app_runtime_passive_lifecycle(runtime);
                    application_instance_owned_by_runtime(application, runtime);
                    task_creation_copy_process_sighand_siglock_deferred(TaskCreationCore);
                    task_creation_copy_process_tasklist_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_pidmap_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_sched_fork_locks_deferred(TaskCreationCore);
                }

            }
        }
    }
}

/*
 * SignalCore 表示 proc_caches_init() 中 signal/sighand cache 的准备边界。
 * signals_init() 暂缓，因此本阶段只推进到 Prepared。
 */
object SignalCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    signal_core_prepared(SignalCore);
                    sighand_cache_ready(SignalCore, SlubSubsystem);
                    signal_struct_cache_ready(SignalCore, SlubSubsystem);
                    sigqueue_cache_deferred(SignalCore);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            signal_core_prepared(SignalCore);
            sighand_cache_ready(SignalCore, SlubSubsystem);
            signal_struct_cache_ready(SignalCore, SlubSubsystem);
            sigqueue_cache_deferred(SignalCore);
        }
    }
}

/*
 * TaskFileContext 表示 proc_caches_init() 中 files_struct/fs_struct cache。
 */
object TaskFileContext: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    task_file_context_prepared(TaskFileContext);
                    files_struct_cache_ready(TaskFileContext, SlubSubsystem);
                    fs_struct_cache_ready(TaskFileContext, SlubSubsystem);
                    vfs_runtime_dependency_deferred(TaskFileContext);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            task_file_context_prepared(TaskFileContext);
            files_struct_cache_ready(TaskFileContext, SlubSubsystem);
            fs_struct_cache_ready(TaskFileContext, SlubSubsystem);
            vfs_runtime_dependency_deferred(TaskFileContext);
        }
    }
}

/*
 * VmaCore 表示 proc_caches_init()/mmap_init() 中 VMA 分配基础准备。
 */
object VmaCore: MemoryObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    MmStructCache.state == State::Ready;
                    AnonVmaCore.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                }

                ensures {
                    vma_core_prepared(VmaCore);
                    vm_area_struct_cache_ready(VmaCore, SlubSubsystem);
                    per_vma_lock_cache_ready(VmaCore, SlubSubsystem);
                    vm_committed_as_counter_ready(VmaCore, PerCpuStorage);
                    vma_runtime_mapping_deferred(VmaCore);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            vma_core_prepared(VmaCore);
            vm_area_struct_cache_ready(VmaCore, SlubSubsystem);
            per_vma_lock_cache_ready(VmaCore, SlubSubsystem);
            vm_committed_as_counter_ready(VmaCore, PerCpuStorage);
            vma_runtime_mapping_deferred(VmaCore);
        }
    }
}

/*
 * NsProxy 表示 task 指向 namespace 实例集合的聚合引用 cache。
 */
object NsProxy: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                }

                ensures {
                    ns_proxy_prepared(NsProxy);
                    nsproxy_cache_ready(NsProxy, SlubSubsystem);
                    namespace_runtime_refs_deferred(NsProxy);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ns_proxy_prepared(NsProxy);
            nsproxy_cache_ready(NsProxy, SlubSubsystem);
            namespace_runtime_refs_deferred(NsProxy);
        }
    }
}

/*
 * UtsNamespace 表示 uts_ns_init() 后 UTS namespace clone/unshare cache。
 */
object UtsNamespace: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    NsProxy.state == State::Prepared;
                    SlubSubsystem.state == State::Ready;
                }

                ensures {
                    uts_namespace_prepared(UtsNamespace);
                    uts_namespace_cache_ready(UtsNamespace, SlubSubsystem);
                    init_uts_namespace_static_root_not_created_here(UtsNamespace);
                    uts_namespace_runtime_ops_deferred(UtsNamespace);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            uts_namespace_prepared(UtsNamespace);
            uts_namespace_cache_ready(UtsNamespace, SlubSubsystem);
            uts_namespace_runtime_ops_deferred(UtsNamespace);
        }
    }
}

/*
 * KeyringCore 表示 key_init() 后 key 分配和内建 key type registry 基础。
 */
object KeyringCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CredentialCore.state == State::Prepared;
                    SlubSubsystem.state == State::Ready;
                }

                ensures {
                    keyring_core_ready(KeyringCore);
                    key_cache_ready(KeyringCore, SlubSubsystem);
                    builtin_key_types_registered(KeyringCore);
                    root_key_user_tracking_ready(KeyringCore);
                    persistent_keyrings_trimmed(KeyringCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            keyring_core_ready(KeyringCore);
            key_cache_ready(KeyringCore, SlubSubsystem);
            builtin_key_types_registered(KeyringCore);
            root_key_user_tracking_ready(KeyringCore);
            persistent_keyrings_trimmed(KeyringCore);
        }
    }
}

/*
 * SecurityCore 表示 security_init() 后 LSM 顺序、blob layout 和 hook
 * dispatcher 的基础装配。
 */
object SecurityCore: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    CredentialCore.state == State::Prepared;
                    KeyringCore.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    StaticBranch.state == State::Ready;
                }

                ensures {
                    security_core_ready(SecurityCore);
                    ordered_lsms_ready(SecurityCore);
                    lsm_blob_layout_ready(SecurityCore);
                    lsm_hook_dispatcher_ready(SecurityCore);
                    capability_lsm_hooks_registered(SecurityCore);
                    optional_lsms_conditionally_deferred(SecurityCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            security_core_ready(SecurityCore);
            ordered_lsms_ready(SecurityCore);
            lsm_blob_layout_ready(SecurityCore);
            lsm_hook_dispatcher_ready(SecurityCore);
            capability_lsm_hooks_registered(SecurityCore);
            optional_lsms_conditionally_deferred(SecurityCore);
        }
    }
}

/*
 * ProcessPrepareTrimmedPaths 保留 start_kernel() 中落在本子阶段、但当前
 * ../linux-6.12/.config 下为空、不可达或暂缓展开的调用点。
 * 这些事实必须结构化记录，不能只留在 checkpoint 或 markdown 表格。
 */
object ProcessPrepareTrimmedPaths: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                    TaskCreationCore.state == State::Ready;
                    VfsCore.state == State::Ready;
                }

                ensures {
                    process_prepare_trimmed_paths_prepared(ProcessPrepareTrimmedPaths);
                    process_prepare_x86_efi_runtime_switch_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_x86_efi_runtime_switch_trimmed_because_arch_riscv(ProcessPrepareTrimmedPaths);
                    process_prepare_shadow_call_stack_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_lockdep_init_task_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_lockdep_init_task_trimmed_because_config_lockdep_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_dbg_late_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_dbg_late_init_trimmed_because_config_kgdb_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_net_namespace_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_net_namespace_deferred_even_if_config_net_ns_enabled(ProcessPrepareTrimmedPaths);
                    process_prepare_pagecache_deferred(ProcessPrepareTrimmedPaths, VfsCore);
                    process_prepare_pagecache_waitqueue_table_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_signal_core_setup_deferred(ProcessPrepareTrimmedPaths, SignalCore);
                    process_prepare_seq_file_core_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_procfs_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_nsfs_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_pidfs_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_vfs_pseudo_filesystems_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_bdev_chrdev_init_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_cpuset_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_cpuset_trimmed_because_config_cpusets_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_cgroup_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_cgroup_trimmed_because_config_cgroups_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_taskstats_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_taskstats_trimmed_because_config_taskstats_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_delayacct_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_delayacct_trimmed_because_config_task_delay_acct_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_acpi_subsystem_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_acpi_trimmed_because_config_acpi_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_arch_post_acpi_subsys_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_kcsan_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_kcsan_trimmed_because_config_kcsan_disabled(ProcessPrepareTrimmedPaths);
                    process_prepare_rcu_tasks_generic_out_of_scope(ProcessPrepareTrimmedPaths);
                    process_prepare_rcu_tasks_generic_belongs_to_kernel_init_freeable(ProcessPrepareTrimmedPaths);
                    process_prepare_trimmed_paths_position_preserved(ProcessPrepareTrimmedPaths);
                    process_prepare_root_pid_runtime_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_vm_stack_hotplug_callbacks_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_key_runtime_sync_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_lsm_runtime_sync_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_pseudo_fs_runtime_locks_deferred(ProcessPrepareTrimmedPaths);
                }

                deferred process_prepare.001 {
                    category: DeferredCategory::Feature;
                    summary: "Model and implement the network namespace runtime object.";
                    evidence { process_prepare_net_namespace_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "Network namespace lifecycle, isolation and enabled-config tests pass.";
                }
                deferred process_prepare.002 {
                    category: DeferredCategory::Feature;
                    summary: "Model page-cache, folio waitqueue and writeback initialization.";
                    evidence {
                        process_prepare_pagecache_deferred(ProcessPrepareTrimmedPaths, VfsCore);
                        process_prepare_pagecache_waitqueue_table_deferred(ProcessPrepareTrimmedPaths);
                    }
                    close_when: "Page-cache lifecycle, folio waits and writeback initialization tests pass.";
                }
                deferred process_prepare.003 {
                    category: DeferredCategory::Feature;
                    summary: "Model and implement seq_file core initialization.";
                    evidence { process_prepare_seq_file_core_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "seq_file lifecycle and representative proc readers pass tests.";
                }
                deferred process_prepare.004 {
                    category: DeferredCategory::Feature;
                    summary: "Model and implement procfs root initialization.";
                    evidence { process_prepare_procfs_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "Procfs mount, root entries and lifecycle tests pass.";
                }
                deferred process_prepare.005 {
                    category: DeferredCategory::Feature;
                    summary: "Model and implement nsfs initialization.";
                    evidence { process_prepare_nsfs_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "nsfs mount and namespace-file lifecycle tests pass.";
                }
                deferred process_prepare.006 {
                    category: DeferredCategory::Feature;
                    summary: "Model and implement pidfs initialization.";
                    evidence { process_prepare_pidfs_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "pidfs mount and PID file lifecycle tests pass.";
                }
                deferred process_prepare.007 {
                    category: DeferredCategory::Feature;
                    summary: "Complete block-device cache initialization inside VFS setup.";
                    evidence { process_prepare_bdev_chrdev_init_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "Block-device registry/cache lifecycle tests pass.";
                }
                deferred process_prepare.008 {
                    category: DeferredCategory::Feature;
                    summary: "Complete character-device registry initialization inside VFS setup.";
                    evidence { process_prepare_bdev_chrdev_init_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "Character-device registry lifecycle and lookup tests pass.";
                }
                deferred process_prepare.009 {
                    category: DeferredCategory::Feature;
                    summary: "Complete SignalCore setup beyond its cache shell.";
                    evidence { process_prepare_signal_core_setup_deferred(ProcessPrepareTrimmedPaths, SignalCore); }
                    close_when: "Signal structures, delivery prerequisites and runtime tests pass.";
                }
                deferred process_prepare.010 {
                    category: DeferredCategory::Feature;
                    summary: "Implement RootPidNamespace allocation, free and lookup actions.";
                    evidence { process_prepare_root_pid_runtime_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "PID allocation/free/hash lookup and namespace tests pass.";
                }
                deferred process_prepare.011 {
                    category: DeferredCategory::Protocol;
                    summary: "Execute and synchronize the vm_stack_cache CPU-hotplug callbacks.";
                    evidence { process_prepare_vm_stack_hotplug_callbacks_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "BP/AP hotplug callback ordering and stack-cache tests pass.";
                }
                deferred process_prepare.012 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement the KeyringCore runtime key_types_sem write protocol.";
                    evidence { process_prepare_key_runtime_sync_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "Concurrent key-type registration/removal and lock-order tests pass.";
                }
                deferred process_prepare.013 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement complete runtime synchronization for the LSM hook dispatcher.";
                    evidence { process_prepare_lsm_runtime_sync_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "LSM hook registration/dispatch ordering and concurrent tests pass.";
                }
                deferred process_prepare.014 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement procfs, nsfs and pidfs runtime mount locking.";
                    evidence { process_prepare_pseudo_fs_runtime_locks_deferred(ProcessPrepareTrimmedPaths); }
                    close_when: "Pseudo-filesystem mount/unmount and concurrent namespace tests pass.";
                }
                trimmed process_prepare.015 {
                    category: TrimmedCategory::Architecture;
                    summary: "efi_enter_virtual_mode is an x86-only call and is unreachable on RISC-V64.";
                    evidence { process_prepare_x86_efi_runtime_switch_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The target architecture changes to x86.";
                }
                trimmed process_prepare.016 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "Shadow-call-stack initialization is absent because CONFIG_SHADOW_CALL_STACK=n.";
                    evidence { process_prepare_shadow_call_stack_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_SHADOW_CALL_STACK.";
                }
                trimmed process_prepare.017 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "Task lockdep initialization is absent because lockdep is disabled.";
                    evidence { process_prepare_lockdep_init_task_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables lockdep.";
                }
                trimmed process_prepare.018 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "dbg_late_init is absent because CONFIG_KGDB=n.";
                    evidence { process_prepare_dbg_late_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_KGDB.";
                }
                trimmed process_prepare.019 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "cpuset_init is a no-op because cpusets are disabled.";
                    evidence { process_prepare_cpuset_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables cpusets.";
                }
                trimmed process_prepare.020 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "cgroup_init is a no-op because CONFIG_CGROUPS=n.";
                    evidence { process_prepare_cgroup_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_CGROUPS.";
                }
                trimmed process_prepare.021 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "taskstats early initialization is absent because CONFIG_TASKSTATS=n.";
                    evidence { process_prepare_taskstats_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_TASKSTATS.";
                }
                trimmed process_prepare.022 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "delay accounting initialization is absent because CONFIG_TASK_DELAY_ACCT=n.";
                    evidence { process_prepare_delayacct_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_TASK_DELAY_ACCT.";
                }
                trimmed process_prepare.023 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "ACPI subsystem initialization is absent because CONFIG_ACPI=n.";
                    evidence { process_prepare_acpi_subsystem_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_ACPI.";
                }
                trimmed process_prepare.024 {
                    category: TrimmedCategory::CompileTimeNoOp;
                    summary: "arch_post_acpi_subsys_init has no work on the current RISC-V64 ACPI-disabled build.";
                    evidence { process_prepare_arch_post_acpi_subsys_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The target architecture supplies a non-empty post-ACPI hook.";
                }
                trimmed process_prepare.025 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "kcsan_init is absent because CONFIG_KCSAN=n.";
                    evidence { process_prepare_kcsan_init_trimmed_noop(ProcessPrepareTrimmedPaths); }
                    revisit_when: "The reference configuration enables CONFIG_KCSAN.";
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            process_prepare_trimmed_paths_prepared(ProcessPrepareTrimmedPaths);
            process_prepare_x86_efi_runtime_switch_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_x86_efi_runtime_switch_trimmed_because_arch_riscv(ProcessPrepareTrimmedPaths);
            process_prepare_shadow_call_stack_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_lockdep_init_task_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_lockdep_init_task_trimmed_because_config_lockdep_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_dbg_late_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_dbg_late_init_trimmed_because_config_kgdb_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_net_namespace_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_net_namespace_deferred_even_if_config_net_ns_enabled(ProcessPrepareTrimmedPaths);
            process_prepare_pagecache_deferred(ProcessPrepareTrimmedPaths, VfsCore);
            process_prepare_pagecache_waitqueue_table_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_signal_core_setup_deferred(ProcessPrepareTrimmedPaths, SignalCore);
            process_prepare_seq_file_core_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_procfs_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_nsfs_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_pidfs_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_vfs_pseudo_filesystems_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_bdev_chrdev_init_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_cpuset_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_cpuset_trimmed_because_config_cpusets_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_cgroup_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_cgroup_trimmed_because_config_cgroups_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_taskstats_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_taskstats_trimmed_because_config_taskstats_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_delayacct_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_delayacct_trimmed_because_config_task_delay_acct_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_acpi_subsystem_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_acpi_trimmed_because_config_acpi_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_arch_post_acpi_subsys_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_kcsan_init_trimmed_noop(ProcessPrepareTrimmedPaths);
            process_prepare_kcsan_trimmed_because_config_kcsan_disabled(ProcessPrepareTrimmedPaths);
            process_prepare_rcu_tasks_generic_out_of_scope(ProcessPrepareTrimmedPaths);
            process_prepare_rcu_tasks_generic_belongs_to_kernel_init_freeable(ProcessPrepareTrimmedPaths);
            process_prepare_trimmed_paths_position_preserved(ProcessPrepareTrimmedPaths);
        }
    }
}

/*
 * ProcessPreparePhase 表示 BootInitFlow.Setup 的第四个 interrupt 叶阶段。它为
 * rest_init() 创建 kernel_init 和 kthreadd 准备 PID、task、cred、VMA、
 * namespace、key/security 等基础结构，但不创建任务，也不进入调度运行。
 */
object ProcessPreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    IrqOpenPreparePhase.state == State::Online;
                    CurrentCPU.trap.interrupt.state == State::Online;
                    Console.state == State::Prepared;
                    SchedClock.state == State::Ready;
                    DelayLoop.state == State::Ready;
                    Cpu0Scheduler.state == State::Online;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Ready;
                    RcuCore.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    MmStructCache.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuCapabilities.state == State::Ready;
                    BootTask.state == State::OnCpu;
                    CurrentCPU.trap.exception.state == State::Ready;
                }

                drives {
                    RootPidNamespace.Transition::Setup;
                    AnonVmaCore.Transition::Setup;
                    TaskCreationCore.Transition::Preset;
                    CredentialCore.Transition::Preset;
                    TaskCreationCore.Transition::Setup;
                    SignalCore.Transition::Preset;
                    TaskFileContext.Transition::Preset;
                    VmaCore.Transition::Preset;
                    NsProxy.Transition::Preset;
                    UtsNamespace.Transition::Preset;
                    KeyringCore.Transition::Setup;
                    SecurityCore.Transition::Setup;
                    VfsCore.Transition::Setup;
                    RamFsType.Transition::Setup;
                    VfsCore.Action::RegisterRamFsType(RamFsType);
                    VfsCore.Action::MountInitialRamFsRoot;
                    FsStruct.Transition::Setup;
                    ProcessPrepareTrimmedPaths.Transition::Preset;
                }

                ensures {
                    process_prepare_ready(ProcessPreparePhase);
                    rest_init_inputs_ready(ProcessPreparePhase, RootPidNamespace, TaskCreationCore, CredentialCore);
                    boot_cpu_local_irq_enabled();
                    interrupt_concurrency_open_for_boot_cpu();
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    kernel_init_task_not_created_yet();
                    kthreadd_task_not_created_yet();
                    system_state_not_scheduling_yet();
                    ramfs_type_registered(VfsCore, RamFsType);
                    rootfs_mount_created(VfsCore);
                    vfs_rootfs_mount_set(VfsCore, Mount);
                    vfs_rootfs_dentry_set(VfsCore, Dentry);
                    fs_struct_root_pwd_same(FsStruct);
                    process_prepare_trimmed_paths_prepared(ProcessPrepareTrimmedPaths);
                    process_prepare_x86_efi_runtime_switch_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_shadow_call_stack_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_lockdep_init_task_trimmed_noop(ProcessPrepareTrimmedPaths);
                    vfs_core_initialized(VfsCore);
                    vfs_core_fs_type_registry_ready(VfsCore);
                    vfs_core_mount_table_ready(VfsCore);
                    vfs_core_dentry_cache_ready(VfsCore);
                    vfs_core_inode_table_ready(VfsCore);
                    vfs_core_file_table_ready(VfsCore);
                    ramfs_type_registered(VfsCore, RamFsType);
                    rootfs_fs_type_uses_ramfs(RamFsType);
                    rootfs_mount_created(VfsCore);
                    vfs_rootfs_mount_set(VfsCore, Mount);
                    vfs_rootfs_dentry_set(VfsCore, Dentry);
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                    fs_struct_pwd_dentry_set(FsStruct, Dentry);
                    fs_struct_root_pwd_same(FsStruct);
                    superblock_root_dentry_bound(SuperBlock, Dentry);
                    superblock_root_inode_bound(SuperBlock, Inode);
                    superblock_root_dentry_inode_matches(SuperBlock, Dentry, Inode);
                    process_prepare_dbg_late_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_net_namespace_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_pagecache_deferred(ProcessPrepareTrimmedPaths, VfsCore);
                    process_prepare_signal_core_setup_deferred(ProcessPrepareTrimmedPaths, SignalCore);
                    process_prepare_seq_file_core_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_procfs_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_nsfs_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_pidfs_deferred(ProcessPrepareTrimmedPaths);
                    process_prepare_cpuset_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_cgroup_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_taskstats_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_delayacct_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_acpi_subsystem_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_arch_post_acpi_subsys_init_trimmed_noop(ProcessPrepareTrimmedPaths);
                    process_prepare_kcsan_init_trimmed_noop(ProcessPrepareTrimmedPaths);
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
                ensures {
                    process_prepare_ready(ProcessPreparePhase);
                    IrqOpenPreparePhase.state == State::Online;
                    rest_init_inputs_ready(ProcessPreparePhase, RootPidNamespace, TaskCreationCore, CredentialCore);
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    kernel_init_task_not_created_yet();
                    kthreadd_task_not_created_yet();
                    system_state_not_scheduling_yet();
                    ramfs_type_registered(VfsCore, RamFsType);
                    rootfs_mount_created(VfsCore);
                    vfs_rootfs_mount_set(VfsCore, Mount);
                    vfs_rootfs_dentry_set(VfsCore, Dentry);
                    fs_struct_root_pwd_same(FsStruct);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            IrqOpenPreparePhase.state == State::Online;
            RootPidNamespace.state == State::Ready;
            AnonVmaCore.state == State::Ready;
            TaskCreationCore.state == State::Ready;
            CredentialCore.state == State::Prepared;
            VectorContext.state == State::Prepared;
            UprobeCore.state == State::Ready;
            SignalCore.state == State::Prepared;
            TaskFileContext.state == State::Prepared;
            VmaCore.state == State::Prepared;
            NsProxy.state == State::Prepared;
            UtsNamespace.state == State::Prepared;
            KeyringCore.state == State::Ready;
            SecurityCore.state == State::Ready;
            ProcessPrepareTrimmedPaths.state == State::Prepared;
            VfsCore.state == State::Ready;
            FsStruct.state == State::Ready;
            RamFsType.state == State::Ready;
            vfs_core_initialized(VfsCore);
            vfs_core_fs_type_registry_ready(VfsCore);
            vfs_core_mount_table_ready(VfsCore);
            vfs_core_dentry_cache_ready(VfsCore);
            vfs_core_inode_table_ready(VfsCore);
            vfs_core_file_table_ready(VfsCore);
            ramfs_type_registered(VfsCore, RamFsType);
            rootfs_fs_type_uses_ramfs(RamFsType);
            rootfs_mount_created(VfsCore);
            vfs_rootfs_mount_set(VfsCore, Mount);
            vfs_rootfs_dentry_set(VfsCore, Dentry);
            fs_struct_root_dentry_set(FsStruct, Dentry);
            fs_struct_pwd_dentry_set(FsStruct, Dentry);
            fs_struct_root_pwd_same(FsStruct);
            process_prepare_ready(ProcessPreparePhase);
            rest_init_inputs_ready(ProcessPreparePhase, RootPidNamespace, TaskCreationCore, CredentialCore);
            boot_cpu_local_irq_enabled();
            task_concurrency_closed();
            smp_concurrency_closed();
            kernel_init_task_not_created_yet();
            kthreadd_task_not_created_yet();
            system_state_not_scheduling_yet();
            process_prepare_trimmed_paths_prepared(ProcessPrepareTrimmedPaths);
            process_prepare_pagecache_deferred(ProcessPrepareTrimmedPaths, VfsCore);
            process_prepare_vfs_pseudo_filesystems_deferred(ProcessPrepareTrimmedPaths);
            process_prepare_rcu_tasks_generic_out_of_scope(ProcessPrepareTrimmedPaths);
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    process_prepare_ready(ProcessPreparePhase);
                    IrqOpenPreparePhase.state == State::Online;
                    rest_init_inputs_ready(ProcessPreparePhase, RootPidNamespace, TaskCreationCore, CredentialCore);
                    task_concurrency_closed();
                    smp_concurrency_closed();
                    kernel_init_task_not_created_yet();
                    kthreadd_task_not_created_yet();
                    system_state_not_scheduling_yet();
                }
            }
        }
    }

    state State::Online {
    }
}
