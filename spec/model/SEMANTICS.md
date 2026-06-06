# Model Semantics Hard Rules

本文档记录 `.spec` 模型语言的硬语义规则。修改规格、模型构建器、推导器、检查器、视图或渲染工具前，应先阅读本文档；工具实现不得用后续阶段的“消歧”或展示逻辑绕过这些规则。

## SEM-NAME-001: Lifecycle State And Event Names Are Controlled

生命周期状态名和生命周期事件名必须来自受控集合。规格不得临时发明新的生命周期名称来表达局部语义；如果确实需要新增名称，必须先修改本文档、`model` 阶段检查器和对应测试。

当前 `object` 状态机语法只用受控生命周期状态和生命周期事件表达启动期对象推进。运行期 Type process、扩展状态、action 引用和独占上下文由本文档后续规则单独约束；它们不得借用或发明新的生命周期状态名来规避本节规则。

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

当前工具已经支持在 `drives` 和 `within` 内引用 `Object.Action::Name(...)`，并把该引用作为成功路径上的 action commit fact；对象内 `state.actions` 的完整定义语法仍处于语义契约阶段，暂未由 parser/model 展开检查。

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
- event 可以通过 `drives` 调用 action，以复用不推进被建模状态的内部动作；这适合表达 `Setup`/`Enable` 这类 lifecycle event 内部的初始化、发布、入队、唤醒、查询或其它属性动作。
- event 调用 action 后，外层 event 仍负责提交自己的状态迁移；action 只贡献自己的 `ensures`，不得替代外层 event 的目标状态提交。
- 如果某个过程会改变已建模生命周期状态或扩展状态，它必须保持为 event，不能为了减少定义而降级成 action。也就是说，复用 action 只能消除无状态迁移的重复动作，不能隐藏真实状态迁移。

示例：`copy_process()` 应建模为 `TaskCreationCore` 在 `Ready` 状态内的参数化 action，而不是为每个目标任务建立 `CopyKernelInitProcess` 之类的专名 action：

```text
on Action::CopyProcess<Src: TaskObject, New: TaskObject>(
    src_task: Src,
    dst_task: New,
    pid_ns: RootPidNamespace,
    creds: CredentialCore,
    signal: SignalCore,
    files: TaskFileContext,
    security: SecurityCore,
    scheduler: Scheduler
) {
    depends_on {
        src_task.state == State::Online;
        dst_task.state == State::Prepared;
        pid_ns.state == State::Ready;
        creds.state == State::Prepared;
        signal.state == State::Prepared;
        files.state == State::Prepared;
        security.state == State::Ready;
        scheduler.state == State::Online;
        task_clone_args_ready(dst_task);
    }

    ensures {
        task_creation_copy_process_committed(TaskCreationCore, src_task, dst_task);
        task_creation_used_clone_args(TaskCreationCore, dst_task);
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
```

## SEM-TYPE-PROCESS-001: Type Processes Define Reusable Runtime Semantics

`type` 定义可复用对象类型的共同属性、owned 子对象、标准生命周期 process、运行期 process、扩展状态和约束。`object X: SomeType` 表示 `X` 是 `SomeType` 的一个具名实例；实例绑定并继承 `SomeType` 上定义的 Type process。也就是说，`X.Event::Setup`、`X.Event::Enable` 或 `X.Action::Done` 若来自 Type 定义，语义上是“对实例 X 执行 Type process”，不是实例重新定义了一套同名过程。

Type body 中的 `key: ValueType;` 是 Type 属性声明，必须被 parse/model 工具保留。`owned { field: ChildType; }` 声明 Type 实例拥有的子对象或内嵌资源；owned 子对象随宿主实例建立实例身份，不是外部引用，也不表示普通参数传递。Type body 中的命名块按语义分类：

- `lifecycle { ... }` 定义 Type 实例的标准生命周期 process，名称应使用受控生命周期事件名。
- `processes { ... }` 定义实例进入可服务状态后的运行期 process；其中 `Event::Name` 是 Operational Event，`Action::Name` 是 action。
- `owned { ... }` 定义实例拥有的内嵌资源或子对象；Type process 可以通过 `self.field.Event::Name` 或 `self.field.Action::Name` 驱动它们。
- 其它命名块可用于 invariant、context、handle 或后续扩展，但不得隐式改变生命周期迁移表。

每个 Type process 必须声明 `state_effect`，其值来自 `StateEffect`：

- `StateEffect::Always`：`Success` 提交时必然推进该 process 声明的被建模状态。
- `StateEffect::Conditional`：`Success` 提交时依据扩展状态、计数、等待队列或目标对象状态选择迁移；迁移表必须写在该 Type process 内。
- `StateEffect::None`：`Success` 不推进被建模状态；这类 process 应归为 `Action::Name`。

`StateEffect` 已在正式规格中以 enum 建模；当前工具保留该声明和 Type process 块，但尚未解析、类型检查或推导 process 内部的 `state_effect`。因此它已经是正式规格术语，不再只是说明性文字；工具执行语义仍在后续扩展范围内。

扩展状态不是 lifecycle state。扩展状态用于表达运行期对象在 `Online` 等生命周期状态内可反复变化的内部语义，例如 completion 的 pending/completed/all-completed。扩展状态集合必须由 enum 或受控值类型声明，不能混入 `State::Ready`、`State::Online` 等生命周期状态名。

process 的结果使用 `ProcessResult` 语义集合：`Success`、`Blocked(reason)`、`Failed(code)`。只有 `Success` 可以提交 `state_effect` 与 `ensures`；`Blocked` 和 `Failed` 都不得推进生命周期状态或扩展状态，也不得假定 `ensures` 成立。

当前工具边界：

- parser/model 保留 Type 属性和 Type 块，包括 `lifecycle`、`processes`、`transitions` 和 `result` 这类嵌套文本。
- derive/view/render 尚不推导 Type process 内部迁移；阶段主线若需要使用某个 Type process 的效果，必须暂时通过对象 lifecycle wrapper、显式 action commit 或普通 predicate fact 承载，并在规格中说明该承载不改变 Type process 的正式语义。

`Completion` 是当前第一个正式 Type process 示例。Linux `struct completion` 内嵌 `swait_queue_head wait`，因此规格中 `Completion` 拥有 `SimpleWaitQueue`，不是引用外部 wait queue。`Completion.Setup` 驱动 owned wait queue 的 setup；`Complete`/`CompleteAll` 驱动 wait queue 的 wake action；`Wait` 驱动 wait queue 的 prepare/finish wait action；`Reinit` 只重置 completion token 状态并保留 wait queue。`Complete`、`CompleteAll`、`Wait`、`TryWait`、`Reinit` 是 `StateEffect::Conditional` 的运行期事件；`Done` 是 `StateEffect::None` 的只读 action。

Completion 也说明了 event/action factoring 的边界：`Completion.Setup` 可以调用 `SimpleWaitQueue.Setup`，`Completion.Complete` 可以调用 `SimpleWaitQueue.WakeOne` action，因为这些子动作本身不推进 Completion 的扩展状态；但 `Completion.Complete` 仍不能改成 action，因为它会把 `CompletionExtState::Pending` 推进到 `CompletionExtState::Completed`，或在其它扩展状态下按条件迁移表提交结果。

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

## SEM-CONTEXT-NESTING-001: Context Effects Compose Monotonically

上下文将来会扩展为多种类型。当前已经落地的是“通过锁人工定义边界”的资源独占上下文；后续还需要正式规格化系统独占上下文，例如启动早期天然只有单执行流可达的系统上下文，或者通过关闭中断、关闭抢占等机制建立的系统上下文。

上下文之间允许嵌套。嵌套的语义不是替换外层上下文，而是把外层和内层的效果叠加为一个累计上下文。累计上下文决定当前流能访问哪些对象、能获得哪些层级的对象句柄，以及能否睡眠、能否被抢占、能否被中断等运行约束。

后续正式规格化时，每类 context 必须声明自己的 effect 向量。最小 effect 维度包括：

- `interruptible`：当前作用域是否允许被中断。
- `preemptible`：当前作用域是否允许被抢占。
- `sleepable`：当前作用域是否允许睡眠或阻塞等待。
- `exclusive_refs`：当前作用域独占或受保护访问的对象引用集合。
- `handle_level`：当前作用域对对象可见的句柄层级或 capability。

嵌套检查必须满足单调加强规则：从外到内可以越来越强，但不能反向削弱外层已经建立的约束。也就是说，内层上下文可以进一步关闭中断、关闭抢占、扩大受保护对象集合或提升对象句柄层级；但不能在外层已经要求不可中断、不可抢占或不可睡眠时，引入语义上允许中断、允许抢占或允许睡眠的上下文。

例如，在一个要求不可中断的自旋锁上下文中，嵌套一个语义上允许中断的关抢占上下文或睡眠锁上下文，应被判定为上下文嵌套违例。原因是内层上下文的 effect 与外层累计 effect 冲突，不能形成合法的叠加效果。

多种上下文和上下文嵌套的正式规格化，是主规格中“组件化内核在不同上下文可以访问对象不同层级句柄”的具体化：上下文 effect 栈给出当前流的访问能力，句柄层级由累计上下文推导，而不是由对象所有权或普通参数传递隐式决定。

当前工具只检查单层 `exclusive_context` 的 `lock_ref`、`obj_refs` 和 `within` 内 action/event 引用边界；多种 context kind、effect 偏序、嵌套合法性和句柄层级推导尚未实现。

## SEM-CURRENT-CPU-MODEL-001: CurrentCPU Is The Per-CPU Self Identity Entry

`RawSpinLock`、本地中断开关和抢占开关的正式建模必须建立在 `CurrentCPU` 上。`CurrentCPU` 表示每个 CPU 启动时天然拥有的“当前 CPU 自我身份入口”；它不是 `Context` 语义中的资源访问上下文，也不是 `within` 使用的资源独占上下文。

`CurrentCPU` 的首轮正式语义如下：

- 每个 CPU 启动时自动产生一个与自身唯一对应的 `CurrentCPU` 实例。
- `CurrentCPU` 独立于 `CpuGroup`，不挂在 `CpuGroup` 对象树下；BP 侧的 `CurrentCPU` 早于几乎所有内核对象存在。
- `CurrentCPU` 拥有与自身对应的 CPU 对象；`CpuGroup` 后续维护对这些 CPU 对象的引用、索引和拓扑组织关系，而不是天然拥有 CPU 本体。
- 内核启动最早期，当前执行 CPU 只通过自己的 `CurrentCPU` 认识自身和访问自己拥有的 CPU 对象；此时规格层尚不应假定已经存在 `BootCPU`。
- `CurrentCPU.Preset` 记录入口或固件交付的 hartid。
- `CurrentCPU.Setup` 记录或确认 logical CPU id。
- `CurrentCPU.Enable` 在 `CpuGroup` 建立后，把自身拥有的 CPU 对象注册/绑定到 `CpuGroup.Cpu[id]` 引用集合。
- `BootCPU` 后续应从早期身份对象逐步退化为 `CurrentCPU.cpu` 所指 CPU 的 bootstrap role、alias 或描述性 fact；现有 `BootCPU` 对象可在迁移期保留，以避免一次性大范围重写。

secondary CPU 的边界必须单独区分。`CpuGroup` 可以先从 `PlatformCpuInfo`、DeviceTree 或 topology 事实中知道 possible secondary CPU，并可维护 `CpuCandidate`、`PossibleCpuDescriptor` 或等价描述；但在某个 AP 真实进入 secondary entry 之前，不应假定该 AP 的 live `CurrentCPU` 已经存在，也不应认为该 AP 已经拥有可操作的 CPU-local interrupt、current task slot 或 task preemption 控制链。BP 侧如果提前为 AP 准备 idle task、hotplug state、runqueue 元数据或同步 completion，这些对象应标记为 BP 侧为 future CPU 准备的描述/资源，不等同于 AP 自己的 `CurrentCPU` 已建立。AP 进入后，才由该 AP 自动产生自己的 `CurrentCPU`，拥有对应 CPU 对象，并把该 CPU 对象注册/绑定回 `CpuGroup.Cpu[id]` 引用集合。

完成上述绑定后，与当前锁建模相关的访问链应为：

```text
CurrentCPU
    -> cpu
    -> LocalInterruptControl
    -> CurrentTaskSlot
    -> current Task
    -> PreemptionControl

CpuGroup
    -> CpuRef[id] -> CurrentCPU.cpu
```

其中 `LocalInterruptControl` 属于具体 CPU，`CurrentTaskSlot` 也属于具体 CPU 并保存当前任务引用；`PreemptionControl` 属于当前 task/thread_info。`preempt_disable()` 和 `preempt_enable()` 不应直接作用于被唤醒或被创建的任务，而应通过 `CurrentCPU -> cpu -> CurrentTaskSlot.current_task` 找到当前正在执行的任务后再驱动该任务的抢占控制事件。

`LocalInterruptControl` 应接管本 CPU local interrupt 开关状态的直接控制能力。`InterruptStream` 不应再直接修改架构中断开关寄存器事实，也不应对外提供普通的 `local_irq_enable/disable` operational event；它只能在自身 lifecycle event 中驱动 `CurrentCPU.cpu.LocalInterruptControl` 完成阶段边界所需的 enable/disable。`LocalInterruptControl` 则只提供可反复调用的 operational events，例如 `Disable`、`Enable`、`SaveAndDisable(out flags)` 和 `Restore(flags)`；本地中断打开/关闭是运行期状态变化，不是 `LocalInterruptControl` 自身 lifecycle event。

`RawSpinLock` 类型建模在该访问链成立后展开。`raw_spin_lock_irqsave()` 对应的事件应同时驱动三类效果：保存并关闭当前 CPU 本地中断、关闭当前任务抢占、获取锁本体。释放路径应先释放锁本体，再恢复本地中断状态并打开当前任务抢占。示例形态如下：

```text
RawSpinLock.Event::LockIrqSave(current_cpu: CurrentCPU) {
    drives {
        current_cpu.cpu.LocalInterruptControl.Event::SaveAndDisable(out flags);
        current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Event::Disable;
        self.Action::Acquire;
    }
}

RawSpinLock.Event::UnlockIrqRestore(current_cpu: CurrentCPU, flags: IrqFlags) {
    drives {
        self.Action::Release;
        current_cpu.cpu.LocalInterruptControl.Event::Restore(flags);
        current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Event::Enable;
    }
}
```

`KernelInitTask.Enable` 的 `within WakeUpNewTaskContext` 由 `KernelInitTaskPiLock.Event::LockIrqSave(current_cpu: CurrentCPU)` 建立进入边界，并由对应的 `UnlockIrqRestore` 建立退出边界。`within` 块内部只保留受保护资源对象的 action/event，例如设置 task runtime state、选择 runqueue 和入队任务。

首轮落地状态：

1. 已在模型规格中新增 `CurrentCPU`、CPU 对象所有权、`LocalInterruptControl`、`CurrentTaskSlot`、`PreemptionControl` 和 `RawSpinLock` 的正式类型语义。
2. 已调整入口前导期规格，使 BP 先由 `CurrentCPU` 记录 hartid，并通过自身拥有的 CPU 对象访问 local interrupt/current task 等 CPU-local 子对象；`CpuGroup` 建立后只维护对该 CPU 对象的引用；`BootCPU` 暂作为迁移期 alias/fact 保留。
3. 已调整调度初始化规格，把现有 `boot_cpu_current_is_idle_task(...)` 事实升级为 CPU current slot 的正式设置动作。
4. 已调整 `rest_init` 规格，使 `KernelInitTask.Enable` 的资源独占上下文进入/退出由 `RawSpinLock.LockIrqSave/UnlockIrqRestore` 表达。
5. 已补充工具语法和检查支持，包括 typed `RawSpinLock` 实例、`within` enter/exit 边界、路径式 action/event 引用，以及上下文 effect 检查的后续扩展点。
6. 已同步 `arceos_ex` 对象实现和 smoke/KUnit 覆盖，并通过 `make test`、`make verify` 和 `git diff --check`。

后续保留项：

- `BootCPU` 进一步退化为 `CurrentCPU.cpu` 的 bootstrap role、alias 或描述性 fact。
- secondary CPU live `CurrentCPU` 的 AP entry 建立路径。
- 多种 context kind、effect 偏序、嵌套合法性和句柄层级推导；这些属于 `SEM-CONTEXT-NESTING-001` 的后续任务。

## SEM-EVENT-RESULT-001: Event And Action Results Are Explicit

`event` 和 `action` 都应具有显式返回结果。正式结果集合由 `ProcessResult` 语义定义，最小包括：

- `Success`：操作成功完成。
- `Failed(code)`：操作失败，可携带具体错误码或原因。

需要表达等待、重试或条件暂未满足时，可以使用：

- `Blocked(reason)`：本次操作没有提交成功结果，但不是语义错误。

对 `event` 的结果约束：

- 只有 `Success` 才提交 `state_effect`，并使目标状态、目标扩展状态及该事件的 `ensures` 成立。
- `Blocked` 和 `Failed` 都不得使对象进入目标状态或目标扩展状态，也不得假定该事件的 `ensures` 成立。
- 对 `StateEffect::Conditional` 的 event，`Success` 后是否发生状态变化、变化到哪里，以 Type process 的条件迁移表为准。

对 `action` 的结果约束：

- `Success` 表示动作完成，但对象仍停留在 action 所属的被建模状态内。
- `Blocked` 和 `Failed` 表示动作未完成或失败，同样不推进被建模状态。

当前推导器把对象 lifecycle event 视为成功路径上的静态推导；失败、阻塞、错误码和 Type process 结果尚未进入推导语法。后续扩展错误路径或运行期 process 推导时，必须保持上述提交语义。

## SEM-LIFECYCLE-OPERATIONAL-001: Lifecycle And Operational Events Have Different Trigger Rules

生命周期事件用于建立或销毁对象生命周期状态，当前正式集合为 `Preset`、`Setup`、`Enable`、`Cleanup`。在同一对象生命周期轮次中，每个生命周期事件原则上至多成功触发一次。当前启动模型采用 forward-only 推进模式，重复触发生命周期事件应视为规格错误。

操作事件用于对象进入某个可服务生命周期状态后的运行期状态迁移，例如锁、队列、映射、分配器、调度器和 I/O 对象中的 `lock`、`try_lock`、`unlock`、`enqueue`、`dequeue`、`map`、`unmap`、`alloc`、`free`、`read`、`write` 等。操作事件可以在对象生命周期内反复触发；每次触发都必须符合该对象当前运行状态的迁移规则。

示例：普通非嵌套自旋锁中，`lock` 和 `try_lock` 都是 `Operational Event`，因为它们成功时都会把锁从 `Unlocked` 推进到 `Locked(owner=current)`。`lock` 在锁被别人持有时可以返回 `Blocked(occupied)`，在已经被自己持有时返回 `Failed(nested_lock)`；`try_lock` 在锁被别人持有时返回 `Failed(busy)`，在已经被自己持有时返回 `Failed(nested_lock)`。二者可以具有相同的成功源状态和目标状态，但它们是两个不同事件。

## Non-Hard Guidance: Object Granularity Is Hierarchical

本节是建模建议，不是硬语义规则；`model`、`derive`、`view`、`render` 不得把本节作为 error 或 warning 的强制来源。

对象可以有不同粒度。当前建模层级中不再继续拆分的最小粒度对象可视为原子对象；除此之外的对象通常是复合对象，由更小粒度对象组合而成。复合对象状态通常由自身属性和子对象状态支撑，复合对象事件通常由子对象事件支撑。

在规格描述和对象实现中，建议优先使用足以表达当前语义的高粒度对象，以便逐级封装实现细节和复杂性。该建议不改变生命周期状态名、事件名、迁移三元组和事件唯一性等硬规则；当现实实现受限时，可以展开较低粒度对象或采用临时承载方式，但应在说明或 coding 规格中记录原因。
