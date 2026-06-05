# Model Semantics Hard Rules

本文档记录 `.spec` 模型语言的硬语义规则。修改规格、模型构建器、推导器、检查器、视图或渲染工具前，应先阅读本文档；工具实现不得用后续阶段的“消歧”或展示逻辑绕过这些规则。

## SEM-NAME-001: Lifecycle State And Event Names Are Controlled

生命周期状态名和生命周期事件名必须来自受控集合。规格不得临时发明新的生命周期名称来表达局部语义；如果确实需要新增名称，必须先修改本文档、`model` 阶段检查器和对应测试。

当前 `.spec` 启动模型只形式化生命周期状态和生命周期事件，即 `Lifecycle Event`。运行期对象未来可以引入 `Operational Event` 和 `Action`，但必须先在本文档补充对应语法、检查器规则和测试，不能混用当前生命周期事件规则。

允许的状态名：

- `Base`
- `Prepared`
- `Ready`
- `Online`
- `Offline`
- `Destroyed`

允许的生命周期事件名：

- `Preset`
- `Setup`
- `Enable`
- `Disable`
- `Cleanup`

语义约定：

- `Base` 的别名包括：初始态、基态、尚未建立。
- `Prepared` 的别名包括：预置态、前置条件已建立。
- `Ready` 的别名包括：就绪态、主要构建已完成。
- `Online` 的别名包括：在线态、已启用、可服务。
- `Offline` 的别名包括：离线态、已退出主要服务、资源已交接、`handoff`。`Handoff` 不是正式状态名。
- `Destroyed` 的别名包括：已销毁、已退出服务、已清理、已预留、`reserved`。`Reserved` 不是正式状态名。
- `Preset` 表示建立进入主要构建流程前的早期前置条件，通常推进到 `Prepared` 或 `Ready`。
- `Setup` 表示完成对象的主要构建，使对象进入 `Ready`。
- `Enable` 表示让已经构建完成的对象进入服务状态，通常推进到 `Online`。别名包括：启用、上线、进入服务、保护、`guard`。当语义是建立栈边界保护或 guard 这类运行约束时，仍使用 `Enable` 作为正式事件名；单纯刷新对象属性的动作不因此升级为生命周期事件。
- `Disable` 表示对象退出主要服务路径或完成资源所有权交接，但对象元数据仍保留给诊断、引用收尾或后续销毁。
- `Cleanup` 表示对象退出服务或释放阶段性抽象，通常推进到 `Destroyed`。

检查点：

- `model` 阶段必须检查对象 `initial_state`、状态声明名、事件声明名和事件目标状态名。
- 任何不在受控集合内的名称必须报 `error`。
- `derive`、`view`、`render` 不得通过名称猜测或展示修正来补偿非法名称。

## SEM-TRANSITION-001: Lifecycle Transitions Are Controlled

当前生命周期模型只能使用预先定义的状态迁移三元组 `(source_state, lifecycle_event, target_state)`。别名不参与迁移定义；`.spec` 中必须使用正式状态名和事件名。

允许的迁移：

- `Base --Preset--> Prepared`
- `Base --Preset--> Ready`
- `Base --Setup--> Ready`
- `Prepared --Setup--> Ready`
- `Prepared --Enable--> Online`
- `Ready --Enable--> Online`
- `Ready --Cleanup--> Destroyed`
- `Online --Disable--> Offline`
- `Online --Cleanup--> Destroyed`
- `Offline --Cleanup--> Destroyed`

检查点：

- `model` 阶段必须检查每个事件声明的源状态、事件名和目标状态三元组。
- 任何不在允许迁移集合内的三元组必须报 `error`。
- 新增迁移必须先修改本文档、`model` 阶段检查器和对应测试。

## SEM-UNIQUE-001: Forward-Only Model Forbids Duplicate States And Events

同一个 `object` 内，`Event::X` 只能定义一次。事件定义身份是 `(Object, Event)`，不是 `(Object, SourceState, Event)`。

原因：

- `drives` 引用形式是 `Object.Event::X`，不包含源状态。
- 如果同一对象内允许多个 `Event::X`，引用目标会变得不唯一。
- 当前状态只决定事件是否可触发，不参与事件命名。
- 即使将来引入可反复触发的 `Operational Event`，重复触发也不等于重复定义；同一个对象状态机中的同名事件定义仍必须唯一，除非显式引入事件重载或可重入事件定义规则。

检查点：

- `model` 阶段必须扫描同一对象的所有 `state.events`。
- 发现重复 `Event::X` 时必须报 `error`，并指向重复定义位置。
- `derive`、`view`、`render` 不得通过 `source_state -> target_state` 为重复事件消歧。

例外：

- 只有将来显式引入“事件重载/可重入事件定义”的对象类型或事件语义后，才能放宽事件定义唯一性；默认所有对象都禁止重复定义同名事件。

## SEM-EVENT-ACTION-001: Event And Action Are Distinct

`event` 表示一次尝试推进被建模状态的操作；`action` 表示依附于某个既有状态执行的状态内动作。

判断规则：

- 操作成功时推进被建模状态，应建模为 `event`。
- 操作成功时仍停留在同一被建模状态内，应建模为 `action`。
- 不得为了表达可重复调用而把应为 `action` 的操作伪造成生命周期事件。
- 不得为了绕过生命周期事件唯一性而把应为 `Operational Event` 的状态迁移伪造成 `action`。

当前 `.spec` 语法只支持生命周期事件；`action` 和 `Operational Event` 仍是后续扩展语义。新增相关语法前，`model`、`derive`、`view`、`render` 不得推测或隐式实现 action 语义。

一等 `action` 的正式规格如下，后续实现语法、检查器、推导器、视图和渲染时必须保持这些规则：

```text
state State::Ready {
    actions {
        on Action::Name<T: Object>(arg: T, other: OtherObject) {
            depends_on {
                ...
            }

            drives {
                ...
            }

            ensures {
                ...
            }

            deferred {
                ...
            }
        }
    }
}
```

- `actions` 块只能出现在 `state` 内；action 所在 state 是该 action 的 owner-state guard。
- action 成功时不得推进 owner 对象生命周期状态；owner 仍停留在 action 所属 state。
- action 名称在 owner 对象内唯一，引用身份是 `Object.Action::Name` 加实参绑定。
- action 可以带类型参数和命名形参；形参必须是对象引用或受控值类型。
- `drives` 可以引用 lifecycle event，也可以引用 action；引用参数化 action 时必须提供完整命名实参，并通过类型检查。
- action 的 `depends_on` 是调用成功前提；被 event 驱动的 action 若 `depends_on` 不满足，该 event 不得提交生命周期迁移。
- action 的 `ensures` 在 action 成功后成立，并可作为驱动它的 event 成功路径上的可用事实。
- action 的 `ensures` 不得直接伪造生命周期提交，例如不得用 action 确保 `SomeObject.state == State::Ready` 来替代 `SomeObject.Event::Setup`。
- 参数化 action 不得把具体目标对象编码进 action 名；具体对象差异应通过实参和对象自身 facts 表达。

示例：`copy_process()` 应建模为 `TaskCreationCore` 在 `Ready` 状态内的参数化 action，而不是为每个目标任务建立 `CopyKernelInitProcess` 之类的专名 action：

```text
on Action::CopyProcess<Src: TaskObject, New: TaskObject>(
    src_task: Src,
    new_task: New,
    pid_ns: RootPidNamespace,
    creds: CredentialCore,
    signal: SignalCore,
    files: TaskFileContext,
    security: SecurityCore,
    scheduler: Scheduler
) {
    depends_on {
        src_task.state == State::Online;
        new_task.state == State::Prepared;
        pid_ns.state == State::Ready;
        creds.state == State::Prepared;
        signal.state == State::Prepared;
        files.state == State::Prepared;
        security.state == State::Ready;
        scheduler.state == State::Online;
        task_clone_args_ready(new_task);
    }

    ensures {
        task_creation_copy_process_committed(TaskCreationCore, src_task, new_task);
        task_creation_used_clone_args(TaskCreationCore, new_task);
        task_struct_allocated(new_task);
        task_duplicated_from(new_task, src_task);
        task_pid_allocated(new_task, pid_ns);
        task_creds_copied(new_task, creds);
        task_file_context_copied_or_shared(new_task, files);
        task_signal_context_ready(new_task, signal);
        task_security_context_allocated(new_task, security);
        task_thread_context_ready(new_task);
        task_sched_entity_initialized(new_task, scheduler);
        task_state_new(new_task);
        task_not_enqueued(new_task);
    }
}
```

## SEM-EXCLUSIVE-CONTEXT-001: Lock And Exclusive Context Are Distinct

`Lock` 表示可建立独占边界的同步对象。`Lock` 不带泛型，不拥有被保护资源；锁与资源的关系由独占上下文表达。

`exclusive_context` 表示通过某个锁引用建立的受保护执行作用域。它不是普通 lifecycle object，不拥有锁，也不拥有资源；它只保存引用关系和作用域语义。

正式结构：

```text
exclusive_context WakeUpNewTaskContext {
    lock_ref: KernelInitTaskPiLock;
    obj_refs: {
        KernelInitTask;
        Scheduler;
        BootRunQueue;
    }
}
```

规则：

- `lock_ref` 必须引用一个 `Lock` 实例；该引用建立上下文的独占边界。
- `obj_refs` 是对象引用集合，至少包含一个对象；上下文不拥有这些对象。
- 同一把锁可以被多个 exclusive context 引用，用于建立不同受保护作用域。
- exclusive context 不需要 lifecycle state；进入上下文是一次受锁保护的独占执行尝试。
- 同一时刻至多一个执行流可以成功进入同一个 exclusive context。
- `within` 块内只能直接驱动 `obj_refs` 中对象的 action/event，除非规格显式声明允许外部对象。
- exclusive context 成功退出后释放独占执行权；失败或 `Blocked` 时，外层 event 不得提交生命周期迁移。

事件或 action 使用 `within` 声明独占执行作用域。`within` 块内可以包含 `depends_on`、`drives`、`ensures` 和 `deferred`。

`KernelInitTask.Enable` 对应 `wake_up_new_task()` 的正式规格形态如下：

```text
state State::Ready {
    events {
        on Event::Enable -> State::Online {
            within WakeUpNewTaskContext {
                depends_on {
                    task_state_new(KernelInitTask);
                    task_not_enqueued(KernelInitTask);
                }

                drives {
                    KernelInitTask.Action::SetTaskState(TaskRuntimeState::Running);
                    Scheduler.Action::SelectRunQueue(selected_rq: BootRunQueue);
                    BootRunQueue.Action::EnqueueTask(task: KernelInitTask);
                }

                ensures {
                    task_state_running(KernelInitTask);
                    task_runqueue_selected(Scheduler, KernelInitTask, BootRunQueue);
                    task_enqueued_on_runqueue(KernelInitTask, BootRunQueue);
                }
            }

            ensures {
                kernel_init_task_online(KernelInitTask);
                kernel_init_task_enqueued(KernelInitTask, BootRunQueue);
            }
        }
    }
}
```

`within` 的语义是：先尝试进入指定 exclusive context；进入成功后，在该独占作用域内执行块内的 `drives`；块内驱动全部成功后，`within` 的 `ensures` 成立，外层 event/action 才能继续提交自己的 `ensures`。`within` 不是普通参数传递，也不是对象所有权转移。

## SEM-EVENT-RESULT-001: Event And Action Results Are Explicit

`event` 和 `action` 都应具有显式返回结果。最小结果集合包括：

- `Success`：操作成功完成。
- `Failed(code)`：操作失败，可携带具体错误码或原因。

需要表达等待、重试或条件暂未满足时，可以使用：

- `Blocked(reason)`：本次操作没有提交成功结果，但不是语义错误。

对 `event` 的结果约束：

- 只有 `Success` 才提交状态迁移，并使目标状态及该事件的 `ensures` 成立。
- `Blocked` 和 `Failed` 都不得使对象进入目标状态，也不得假定该事件的 `ensures` 成立。

对 `action` 的结果约束：

- `Success` 表示动作完成，但对象仍停留在 action 所属的被建模状态内。
- `Blocked` 和 `Failed` 表示动作未完成或失败，同样不推进被建模状态。

当前推导器把生命周期事件视为成功路径上的静态推导；失败、阻塞和错误码尚未进入 `.spec` 推导语法。后续扩展错误路径推导时，必须保持上述提交语义。

## SEM-LIFECYCLE-OPERATIONAL-001: Lifecycle And Operational Events Have Different Trigger Rules

生命周期事件用于建立或销毁对象生命周期状态，当前正式集合为 `Preset`、`Setup`、`Enable`、`Cleanup`。在同一对象生命周期轮次中，每个生命周期事件原则上至多成功触发一次。当前启动模型采用 forward-only 推进模式，重复触发生命周期事件应视为规格错误。

操作事件用于对象进入某个可服务生命周期状态后的运行期状态迁移，例如锁、队列、映射、分配器、调度器和 I/O 对象中的 `lock`、`try_lock`、`unlock`、`enqueue`、`dequeue`、`map`、`unmap`、`alloc`、`free`、`read`、`write` 等。操作事件可以在对象生命周期内反复触发；每次触发都必须符合该对象当前运行状态的迁移规则。

示例：普通非嵌套自旋锁中，`lock` 和 `try_lock` 都是 `Operational Event`，因为它们成功时都会把锁从 `Unlocked` 推进到 `Locked(owner=current)`。`lock` 在锁被别人持有时可以返回 `Blocked(occupied)`，在已经被自己持有时返回 `Failed(nested_lock)`；`try_lock` 在锁被别人持有时返回 `Failed(busy)`，在已经被自己持有时返回 `Failed(nested_lock)`。二者可以具有相同的成功源状态和目标状态，但它们是两个不同事件。

## Non-Hard Guidance: Object Granularity Is Hierarchical

本节是建模建议，不是硬语义规则；`model`、`derive`、`view`、`render` 不得把本节作为 error 或 warning 的强制来源。

对象可以有不同粒度。当前建模层级中不再继续拆分的最小粒度对象可视为原子对象；除此之外的对象通常是复合对象，由更小粒度对象组合而成。复合对象状态通常由自身属性和子对象状态支撑，复合对象事件通常由子对象事件支撑。

在规格描述和对象实现中，建议优先使用足以表达当前语义的高粒度对象，以便逐级封装实现细节和复杂性。该建议不改变生命周期状态名、事件名、迁移三元组和事件唯一性等硬规则；当现实实现受限时，可以展开较低粒度对象或采用临时承载方式，但应在说明或 coding 规格中记录原因。
