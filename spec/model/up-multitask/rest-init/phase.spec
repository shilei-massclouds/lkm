/*
 * Rest Init Phase Specification
 *
 * This is UP Multitask Phase subphase 1. It covers Linux rest_init(): RCU
 * scheduler start, creation of PID 1 and kthreadd, publication of
 * SYSTEM_SCHEDULING, completion of kthreadd_done, and boot idle runtime entry.
 * Scheduler.schedule_preempt_disabled() publishes KernelInitDispatchGate,
 * which lets KernelInitTask enter PreSmpInitPhase while BootInitTask continues
 * the rest_init tail and becomes BootIdleTask.
 */

/*
 * rcu_scheduler_starting() 是 RcuCore.Ready 状态内 action。它不推进
 * RcuCore 生命周期，只把 RCU 从 early boot/no-op 模式切到
 * RCU_SCHEDULER_INIT，同步 GP 序号基线，并保持 GP kthread deferred。
 */

lock KernelInitTaskPiLock: RawSpinLock;

context WakeUpNewTaskContext: ResourceExclusiveContext {
    guard: RawSpinLockIrqSaveGuard {
        lock_ref: KernelInitTaskPiLock;

        entered_by {
            KernelInitTaskPiLock.Event::LockIrqSave;
        }

        exited_by {
            KernelInitTaskPiLock.Event::UnlockIrqRestore;
        }
    }

    obj_refs {
        KernelInitTask;
        Scheduler;
        BootRunQueue;
    }

    effects {
        interruptible: false;
        preemptible: false;
        sleepable: false;
        exclusive_refs: obj_refs;
    }
}

/*
 * KernelInitTask 表示 user_mode_thread(kernel_init, NULL, CLONE_FS) 创建的
 * PID 1。它在本阶段变为 Online，但其 kernel_init_freeable() 执行属于下一子阶段。
 *
 * KernelInitTask.Preset 对应 user_mode_thread() 内部构造临时
 * kernel_clone_args 的过程。kernel_clone_args 是栈上传参结构，没有独立
 * 生命周期，因此不建模为对象；Preset 把 fn、fn_arg、clone flags、
 * exit_signal 等信息固化为 KernelInitTask.Prepared 上的 facts，并明确
 * 尚未分配 task_struct、尚未 attach pid、尚未入队。
 *
 * KernelInitTask.Setup 对应 kernel_clone() 调用 copy_process() 的成功路径。
 * copy_process() 是 TaskCreationCore.Ready 状态内的参数化 action：
 * TaskCreationCore.Action::CopyProcess(src_task: BootInitTask,
 * dst_task: KernelInitTask, ...)。该 action 以 src_task/current 为模板创建
 * dst_task/task_struct，初始化 pid、凭据、fs/files、signal、安全上下文、
 * thread context 与 sched entity，但保持 task_state_new 且 task_not_enqueued。
 * KernelInitTask.Setup 只提交 KernelInitTask Prepared -> Ready 的生命周期结果。
 *
 * KernelInitTask.Enable 对应 wake_up_new_task()。该路径受 p->pi_lock 保护，
 * 因此 Enable 先通过 KernelInitTaskPiLock.LockIrqSave 进入
 * WakeUpNewTaskContext，再在该独占上下文内执行受保护资源动作；退出时
 * 通过 KernelInitTaskPiLock.UnlockIrqRestore 恢复本地中断和当前任务抢占。
 * 上下文通过 KernelInitTaskPiLock 建立边界，引用 KernelInitTask、
 * Scheduler、BootRunQueue 三个受保护对象。
 * Enable 的 within WakeUpNewTaskContext 块直接驱动
 * Task.Event::SetRuntimeState(Running)、Scheduler.Action::SelectRunQueue
 * (selected_rq: BootRunQueue) 和 BootRunQueue.Action::EnqueueTask
 * (task: KernelInitTask)。三者都成功后，Enable 才提交
 * KernelInitTask Ready -> Online。
 */
object KernelInitTask: Task {
    initial_state: State::Base;

    /*
     * Base 表示 PID 1 的创建规格尚未建立。
     */
    state State::Base {
        events {
            /*
             * Preset 选择 kernel_init 入口并记录 CLONE_FS 创建约束。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    BootInitTask.state == State::Online;
                }

                ensures {
                    kernel_init_spawn_spec_ready(KernelInitTask);
                    kernel_init_entry_selected(KernelInitTask);
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
            kernel_init_clone_fs_flag_set(KernelInitTask);
        }

        events {
            /*
             * Setup 对应 user_mode_thread(kernel_init, NULL, CLONE_FS) 创建 PID 1。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                drives {
                    TaskCreationCore.Action::CopyProcess(
                        src_task: BootInitTask,
                        dst_task: KernelInitTask,
                        pid_ns: RootPidNamespace,
                        creds: CredentialCore,
                        signal: SignalCore,
                        files: TaskFileContext,
                        security: SecurityCore,
                        scheduler: Scheduler
                    );
                }

                ensures {
                    kernel_init_task_ready(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_thread_context_ready(KernelInitTask);
                    kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
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
            kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
            task_state_new(KernelInitTask);
            task_not_enqueued(KernelInitTask);
            kernel_init_waits_for_kthreadd_done(KernelInitTask);
        }

        events {
            /*
             * Enable 把 kernel_init 放入可调度集合，但仍等待 kthreadd_done 释放。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                    BootCurrentCPU.state == State::Online;
                    BootCpuLocalInterrupt.state == State::Ready;
                    BootCpuCurrentTask.state == State::Ready;
                    current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    task_preemption_control_ready(BootIdleTask);
                    task_state_new(KernelInitTask);
                    task_not_enqueued(KernelInitTask);
                }

                within WakeUpNewTaskContext {
                    depends_on {
                        task_state_new(KernelInitTask);
                        task_not_enqueued(KernelInitTask);
                        current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    }

                    drives {
                        KernelInitTask.Event::SetRuntimeState(state: TaskRuntimeState::Running);
                        Scheduler.Action::SelectRunQueue(selected_rq: BootRunQueue);
                        BootRunQueue.Action::EnqueueTask(task: KernelInitTask);
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(KernelInitTaskPiLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(KernelInitTaskPiLock, BootCurrentCPU);
                        task_state_running(KernelInitTask);
                        task_runqueue_selected(Scheduler, KernelInitTask, BootRunQueue);
                        task_enqueued_on_runqueue(KernelInitTask, BootRunQueue);
                    }
                }

                ensures {
                    kernel_init_task_online(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
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
            kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
            task_state_running(KernelInitTask);
            task_enqueued_on_runqueue(KernelInitTask, BootRunQueue);
        }
    }
}

/*
 * KernelInitAffinity 表示 rest_init() 中对 PID 1 设置 PF_NO_SETAFFINITY 并
 * 临时固定到 boot CPU。
 */
object KernelInitAffinity: TaskObject {
    initial_state: State::Base;
    parent: KernelInitTask;

    /*
     * Base 表示 PID 1 尚未被 rest_init() 固定到 boot CPU。
     */
    state State::Base {
        events {
            /*
             * Setup 设置 PF_NO_SETAFFINITY，并把 PID 1 临时固定到 boot CPU。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    RootPidNamespace.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    kernel_init_affinity_ready(KernelInitAffinity, KernelInitTask);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    kernel_init_pid_lookup_used_root_namespace(KernelInitTask, RootPidNamespace);
                }
            }
        }
    }

    /*
     * Ready 表示 PID 1 的临时亲和性约束已经发布。
     */
    state State::Ready {
        invariant {
            kernel_init_affinity_ready(KernelInitAffinity, KernelInitTask);
            kernel_init_pf_no_setaffinity(KernelInitTask);
            kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
        }
    }
}

/*
 * KthreaddTask 表示 kernel_thread(kthreadd, NULL, CLONE_FS | CLONE_FILES)
 * 创建的全局内核线程管理者。
 */
object KthreaddTask: TaskObject {
    initial_state: State::Base;

    /*
     * Base 表示 kthreadd 的创建规格尚未建立。
     */
    state State::Base {
        events {
            /*
             * Preset 选择 kthreadd 入口并记录 CLONE_FS | CLONE_FILES 创建约束。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    BootInitTask.state == State::Online;
                }

                ensures {
                    kthreadd_spawn_spec_ready(KthreaddTask);
                    kthreadd_entry_selected(KthreaddTask);
                    kthreadd_clone_fs_files_flags_set(KthreaddTask);
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
            kthreadd_is_kernel_thread_provider(KthreaddTask);
        }

        events {
            /*
             * Setup 对应 kernel_thread(kthreadd, NULL, CLONE_FS | CLONE_FILES)。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                ensures {
                    kthreadd_task_ready(KthreaddTask);
                    kthreadd_task_pid_allocated(KthreaddTask, RootPidNamespace);
                    kthreadd_thread_context_ready(KthreaddTask);
                    kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
                    kthreadd_global_ref_bound(KthreaddTask);
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
            kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
            kthreadd_global_ref_bound(KthreaddTask);
        }

        events {
            /*
             * Enable 把 kthreadd 放入可调度集合。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                }

                ensures {
                    kthreadd_task_online(KthreaddTask);
                    kthreadd_task_enqueued(KthreaddTask, Scheduler);
                    kthreadd_global_ref_bound(KthreaddTask);
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
            kthreadd_task_enqueued(KthreaddTask, Scheduler);
            kthreadd_global_ref_bound(KthreaddTask);
        }
    }
}

/*
 * SystemState 表示 Linux 全局 system_state 枚举。生命周期 Ready 对应本阶段
 * 把内部值推进到 SYSTEM_SCHEDULING；后续 FinalizePhase 通过 Enable
 * 推进到 SYSTEM_RUNNING / Online。
 */
object SystemState: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示 system_state 尚未在本阶段重新确认启动中状态。
     */
    state State::Base {
        events {
            /*
             * Preset 记录 SYSTEM_BOOTING 仍是当前全局系统状态。
             */
            on Event::Preset -> State::Prepared {
                ensures {
                    system_state_booting(SystemState);
                }
            }
        }
    }

    /*
     * Prepared 表示 SYSTEM_BOOTING 已确认，等待 PID 1 和 kthreadd 都创建完成。
     */
    state State::Prepared {
        invariant {
            system_state_booting(SystemState);
        }

        events {
            /*
             * Setup 对应 rest_init() 中 system_state = SYSTEM_SCHEDULING。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    system_state_scheduling(SystemState);
                    up_multitask_scheduling_open();
                    smp_concurrency_closed();
                }
            }
        }
    }

    /*
     * Ready 表示系统已进入 SYSTEM_SCHEDULING，UP 多任务调度边界打开。
     */
    state State::Ready {
        invariant {
            system_state_scheduling(SystemState);
            up_multitask_scheduling_open();
            smp_concurrency_closed();
        }

        events {
            /*
             * Enable 预留给 FinalizePhase 将 system_state 推进到 SYSTEM_RUNNING。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    InitMemoryCleanupDeferred.state == State::Ready;
                    KernelMappingProtectionDeferred.state == State::Ready;
                    PtiFinalizeTrimmed.state == State::Ready;
                }

                ensures {
                    system_state_running(SystemState);
                    up_multitask_scheduling_open();
                    smp_concurrency_open(CpuGroup);
                }
            }
        }
    }

    /*
     * Online 表示系统已进入 SYSTEM_RUNNING，多核运行期并发边界打开。
     */
    state State::Online {
        invariant {
            system_state_running(SystemState);
            up_multitask_scheduling_open();
            smp_concurrency_open(CpuGroup);
        }
    }
}

/*
 * KthreaddReadyGate 表示静态 completion kthreadd_done，是 Completion Type
 * 的具名实例。它本身不重新定义 Setup/Enable；下面对象事件是当前工具对
 * inherited Type process 的阶段推导展开。Setup 来自 Completion.Setup，
 * 建立 done=0 和 owned wait_queue；Enable 在对象主状态迁移中先应用
 * Completion.Enable，使该 completion 可被运行期 process 操作，再应用
 * Completion.Complete 的成功效果，即 complete(&kthreadd_done) 已发布，
 * PID 1 可继续执行下一子阶段。
 */
object KthreaddReadyGate: Completion {
    initial_state: State::Base;

    /*
     * Base 表示 kthreadd_done completion 尚未建模为 pending 门。
     */
    state State::Base {
        events {
            /*
             * Setup 建立 kthreadd_done pending 门，PID 1 仍被阻塞在等待点。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    kthreadd_ready_gate_ready(KthreaddReadyGate);
                    kthreadd_ready_gate_pending(KthreaddReadyGate);
                    completion_owns_wait_queue(KthreaddReadyGate);
                    completion_ready(KthreaddReadyGate);
                    completion_pending(KthreaddReadyGate);
                    completion_done_count_is_zero(KthreaddReadyGate);
                    completion_wait_queue_ready(KthreaddReadyGate);
                    kernel_init_waits_for_kthreadd_done(KernelInitTask);
                }
            }
        }
    }

    /*
     * Ready 表示 kthreadd_done 已存在但尚未 complete。
     */
    state State::Ready {
        invariant {
            kthreadd_ready_gate_ready(KthreaddReadyGate);
            completion_ready(KthreaddReadyGate);
            completion_pending(KthreaddReadyGate);
        }

        events {
            /*
             * Enable 对应 complete(&kthreadd_done)，释放 PID 1 进入下一子阶段。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    SystemState.state == State::Ready;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    completion_online(KthreaddReadyGate);
                    completion_handle_published(KthreaddReadyGate);
                    completion_complete_committed(KthreaddReadyGate);
                    completion_token_available(KthreaddReadyGate);
                    completion_wakes_one_waiter(KthreaddReadyGate);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
                }
            }
        }
    }

    /*
     * Online 表示 kthreadd_done 已完成，PID 1 可以推进 PreSmpInitPhase。
     */
    state State::Online {
        invariant {
            kthreadd_ready_gate_completed(KthreaddReadyGate);
            completion_online(KthreaddReadyGate);
            completion_complete_committed(KthreaddReadyGate);
            completion_token_available(KthreaddReadyGate);
            completion_wakes_one_waiter(KthreaddReadyGate);
            kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
            kernel_init_released_for_pre_smp_init(KernelInitTask);
        }
    }
}

/*
 * KernelInitDispatchGate 表示 schedule_preempt_disabled() 形成的分叉边界。
 * 它释放 KernelInitTask 进入 PreSmpInitPhase，但不表示 BootInitTask 的
 * boot idle 尾部已经完成。
 */
object KernelInitDispatchGate: TaskObject {
    initial_state: State::Base;

    /*
     * Base 表示 schedule_preempt_disabled() 的分叉边界尚未提交。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 schedule_preempt_disabled()，把 PID 1 分派到 PreSmpInitPhase。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                    SystemState.state == State::Ready;
                    KthreaddReadyGate.state == State::Online;
                }

                ensures {
                    kernel_init_dispatch_gate_ready(KernelInitDispatchGate);
                    scheduler_first_schedule_committed(Scheduler);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    boot_init_task_continues_rest_init_tail(BootInitTask);
                    rest_init_boot_idle_tail_pending(BootIdleRuntime);
                }
            }
        }
    }

    /*
     * Ready 表示 PID 1 已分派，BootInitTask 继续执行 rest_init() 尾部。
     */
    state State::Ready {
        invariant {
            kernel_init_dispatch_gate_ready(KernelInitDispatchGate);
            scheduler_first_schedule_committed(Scheduler);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            boot_init_task_continues_rest_init_tail(BootInitTask);
        }
    }
}

/*
 * BootIdleRuntime 表示分叉点之后 cpu_startup_entry() 确认 boot CPU idle
 * runtime 入口。它复用 SchedInitPhase 已建立的 BootIdleTask。
 */
object BootIdleRuntime: TaskObject {
    initial_state: State::Base;
    parent: BootIdleTask;

    /*
     * Base 表示 boot idle 任务已存在，但尚未进入 cpu_startup_entry() 运行期入口。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 rest_init() 尾部的 cpu_startup_entry(CPUHP_ONLINE) 边界。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Scheduler.state == State::Online;
                    BootIdleTask.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                    KthreaddReadyGate.state == State::Online;
                    KernelInitDispatchGate.state == State::Ready;
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    scheduler_first_schedule_committed(Scheduler);
                    boot_idle_runtime_ready(BootIdleRuntime, BootIdleTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleRuntime, BootCPU);
                    boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
                    boot_cpu_hotplug_state_online(BootCPU);
                    secondary_cpus_not_started(CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示 BootInitTask 已完成运行期交接，boot CPU idle runtime 已进入。
     */
    state State::Ready {
        invariant {
            scheduler_first_schedule_committed(Scheduler);
            boot_idle_runtime_ready(BootIdleRuntime, BootIdleTask);
            boot_idle_cpu_startup_entry_ready(BootIdleRuntime, BootCPU);
            boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
            secondary_cpus_not_started(CpuGroup);
        }
    }
}

/*
 * RestInitPhase 表示 UP MultitaskPhase 的第一个子阶段。它创建 PID 1 和
 * kthreadd，发布 SYSTEM_SCHEDULING，并完成 boot idle runtime 入口。
 */
object RestInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: UpMultitaskPhase;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    ProcessPreparePhase.state == State::Ready;
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                    BootIdleTask.state == State::Ready;
                    RcuCore.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    InterruptStream.state == State::Online;
                }

                drives {
                    KernelInitTask.Event::Preset;
                    KernelInitTask.Event::Setup;
                    KernelInitTask.Event::Enable;
                    KernelInitAffinity.Event::Setup;
                    KthreaddTask.Event::Preset;
                    KthreaddTask.Event::Setup;
                    KthreaddTask.Event::Enable;
                    SystemState.Event::Preset;
                    SystemState.Event::Setup;
                    KthreaddReadyGate.Event::Setup;
                    KthreaddReadyGate.Event::Enable;
                    KernelInitDispatchGate.Event::Setup;
                }

                ensures {
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_single_online_cpu_at_scheduler_start(RcuCore, CpuGroup);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    rest_init_dispatch_ready(RestInitPhase, KernelInitDispatchGate);
                    kernel_init_task_created(KernelInitTask);
                    kthreadd_task_created(KthreaddTask);
                    system_state_scheduling(SystemState);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    rest_init_boot_idle_tail_pending(BootIdleRuntime);
                    task_concurrency_open();
                    smp_concurrency_closed();
                    workqueue_workers_still_deferred();
                    rcu_gp_threads_still_deferred(RcuCore);
                    numa_default_policy_trimmed();
                }

                deferred {
                    "KthreaddTask 消费 kthread_create_list 和后续 kthread 创建服务留给运行期模型。";
                    "真实抢占、上下文切换和任务栈切换不在当前对象级实现中执行，只发布调度分叉事实。";
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ProcessPreparePhase.state == State::Ready;
            KernelInitDispatchGate.state == State::Ready;
            rcu_scheduler_starting_ready(RcuCore);
            rcu_scheduler_active_level_init(RcuCore);
            rcu_gp_seq_baseline_synced(RcuCore);
            KernelInitTask.state == State::Online;
            KthreaddTask.state == State::Online;
            SystemState.state == State::Ready;
            KthreaddReadyGate.state == State::Online;
            rest_init_dispatch_ready(RestInitPhase, KernelInitDispatchGate);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            rest_init_boot_idle_tail_pending(BootIdleRuntime);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitDispatchGate.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    BootIdleTask.state == State::Ready;
                }

                drives {
                    BootIdleRuntime.Event::Setup;
                }

                ensures {
                    rest_init_ready(RestInitPhase);
                    up_multitask_runtime_ready(RestInitPhase, KernelInitTask, KthreaddTask, BootIdleRuntime);
                    boot_cpu_idle_runtime_entered(BootIdleRuntime);
                    boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    system_state_scheduling(SystemState);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    task_concurrency_open();
                    smp_concurrency_closed();
                }

                deferred {
                    "idle loop 的真实执行不在当前对象级实现中执行，只发布 boot idle 入口边界事实。";
                    "secondary CPU 启动仍保持 deferred，后续 SMP Runtime Phase 再推进。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ProcessPreparePhase.state == State::Ready;
            rcu_scheduler_starting_ready(RcuCore);
            rcu_scheduler_active_level_init(RcuCore);
            rcu_gp_seq_baseline_synced(RcuCore);
            KernelInitTask.state == State::Online;
            KernelInitAffinity.state == State::Ready;
            KthreaddTask.state == State::Online;
            SystemState.state == State::Ready;
            KthreaddReadyGate.state == State::Online;
            KernelInitDispatchGate.state == State::Ready;
            BootIdleRuntime.state == State::Ready;
            rest_init_ready(RestInitPhase);
            system_state_scheduling(SystemState);
            kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            task_concurrency_open();
            smp_concurrency_closed();
        }
    }
}
