/*
 * Rest Init Phase Specification
 *
 * The UP Multitask rest_init path is split by execution owner:
 * BootInitRestInitPhase creates PID 1/kthreadd and completes kthreadd_done,
 * BootInitScheduleHandoffPhase commits the first schedule handoff, and
 * BootIdleEntryPhase enters the BootTask cpu_startup_entry()/idle-loop
 * continuation. No RestInitPhase wrapper object is modeled; rest_init() stays
 * only as the Linux control-flow name for this owner-split path.
 */

/*
 * rcu_scheduler_starting() 是 RcuCore.Ready 状态内 action。它不推进
 * RcuCore 生命周期，只把 RCU 从 early boot/no-op 模式切到
 * RCU_SCHEDULER_INIT，同步 GP 序号基线，并保持 GP kthread deferred。
 */

lock KernelInitTaskPiLock: RawSpinLock;
lock KthreaddTaskPiLock: RawSpinLock;
lock KthreaddReadyGateWaitLock: RawSpinLock;

context WakeUpNewTaskContext: ResourceExclusiveContext {
    guard {
        lock_ref: KernelInitTaskPiLock;

        entered_by {
            KernelInitTaskPiLock.Transition::LockIrqSave;
        }

        exited_by {
            KernelInitTaskPiLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        KernelInitTask;
        Scheduler;
        BootRunQueue;
    }
}

context WakeUpKthreaddTaskContext: ResourceExclusiveContext {
    guard {
        lock_ref: KthreaddTaskPiLock;

        entered_by {
            KthreaddTaskPiLock.Transition::LockIrqSave;
        }

        exited_by {
            KthreaddTaskPiLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        KthreaddTask;
        Scheduler;
        BootRunQueue;
    }
}

context EnqueueSelectedRunQueueContext: ResourceExclusiveContext {
    /*
     * Per SEM-EXCLUSIVE-CONTEXT-001, nested within uses the standard
     * no-argument form and consumes lexically visible bindings. The selected_rq
     * binding produced by Scheduler.Action::SelectRunQueue is visible inside
     * this context. In the current UP path, selected_rq is proven to target
     * BootRunQueue and BootCPURef, so this context is guarded by
     * BootRunQueueLock and the task CPU update consumes that selected_rq CPU
     * fact as BootCPURef. Future generic runqueue enqueue modeling should
     * resolve the lock, obj_refs and CPU fact from the selected RunQueueRef
     * instead of this BootRunQueue specialization.
     */
    guard {
        lock_ref: BootRunQueueLock;

        entered_by {
            BootRunQueueLock.Transition::LockIrqSave;
        }

        exited_by {
            BootRunQueueLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        BootRunQueue;
    }
}

context KernelInitPidLookupRcuReadSideContext: Context {
    /*
     * rest_init() re-finds PID 1 with find_task_by_pid_ns() while holding an
     * RCU read-side critical section before setting PF_NO_SETAFFINITY and the
     * temporary boot-CPU affinity mask.
     */
    guard {
        lock_ref: BootIdleRcuReadSide;

        entered_by {
            BootIdleRcuReadSide.Transition::ReadLock;
        }

        exited_by {
            BootIdleRcuReadSide.Transition::ReadUnlock;
        }
    }

    obj_refs {
        KernelInitTask;
        RootPidNamespace;
        BootIdleRcuReadSide;
    }
}

context KthreaddPidLookupRcuReadSideContext: Context {
    /*
     * rest_init() publishes kthreadd_task after a second
     * find_task_by_pid_ns() lookup protected by rcu_read_lock()/unlock().
     */
    guard {
        lock_ref: BootIdleRcuReadSide;

        entered_by {
            BootIdleRcuReadSide.Transition::ReadLock;
        }

        exited_by {
            BootIdleRcuReadSide.Transition::ReadUnlock;
        }
    }

    obj_refs {
        KthreaddTask;
        RootPidNamespace;
        BootIdleRcuReadSide;
    }
}

context KthreaddReadyGateWaitLockContext: ResourceExclusiveContext {
    /*
     * complete(&kthreadd_done) takes the completion wait queue lock with
     * irqsave, increments done, wakes one waiter and then unlocks/restores.
     * The generic Completion Type records token and wake-one semantics; this
     * rest_init instance supplies the concrete wait.lock guard.
     */
    guard {
        lock_ref: KthreaddReadyGateWaitLock;

        entered_by {
            KthreaddReadyGateWaitLock.Transition::LockIrqSave;
        }

        exited_by {
            KthreaddReadyGateWaitLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        KthreaddReadyGate;
        KernelInitTask;
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
    guard {
        entered_by {
            BootIdlePreemption.Transition::Disable;
        }

        exited_by {
            BootIdlePreemption.Transition::EnableNoResched;
        }
    }

    obj_refs {
        BootTask;
        Scheduler;
        BootRunQueue;
        BootCpuLocalInterrupt;
    }
}

context ScheduleLocalInterruptContext: Context {
    /*
     * This context models __schedule() disabling local interrupts before
     * taking rq->lock. The guard is backed by BootCpuLocalInterrupt, not by a
     * lock; it establishes only a CPU-local interrupt-disabled boundary. The
     * preemption-disabled and voluntary-switching-disabled facts are inherited
     * from the outer SchedulePreemptionContext.
     */
    guard {
        entered_by {
            BootCpuLocalInterrupt.Transition::SaveAndDisable;
        }

        exited_by {
            BootCpuLocalInterrupt.Transition::Restore;
        }
    }

    obj_refs {
        BootTask;
        Scheduler;
        BootRunQueue;
        BootCpuLocalInterrupt;
    }
}

context ScheduleRunQueueContext: ResourceExclusiveContext {
    /*
     * This context models the rq_lock() region inside __schedule(). Current
     * tooling infers the irq-save spinlock contribution from the BootRunQueueLock
     * LockIrqSave/UnlockIrqRestore boundary;
     * the Linux path has local interrupts already disabled before rq_lock().
     * A narrower runqueue-lock guard can replace this once rq_lock/raw rq lock
     * is modeled separately from irq-save spinlock.
     */
    guard {
        lock_ref: BootRunQueueLock;

        entered_by {
            BootRunQueueLock.Transition::LockIrqSave;
        }

        exited_by {
            BootRunQueueLock.Transition::UnlockIrqRestore;
        }
    }

    obj_refs {
        BootTask;
        Scheduler;
        BootRunQueue;
    }
}

/*
 * Scheduler 的生命周期由 boot/sched-init/phase.spec 中的 Scheduler object
 * 建立：Preset/Setup/Enable 使其进入 Online。rest_init 三子阶段不再推进
 * Scheduler 生命周期，只消费 Scheduler.state == Online，并在
 * BootInitScheduleHandoffPhase 只关闭首次切换的可逆预检与 BootIdleFlow
 * binding；真正的首次 handoff 由其父 BootInitFlow 提交 Online 后，交给
 * BootInitFlow.Enable 提交 Online 后直接发送 Scheduler.Action::Schedule。该 action 自身通过嵌套 within 进入
 * SchedulePreemptionContext -> ScheduleLocalInterruptContext ->
 * ScheduleRunQueueContext，分别覆盖 schedule-owned preempt-disabled guard、
 * __schedule() local-irq-disabled guard 和 rq->lock 独占区。
 *
 * 本文件中的 rest_init 相关同步边界均应保持为 within 或通用 Type process：
 * wake_up_new_task() 路径用 WakeUpNewTaskContext / WakeUpKthreaddTaskContext
 * 包住 p->pi_lock irqsave 区，并在其中嵌套 EnqueueSelectedRunQueueContext
 * 表达 BootRunQueueLock irqsave 区；PID lookup 使用
 * KernelInitPidLookupRcuReadSideContext / KthreaddPidLookupRcuReadSideContext
 * 表达 rcu_read_lock()/unlock() 读侧边界；kthreadd_done 使用
 * KthreaddReadyGateWaitLockContext 包住 Completion Type 的 Complete process；
 * BootIdleStartupContext 覆盖整个 BootIdleEntryPhase。
 */

context BootIdleStartupContext: Context {
    /*
     * This context corresponds to the post-schedule preempt-disabled atomic
     * context established by schedule_preempt_disabled() before
     * cpu_startup_entry(CPUHP_ONLINE). The pre-schedule context is inherited
     * from boot/sched_init and is exited explicitly by
     * BootIdlePreemption.Transition::EnableNoResched in
     * BootInitScheduleHandoffPhase. The normal startup path never exits this
     * post-schedule context; exited_by uses Never.
     *
     * 参照 Linux 的 idle 设计原理，boot CPU 的整个 idle 入口准备和 idle
     * loop 都运行在抢占关闭上下文中，避免 idle/current task、polling、
     * nohz 和 need_resched 等 CPU 本地状态被普通抢占打断后出现不一致。
     * idle task 不通过普通抢占被动切出；它只在本 CPU idle loop 观察到
     * need_resched 后，主动进入 schedule_idle()/scheduler 调度边界。
     */
    guard {
        entered_by {
            BootIdlePreemption.Transition::Disable;
        }

        exited_by {
            Never;
        }
    }

    obj_refs {
        BootTask;
        BootIdleFlow;
        Scheduler;
    }
}

context BootIdleWaitLocalInterruptContext: Context {
    /*
     * do_idle() sets polling and enters tick_nohz_idle_enter(), then each
     * representative !need_resched wait iteration disables local interrupts
     * before arch_cpu_idle_enter()/cpuidle path and keeps that closed until
     * the idle handler returns. This context captures that CPU-local IRQ
     * disabled window; full cpuidle/poll/WFI details remain deferred.
     */
    guard {
        entered_by {
            BootCpuLocalInterrupt.Transition::SaveAndDisable;
        }

        exited_by {
            BootCpuLocalInterrupt.Transition::Restore;
        }
    }

    obj_refs {
        BootTask;
        BootIdleFlow;
        BootCpuLocalInterrupt;
    }
}

/*
 * KernelInitTask 表示 user_mode_thread(kernel_init, NULL, CLONE_FS) 创建的
 * PID 1。它在本阶段变为 Online，但其 kernel_init_freeable() 执行属于下一子阶段。
 *
 * KernelInitTask.Preset 对应 user_mode_thread() 构造临时 kernel_clone_args
 * 并由 kernel_clone() 完成 copy_process() 的成功路径。kernel_clone_args
 * 是栈上传参结构，没有独立生命周期，因此不建模为对象；Preset 先把
 * fn、fn_arg、clone flags、exit_signal 与 KernelInitFlow 固化为结构 facts，
 * 再执行 TaskCreationCore.Ready 状态内的参数化 action：
 * TaskCreationCore.Action::CopyProcess(src_task: BootTask,
 * dst_task: KernelInitTask, flow: KernelInitFlow, ...)。该 action
 * 以 src_task/current 为模板创建 dst_task/task_struct，初始化 pid、凭据、
 * fs/files、signal、安全上下文、thread context、sched entity 与启动入口，
 * 但保持 task_state_new 且 task_not_enqueued，随后提交 Prepared。
 * KernelInitTask.Setup 当前没有业务动作，只提交 Prepared -> Ready。
 *
 * KernelInitTask.Enable 对应 wake_up_new_task()。该路径受 p->pi_lock 保护，
 * 因此 Enable 先通过 KernelInitTaskPiLock.LockIrqSave 进入
 * WakeUpNewTaskContext，再在该独占上下文内执行受保护资源动作；退出时
 * 通过 KernelInitTaskPiLock.UnlockIrqRestore 恢复本地中断和当前任务抢占。
 * 上下文通过 KernelInitTaskPiLock 建立边界，引用 KernelInitTask、
 * Scheduler、BootRunQueue 三个受保护对象。
 * Enable 的 within WakeUpNewTaskContext 块直接驱动
 * Task.Transition::SetRuntimeState(Running)，再通过 action result binding 把
 * Scheduler.Action::SelectRunQueue(KernelInitTaskRef) 返回的
 * runqueue ref 绑定为 selected_rq。随后用标准无实参 within 进入
 * EnqueueSelectedRunQueueContext；selected_rq 作为外层 action result
 * binding 在嵌套 within 中直接可见。当前 UP 路径证明 selected_rq
 * 指向 BootRunQueue 且 CPU 事实为 BootCPURef，因此该 context 仍由
 * BootRunQueueLock 建立边界，Task.SetTaskCpu 消费 selected_rq 的 CPU 事实，
 * 并驱动 RunQueue.Transition::EnqueueTask(KernelInitTaskRef)。SelectRunQueue
 * 当前固定返回 BootRunQueueRef；完整选择策略后续 deferred。三者都成功后，
 * Enable 才提交 KernelInitTask Ready -> Online。
 *
 * KernelInitTask.PinToBootCpu 对应 rest_init() 随后的 PF_NO_SETAFFINITY
 * 与 set_cpus_allowed_ptr(tsk, cpumask_of(smp_processor_id()))。它不是独立
 * lifecycle object，而是 KernelInitTask 的属性 action；源码中的
 * find_task_by_pid_ns(pid, &init_pid_ns) 只是用 pid 重新取回 task 指针，
 * 规格层已经通过 KernelInitTask receiver 持有目标 task。该 action 当前
 * 直接提交 flags 和 cpumask 属性，并用 KernelInitPidLookupRcuReadSideContext
 * 表达 Linux rcu_read_lock()/unlock() 定界的读侧上下文。
 *
 * KernelInitFlow.ObserveKthreaddDoneRelease 对应 PID 1 执行线在
 * kernel_init() 入口处的 wait_for_completion(&kthreadd_done) 返回。
 * BootTask 只通过 KthreaddReadyGate.Complete 发布 completion token/wake
 * 事实；KernelInitFlow 必须通过 KthreaddReadyGate.Wait 观察或消费该
 * completion，才提交 kernel_init_released_for_pre_smp_init() 并进入
 * PreSmpInitPhase 主体。这样同时覆盖 complete 早于 wait 的 fast observe
 * 路径和 wait 早于 complete 的阻塞后返回语义；当前线性启动 trace 采用前者。
 */


/*
 * KernelInitKthreaddDoneWait 是 PID 1 执行线上的 kthreadd_done wait 边界。
 * BootTask 只创建 PID 1、发布 KthreaddReadyGate completion token，并把
 * 该 wait 边界留在 Ready；KernelInitTask 被首次调度后，在 PreSmpInitPhase
 * 开头通过 Enable 观察 completion 并推进为 Online。这样 complete side 和
 * wait side 的事实分属两个任务：complete 建立 gate 已释放，wait 观察 gate
 * 已释放并产生 kernel_init_released_for_pre_smp_init()。
 */
object KernelInitKthreaddDoneWait: KernelObject {
    initial_state: State::Base;
    parent: KernelInitTask;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    KthreaddReadyGate.state == State::Online;
                }

                ensures {
                    kernel_init_kthreadd_done_wait_ready(
                        KernelInitKthreaddDoneWait,
                        KernelInitTask,
                        KthreaddReadyGate
                    );
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            KernelInitTask.state == State::Online;
            KthreaddReadyGate.state == State::Online;
            kernel_init_kthreadd_done_wait_ready(
                KernelInitKthreaddDoneWait,
                KernelInitTask,
                KthreaddReadyGate
            );
            kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    KthreaddReadyGate.state == State::Online;
                    completion_complete_committed(KthreaddReadyGate);
                    completion_token_available(KthreaddReadyGate);
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                }

                drives {
                    KernelInitFlow.Action::ObserveKthreaddDoneRelease;
                }

                ensures {
                    kernel_init_kthreadd_done_wait_released(
                        KernelInitKthreaddDoneWait,
                        KernelInitTask,
                        KthreaddReadyGate
                    );
                    kernel_init_observed_kthreadd_done_release(KernelInitTask, KthreaddReadyGate);
                    kernel_init_released_for_pre_smp_init(KernelInitTask);
                }
            }
        }
    }

    state State::Online {
        invariant {
            kernel_init_kthreadd_done_wait_released(
                KernelInitKthreaddDoneWait,
                KernelInitTask,
                KthreaddReadyGate
            );
            kernel_init_observed_kthreadd_done_release(KernelInitTask, KthreaddReadyGate);
            kernel_init_released_for_pre_smp_init(KernelInitTask);
        }
    }
}

/*
 * KthreaddTask（定义集中在 task.spec）表示
 * kernel_thread(kthreadd, NULL, CLONE_FS | CLONE_FILES)
 * 创建的全局内核线程管理者。它复用 KernelInitTask 的创建/唤醒形态：
 * Preset 固化临时 kernel_clone_args 并驱动 copy_process，Setup 当前没有
 * 业务动作，Enable 对应 wake_up_new_task。区别在于入口为 kthreadd、flags 包含
 * kernel_thread 固有的 CLONE_VM | CLONE_UNTRACED 以及传入的
 * CLONE_FS | CLONE_FILES，并且创建和唤醒后还要发布 kthreadd_task
 * 全局 provider 引用。KthreaddTask 的第一执行入口不是 kernel_init 线；
 * 当前模型只把 kthreadd 入口抽象为一个服务循环边界：进入后反复等待
 * kthread 请求，并在无可运行工作时调用 schedule() 尝试切出。
 */


/*
 * SystemState 表示 Linux 全局 system_state 枚举。生命周期 Ready 对应本阶段
 * 把内部值推进到 SYSTEM_SCHEDULING；后续 FinalizePhase 先在 Ready 生命周期内
 * 记录 SYSTEM_FREEING_INITMEM 窗口，再通过 Enable 推进到 SYSTEM_RUNNING / Online。
 */
object SystemState: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示 system_state 尚未在本阶段重新确认启动中状态。
     */
    state State::Base {
        transitions {
            /*
             * Preset 记录 SYSTEM_BOOTING 仍是当前全局系统状态。
             */
            on Transition::Preset -> State::Prepared {
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

        transitions {
            /*
             * Setup 对应 rest_init() 中 system_state = SYSTEM_SCHEDULING。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                }

                ensures {
                    system_state_scheduling(SystemState);
                    boot_init_scheduling_open();
                    smp_concurrency_closed();
                }
            }
        }
    }

    /*
     * Ready 表示系统已进入 SYSTEM_SCHEDULING，UP 多任务调度边界打开。
     * FinalizePhase 可在该生命周期内把内部值推进到 SYSTEM_FREEING_INITMEM，
     * 但只有 Enable 会把对象生命周期推进到 Online。
     */
    state State::Ready {
        invariant {
            system_state_scheduling(SystemState);
            boot_init_scheduling_open();
            smp_concurrency_closed();
        }

        transitions {
            /*
             * Enable 预留给 FinalizePhase 将 system_state 从
             * SYSTEM_FREEING_INITMEM 推进到 SYSTEM_RUNNING。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    system_state_freeing_initmem_window_entered(SystemState);
                    InitMemoryCleanupDeferred.state == State::Ready;
                    KernelMappingProtectionDeferred.state == State::Ready;
                    PtiFinalizeTrimmed.state == State::Ready;
                }

                ensures {
                    system_state_running(SystemState);
                    boot_init_scheduling_open();
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
            boot_init_scheduling_open();
            smp_concurrency_open(CpuGroup);
        }
    }

    actions {
        /*
         * EnterFreeingInitmem 对应 kernel_init() 中 async_synchronize_full()
         * 之后、free_initmem() 之前的 system_state =
         * SYSTEM_FREEING_INITMEM 赋值；它不改变对象生命周期，只改变
         * SystemState 的内部枚举值并发布 paired checkpoint。
         */
        Action::EnterFreeingInitmem {
            state_effect: StateEffect::None;
            depends_on {
                SystemState.state == State::Ready;
                AsyncFullSyncDeferred.state == State::Ready;
                system_state_scheduling(SystemState);
            }

            ensures {
                system_state_freeing_initmem_window_entered(SystemState);
                boot_init_scheduling_open();
                smp_concurrency_closed();
            }
        }
    }
}

/*
 * KthreaddReadyGate 表示静态 completion kthreadd_done，是 Completion Type
 * 的具名实例。Completion 的 token、wait_queue 和 wake-one 行为必须由
 * Completion Type process 承载，实例不复制这些通用 facts。下面的对象
 * lifecycle event 只是当前工具的实例 wrapper：Setup/Enable 提交
 * kthreadd_done 在 rest_init 场景中的 gate 生命周期状态；真正的
 * complete(&kthreadd_done) 表达为 KthreaddReadyGate.Transition::Complete。
 * Completion 通用结果来自 Type process；释放 PID 1 进入下一执行线的
 * 场景事实由 KernelInitFlow.Action::ObserveKthreaddDoneRelease 在 wait side
 * 承载，避免让 BootTask 的 complete 动作代替 PID 1 的 wait 返回。
 * 该实例的 wait.lock irqsave 边界由 KthreaddReadyGateWaitLockContext
 * 包住 Complete process，避免把 completion 内部锁误表达为生命周期。
 */
object KthreaddReadyGate: Completion {
    initial_state: State::Base;

    /*
     * Base 表示 kthreadd_done completion 尚未建模为 pending 门。
     */
    state State::Base {
        transitions {
            /*
             * Setup 建立 kthreadd_done pending 门，PID 1 仍被阻塞在等待点。
             */
            on Transition::Setup -> State::Ready {
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

        transitions {
            /*
             * Enable 发布 completion handle，使其可被运行期 Complete process 操作。
             */
            on Transition::Enable -> State::Online {
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
 * BootIdleFlow（定义集中在 task_flow.spec）表示分叉点之后
 * cpu_startup_entry() 确认 boot CPU idle
 * runtime 入口。它复用 SchedInitPhase 已建立的 BootTask，并抽象 Linux
 * cpu_startup_entry() -> do_idle() -> schedule_idle() 的循环主线：boot CPU
 * 先执行 current->flags |= PF_IDLE、arch_cpu_idle_prepare() 和
 * cpuhp_online_idle(CPUHP_ONLINE)，然后进入 while (1) do_idle()；当本 CPU
 * 在 do_idle() 中观察到 need_resched 时，驱动 idle 专用调度。当前实现可先
 * 在不可逆切换前建立首个 owner/active binding；若 KernelInitTask 后续
 * 切回，BootTask 才从该 continuation 继续并启动 BootIdleEntryPhase。
 */


/*
 * BootInitRestInitPhase 是 BootTask 视角下 rest_init() 的前半段。
 * 它创建并唤醒 PID 1 和 kthreadd，发布 SYSTEM_SCHEDULING，完成
 * kthreadd_done。PID 1 何时通过 wait 观察该 completion 并解除等待，
 * 属于 KernelInitTask 自己的执行线。它不提交首次 scheduler handoff，也不
 * 执行 kthreadd 或 boot idle 自己的子阶段。
 */
object BootInitRestInitPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    ProcessPreparePhase.state == State::Online;
                    InterruptStream.state == State::Online;
                    TaskCreationCore.state == State::Ready;
                    RootPidNamespace.state == State::Ready;
                    CredentialCore.state == State::Prepared;
                    SignalCore.state == State::Prepared;
                    TaskFileContext.state == State::Prepared;
                    SecurityCore.state == State::Ready;
                    Scheduler.state == State::Online;
                    BootRunQueue.state == State::Ready;
                    BootIdleSetup.state == State::Ready;
                    RcuCore.state == State::Ready;
                    CpuGroup.state == State::Ready;
                    smp_concurrency_closed();
                }

                drives {
                    RcuCore.Action::SchedulerStarting;
                    KernelInitTask.Transition::Preset(
                        parent_task: BootTask,
                        task_ref: KernelInitTaskRef,
                        initial_flow: KernelInitFlow
                    );
                    TaskCreationCore.Action::CopyProcess(
                        src_task: BootTask,
                        src_task_ref: BootTaskRef,
                        current_task_ref: CurrentTaskRef,
                        dst_task: KernelInitTask,
                        pid_ns: RootPidNamespace,
                        creds: CredentialCore,
                        signal: SignalCore,
                        files: TaskFileContext,
                        security: SecurityCore,
                        scheduler: Scheduler,
                        flow: KernelInitFlow
                    );
                    KernelInitTask.Transition::Setup(
                        parent_task: BootTask,
                        pid_ns: RootPidNamespace,
                        scheduler: Scheduler,
                        initial_flow: KernelInitFlow
                    );
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
                        KernelInitTask.Transition::SetRuntimeState(
                            TaskRuntimeState::Running
                        );
                        let selected_rq: RunQueueRef <-
                            Scheduler.Action::SelectRunQueue(KernelInitTaskRef);
                        KernelInitTask.Action::SetTaskCpu(BootCPURef);
                    }

                    within EnqueueSelectedRunQueueContext {
                        depends_on {
                            runqueue_ref_targets(
                                selected_rq,
                                BootRunQueue
                            );
                            runqueue_ref_cpu_is(
                                selected_rq,
                                BootCPURef
                            );
                            task_cpu_ref_is(KernelInitTask, BootCPURef);
                        }

                        drives {
                            selected_rq.Transition::EnqueueTask(
                                KernelInitTaskRef
                            );
                        }

                        ensures {
                            raw_spinlock_irqsave_entered(
                                BootRunQueueLock,
                                BootCurrentCPU
                            );
                            raw_spinlock_irqrestore_exited(
                                BootRunQueueLock,
                                BootCurrentCPU
                            );
                            runqueue_contains_task(
                                BootRunQueue,
                                KernelInitTaskRef
                            );
                        }
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(
                            KernelInitTaskPiLock,
                            BootCurrentCPU
                        );
                        raw_spinlock_irqrestore_exited(
                            KernelInitTaskPiLock,
                            BootCurrentCPU
                        );
                        scheduler_select_runqueue_returns(
                            Scheduler,
                            KernelInitTaskRef,
                            BootRunQueueRef
                        );
                        task_runqueue_selected(
                            Scheduler,
                            KernelInitTaskRef,
                            BootRunQueueRef
                        );
                        task_cpu_ref_is(KernelInitTask, BootCPURef);
                        task_enqueued_on_runqueue(
                            KernelInitTaskRef,
                            BootRunQueueRef
                        );
                        task_wakeup_new_rq_clock_updated(
                            KernelInitTask,
                            BootRunQueue
                        );
                        task_wakeup_new_initial_util_avg_posted(
                            KernelInitTask,
                            BootRunQueue
                        );
                        task_wakeup_new_trace_emitted(KernelInitTask);
                        task_wakeup_new_preempt_check_done(
                            KernelInitTask,
                            BootRunQueue
                        );
                        task_wakeup_new_task_woken_hook_deferred(KernelInitTask);
                        task_state_running(KernelInitTask);
                        task_runqueue_publication_committed(KernelInitTask);
                        task_at_most_one_flow_online(KernelInitTask);
                        task_initial_flow_is(KernelInitTask, KernelInitFlow);
                        task_initial_flow_binding_consistent(KernelInitTask);
                        KernelInitFlow.state == State::Base;
                    }
                }

                drives {
                    KernelInitTask.Transition::Enable;
                }

                within KernelInitPidLookupRcuReadSideContext {
                    drives {
                        KernelInitTask.Action::PinToBootCpu(BootCPURef);
                    }

                    ensures {
                        rcu_read_side_entered(BootIdleRcuReadSide, BootCurrentCPU);
                        rcu_read_side_exited(BootIdleRcuReadSide, BootCurrentCPU);
                        task_pid_lookup_under_rcu_read(
                            KernelInitTask,
                            RootPidNamespace,
                            BootIdleRcuReadSide
                        );
                        task_pid_lookup_rcu_guard_used(
                            KernelInitTask,
                            BootIdleRcuReadSide
                        );
                        kernel_init_pf_no_setaffinity(KernelInitTask);
                        kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    }
                }

                drives {
                    KthreaddTask.Transition::Preset(
                        parent_task: BootTask,
                        task_ref: KthreaddTaskRef,
                        initial_flow: KthreaddFlow
                    );
                    TaskCreationCore.Action::CopyProcess(
                        src_task: BootTask,
                        src_task_ref: BootTaskRef,
                        current_task_ref: CurrentTaskRef,
                        dst_task: KthreaddTask,
                        pid_ns: RootPidNamespace,
                        creds: CredentialCore,
                        signal: SignalCore,
                        files: TaskFileContext,
                        security: SecurityCore,
                        scheduler: Scheduler,
                        flow: KthreaddFlow
                    );
                    KthreaddTask.Transition::Setup(
                        parent_task: BootTask,
                        pid_ns: RootPidNamespace,
                        scheduler: Scheduler,
                        initial_flow: KthreaddFlow
                    );
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
                        KthreaddTask.Transition::SetRuntimeState(
                            TaskRuntimeState::Running
                        );
                        let selected_rq: RunQueueRef <-
                            Scheduler.Action::SelectRunQueue(KthreaddTaskRef);
                        KthreaddTask.Action::SetTaskCpu(BootCPURef);
                    }

                    within EnqueueSelectedRunQueueContext {
                        depends_on {
                            runqueue_ref_targets(
                                selected_rq,
                                BootRunQueue
                            );
                            runqueue_ref_cpu_is(selected_rq, BootCPURef);
                            task_cpu_ref_is(KthreaddTask, BootCPURef);
                        }

                        drives {
                            selected_rq.Transition::EnqueueTask(
                                KthreaddTaskRef
                            );
                        }

                        ensures {
                            raw_spinlock_irqsave_entered(
                                BootRunQueueLock,
                                BootCurrentCPU
                            );
                            raw_spinlock_irqrestore_exited(
                                BootRunQueueLock,
                                BootCurrentCPU
                            );
                            runqueue_contains_task(
                                BootRunQueue,
                                KthreaddTaskRef
                            );
                        }
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(
                            KthreaddTaskPiLock,
                            BootCurrentCPU
                        );
                        raw_spinlock_irqrestore_exited(
                            KthreaddTaskPiLock,
                            BootCurrentCPU
                        );
                        scheduler_select_runqueue_returns(
                            Scheduler,
                            KthreaddTaskRef,
                            BootRunQueueRef
                        );
                        task_runqueue_selected(
                            Scheduler,
                            KthreaddTaskRef,
                            BootRunQueueRef
                        );
                        task_cpu_ref_is(KthreaddTask, BootCPURef);
                        task_enqueued_on_runqueue(
                            KthreaddTaskRef,
                            BootRunQueueRef
                        );
                        task_wakeup_new_rq_clock_updated(
                            KthreaddTask,
                            BootRunQueue
                        );
                        task_wakeup_new_initial_util_avg_posted(
                            KthreaddTask,
                            BootRunQueue
                        );
                        task_wakeup_new_trace_emitted(KthreaddTask);
                        task_wakeup_new_preempt_check_done(
                            KthreaddTask,
                            BootRunQueue
                        );
                        task_wakeup_new_task_woken_hook_deferred(KthreaddTask);
                        task_state_running(KthreaddTask);
                        task_runqueue_publication_committed(KthreaddTask);
                        task_at_most_one_flow_online(KthreaddTask);
                        task_initial_flow_is(KthreaddTask, KthreaddFlow);
                        task_initial_flow_binding_consistent(KthreaddTask);
                        KthreaddFlow.state == State::Base;
                    }
                }

                drives {
                    KthreaddTask.Transition::Enable;
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
                        task_pid_lookup_rcu_guard_used(
                            KthreaddTask,
                            BootIdleRcuReadSide
                        );
                        kthreadd_global_ref_bound(KthreaddTask);
                        kthreadd_provider_ref_targets(
                            KthreaddTaskRef,
                            KthreaddTask
                        );
                        kthreadd_provider_ready(KthreaddTask);
                    }
                }

                drives {
                    SystemState.Transition::Preset;
                    SystemState.Transition::Setup;
                    KthreaddReadyGate.Transition::Setup;
                    KthreaddReadyGate.Transition::Enable;
                    KernelInitKthreaddDoneWait.Transition::Setup;
                }

                within KthreaddReadyGateWaitLockContext {
                    drives {
                        KthreaddReadyGate.Transition::Complete;
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(KthreaddReadyGateWaitLock, BootCurrentCPU);
                        raw_spinlock_irqrestore_exited(KthreaddReadyGateWaitLock, BootCurrentCPU);
                        completion_wait_lock_irqsave_entered(KthreaddReadyGate, BootCurrentCPU);
                        completion_wait_lock_irqrestore_exited(KthreaddReadyGate, BootCurrentCPU);
                        completion_done_increment_guarded_by_wait_lock(KthreaddReadyGate);
                        completion_wake_guarded_by_wait_lock(KthreaddReadyGate);
                        completion_wait_lock_guard_used(
                            KthreaddReadyGate,
                            KthreaddReadyGateWaitLock
                        );
                    }
                }

                ensures {
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_single_online_cpu_at_scheduler_start(RcuCore, CpuGroup);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    rcu_scheduler_starting_local_irq_guard_used(RcuCore, BootCpuLocalInterrupt);
                    rcu_scheduler_starting_gp_seq_update_guarded(RcuCore);
                    boot_init_rest_init_ready(BootInitRestInitPhase);
                    rest_init_dispatch_ready(BootInitRestInitPhase);
                    KernelInitKthreaddDoneWait.state == State::Ready;
                    kernel_init_kthreadd_done_wait_ready(
                        KernelInitKthreaddDoneWait,
                        KernelInitTask,
                        KthreaddReadyGate
                    );
                    kernel_init_task_created(KernelInitTask);
                    kernel_init_spawn_spec_ready(KernelInitTask);
                    kernel_init_entry_selected(KernelInitTask);
                    kernel_init_clone_fs_flag_set(KernelInitTask);
                    kernel_init_not_user_mm_yet(KernelInitTask);
                    kernel_init_task_ready(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_thread_context_ready(KernelInitTask);
                    kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
                    kernel_init_waits_for_kthreadd_done(KernelInitTask);
                    kernel_init_task_online(KernelInitTask);
                    kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    task_pid_lookup_rcu_guard_used(KernelInitTask, BootIdleRcuReadSide);
                    kthreadd_task_created(KthreaddTask);
                    kthreadd_spawn_spec_ready(KthreaddTask);
                    kthreadd_entry_selected(KthreaddTask);
                    kthreadd_clone_fs_files_flags_set(KthreaddTask);
                    kthreadd_clone_vm_flag_set(KthreaddTask);
                    kthreadd_clone_untraced_flag_set(KthreaddTask);
                    kthreadd_kernel_thread_flag_set(KthreaddTask);
                    kthreadd_is_kernel_thread_provider(KthreaddTask);
                    kthreadd_task_ready(KthreaddTask);
                    kthreadd_task_pid_allocated(KthreaddTask, RootPidNamespace);
                    kthreadd_thread_context_ready(KthreaddTask);
                    kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
                    kthreadd_task_online(KthreaddTask);
                    kthreadd_task_enqueued(KthreaddTask, BootRunQueue);
                    task_owns_flow(KthreaddTask, KthreaddFlow);
                    KthreaddFlow.state == State::Base;
                    kthreadd_schedule_loop_deferred_until_on_cpu(
                        KthreaddTask,
                        KthreaddFlow
                    );
                    kthreadd_global_ref_bound(KthreaddTask);
                    kthreadd_provider_ref_targets(KthreaddTaskRef, KthreaddTask);
                    kthreadd_provider_ready(KthreaddTask);
                    task_pid_lookup_rcu_guard_used(KthreaddTask, BootIdleRcuReadSide);
                    system_state_scheduling(SystemState);
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    completion_complete_committed(KthreaddReadyGate);
                    completion_token_available(KthreaddReadyGate);
                    completion_wait_lock_guard_used(
                        KthreaddReadyGate,
                        KthreaddReadyGateWaitLock
                    );
                    completion_done_increment_guarded_by_wait_lock(KthreaddReadyGate);
                    completion_wake_guarded_by_wait_lock(KthreaddReadyGate);
                    smp_concurrency_closed();
                    workqueue_workers_still_deferred();
                    rcu_gp_threads_still_deferred(RcuCore);
                }

                deferred kthreadd.001 {
                    category: DeferredCategory::Feature;
                    summary: "Complete kthreadd request consumption, completion, wait, park, stop and long-running service semantics.";
                    evidence { kthreadd_schedule_loop_deferred_until_on_cpu(KthreaddTask, KthreaddFlow); }
                    close_when: "kthread request creation/consumption, wait/park/stop and sustained service tests pass.";
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
                    ProcessPreparePhase.state == State::Online;
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    rcu_scheduler_starting_local_irq_guard_used(RcuCore, BootCpuLocalInterrupt);
                    rcu_scheduler_starting_gp_seq_update_guarded(RcuCore);
                    boot_init_rest_init_ready(BootInitRestInitPhase);
                    rest_init_dispatch_ready(BootInitRestInitPhase);
                    KernelInitTask.state == State::Online;
                    kernel_init_spawn_spec_ready(KernelInitTask);
                    kernel_init_entry_selected(KernelInitTask);
                    kernel_init_clone_fs_flag_set(KernelInitTask);
                    kernel_init_not_user_mm_yet(KernelInitTask);
                    kernel_init_task_ready(KernelInitTask);
                    kernel_init_task_pid_is_one(KernelInitTask);
                    kernel_init_thread_context_ready(KernelInitTask);
                    kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
                    kernel_init_waits_for_kthreadd_done(KernelInitTask);
                    kernel_init_task_online(KernelInitTask);
                    kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    KthreaddTask.state == State::Online;
                    kthreadd_spawn_spec_ready(KthreaddTask);
                    kthreadd_entry_selected(KthreaddTask);
                    kthreadd_clone_fs_files_flags_set(KthreaddTask);
                    kthreadd_clone_vm_flag_set(KthreaddTask);
                    kthreadd_clone_untraced_flag_set(KthreaddTask);
                    kthreadd_kernel_thread_flag_set(KthreaddTask);
                    kthreadd_is_kernel_thread_provider(KthreaddTask);
                    kthreadd_task_ready(KthreaddTask);
                    kthreadd_task_pid_allocated(KthreaddTask, RootPidNamespace);
                    kthreadd_thread_context_ready(KthreaddTask);
                    kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
                    kthreadd_task_online(KthreaddTask);
                    kthreadd_task_enqueued(KthreaddTask, BootRunQueue);
                    task_owns_flow(KthreaddTask, KthreaddFlow);
                    KthreaddFlow.state == State::Base;
                    kthreadd_schedule_loop_deferred_until_on_cpu(
                        KthreaddTask,
                        KthreaddFlow
                    );
                    kthreadd_global_ref_bound(KthreaddTask);
                    kthreadd_provider_ref_targets(KthreaddTaskRef, KthreaddTask);
                    kthreadd_provider_ready(KthreaddTask);
                    SystemState.state == State::Ready;
                    KthreaddReadyGate.state == State::Online;
                    KernelInitKthreaddDoneWait.state == State::Ready;
                    kernel_init_kthreadd_done_wait_ready(
                        KernelInitKthreaddDoneWait,
                        KernelInitTask,
                        KthreaddReadyGate
                    );
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    completion_complete_committed(KthreaddReadyGate);
                    completion_wait_lock_guard_used(
                        KthreaddReadyGate,
                        KthreaddReadyGateWaitLock
                    );
                    completion_done_increment_guarded_by_wait_lock(KthreaddReadyGate);
                    completion_wake_guarded_by_wait_lock(KthreaddReadyGate);
                    smp_concurrency_closed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ProcessPreparePhase.state == State::Online;
            rcu_scheduler_starting_ready(RcuCore);
            rcu_scheduler_active_level_init(RcuCore);
            rcu_gp_seq_baseline_synced(RcuCore);
            rcu_scheduler_starting_local_irq_guard_used(RcuCore, BootCpuLocalInterrupt);
            rcu_scheduler_starting_gp_seq_update_guarded(RcuCore);
            KernelInitTask.state == State::Online;
            kernel_init_spawn_spec_ready(KernelInitTask);
            kernel_init_entry_selected(KernelInitTask);
            kernel_init_clone_fs_flag_set(KernelInitTask);
            kernel_init_not_user_mm_yet(KernelInitTask);
            kernel_init_task_ready(KernelInitTask);
            kernel_init_task_pid_is_one(KernelInitTask);
            kernel_init_thread_context_ready(KernelInitTask);
            kernel_init_sched_entity_ready(KernelInitTask, Scheduler);
            kernel_init_waits_for_kthreadd_done(KernelInitTask);
            kernel_init_still_waiting_for_kthreadd_done(KernelInitTask);
            kernel_init_task_online(KernelInitTask);
            kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
            task_owns_flow(KernelInitTask, KernelInitFlow);
            kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
            kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
            kernel_init_pf_no_setaffinity(KernelInitTask);
            kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
            KthreaddTask.state == State::Online;
            kthreadd_spawn_spec_ready(KthreaddTask);
            kthreadd_entry_selected(KthreaddTask);
            kthreadd_clone_fs_files_flags_set(KthreaddTask);
            kthreadd_clone_vm_flag_set(KthreaddTask);
            kthreadd_clone_untraced_flag_set(KthreaddTask);
            kthreadd_kernel_thread_flag_set(KthreaddTask);
            kthreadd_is_kernel_thread_provider(KthreaddTask);
            kthreadd_task_ready(KthreaddTask);
            kthreadd_task_pid_allocated(KthreaddTask, RootPidNamespace);
            kthreadd_thread_context_ready(KthreaddTask);
            kthreadd_sched_entity_ready(KthreaddTask, Scheduler);
            kthreadd_task_online(KthreaddTask);
            kthreadd_task_enqueued(KthreaddTask, BootRunQueue);
            task_owns_flow(KthreaddTask, KthreaddFlow);
            KthreaddFlow.state == State::Base;
            kthreadd_schedule_loop_deferred_until_on_cpu(
                KthreaddTask,
                KthreaddFlow
            );
            kthreadd_global_ref_bound(KthreaddTask);
            kthreadd_provider_ref_targets(KthreaddTaskRef, KthreaddTask);
            kthreadd_provider_ready(KthreaddTask);
            SystemState.state == State::Ready;
            KthreaddReadyGate.state == State::Online;
            KernelInitKthreaddDoneWait.state == State::Ready;
            kernel_init_kthreadd_done_wait_ready(
                KernelInitKthreaddDoneWait,
                KernelInitTask,
                KthreaddReadyGate
            );
            kthreadd_ready_gate_completed(KthreaddReadyGate);
            completion_complete_committed(KthreaddReadyGate);
            completion_wait_lock_guard_used(KthreaddReadyGate, KthreaddReadyGateWaitLock);
            completion_done_increment_guarded_by_wait_lock(KthreaddReadyGate);
            completion_wake_guarded_by_wait_lock(KthreaddReadyGate);
            boot_init_rest_init_ready(BootInitRestInitPhase);
            rest_init_dispatch_ready(BootInitRestInitPhase);
            smp_concurrency_closed();
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    ProcessPreparePhase.state == State::Online;
                    rcu_scheduler_starting_ready(RcuCore);
                    rcu_scheduler_active_level_init(RcuCore);
                    rcu_gp_seq_baseline_synced(RcuCore);
                    rcu_scheduler_starting_local_irq_guard_used(RcuCore, BootCpuLocalInterrupt);
                    rcu_scheduler_starting_gp_seq_update_guarded(RcuCore);
                    boot_init_rest_init_ready(BootInitRestInitPhase);
                    rest_init_dispatch_ready(BootInitRestInitPhase);
                    KernelInitTask.state == State::Online;
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_pf_no_setaffinity(KernelInitTask);
                    kernel_init_pinned_to_boot_cpu(KernelInitTask, BootCPU);
                    KthreaddTask.state == State::Online;
                    task_owns_flow(KthreaddTask, KthreaddFlow);
                    KthreaddFlow.state == State::Base;
                    kthreadd_schedule_loop_deferred_until_on_cpu(
                        KthreaddTask,
                        KthreaddFlow
                    );
                    kthreadd_global_ref_bound(KthreaddTask);
                    kthreadd_provider_ready(KthreaddTask);
                    SystemState.state == State::Ready;
                    KthreaddReadyGate.state == State::Online;
                    KernelInitKthreaddDoneWait.state == State::Ready;
                    kernel_init_kthreadd_done_wait_ready(
                        KernelInitKthreaddDoneWait,
                        KernelInitTask,
                        KthreaddReadyGate
                    );
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    completion_complete_committed(KthreaddReadyGate);
                    completion_token_available(KthreaddReadyGate);
                    completion_wait_lock_guard_used(
                        KthreaddReadyGate,
                        KthreaddReadyGateWaitLock
                    );
                    completion_done_increment_guarded_by_wait_lock(KthreaddReadyGate);
                    completion_wake_guarded_by_wait_lock(KthreaddReadyGate);
                    smp_concurrency_closed();
                }
            }
        }
    }

    state State::Online {
    }
}

/*
 * BootInitScheduleHandoffPhase 仍由 BootTask 执行。它只表示
 * schedule_preempt_disabled() 中退出 inherited preempt-disabled guard、建立
 * BootIdleFlow owner/active binding 并完成首次 scheduler handoff 的可逆预检。
 * 真正的 task stack switch 不属于本阶段。
 */
object BootInitScheduleHandoffPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootInitRestInitPhase.state == State::Online;
                    KernelInitTask.state == State::Online;
                    KthreaddTask.state == State::Online;
                    SystemState.state == State::Ready;
                    system_state_scheduling(SystemState);
                    KthreaddReadyGate.state == State::Online;
                    kthreadd_ready_gate_completed(KthreaddReadyGate);
                    completion_complete_committed(KthreaddReadyGate);
                    Scheduler.state == State::Online;
                    BootIdleSetup.state == State::Ready;
                }

                drives {
                    BootIdlePreemption.Transition::EnableNoResched;
                    BootIdleFlow.Transition::Setup;
                }

                ensures {
                    boot_init_schedule_handoff_ready(BootInitScheduleHandoffPhase);
                    BootIdleFlow.state == State::Ready;
                    task_owns_flow(BootTask, BootIdleFlow);
                    task_flow_owner_is(BootIdleFlow, BootTask);
                    task_flow_owner_exclusive(BootIdleFlow);
                    task_active_flow_is(BootTask, BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    scheduler_switch_prepare_validates_prev_live_active_flow(
                        Scheduler,
                        CurrentTaskRef
                    );
                    scheduler_switch_prepare_validates_next_breakpoint_flow_ref(
                        Scheduler,
                        KernelInitTaskRef
                    );
                    task_concurrency_open();
                    smp_concurrency_closed();
                    secondary_cpus_not_started(CpuGroup);
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
                    BootInitRestInitPhase.state == State::Online;
                    boot_init_schedule_handoff_ready(BootInitScheduleHandoffPhase);
                    BootIdleFlow.state == State::Ready;
                    task_active_flow_is(BootTask, BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    task_concurrency_open();
                    smp_concurrency_closed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootInitRestInitPhase.state == State::Online;
            boot_init_schedule_handoff_ready(BootInitScheduleHandoffPhase);
            BootIdleFlow.state == State::Ready;
            task_active_flow_is(BootTask, BootIdleFlow);
            scheduler_switch_prepare_validates_prev_live_active_flow(
                Scheduler,
                CurrentTaskRef
            );
            scheduler_switch_prepare_validates_next_breakpoint_flow_ref(
                Scheduler,
                KernelInitTaskRef
            );
            task_concurrency_open();
            smp_concurrency_closed();
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    BootInitRestInitPhase.state == State::Online;
                    boot_init_schedule_handoff_ready(BootInitScheduleHandoffPhase);
                    BootIdleFlow.state == State::Ready;
                    task_active_flow_is(BootTask, BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    task_concurrency_open();
                    smp_concurrency_closed();
                }
            }
        }
    }

    state State::Online {
        invariant {
            BootInitRestInitPhase.state == State::Online;
            boot_init_schedule_handoff_ready(BootInitScheduleHandoffPhase);
            BootIdleFlow.state == State::Ready;
            task_active_flow_is(BootTask, BootIdleFlow);
            task_concurrency_open();
            smp_concurrency_closed();
        }
    }
}

/*
 * BootIdleEntryPhase 由首次调度交接之后的 BootTask continuation 执行。
 * 整个子阶段处于 BootIdleStartupContext 中；该 context 在正常启动路径中
 * 通过 Never 标记无普通退出事件，within 结束不代表抢占 guard 退出。
 */
object BootIdleEntryPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootIdleFlow;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootInitScheduleHandoffPhase.state == State::Online;
                    scheduler_first_schedule_committed(Scheduler);
                    BootIdleSetup.state == State::Ready;
                    BootIdleFlow.state == State::Ready;
                    task_active_flow_is(BootTask, BootIdleFlow);
                    CpuGroup.state == State::Ready;
                    task_concurrency_open();
                }

                within BootIdleStartupContext {
                    drives {
                        BootIdleFlow.Action::PrepareIdleEntry;
                        BootIdleFlow.Action::RunIdleLoop;
                    }

                    ensures {
                        task_preemption_disabled(BootTask);
                        boot_idle_runtime_ready(BootIdleFlow, BootTask);
                        boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
                        boot_idle_entry_prepared(BootIdleFlow, BootTask);
                        boot_task_idle_flow_entered(BootTask, BootIdleFlow);
                        boot_idle_task_pf_idle(BootTask);
                        boot_idle_runtime_loop_entered(BootIdleFlow, BootTask);
                        boot_idle_nohz_run_idle_balance_done(BootIdleFlow, BootCPU);
                        boot_idle_polling_rmb_before_sleep_check(BootTask);
                        boot_idle_local_irq_disabled_for_sleep(
                            BootIdleFlow,
                            BootCpuLocalInterrupt
                        );
                        boot_idle_arch_cpu_idle_enter_done(BootIdleFlow, BootCPU);
                        boot_idle_rcu_nocb_deferred_wakeup_flushed(BootIdleFlow);
                        boot_idle_cpu_offline_dead_path_not_taken(BootIdleFlow, BootCPU);
                        boot_idle_poll_or_cpuidle_path_deferred(BootIdleFlow);
                        boot_idle_arch_cpu_idle_exit_done(BootIdleFlow, BootCPU);
                        boot_idle_preempt_need_resched_set(BootTask);
                        boot_idle_polling_clear_mb_before_flush(BootTask);
                        boot_idle_smp_call_function_queue_flushed(BootIdleFlow);
                        boot_idle_loop_cycle_committed(BootIdleFlow);
                        boot_idle_loop_continues(BootIdleFlow);
                    }
                }

                ensures {
                    boot_idle_entry_phase_ready(BootIdleEntryPhase);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    boot_idle_runtime_ready(BootIdleFlow, BootTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
                    boot_idle_entry_prepared(BootIdleFlow, BootTask);
                    boot_task_idle_flow_entered(BootTask, BootIdleFlow);
                    boot_idle_task_pf_idle(BootTask);
                    boot_idle_runtime_loop_entered(BootIdleFlow, BootTask);
                    boot_idle_nohz_run_idle_balance_done(BootIdleFlow, BootCPU);
                    boot_idle_polling_rmb_before_sleep_check(BootTask);
                    boot_idle_local_irq_disabled_for_sleep(
                        BootIdleFlow,
                        BootCpuLocalInterrupt
                    );
                    boot_idle_arch_cpu_idle_enter_done(BootIdleFlow, BootCPU);
                    boot_idle_rcu_nocb_deferred_wakeup_flushed(BootIdleFlow);
                    boot_idle_cpu_offline_dead_path_not_taken(BootIdleFlow, BootCPU);
                    boot_idle_poll_or_cpuidle_path_deferred(BootIdleFlow);
                    boot_idle_full_tick_runtime_deferred(BootIdleFlow);
                    boot_idle_full_rcu_runtime_deferred(BootIdleFlow);
                    boot_idle_full_irq_idle_deferred(BootIdleFlow);
                    boot_idle_arch_cpu_idle_exit_done(BootIdleFlow, BootCPU);
                    boot_idle_preempt_need_resched_set(BootTask);
                    boot_idle_polling_clear_mb_before_flush(BootTask);
                    boot_idle_smp_call_function_queue_flushed(BootIdleFlow);
                    boot_idle_loop_cycle_committed(BootIdleFlow);
                    boot_idle_loop_continues(BootIdleFlow);
                    boot_cpu_idle_runtime_entered(BootIdleFlow);
                    secondary_cpus_not_started(CpuGroup);
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                }

                deferred boot_idle.001 {
                    category: DeferredCategory::ModelDetail;
                    summary: "Represent the unbounded boot idle-loop continuation beyond one finite derivation cycle.";
                    evidence { boot_idle_loop_continues(BootIdleFlow); }
                    close_when: "The model represents repeated idle/schedule cycles without a finite unrolling assumption.";
                }
                deferred boot_idle.002 {
                    category: DeferredCategory::Feature;
                    summary: "Complete periodic tick behavior while the boot CPU is idle.";
                    evidence { boot_idle_full_tick_runtime_deferred(BootIdleFlow); }
                    close_when: "Idle tick/nohz transitions and accounting tests pass.";
                }
                deferred boot_idle.003 {
                    category: DeferredCategory::Protocol;
                    summary: "Complete RCU idle/EQS behavior in the idle loop.";
                    evidence { boot_idle_full_rcu_runtime_deferred(BootIdleFlow); }
                    close_when: "RCU idle entry/exit, deferred wake and grace-period tests pass.";
                }
                deferred boot_idle.004 {
                    category: DeferredCategory::Feature;
                    summary: "Complete cpuidle polling and low-power state selection.";
                    evidence { boot_idle_poll_or_cpuidle_path_deferred(BootIdleFlow); }
                    close_when: "Polling and cpuidle state selection/exit tests pass.";
                }
                deferred boot_idle.005 {
                    category: DeferredCategory::Protocol;
                    summary: "Complete interrupt entry and exit behavior while idle.";
                    evidence { boot_idle_full_irq_idle_deferred(BootIdleFlow); }
                    close_when: "Idle interrupt wake, nested entry and resume tests pass.";
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
                    BootInitScheduleHandoffPhase.state == State::Online;
                    BootIdleFlow.state == State::Ready;
                    boot_idle_entry_phase_ready(BootIdleEntryPhase);
                    boot_idle_runtime_ready(BootIdleFlow, BootTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
                    boot_idle_entry_prepared(BootIdleFlow, BootTask);
                    boot_task_idle_flow_entered(BootTask, BootIdleFlow);
                    boot_idle_runtime_loop_entered(BootIdleFlow, BootTask);
                    boot_idle_nohz_run_idle_balance_done(BootIdleFlow, BootCPU);
                    boot_idle_local_irq_disabled_for_sleep(
                        BootIdleFlow,
                        BootCpuLocalInterrupt
                    );
                    boot_idle_preempt_need_resched_set(BootTask);
                    boot_idle_smp_call_function_queue_flushed(BootIdleFlow);
                    boot_idle_loop_continues(BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    task_concurrency_open();
                    smp_concurrency_closed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            BootInitScheduleHandoffPhase.state == State::Online;
            BootIdleFlow.state == State::Ready;
            boot_idle_entry_phase_ready(BootIdleEntryPhase);
            boot_idle_runtime_ready(BootIdleFlow, BootTask);
            boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
            boot_idle_entry_prepared(BootIdleFlow, BootTask);
            boot_task_idle_flow_entered(BootTask, BootIdleFlow);
            boot_idle_runtime_loop_entered(BootIdleFlow, BootTask);
            boot_idle_nohz_run_idle_balance_done(BootIdleFlow, BootCPU);
            boot_idle_local_irq_disabled_for_sleep(BootIdleFlow, BootCpuLocalInterrupt);
            boot_idle_preempt_need_resched_set(BootTask);
            boot_idle_smp_call_function_queue_flushed(BootIdleFlow);
            boot_idle_loop_continues(BootIdleFlow);
            kernel_init_task_stack_switch_committed(
                Scheduler,
                BootTask,
                KernelInitTask
            );
            task_concurrency_open();
            smp_concurrency_closed();
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    BootInitScheduleHandoffPhase.state == State::Online;
                    BootIdleFlow.state == State::Ready;
                    boot_idle_entry_phase_ready(BootIdleEntryPhase);
                    boot_idle_runtime_ready(BootIdleFlow, BootTask);
                    boot_idle_cpu_startup_entry_ready(BootIdleFlow, BootCPU);
                    boot_idle_entry_prepared(BootIdleFlow, BootTask);
                    boot_task_idle_flow_entered(BootTask, BootIdleFlow);
                    boot_idle_runtime_loop_entered(BootIdleFlow, BootTask);
                    boot_idle_nohz_run_idle_balance_done(BootIdleFlow, BootCPU);
                    boot_idle_local_irq_disabled_for_sleep(
                        BootIdleFlow,
                        BootCpuLocalInterrupt
                    );
                    boot_idle_preempt_need_resched_set(BootTask);
                    boot_idle_smp_call_function_queue_flushed(BootIdleFlow);
                    boot_idle_loop_continues(BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    task_concurrency_open();
                    smp_concurrency_closed();
                }
            }
        }
    }

    state State::Online {
    }
}
