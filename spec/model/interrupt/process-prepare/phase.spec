/*
 * Process Prepare Phase Specification
 *
 * This is InterruptPhase subphase 4. It starts after IrqOpenPreparePhase has
 * prepared the interrupt-open late core/platform boundary and covers the Linux
 * start_kernel() segment from pid_idr_init() through kcsan_init(), stopping
 * before rest_init() creates PID 1 and kthreadd.
 */

/*
 * RootPidNamespace 表示 pid_idr_init() 后 init_pid_ns 的 PID 分配基础。
 */
object RootPidNamespace: TaskObject {
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
object CredentialCore: TaskObject {
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
                    ExceptionStream.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                }

                ensures {
                    uprobe_core_ready(UprobeCore);
                    uprobes_hash_mutex_ready(UprobeCore);
                    uprobes_die_notifier_registered(UprobeCore, ExceptionStream);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            uprobe_core_ready(UprobeCore);
            uprobes_hash_mutex_ready(UprobeCore);
            uprobes_die_notifier_registered(UprobeCore, ExceptionStream);
        }
    }
}

/*
 * TaskCreationCore 表示 thread_stack_cache_init() 与 fork_init() 后的 task
 * 创建基础。它不创建 PID 1/kthreadd；这些属于 rest_init()。它提供
 * copy_process() 的通用创建契约：rest_init() 传入的 entry 是新任务的
 * 第一执行入口，并由该入口决定任务之后进入 kernel_init 线还是 kthreadd
 * 入口循环。
 */
object TaskCreationCore: TaskObject {
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
                    BootInitTask.state == State::Online;
                }

                drives {
                    VectorContext.Transition::Preset;
                    UprobeCore.Transition::Setup;
                }

                ensures {
                    task_creation_core_ready(TaskCreationCore);
                    task_struct_cache_ready(TaskCreationCore, SlubSubsystem);
                    max_threads_configured(TaskCreationCore, CpuGroup);
                    init_task_rlimits_ready(TaskCreationCore, BootInitTask);
                    init_user_namespace_ucounts_ready(TaskCreationCore);
                    fork_vm_stack_cpuhp_registered(TaskCreationCore);
                    task_creation_entry_contract_ready(TaskCreationCore);
                    rest_init_task_creation_inputs_ready(TaskCreationCore, RootPidNamespace, CredentialCore);
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
            init_task_rlimits_ready(TaskCreationCore, BootInitTask);
            init_user_namespace_ucounts_ready(TaskCreationCore);
            fork_vm_stack_cpuhp_registered(TaskCreationCore);
            task_creation_entry_contract_ready(TaskCreationCore);
            rest_init_task_creation_inputs_ready(TaskCreationCore, RootPidNamespace, CredentialCore);
        }

        actions {
            /*
             * CopyProcess 对应 kernel_clone()/copy_process() 的成功路径。
             * entry 是调用者传入的启动入口，不是创建后的普通属性补丁：
             * TaskCreationCore 必须把它绑定到新 task 的初始 thread context，
             * 后续任务第一次被调度时就从该 entry 对应的执行线开始。
             */
            Action::CopyProcess<Src: TaskObject, New: TaskObject>(
                src_task: Src,
                dst_task: New,
                pid_ns: RootPidNamespace,
                creds: CredentialCore,
                signal: SignalCore,
                files: TaskFileContext,
                security: SecurityCore,
                scheduler: Scheduler,
                entry: TaskEntry
            ) {
                state_effect: StateEffect::None;
                depends_on {
                    src_task.state == State::Online;
                    dst_task.state == State::Prepared;
                    pid_ns.state == State::Ready;
                    creds.state == State::Prepared;
                    signal.state == State::Prepared;
                    files.state == State::Prepared;
                    security.state == State::Ready;
                    scheduler.state == State::Online;
                    task_creation_entry_contract_ready(TaskCreationCore);
                    task_clone_args_ready(dst_task);
                    task_entry_bound(dst_task, entry);
                }

                ensures {
                    task_creation_copy_process_committed(TaskCreationCore, src_task, dst_task);
                    task_creation_used_clone_args(TaskCreationCore, dst_task);
                    task_creation_bound_entry(TaskCreationCore, dst_task, entry);
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

                deferred {
                    "copy_process() 内部的 current->sighand->siglock、tasklist_lock、PID allocator/pidmap 锁、cgroup/cred/file/fs/mm 引用同步和 sched_fork()/PI 初始化锁属于 TaskCreationCore 的共享创建协议，后续在 TaskCreationCore 内展开；rest_init 子阶段只消费 CopyProcess 的成功提交结果。";
                }
            }

            /*
             * CopyUserProcess 是用户态 clone(220)/fork 首片使用的
             * copy_process() 变体。它复用 TaskCreationCore.Ready 内的
             * shared creation contract，但输入从 rest_init 内核线程 entry
             * 切换为当前 UserInitProcess 的 trap frame、mm/files/fs/signal
             * 可见状态。当前只覆盖 BusyBox /bin/sh 触发的 plain fork：
             * clone_flags 去掉 CSIGNAL 后为 0，exit_signal 为 SIGCHLD。
             */
            Action::CopyUserProcess(
                src_process: UserInitProcess,
                dst_process: UserChildProcess,
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
                    src_process.state == State::Online;
                    dst_process.state == State::Prepared;
                    pid_ns.state == State::Ready;
                    scheduler.state == State::Online;
                    fs.state == State::Ready;
                    files.state == State::Ready;
                    address_space.state == State::Online;
                    trap_frame.state == State::Ready;
                    boundaries.state == State::Ready;
                    task_clone_args_ready(dst_process);
                    task_entry_bound(dst_process, TaskEntry::UserChild);
                    user_clone_plain_fork_first_slice_bound(boundaries);
                }

                drives {
                    let runqueue: RunQueueRef <- scheduler.Action::SelectRunQueue(UserChildTaskRef);
                    UserChildTaskRef.Action::SetTaskCpu(BootCPURef);
                    BootRunQueue.Action::EnqueueTask(runqueue, UserChildTaskRef);
                }

                ensures {
                    task_creation_copy_process_committed(TaskCreationCore, src_process, dst_process);
                    task_creation_used_clone_args(TaskCreationCore, dst_process);
                    task_creation_bound_entry(TaskCreationCore, dst_process, TaskEntry::UserChild);
                    task_struct_allocated(dst_process);
                    task_duplicated_from(dst_process, src_process);
                    task_pid_allocated(dst_process, pid_ns);
                    task_thread_context_ready(dst_process);
                    task_sched_entity_initialized(dst_process, scheduler);
                    task_state_new(dst_process);
                    user_child_process_parent_pid1(dst_process, src_process);
                    user_child_process_pid_allocated(dst_process, pid_ns);
                    user_child_process_tgid_equals_pid(dst_process);
                    user_child_process_exit_signal_sigchld(dst_process);
                    user_child_process_files_struct_copied(dst_process, files);
                    user_child_process_fs_struct_copied(dst_process, fs);
                    user_child_process_credentials_copied(dst_process, src_process);
                    user_child_process_signal_state_copied(dst_process, src_process);
                    user_child_process_user_address_space_snapshot(dst_process, address_space);
                    user_child_process_trap_frame_copied(dst_process, trap_frame);
                    user_child_process_trap_frame_child_return_zero(dst_process);
                    user_child_process_tls_inherited(dst_process);
                    user_child_process_enqueued(dst_process, BootRunQueue);
                    task_enqueued_on_runqueue(UserChildTaskRef, BootRunQueue);
                    task_creation_copy_process_sighand_siglock_deferred(TaskCreationCore);
                    task_creation_copy_process_tasklist_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_pidmap_lock_deferred(TaskCreationCore);
                    task_creation_copy_process_sched_fork_locks_deferred(TaskCreationCore);
                }

                deferred {
                    "用户态 CopyUserProcess 当前只覆盖 observed plain fork。Linux copy_process() 中 sighand->siglock、tasklist_lock、PID allocator/pidmap、copy_creds/copy_files/copy_fs/copy_sighand/copy_signal/copy_mm、sched_fork、wake_up_new_task 以及失败回滚均保留为对象事实或 deferred 边界；完整 COW mm、共享 fdtable、thread group、ptrace/seccomp/cgroup/audit、namespace、robust futex、clear_child_tid futex wake、wait/exit/reap 后续按真实 guest 证据展开。";
                }
            }
        }
    }
}

/*
 * SignalCore 表示 proc_caches_init() 中 signal/sighand cache 的准备边界。
 * signals_init() 暂缓，因此本阶段只推进到 Prepared。
 */
object SignalCore: TaskObject {
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
object TaskFileContext: TaskObject {
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
object NsProxy: TaskObject {
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
object UtsNamespace: TaskObject {
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
                }

                deferred {
                    "pagecache_init() 在当前模型中不建立 PageCache 生命周期；其 folio waitqueue table 和 writeback 初始化保留为 VfsCore/PageCache deferred 事实。";
                    "seq_file_init()、proc_root_init()、nsfs_init() 和 pidfs_init() 在当前 CONFIG_PROC_FS/CONFIG_PID_NS 路径下存在，但本阶段只保留为 VFS pseudo filesystem deferred，不声称 proc/nsfs/pidfs 已可用。";
                    "vfs_caches_init() 中的 bdev_cache_init() 和 chrdev_init() 属于后续 block/char device registry 能力，本阶段只覆盖 VFS cache 与 ramfs-backed 初始 rootfs mount。";
                    "net_ns_init() 在 CONFIG_NET_NS=y 下存在，但网络 namespace 运行期对象不在本阶段展开。";
                    "rcu_init_tasks_generic() 不在 start_kernel() 的本区间内；它位于 rest_init() 后 kernel_init_freeable()，对 ProcessPreparePhase 是 out-of-scope。";
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
 * ProcessPreparePhase 表示 InterruptPhase 的第四个子阶段。它为
 * rest_init() 创建 kernel_init 和 kthreadd 准备 PID、task、cred、VMA、
 * namespace、key/security 等基础结构，但不创建任务，也不进入调度运行。
 */
object ProcessPreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: InterruptPhase;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    IrqOpenPreparePhase.state == State::Ready;
                    InterruptStream.state == State::Online;
                    Console.state == State::Prepared;
                    SchedClock.state == State::Ready;
                    DelayLoop.state == State::Ready;
                    Scheduler.state == State::Online;
                    Workqueue.state == State::Prepared;
                    Softirq.state == State::Ready;
                    RcuCore.state == State::Ready;
                    SlubSubsystem.state == State::Ready;
                    KmallocCaches.state == State::Ready;
                    MmStructCache.state == State::Ready;
                    PerCpuStorage.state == State::Ready;
                    CpuCapabilities.state == State::Ready;
                    BootInitTask.state == State::Online;
                    ExceptionStream.state == State::Ready;
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

                deferred {
                    "NetNamespace.setup() 保留 Linux net_ns_init() 时序位置，当前不推进网络 namespace 运行期对象。";
                    "VfsCore.setup() 当前覆盖 Linux vfs_caches_init()/mnt_init() 中 rootfs 初始挂载：rootfs_fs_type 默认使用 ramfs backing，并由 init_mount_tree() 建立初始 root mount；PageCache.setup()、SeqFileCore.setup()、Procfs.setup()、Nsfs.setup() 和 Pidfs.setup() 仍保留为 deferred。";
                    "SignalCore.setup()/signals_init() 暂缓；本阶段只要求 sighand/signal cache 进入 Prepared。";
                    "RootPidNamespace 的 alloc_pid/free_pid/find_pid_ns 等运行期 action 留给 rest_init() 和后续任务创建路径。";
                    "TaskCreationCore 不创建 kernel_init 或 kthreadd；rest_init() 才推进这些任务对象和 SYSTEM_SCHEDULING。";
                    "fork_init() 中 cpuhp_setup_state(\"fork:vm_stack_cache\") 只注册后续 CPU hotplug 回调；AP/BP 同步和 hotplug 状态机执行属于后续 SMP bring-up。";
                    "key_init() 的 key_types_sem 写侧保护、security_init() 的 LSM hook dispatcher 和 proc/nsfs/pidfs mount 内部锁在本阶段只发布启动期 registry 事实；完整运行期同步协议留给对应对象 action。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            IrqOpenPreparePhase.state == State::Ready;
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
    }
}
