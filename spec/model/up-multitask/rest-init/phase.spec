/*
 * Rest Init Phase Specification
 *
 * This is UP Multitask Phase subphase 1. It covers Linux rest_init(): RCU
 * scheduler start, creation of PID 1 and kthreadd, publication of
 * SYSTEM_SCHEDULING, completion of kthreadd_done, and boot idle runtime entry.
 * schedule_preempt_disabled() is expanded into three formal steps: the boot
 * idle preemption guard exits with EnableNoResched, Scheduler.Action::Schedule
 * commits the first scheduling boundary, and BootIdleStartupContext enters the
 * boot-idle atomic context for cpu_startup_entry(). The boot-idle tail is
 * modeled as an abstract idle loop: the boot CPU waits while need_resched is
 * clear, observes need_resched when the environment requests scheduling, drives
 * Scheduler.Action::ScheduleIdle, and then returns to the same idle-loop point.
 */

/*
 * rcu_scheduler_starting() 是 RcuCore.Ready 状态内 action。它不推进
 * RcuCore 生命周期，只把 RCU 从 early boot/no-op 模式切到
 * RCU_SCHEDULER_INIT，同步 GP 序号基线，并保持 GP kthread deferred。
 */

lock KernelInitTaskPiLock: RawSpinLock;
lock KthreaddTaskPiLock: RawSpinLock;
lock BootRunQueueLock: RawSpinLock;

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

context WakeUpKthreaddTaskContext: ResourceExclusiveContext {
    guard: RawSpinLockIrqSaveGuard {
        lock_ref: KthreaddTaskPiLock;

        entered_by {
            KthreaddTaskPiLock.Event::LockIrqSave;
        }

        exited_by {
            KthreaddTaskPiLock.Event::UnlockIrqRestore;
        }
    }

    obj_refs {
        KthreaddTask;
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

context EnqueueSelectedRunQueueContext: ResourceExclusiveContext {
    /*
     * Per SEM-EXCLUSIVE-CONTEXT-001, nested within uses the standard
     * no-argument form and consumes lexically visible bindings. The selected_rq
     * binding produced by Scheduler.Action::SelectRunQueue is visible inside
     * this context. In the current UP path, selected_rq is proven to target
     * BootRunQueue and BootCPURef, so this context is guarded by
     * BootRunQueueLock and the task CPU update is temporarily driven with
     * BootCPURef. Future generic runqueue enqueue modeling should resolve the
     * lock, obj_refs and cpu_of(selected_rq) from the selected RunQueueRef
     * instead of this BootRunQueue specialization.
     */
    guard: RawSpinLockIrqSaveGuard {
        lock_ref: BootRunQueueLock;

        entered_by {
            BootRunQueueLock.Event::LockIrqSave;
        }

        exited_by {
            BootRunQueueLock.Event::UnlockIrqRestore;
        }
    }

    obj_refs {
        BootRunQueue;
    }

    effects {
        interruptible: false;
        preemptible: false;
        sleepable: false;
        exclusive_refs: obj_refs;
    }
}

context SchedulePreemptionContext: Context {
    /*
     * This context models schedule() entering __schedule_loop(): schedule()
     * disables preemption before entering __schedule(). In rest_init(), the
     * caller has just executed EnableNoResched from the inherited
     * preempt-disabled context, so this context is the schedule-owned
     * preemption boundary rather than the caller's inherited guard.
     */
    guard: PreemptionGuard {
        entered_by {
            BootIdlePreemption.Event::Disable;
        }

        exited_by {
            BootIdlePreemption.Event::EnableNoResched;
        }
    }

    obj_refs {
        BootIdleTask;
        Scheduler;
        BootRunQueue;
        BootCpuLocalInterrupt;
    }

    effects {
        interruptible: true;
        preemptible: false;
        sleepable: false;
        exclusive_refs: none;
    }
}

context ScheduleLocalInterruptContext: Context {
    /*
     * This context models __schedule() disabling local interrupts before
     * taking rq->lock. The guard is backed by BootCpuLocalInterrupt, not by a
     * lock; it establishes a CPU-local interrupt-disabled boundary.
     */
    guard: LocalInterruptGuard {
        entered_by {
            BootCpuLocalInterrupt.Event::SaveAndDisable;
        }

        exited_by {
            BootCpuLocalInterrupt.Event::Restore;
        }
    }

    obj_refs {
        BootIdleTask;
        Scheduler;
        BootRunQueue;
        BootCpuLocalInterrupt;
    }

    effects {
        interruptible: false;
        preemptible: false;
        sleepable: false;
        exclusive_refs: none;
    }
}

context ScheduleRunQueueContext: ResourceExclusiveContext {
    /*
     * This context models the rq_lock() region inside __schedule(). Current
     * tooling reuses RawSpinLockIrqSaveGuard for the BootRunQueueLock boundary;
     * the Linux path has local interrupts already disabled before rq_lock().
     * A narrower runqueue-lock guard can replace this once rq_lock/raw rq lock
     * is modeled separately from irq-save spinlock.
     */
    guard: RawSpinLockIrqSaveGuard {
        lock_ref: BootRunQueueLock;

        entered_by {
            BootRunQueueLock.Event::LockIrqSave;
        }

        exited_by {
            BootRunQueueLock.Event::UnlockIrqRestore;
        }
    }

    obj_refs {
        BootIdleTask;
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

context BootIdleStartupContext: Context {
    /*
     * This context corresponds to the post-schedule preempt-disabled atomic
     * context established by schedule_preempt_disabled() before
     * cpu_startup_entry(CPUHP_ONLINE). The pre-schedule context is inherited
     * from boot/sched_init and is exited explicitly by
     * BootIdlePreemption.Event::EnableNoResched in RestInitPhase.Preset.
     *
     * 参照 Linux 的 idle 设计原理，boot CPU 的整个 idle 入口准备和 idle
     * loop 都运行在抢占关闭上下文中，避免 idle/current task、polling、
     * nohz 和 need_resched 等 CPU 本地状态被普通抢占打断后出现不一致。
     * idle task 不通过普通抢占被动切出；它只在本 CPU idle loop 观察到
     * need_resched 后，主动进入 schedule_idle()/scheduler 调度边界。
     */
    guard: PreemptionGuard {
        entered_by {
            BootIdlePreemption.Event::Disable;
        }

        exited_by {
            BootIdlePreemption.Event::Enable;
        }
    }

    obj_refs {
        BootIdleTask;
        BootIdleRuntime;
        Scheduler;
    }

    effects {
        interruptible: true;
        preemptible: false;
        sleepable: false;
        exclusive_refs: none;
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
 * Task.Event::SetRuntimeState(Running)，再通过 action result binding 把
 * Scheduler.Action::SelectRunQueue(KernelInitTaskRef) 返回的
 * runqueue ref 绑定为 selected_rq。随后用标准无实参 within 进入
 * EnqueueSelectedRunQueueContext；selected_rq 作为外层 action result
 * binding 在嵌套 within 中直接可见。当前 UP 路径证明 selected_rq
 * 指向 BootRunQueue，因此该 context 仍由 BootRunQueueLock 建立边界，并驱动
 * RunQueue.Event::EnqueueTask(KernelInitTaskRef)。SelectRunQueue
 * 当前固定返回 BootRunQueueRef；完整选择策略后续 deferred。三者都成功后，
 * Enable 才提交 KernelInitTask Ready -> Online。
 *
 * KernelInitTask.PinToBootCpu 对应 rest_init() 随后的 PF_NO_SETAFFINITY
 * 与 set_cpus_allowed_ptr(tsk, cpumask_of(smp_processor_id()))。它不是独立
 * lifecycle object，而是 KernelInitTask 的属性 action；源码中的
 * find_task_by_pid_ns(pid, &init_pid_ns) 只是用 pid 重新取回 task 指针，
 * 规格层已经通过 KernelInitTask receiver 持有目标 task。该 action 当前
 * 直接提交 flags 和 cpumask 属性。Linux 路径处在 rcu_read_lock()/unlock()
 * 定界的读侧上下文中；该上下文是否归入资源独占上下文，还是应建模为
 * 单独的 RCU/读侧上下文，后续讨论。
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
            kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
            task_ref_targets(KernelInitTaskRef, KernelInitTask);
            task_ref_ready(KernelInitTaskRef);
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
                        current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    }

                    drives {
                        KernelInitTask.Event::SetRuntimeState(TaskRuntimeState::Running);
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
                            selected_rq.Event::EnqueueTask(KernelInitTaskRef);
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
            task_cpu_ref_is(KernelInitTask, BootCPURef);
            task_enqueued_on_runqueue(KernelInitTaskRef, BootRunQueueRef);
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
            }
            ensures {
                task_flag_no_setaffinity(KernelInitTask);
                task_cpumask_is(KernelInitTask, cpu_ref);
                kernel_init_pf_no_setaffinity(KernelInitTask);
                kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
            }
            deferred {
                "PinToBootCpu 当前未建模 rcu_read_lock()/unlock() 定界的读侧上下文；它是否属于资源独占上下文，还是应作为 RCU/读侧上下文单独建模，后续讨论。";
            }
        }
    }
}

/*
 * KthreaddTask 表示 kernel_thread(kthreadd, NULL, CLONE_FS | CLONE_FILES)
 * 创建的全局内核线程管理者。它复用 KernelInitTask 的创建/唤醒形态：
 * Preset 固化临时 kernel_clone_args，Setup 驱动 copy_process，Enable
 * 对应 wake_up_new_task。区别在于入口为 kthreadd、flags 包含
 * kernel_thread 固有的 CLONE_VM | CLONE_UNTRACED 以及传入的
 * CLONE_FS | CLONE_FILES，并且创建和唤醒后还要发布 kthreadd_task
 * 全局 provider 引用。
 */
object KthreaddTask: Task {
    initial_state: State::Base;

    /*
     * Base 表示 kthreadd 的创建规格尚未建立。
     */
    state State::Base {
        events {
            /*
             * Preset 选择 kthreadd 入口并记录 kernel_thread 创建约束。
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
                    kthreadd_spawn_spec_ready(KthreaddTask);
                    kthreadd_entry_selected(KthreaddTask);
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
            kthreadd_clone_fs_files_flags_set(KthreaddTask);
            kthreadd_clone_vm_flag_set(KthreaddTask);
            kthreadd_clone_untraced_flag_set(KthreaddTask);
            kthreadd_kernel_thread_flag_set(KthreaddTask);
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

                drives {
                    TaskCreationCore.Action::CopyProcess(
                        src_task: BootInitTask,
                        dst_task: KthreaddTask,
                        pid_ns: RootPidNamespace,
                        creds: CredentialCore,
                        signal: SignalCore,
                        files: TaskFileContext,
                        security: SecurityCore,
                        scheduler: Scheduler
                    );
                }

                ensures {
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
            kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
            task_ref_targets(KthreaddTaskRef, KthreaddTask);
            task_ref_ready(KthreaddTaskRef);
            task_state_new(KthreaddTask);
            task_not_enqueued(KthreaddTask);
        }

        events {
            /*
             * Enable 把 kthreadd 放入可调度集合。
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
                        current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                    }

                    drives {
                        KthreaddTask.Event::SetRuntimeState(TaskRuntimeState::Running);
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
                            selected_rq.Event::EnqueueTask(KthreaddTaskRef);
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
                    }
                }

                ensures {
                    kthreadd_task_online(KthreaddTask);
                    kthreadd_task_enqueued(KthreaddTask, BootRunQueue);
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
            }
            ensures {
                kthreadd_global_ref_bound(KthreaddTask);
                kthreadd_provider_ref_targets(KthreaddTaskRef, KthreaddTask);
                kthreadd_provider_ready(KthreaddTask);
            }
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
 * 的具名实例。Completion 的 token、wait_queue 和 wake-one 行为必须由
 * Completion Type process 承载，实例不复制这些通用 facts。下面的对象
 * lifecycle event 只是当前工具的实例 wrapper：Setup/Enable 提交
 * kthreadd_done 在 rest_init 场景中的 gate 生命周期状态；真正的
 * complete(&kthreadd_done) 表达为 KthreaddReadyGate.Event::Complete。
 * Completion 通用结果来自 Type process；释放 PID 1 进入下一子阶段的
 * 场景事实由 RestInitPhase 承载，不额外引入 Linux 中不存在的 action。
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
            kthreadd_ready_gate_pending(KthreaddReadyGate);
        }

        events {
            /*
             * Enable 发布 completion handle，使其可被运行期 Complete process 操作。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    SystemState.state == State::Ready;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    kthreadd_ready_gate_ready(KthreaddReadyGate);
                }
            }
        }
    }

    /*
     * Online 表示 kthreadd_done handle 已发布，可被 Completion Complete
     * process 操作；是否完成由 Completion 扩展状态和外层阶段事实表达。
     */
    state State::Online {
        invariant {
            kthreadd_ready_gate_ready(KthreaddReadyGate);
        }
    }
}

/*
 * BootIdleRuntime 表示分叉点之后 cpu_startup_entry() 确认 boot CPU idle
 * runtime 入口。它复用 SchedInitPhase 已建立的 BootIdleTask，并抽象 Linux
 * cpu_startup_entry() -> do_idle() -> schedule_idle() 的循环主线：boot CPU
 * 先执行 current->flags |= PF_IDLE、arch_cpu_idle_prepare() 和
 * cpuhp_online_idle(CPUHP_ONLINE)，然后进入 while (1) do_idle()；当本 CPU
 * 在 do_idle() 中观察到 need_resched 时，驱动 idle 专用调度，调度返回后
 * 继续回到原 idle loop 点。
 */
object BootIdleRuntime: BootIdleRuntimeObject {
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
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    scheduler_first_schedule_committed(Scheduler);
                    boot_idle_runtime_ready(BootIdleRuntime, BootIdleTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleRuntime, BootCPU);
                    boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
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
                    KernelInitTask.Action::PinToBootCpu(BootCPURef);
                    KthreaddTask.Event::Preset;
                    KthreaddTask.Event::Setup;
                    KthreaddTask.Event::Enable;
                    KthreaddTask.Action::BindGlobalRef;
                    SystemState.Event::Preset;
                    SystemState.Event::Setup;
                    KthreaddReadyGate.Event::Setup;
                    KthreaddReadyGate.Event::Enable;
                    KthreaddReadyGate.Event::Complete;
                    BootIdlePreemption.Event::EnableNoResched;
                    Scheduler.Action::Schedule;
                }

                ensures {
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_single_online_cpu_at_scheduler_start(RcuCore, CpuGroup);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    rest_init_dispatch_ready(RestInitPhase);
                    kernel_init_task_created(KernelInitTask);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    kthreadd_task_created(KthreaddTask);
                    kthreadd_global_ref_bound(KthreaddTask);
                    kthreadd_provider_ready(KthreaddTask);
                    system_state_scheduling(SystemState);
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
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
                    "KernelInitTask.PinToBootCpu 当前保留 rcu_read_lock()/unlock() 读侧上下文建模问题：它是否属于资源独占上下文，还是应作为 RCU/读侧上下文单独建模，后续讨论。";
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ProcessPreparePhase.state == State::Ready;
            rcu_scheduler_starting_ready(RcuCore);
            rcu_scheduler_active_level_init(RcuCore);
            rcu_gp_seq_baseline_synced(RcuCore);
            KernelInitTask.state == State::Online;
            kernel_init_pf_no_setaffinity(KernelInitTask);
            kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
            KthreaddTask.state == State::Online;
            kthreadd_global_ref_bound(KthreaddTask);
            kthreadd_provider_ready(KthreaddTask);
            SystemState.state == State::Ready;
            KthreaddReadyGate.state == State::Online;
            kthreadd_ready_gate_completed(KthreaddReadyGate);
            kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
            kernel_init_released_for_pre_smp_init(KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            rest_init_dispatch_ready(RestInitPhase);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            rest_init_boot_idle_tail_pending(BootIdleRuntime);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    CpuGroup.state == State::Ready;
                    BootIdleTask.state == State::Ready;
                }

                within BootIdleStartupContext {
                    drives {
                        BootIdleRuntime.Event::Setup;
                        BootIdleRuntime.Action::PrepareIdleEntry;
                        BootIdleRuntime.Action::RunIdleLoop;
                    }

                    ensures {
                        task_preemption_disabled(BootIdleTask);
                        boot_idle_runtime_ready(BootIdleRuntime, BootIdleTask);
                        boot_idle_entry_prepared(BootIdleRuntime, BootIdleTask);
                        boot_idle_task_identity_entered(BootInitTask, BootIdleTask);
                        boot_idle_task_pf_idle(BootIdleTask);
                        boot_idle_runtime_loop_entered(BootIdleRuntime, BootIdleTask);
                        boot_idle_loop_cycle_committed(BootIdleRuntime);
                        boot_idle_loop_continues(BootIdleRuntime);
                    }
                }

                ensures {
                    rest_init_ready(RestInitPhase);
                    up_multitask_runtime_ready(RestInitPhase, KernelInitTask, KthreaddTask, BootIdleRuntime);
                    boot_cpu_idle_runtime_entered(BootIdleRuntime);
                    boot_init_task_runtime_handoff_complete(BootInitTask, BootIdleTask);
                    boot_idle_entry_prepared(BootIdleRuntime, BootIdleTask);
                    boot_idle_task_identity_entered(BootInitTask, BootIdleTask);
                    boot_idle_task_pf_idle(BootIdleTask);
                    boot_idle_runtime_loop_entered(BootIdleRuntime, BootIdleTask);
                    boot_idle_loop_continues(BootIdleRuntime);
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    system_state_scheduling(SystemState);
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    task_concurrency_open();
                    smp_concurrency_closed();
                }

                deferred {
                    "当前只建模 boot idle loop 的抽象主线和一轮代表性 no-need-resched -> need-resched -> schedule_idle -> return-to-idle-cycle；完整 tick/RCU/cpuidle/irq idle 细节后续展开。";
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
            KthreaddTask.state == State::Online;
            SystemState.state == State::Ready;
            KthreaddReadyGate.state == State::Online;
            BootIdleRuntime.state == State::Ready;
            rest_init_ready(RestInitPhase);
            system_state_scheduling(SystemState);
            kernel_init_pf_no_setaffinity(KernelInitTask);
            kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
            kthreadd_ready_gate_completed(KthreaddReadyGate);
            kthreadd_done_release_committed(KthreaddReadyGate, KernelInitTask);
            kernel_init_released_for_pre_smp_init(KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            task_concurrency_open();
            smp_concurrency_closed();
        }
    }
}
