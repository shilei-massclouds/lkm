# Model Semantics Hard Rules

本文档记录 `.spec` 模型语言的硬语义规则。修改规格、模型构建器、推导器、检查器、视图或渲染工具前，应先阅读本文档；工具实现不得用后续阶段的“消歧”或展示逻辑绕过这些规则。

## SEM-BOUNDARY-001: Deferred And Trimmed Are Structured Boundaries

`deferred` 只表示当前仍未实现或未证明的责任；`trimmed` 只表示在当前参考构建配置、
目标架构或参考输入下可证明不可达或无操作的边界。两者都不是对象状态，不得用来承载已实现
事实、checkpoint 说明、历史调试记录或不受控的自由文本 backlog。

结构化语法为：

```text
deferred user_clone.001 {
    category: DeferredCategory::Feature;
    summary: "Support thread-group clone semantics.";
    evidence {
        user_clone_thread_groups_deferred(UserCloneBoundaries);
    }
    close_when: "Thread-group clone facts, implementation and differential tests pass.";
}

trimmed mm_core.001 {
    category: TrimmedCategory::BuildConfig;
    summary: "page_ext initialization is absent when CONFIG_PAGE_EXTENSION=n.";
    evidence {
        mm_core_page_ext_flatmem_trimmed(MmCoreTrimmedPaths);
    }
    revisit_when: "The reference configuration enables CONFIG_PAGE_EXTENSION.";
}
```

硬约束：

- `deferred` 分类只能是 `Feature`、`Protocol`、`ModelDetail`、`Proof` 或
  `AlternatePath`。
- `trimmed` 分类只能是 `BuildConfig`、`Architecture`、`ReferenceInput` 或
  `CompileTimeNoOp`。“当前没有实现”不是 trimmed evidence。
- 每条记录只承载一项可关闭责任，必须声明非空 `summary`、非空 `evidence`，
  以及 `deferred` 的 `close_when` 或 `trimmed` 的 `revisit_when`。验收条件不得使用
  “以后处理”等无边界表述。
- ID 必须匹配“域名 + 三位序号”，例如 `user_clone.001`。ID 在整个模型中唯一，
  一旦分配永不复用；标题、owner 或状态变化不改变 ID。
- owner object、owner state/transition、`within` context 和源文件位置由 AST/model
  根据词法位置推导；源文本不得手工重复这些字段。
- model 阶段必须拒绝缺字段、非法分类、重复/非法 ID 和最终门禁中的 legacy
  `deferred { "..." }`。parser 可以为迁移诊断保留 legacy AST，但根模型验证不允许它通过。
- derive 阶段必须在 owner 边界到达时验证 `evidence`。无法从模型、参考配置事实、
  架构事实、参考输入事实或已证明前序事实推出的 evidence 必须产生 verification
  obligation；工具不得因为记录被标记为 deferred/trimmed 就假定 evidence 成立。
- 默认 check policy 允许 evidence 成立的 deferred/trimmed inventory 存在，但必须拒绝
  结构错误、legacy 记录和任何未解 verification obligation。

生命周期规则：

- 新发现的未闭合责任必须在定位 owner 后新增结构化记录；责任变化语义时先重分类或迁移
  owner，不得通过改摘要隐藏语义变化。
- 责任完成时，同一变更必须删除 boundary、在 `ensures`/`invariant`/coding 中补齐正式
  事实与验证，并在对应专题完成归档中记录原 ID、完成证据和提交。
- active roadmap 只引用仍需推进的 boundary ID，不复制第二套责任描述。

## SEM-NAME-001: Lifecycle State And Transition Names Are Controlled

生命周期状态名和生命周期转移名必须来自受控集合。规格不得临时发明新的生命周期名称来表达局部语义；如果确实需要新增名称，必须先修改本文档、`model` 阶段检查器和对应测试。

术语边界如下：`State` 表示对象生命周期状态；`Transition` 表示成功提交时会改变生命周期状态或扩展状态的转移过程；`Action` 表示不改变当前被建模状态的对象行为；`Event` 保留给外部信号、异步事件、硬件事件、trace event 等触发源或可观测事件。`.spec` 源语法必须使用 `Transition::Setup` / `Object.Transition::Name` 表达 state-changing process；旧对象过程事件语法不属于正式语言。

当前 `object` 状态机语法只用受控生命周期状态和生命周期 transition 表达启动期对象推进。运行期 Type process、扩展状态、action 引用和独占上下文由本文档后续规则单独约束；它们不得借用或发明新的生命周期状态名来规避本节规则。

允许的状态名：

- `Base`
- `Prepared`
- `Ready`
- `Online`
- `Offline`
- `Destroyed`

允许的生命周期 transition 名：

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
- `Enable` 表示让已经构建完成的对象进入服务状态，通常推进到 `Online`。别名包括：启用、上线、进入服务、保护、`guard`。当语义是建立栈边界保护或 guard 这类运行约束时，仍使用 `Enable` 作为正式 transition 名；单纯刷新对象属性的动作不因此升级为生命周期 transition。
- `Disable` 表示对象退出主要服务路径或完成资源所有权交接，但对象元数据仍保留给诊断、引用收尾或后续销毁。
- `Cleanup` 表示对象退出服务或释放阶段性抽象，通常推进到 `Destroyed`。

检查点：

- `model` 阶段必须检查对象 `initial_state`、状态声明名、transition 声明名和 transition 目标状态名。
- 任何不在受控集合内的名称必须报 `error`。
- `derive`、`view`、`render` 不得通过名称猜测或展示修正来补偿非法名称。

## SEM-TRANSITION-001: Lifecycle Transitions Are Controlled

当前生命周期模型只能使用预先定义的状态迁移三元组 `(source_state, lifecycle_transition, target_state)`。别名不参与迁移定义；`.spec` 中必须使用正式状态名和 transition 名。

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

- `model` 阶段必须检查每个 transition 声明的源状态、transition 名和目标状态三元组。
- 任何不在允许迁移集合内的三元组必须报 `error`。
- 新增迁移必须先修改本文档、`model` 阶段检查器和对应测试。

## SEM-UNIQUE-001: Forward-Only Model Forbids Duplicate States And Transitions

同一个 `object` 内，`Transition::X` 只能定义一次。transition 定义身份是 `(Object, Transition)`，不是 `(Object, SourceState, Transition)`。

原因：

- `drives` 引用形式是 `Object.Transition::X`，不包含源状态。
- 如果同一对象内允许多个 `Transition::X`，引用目标会变得不唯一。
- 当前状态只决定 transition 是否可触发，不参与 transition 命名。
- 即使将来引入可反复触发的运行期 transition，重复触发也不等于重复定义；同一个对象状态机中的同名 transition 定义仍必须唯一，除非显式引入 transition 重载或可重入定义规则。

检查点：

- `model` 阶段必须扫描同一对象的所有 `state.transitions`。
- 发现重复 `Transition::X` 时必须报 `error`，并指向重复定义位置。
- `derive`、`view`、`render` 不得通过 `source_state -> target_state` 为重复 transition 消歧。

例外：

- 只有将来显式引入“transition 重载/可重入 transition 定义”的对象类型或语义后，才能放宽 transition 定义唯一性；默认所有对象都禁止重复定义同名 transition。

## SEM-TRANSITION-ACTION-001: Transition And Action Are Distinct

`transition` 表示一次尝试推进被建模状态的操作；`action` 表示依附于某个既有状态执行的状态内动作。迁移期语法中的 `Transition::Name` 若带 `StateEffect::Always` 或 `StateEffect::Conditional`，语义上属于 transition。

判断规则：

- 操作成功时推进被建模状态，应建模为 transition。
- 操作成功时仍停留在同一被建模状态内，应建模为 `action`。
- 不得为了表达可重复调用而把应为 `action` 的操作伪造成生命周期 transition。
- 不得为了绕过 transition 唯一性而把应为运行期 transition 的状态迁移伪造成 `action`。

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

            deferred action_domain.001 {
                category: DeferredCategory::Protocol;
                summary: "Complete the remaining action protocol.";
                evidence { action_protocol_deferred(self); }
                close_when: "The protocol and its failure paths are specified and tested.";
            }
        }
    }
}
```

- `actions` 块只能出现在 `state` 内；action 所在 state 是该 action 的 owner-state guard。
- action 成功时不得推进 owner 对象生命周期状态；owner 仍停留在 action 所属 state。
- action 名称在 owner 对象内唯一，引用身份是 `Object.Action::Name` 加实参绑定。
- action 可以带类型参数和命名形参；形参必须是对象引用或受控值类型。
- `drives` 可以引用 lifecycle transition，也可以引用 action；正式 process 定义必须声明命名形参并通过类型检查。调用点允许单参数 process 使用位置实参简写，例如 `SetRuntimeState(TaskRuntimeState::Running)`，工具按签名规范化为 `SetRuntimeState(state: TaskRuntimeState::Running)`；多参数 process 仍必须使用完整命名实参，避免同类型参数互换后仍通过类型检查。
- action 的 `depends_on` 是调用成功前提；被 transition 驱动的 action 若 `depends_on` 不满足，该 transition 不得提交生命周期迁移。
- action 的 `ensures` 在 action 成功后成立，并可作为驱动它的 event 成功路径上的可用事实。
- action 的 `ensures` 不得直接伪造生命周期提交，例如不得用 action 确保 `SomeObject.state == State::Ready` 来替代 `SomeObject.Transition::Setup`。
- 参数化 action 不得把具体目标对象编码进 action 名；具体对象差异应通过实参和对象自身 facts 表达。
- transition 可以通过 `drives` 调用 action，以复用不推进被建模状态的内部动作；这适合表达 `Setup`/`Enable` 这类 lifecycle transition 内部的初始化、发布、入队、唤醒、查询或其它属性动作。
- transition 调用 action 后，外层 transition 仍负责提交自己的状态迁移；action 只贡献自己的 `ensures`，不得替代外层 transition 的目标状态提交。
- 如果某个过程会改变已建模生命周期状态或扩展状态，它必须保持为 transition，不能为了减少定义而降级成 action。也就是说，复用 action 只能消除无状态迁移的重复动作，不能隐藏真实状态迁移。

示例：`copy_process()` 应建模为 `TaskCreationCore` 在 `Ready` 状态内的参数化 action，而不是为每个目标任务建立 `CopyKernelInitProcess` 之类的专名 action：

```text
on Action::CopyProcess<Src: Task, New: Task>(
    src_task: Src,
    dst_task: New,
    pid_ns: RootPidNamespace,
    creds: CredentialCore,
    signal: SignalCore,
    files: TaskFileContext,
    security: SecurityCore,
    scheduler: Scheduler,
    flow: TaskFlow
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
        task_owns_flow(dst_task, flow);
    }

    ensures {
        task_creation_copy_process_committed(TaskCreationCore, src_task, dst_task);
        task_creation_used_clone_args(TaskCreationCore, dst_task);
        task_creation_bound_flow(TaskCreationCore, dst_task, flow);
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

## SEM-TRANSITION-BODY-001: Transition And Within Body Members Are Ordered

state-changing transition 和 `within` 的 body 是 source-ordered member 序列。`depends_on`、`drives`、
`within`、`ensures`、`deferred` 等块在源码中的相对顺序必须由 parser/AST/JSON
保留；工具可以继续维护按 kind 分类的兼容字段，但不得用分类字段替代正式顺序语义。

执行语义：

- `depends_on` 仍表示进入所属 transition/within 成功路径前必须满足的前提。
- `drives` 按 source order 执行；同一个 transition/within 中多个 `drives` 块不得被合并到
  `within` 前或 `within` 后。
- `within Context { ... }` 只保护自己的词法 body；它前后的 `drives`、`ensures`
  或其它块不属于该 context。
- 嵌套 `within` 按 body member 顺序进入和退出；外层 context 的有效上下文覆盖其
  词法 body，内层 context 在该范围内继续叠加。
- `ensures` 仍表示所属 transition/within 成功退出后成立的事实；它可以承接前面按顺序
  执行的 action/type-process ensures，但不得被当成提前成立的前置事实。

这个规则用于表达 Linux 中常见的局部保护区，例如：

```text
drives {
    Object.Action::Prepare;
}

within SomeContext {
    drives {
        Object.Action::ProtectedCommit;
    }
}

drives {
    Object.Action::Finalize;
}
```

上述形式表示 `Prepare` 和 `Finalize` 都在 `SomeContext` 之外，只有
`ProtectedCommit` 在 `SomeContext` 之内。代码生成或 lowering 不得把这三个动作
重排为“全部先执行 drives，再进入 within”，也不得把 context 扩大到整个 transition body。

## SEM-TRANSITION-EMITS-001: Completion Events Are Post-Commit Events

状态机是惰性的；外部事件或迁移完成事件触发 transition，transition 自身不得在未被
事件触发时主动推进。`emits` 是 transition 的一等子块，用于声明“当前 transition
完成后发出的迁移完成事件”。它可以触发本对象的后续 transition，也可以触发其它对象
的 transition，用于表达状态提交后的跨对象事件交接：

```text
on Transition::Preset -> State::Prepared {
    drives {
        Child.Transition::Setup;
    }

    emits {
        Transition::Setup;
        OtherObject.Transition::Preset;
    }
}
```

执行顺序固定为：

- 执行当前 transition 的 `depends_on`、source-ordered body、`drives` 和 `within`。
- 确认 owner 对象仍在当前 transition 的 source state。
- 提交 owner 对象状态到 target state。
- 验证 target state invariant。
- 按 `emits` 中的 source order 触发对应 transition。本对象的 `Transition::Name`
  必须从当前 target state 出发；跨对象的 `OtherObject.Transition::Name` 从目标对象
  当前状态出发，由目标 transition 自身的 source state 和 `depends_on` 约束判定是否
  可执行。

`emits` 的约束：

- `emits` 条目可以写成 `Transition::Name` 或 `OtherObject.Transition::Name`。
- 被发出的 transition 必须存在。
- 当 `emits` 使用本对象 transition 时，被发出的 transition source state 必须等于
  当前 transition 的 target state。
- 当 `emits` 使用跨对象 transition 时，不要求目标 transition 的 source state 等于
  当前 transition 的 target state；目标对象状态由事件链执行时判定。
- `emits` 本身不跨推导链。跨对象 `emits` 仍属于当前推导链内的完成事件交接；未来
  只有任务创建、中断流建立等会产生新推导链的 action/migration 结果，才能触发链的
  创建语义。
- `emits` 不是 `drives` 的别名；`drives` 表达当前迁移过程中的外部驱动，
  `emits` 表达当前迁移已经提交并通过目标状态 invariant 后的 completion event。
- `emits` 目前只覆盖 lifecycle transition completion event。state-local
  `actions { ... }` 的正式 action event 语法落地后，可以在本规则基础上扩展
  action event，但本轮不展开。

默认推导入口是 `ComputerProject.Transition::Preset`。该入口表示唯一外部工程启动
事件 `PRESET`；后续 `ComputerProject`、`KernelProject` 和 `Kernel` 的启动推进必须
通过 `emits` completion-event 链或显式 `drives` 表达，不得依赖工具隐式补齐
`Setup`/`Enable`。

## SEM-TYPE-PROCESS-001: Type Processes Define Reusable Runtime Semantics

`type` 定义可复用对象类型的共同属性、owned 子对象、标准生命周期 process、运行期 process、扩展状态和约束。`object X: SomeType` 表示 `X` 是 `SomeType` 的一个具名实例；实例绑定并继承 `SomeType` 上定义的 Type process。也就是说，`X.Transition::Setup`、`X.Transition::Enable` 或 `X.Action::Done` 若来自 Type 定义，语义上是“对实例 X 执行 Type process”；其中 `Transition::Setup` / `Transition::Enable` 是 lifecycle transition 的迁移期语法，不是外部事件。

Type body 中的 `key: ValueType;` 是 Type 属性声明，必须被 parse/model 工具保留。`owned { field: ChildType; }` 声明 Type 实例拥有的子对象或内嵌资源；owned 子对象随宿主实例建立实例身份，不是外部引用，也不表示普通参数传递。Type body 中的命名块按语义分类：

- `lifecycle { ... }` 定义 Type 实例的标准生命周期 transition process，名称应使用受控生命周期 transition 名。
- `processes { ... }` 定义实例进入可服务状态后的运行期 process；其中 `Transition::Name` 是迁移期语法，若 `state_effect` 不是 `None`，语义上是运行期 transition；`Action::Name` 是 action。
- `owned { ... }` 定义实例拥有的内嵌资源或子对象；Type process 可以通过 `self.field.Transition::Name` 或 `self.field.Action::Name` 驱动它们。
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
- 当前 derive 已能从 Type process 的 `ensures` 推导被驱动 action/transition 的成功事实；对象实例自己声明的 `actions { ... }` 暂时只作为可驱动 action commit 展示，不自动把该 action 内部 `ensures` 推导为后续 `depends_on` 的可用事实。阶段主线若需要消费对象 action 的结果，必须暂时在外层 transition/phase `ensures` 中显式承载，后续再扩展对象 action ensures 推导。

Type/Instance 的基本原则是：可复用 Type 已定义的行为、状态效果和通用 facts，必须落在 Type 上；实例只负责把该 Type 行为绑定到具体场景，并补充场景级语义。实例不得复制 Type 内部 bookkeeping，例如 `Completion` 实例不应重新手写 token、wait queue 或 wake-one 的通用结果；这些结果应来自被驱动的 `Completion` Type process。若当前工具尚不能自动把 inherited lifecycle transition 展开为实例生命周期迁移，可以暂时使用实例 wrapper transition 提交实例状态，但 wrapper 的职责只能是驱动或承接 Type process，并提交场景级 facts。

没有显式拆分 `type T` 和 `object X: T` 的对象，规格语义上可以视为匿名 singleton type 的唯一实例：对象自身同时承载唯一实例身份和该实例的专属行为定义。该形式只适用于确实只有一个实例且短期没有复用需求的对象；一旦需要多个实例、对象引用泛化、owned 子对象复用或跨实例不变量，必须拆出显式 Type，让实例通过 `object X: T` 绑定。

`Completion` 是当前第一个正式 Type process 示例。Linux `struct completion` 内嵌 `swait_queue_head wait`，因此规格中 `Completion` 拥有 `SimpleWaitQueue`，不是引用外部 wait queue。`Completion.Setup` 驱动 owned wait queue 的 setup；`Complete`/`CompleteAll` 驱动 wait queue 的 wake action；`Wait` 驱动 wait queue 的 prepare/finish wait action；`Reinit` 只重置 completion token 状态并保留 wait queue。`Complete`、`CompleteAll`、`Wait`、`TryWait`、`Reinit` 是 `StateEffect::Conditional` 的运行期 transition；`Done` 是 `StateEffect::None` 的只读 action。

Completion 也说明了 transition/action factoring 的边界：`Completion.Setup` 可以调用 `SimpleWaitQueue.Setup`，`Completion.Complete` 可以调用 `SimpleWaitQueue.WakeOne` action，因为这些子动作本身不推进 Completion 的扩展状态；但 `Completion.Complete` 仍不能改成 action，因为它会把 `CompletionExtState::Pending` 推进到 `CompletionExtState::Completed`，或在其它扩展状态下按条件迁移表提交结果。

`Task` 是唯一的 task_struct-like 载体类型；运行时可以同时存在多个彼此独立的 `Task` 实例。`BootTask`、`KernelInitTask`、`KthreaddTask` 与具名见证 `UserChildTask1` 都是这个同一类型的实例。boot idle 只是 `BootTask` 的 Flow handoff，不产生第二个 Task。PID 1 的用户态身份、地址空间、文件、凭据、信号与 trap frame 直接关联稳定实例 `KernelInitTask`，不经过 persona wrapper。`UserTaskSet` 表示一般用户 Task 集合；每次 fork/clone 都向集合加入 fresh Task，该 Task 具有独立 PID 和 lifecycle。DSL 尚不能动态声明匿名对象，所以 `UserChildTask1` 只见证一个具体 child，禁止用它代表后续 child 身份。

`Task` 保存调度身份、任务执行上下文和 `TaskThreadContext`；每个 `TaskFlow` 实例则保存自己的 lifecycle state。Flow 类型与实例必须同时存在：`BootInitFlowType` 的临时具名实例是 `RootStream`，`KernelInitFlowType` 的实例是 `KernelInitFlow`，`KthreaddFlowType` 的实例是 `KthreaddFlow`，`BootIdleFlowType` 的实例是 `BootIdleFlow`；所有用户应用映像共用 `UserAppFlow` 类型，但每次 exec 或 fork continuation 都创建独立实例。应用映像不同不产生新的 Flow 类型。

Flow 关系分为三类，不能混用：`task_owns_flow(task, flow)` 记录 Task 曾经拥有该实例，`task_active_flow_is(task, flow)` 记录当前 active binding，`task_flow_handoff(task, from, to)` 记录 handoff 历史。每个 Flow 只有一个 owner；一个 Task 可以按 exec 顺序拥有多个 Flow，但任一时刻最多一个 Flow Online。当前正式 handoff 是 `BootTask: RootStream -> BootIdleFlow`、`KernelInitTask: KernelInitFlow -> Pid1UserAppFlow` 和 `UserChildTask1: UserChildForkFlow1 -> UserChildExecFlow1`。不同 Task 绝不共享同一 `UserAppFlow` 实例。

`UserAppFlow` 的统一 lifecycle 是：Preset 绑定唯一 owner 与入口来源并建立 fresh/独占关系；Setup 准备 exec 映像或 fork continuation 的执行上下文；Enable 成为 owner 唯一 Online Flow 并跨入用户应用黑盒；Disable 处理 exit、exit_group 或 successful-exec replacement；Cleanup 释放实例并保证它不再 active。用户应用内部不声明 action 或 transition；syscall、trap、files 和其它内核资源操作仍属于相应内核对象。successful exec 不替换 Task：新 Flow 先 Preset/Setup，旧 Flow 再 Disable，随后提交 active binding handoff、新 Flow Enable，最后旧 Flow Cleanup。

Task 退出必须先 Disable/Cleanup 当前 Flow；`Task.Disable` 要求所有 owned Flow 已 inactive，`Task.Cleanup` 进一步要求它们都已 Destroyed，因此 Flow 仍存活时 Task 不得进入 Destroyed。动态匿名 Task/Flow 创建、owned Flow 集合与泛型实例 lifecycle 调用仍需要后续 DSL/验证器能力。`RootStream` 作为 `BootInitFlowType` 的临时具名封装保留；它的移除以及全仓 Stream -> Flow 迁移继续 deferred。

`TaskRuntimeState` 是 `Task` 的扩展运行态，不是对象 lifecycle state。因此，设置任务运行态应建模为 `Task.Transition::SetRuntimeState(state: TaskRuntimeState)` 这样的运行期 transition，而不是 `Action::SetTaskState`。当前实现先使用简单的 `StateEffect::Conditional` 和普通 fact 表达运行态提交；后续引入状态机模型后，每次进入特定 `TaskRuntimeState` 时应执行 transition guard、leave-state check 和 enter-state consistency check，例如确认调度实体、runqueue 选择、锁/抢占/中断上下文和跨对象不变量。

`task_state_running(task)` 是 `task_runtime_state_is(task, TaskRuntimeState::Running)` 的派生别名。调用 `Task.Transition::SetRuntimeState(TaskRuntimeState::Running)` 后，Type process ensures 先提交运行态事实，再由 derive 自动推出该别名；阶段规格不应在 `Enable.ensures` 中重复手写这个别名。

`Task.Action::PinToBootCpu(cpu_ref: CpuRef)` 是状态内 action，用于提交 task 的亲和性约束属性，不推进 task lifecycle，也不改变 `TaskRuntimeState`。在 `rest_init()` 中，调用点写为 `KernelInitTask.Action::PinToBootCpu(BootCPURef)`：receiver 已经确定目标 task，`BootCPURef` 是 `BootCPU` 发布的 CPU 引用。该 action 只提交两类属性事实：设置 `PF_NO_SETAFFINITY` 等价的 task flag，以及把 task cpumask 限制到 boot CPU。Linux 源码中的 `find_task_by_pid_ns(pid, &init_pid_ns)` 是用局部 pid 重新取回 task 指针的实现路径，不作为正式参数或 drives；规格层已经持有 `KernelInitTask` receiver。该源码路径由 `rcu_read_lock()/unlock()` 定界，当前保留为 deferred 上下文建模问题：它是否属于资源独占上下文，还是应建模为独立的 RCU/读侧上下文，后续讨论。

正式规格必须区分对象和对象引用。对象是被规格化的实体本身，拥有 lifecycle state、runtime state、facts 和 invariants；引用是某个上下文中可持有、传递和访问对象的能力或句柄。`TaskRef`、`RunQueueRef` 这类引用值通过 `task_ref_targets(ref, object)`、`runqueue_ref_targets(ref, object)` 绑定目标对象。`CurrentTaskRef` 是当前 CPU 视角下只属于本 CPU 的任务引用对象；规格不引入 `CurrentTask` 这种描述性对象，也不把 `CurrentTaskRef` 建模为全局 singleton。BP 规格中的 `CurrentTaskRef` 属于 `BootCurrentCPU` 的 current-task 视图，当前最小路径绑定到 `BootTask`；未来 AP 规格应建立各自 CPU 视角下的私有 current-task 引用，而不是复用 BP 的引用。`CurrentRunQueueRef` 是当前 CPU 视角下只属于本 CPU 的当前 runqueue 引用对象；BP 最小路径中它指向 `BootRunQueue`，CPU 归属是 `BootCPURef`，并由本 CPU 的 `CurrentTaskRef` 指向任务的 CPU 归属间接确定。规格不引入描述性 current-runqueue 对象，也不把 `CurrentRunQueueRef` 建模为全局 singleton。action 返回对象引用时，调用方必须用 action result binding 显式承接返回值，例如 `let selected_rq: RunQueueRef <- Scheduler.Action::SelectRunQueue(...)` 或 `let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(...)`。该绑定是局部 SSA 风格值，作用域覆盖后续 drives 语句和嵌套 `within`；嵌套上下文直接使用该词法可见绑定，不通过 `within` 传参或重命名。后续对目标对象的操作应使用引用 receiver，例如 `selected_rq.Transition::EnqueueTask(...)`、`prev_ref.Action::SaveCoreContext`，而不是把当前策略结果硬编码为 `BootRunQueue.Event` 或 `BootTask.Action`。任务记录的当前 CPU 归属是 `Task` 类型的公共属性，正式更新形态是 `Task.Action::SetTaskCpu(cpu_ref)`，调用点使用具体 task 对象 receiver，例如 `KernelInitTask.Action::SetTaskCpu(BootCPURef)`。

Ref receiver 的正式分发规则是：若 `R` 是 `XXXRef` 类型的引用值，且 `XXXRef` 的目标对象类型 `XXX` 声明了 `Transition::E` 或 `Action::A`，则 `R.Transition::E(...)` / `R.Action::A(...)` 表示通过引用对目标对象执行 `XXX` 类型定义的 process；process 内部的 `self` 绑定到引用当前指向的目标对象。引用类型也可以声明“引用自身”的 process，例如 `TaskRef.Action::SetCurrent(task)` 更新引用目标本身；这类 process 不分发到目标 `Task`，其 `self` 是引用对象。引用目标的属性或 owned 子对象透明访问是同一语义方向，但当前工具尚未提供统一语法和类型检查；正式规格暂时使用 `task_ref_targets(...)`、`runqueue_ref_targets(...)`、`runqueue_ref_cpu_is(...)` 等 fact 承载，后续再引入 `Ref.attr` / `Ref.child` 的解析规则。

`CurrentTaskRef` 的正式语义是 CPU 视角私有的 current-task 引用：它由本 CPU 的 current-task 机制产生，可能由实现通过私有寄存器组、CPU-local 存储或其它架构设施承载，但模型层不把这些实现承载方式称为 `CurrentTaskRef` 的本体。发生本 CPU 任务切换时，`SchedulerObject.Action::SwitchTo(prev_ref, next_ref)` 必须提交 `next_ref` 成为本 CPU current-task 引用目标的事实。其它 CPU 的 current-task 进展对本 CPU 规格来说只能作为可观察环境事实进入，而不是由本 CPU 的 `CurrentTaskRef` 直接表达。

`CurrentRunQueueRef` 的正式语义是 CPU 视角私有的 current-runqueue 引用：它不是全局 runqueue 单例，也不是 `BootRunQueue` 的别名。调度路径应先从本 CPU `CurrentTaskRef` 得到当前 task，再读取该 task 记录的 CPU logical-id / `cpu_ref`，最后通过 `CpuGroup.Cpu[id].RunQueue` 与 scheduler runqueue metadata 解析当前 runqueue 引用。BP 当前最小路径中这个解析固定为 `CurrentRunQueueRef -> BootRunQueue` 和 `CurrentRunQueueRef -> BootCPURef`，但原因是 `CurrentTaskRef -> BootTask` 且 `task_cpu_ref_is(BootTask, BootCPURef)`，不是因为 current runqueue 解析以 `CpuGroup.boot_cpu()` 为起点。未来 SMP 泛化时，AP 视角应拥有自己的 `CurrentRunQueueRef`，并由对应 CPU 的 current task 与 CPUGroup/runqueue topology 解析目标 runqueue。

`SchedulerObject.Action::SelectRunQueue(task_ref: TaskRef) -> RunQueueRef` 是状态内 action。它只根据任务引用和当前调度条件选择目标 runqueue 引用，不推进 `Scheduler` lifecycle state，不提交 runqueue 成员关系，也不直接更新 task 记录的 CPU id。参照 Linux，`select_task_rq()` 只返回目标 CPU，后续由 `set_task_cpu()` / `__set_task_cpu()` 更新 `task_struct.thread_info.cpu`，再进入 task rq lock 和 enqueue/activate。规格中该更新表达为 `task.Action::SetTaskCpu(cpu_ref)`，并位于 `SelectRunQueue` 和 `EnqueueTask` 之间。当前 `rest_init()` 最小路径固定返回 `BootRunQueueRef`，即 boot CPU runqueue，并通过 `runqueue_ref_cpu_is(selected_rq, BootCPURef)` 证明调用方写入 `Task.SetTaskCpu(BootCPURef)`；后续 SMP 选择策略应继续由 selected `RunQueueRef` 的 CPU 事实驱动 `Task.SetTaskCpu`，而不是由调用方硬编码 boot CPU。`RunQueueRef` 与 `CurrentRunQueueRef` 必须保持类型语义分离：`CurrentRunQueueRef` 只表示当前 CPU 视角的 current runqueue，不能作为 `SelectRunQueue` 返回值、普通入队目标或 smoke task 入队目标复用。完整 `select_task_rq()` 策略，包括 affinity、wake flags、scheduler class、load balance、SMP、migration disabled 和 cpuset 等，后续作为 deferred 策略展开。

`SchedulerObject.Action::Schedule` 是 `Scheduler.Online` 后的调度分界 action。

### Replicated PhaseObject family

当 phase 规格明确声明对象按 target key（例如 secondary CPU `logical_id`）replicate 时，model
中的单个 `PhaseObject` 表示该 key 集合上的实例族。每个 key 独立拥有对象状态；对象的
`depends_on`、`ensures`、invariant 和同对象 `emits` 都按该 key 解释。

父 transition 中针对 replicated siblings 的 source-ordered `drives` 是 pointwise order：同一
key 上前一 sibling Online 后才能推进后一 sibling，不同 key 之间允许交错。family 级
`state == State::Online` 表示目标集合中所有实例 Online，只提供聚合完成事实，不隐含跨 key 的
阶段屏障，也不把实例 transition 的 owner 转移给聚合读取者。当前语法不增加 indexed-object
表达式；具体 family key 和聚合解释必须由对应 charter/model 注释与 coding 映射共同固定。
它不推进 `Scheduler` lifecycle state，但会提交一次调度边界的运行期事实。
`schedule_preempt_disabled()` 仍由调用方展开为三段式：
先退出调用方继承的 preempt-disabled guard，再调用
`Scheduler.Action::Schedule`，最后进入新的 boot-idle preempt-disabled
上下文。`Schedule` 自身内部则建模 `schedule()`/`__schedule()` 的最小边界：
先由 `PreemptionControl.Disable` 边界建立 schedule-owned 不可抢占上下文，再由
`LocalInterruptControl.SaveAndDisable` 边界关闭本 CPU 本地中断，然后在 runqueue lock context 中
先从本 CPU current-task 视图得到 `CurrentTaskRef`，再执行
`let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef)`，
最后进入 `SchedulerObject.Action::SwitchTo(CurrentTaskRef, next)`。

`SwitchTo` 对应 Linux `context_switch()` 中 `prepare_task_switch()` 和
`finish_task_switch()` 之间的 `switch_to(prev, next, last)` 核心位置。RISC-V
实现中 `switch_to` 先处理 `thread.prev_cpu`、FPU/vector/icache 等架构钩子，
再由 `__switch_to` 保存 `prev->thread` 并恢复 `next->thread` 的核心寄存器。
正式规格当前只覆盖 `__switch_to` 的核心寄存器组：`ra`、`sp` 和
callee-saved `s0..s11`。这些寄存器不属于 `Scheduler`，而属于每个 `Task`
拥有的 `TaskThreadContext` 内嵌结构。`SwitchTo(prev_ref, next_ref)` 内部通过
`prev_ref.Action::SaveCoreContext` 和 `next_ref.Action::RestoreCoreContext`
提交保存/恢复事实；完成后 next 必须成为本 CPU current-task 引用目标。当前 UP
最小路径允许 `prev == next == CurrentTaskRef`，该引用目标是 `BootTask`，
因此 `SwitchTo` 只提交 identity switch 框架事实和核心上下文保存/恢复事实，
不执行真实 task stack switch。完整 `prev != next` 切换、`last` 返回值、MM 切换、FPU/vector、
`prepare_task_switch()`/`finish_task_switch()` 钩子、`sched_submit_work()`、
worker sleep/running hook、RCU context switch 和 scheduler class pick 细节后续按对象展开。

`RunQueue` 使用 `RunQueueRuntimeState::{None, Some}` 表示是否至少存在一个可运行 task ref。`task_refs: TaskRefSet` 是该状态关联的数据视图，`nr_running` 不作为独立源状态，而是 `count(task_refs)` 的派生度量。当前 `RunQueue.task_refs` 是调度类队列尚未展开前的汇总视图；未来引入 CFS/RT/DL 等调度类子队列后，具体成员关系应由这些子队列维护，`RunQueue.task_refs` 退化为派生视图。`RunQueue.Transition::EnqueueTask(task_ref: TaskRef)` 是运行期 transition，因为它提交 runqueue 成员关系并推动 `None -> Some` 或 `Some -> Some` 的运行态迁移；重复入队应作为失败结果处理。该 transition 的基础成员事实统一表达为 `runqueue_contains_task(self, task_ref)`；阶段级或跨对象派生事实可以继续使用 `task_enqueued_on_runqueue(task_ref, runqueue_ref)` 表示已经经过 `SelectRunQueue`、`Task.SetTaskCpu` 和入队的整体结果。`EnqueueTask` 不能直接编码为 `BootRunQueue` 专属动作：调用方应先消费 `SelectRunQueue` 返回的 `RunQueueRef`，确认或更新 `task_ref` 的 CPU id，再在该 runqueue 的锁建立的资源独占上下文内通过 `selected_rq.Transition::EnqueueTask(...)` 提交入队。

## SEM-EXCLUSIVE-CONTEXT-001: Guard And Resource Exclusive Context Are Distinct

`Lock` 表示可建立独占边界的同步对象。`Lock` 不带泛型，不拥有被保护资源；锁与资源的关系由资源独占上下文表达。

`context ... : ResourceExclusiveContext` 表示通过某个 guard 建立的受保护执行作用域。它不是普通 lifecycle object，不拥有 guard、锁或资源；它只保存引用关系、guard 边界和作用域语义。

`guard` 表示某个上下文为何成立。它可以是运行时显式进入/退出的机制，
例如锁、RCU 读侧、关抢占或关中断；也可以是阶段边界或启动早期事实这类
天然成立的边界。Context 的正式建模机制是统一的：
`within ContextName { ... }` 进入由 guard 定义的上下文，执行块内行为，再按
guard 定义退出。对于没有独立运行时 enter/exit 动作的天然 guard，
`within` 的词法范围本身就是进入和退出边界。
临界区上下文、原子上下文、RCU 读侧上下文等分类主要是规格语义描述上的分类；
在 formal 结构上不需要拆成不同的 context 语法类别，也不需要在源语法中给
guard 再命名一个 kind。差异来自 guard 引用的对象、进入/退出边界事件、
guard 明确保持的属性以及上下文引用集合：RawSpinLock 边界产生资源互斥效果，
PreemptionControl guard 产生不可被普通抢占打断的原子上下文效果，
LocalInterruptControl guard 产生本 CPU 本地中断关闭效果。嵌套检查和上下文强度
叠加也应基于 guard 推导出的上下文贡献，而不是基于 context 名称。

对于当前 wake-up 试验对象，guard 引用一个 `RawSpinLock` 实例，并通过该锁实例的
`LockIrqSave`/`UnlockIrqRestore` 事件
建立进入和退出边界。guard 本身不是锁实例；锁实例仍由 `lock Name: RawSpinLock`
定义。

正式结构：

```text
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

    obj_refs: {
        KernelInitTask;
        Scheduler;
        BootRunQueue;
    }
}
```

规则：

- `guard.lock_ref` 可以引用一个 `lock` 声明或可作为同步实例的对象；具体同步语义由该引用目标的类型和边界事件推导。
- `guard` 必须采用两种形态之一：运行时边界形态声明成对的
  `guard.entered_by` 和 `guard.exited_by`；天然上下文形态只声明
  `guard.holds`。不得只声明单侧边界，也不得把 `entered_by`/`exited_by`
  与 `holds` 混写在同一个 guard 中。
- `guard.entered_by` 和 `guard.exited_by` 声明进入和退出上下文边界事件。
- 对非锁 guard，`entered_by`/`exited_by` 声明对应控制对象的边界事件；这些边界事件是 guard 行为，不写入 `within` 内部的 `drives`。
- `guard.exited_by { Never; }` 是唯一允许的显式无正常退出标记。它必须和
  `entered_by` 成对出现，表示该 context 在当前正常控制流中进入后没有对应
  runtime 退出事件；`within` 词法结束不代表 guard 退出，外层 phase/action
  仍可在 context 持续成立时提交 ready facts。后续 shutdown、panic、CPU
  offline 或其它非正常路径若需要退出语义，必须另建 terminal/context 路径，
  不得把 `Never` 偷换成普通 enable/restore 事件。
- 对阶段边界或天然上下文 guard，`entered_by`/`exited_by` 可以不存在；`within` 的词法范围提供边界。
- `guard.holds` 声明天然上下文在作用域内明确保持的属性；未声明的维度表示该 guard 不作保证，在 Effective Context 叠加时保持中性。
- `ResourceExclusiveContext.obj_refs` 是受保护对象引用集合，至少包含一个对象；
  普通 `Context` 可以省略 `obj_refs`，省略时不作为对象驱动白名单。上下文不拥有这些对象。
- 纯 `guard.holds` 的正式写法是“名词或名词短语 key + 状态形容词 value”：
  `local_interrupts: enabled|disabled`、`preemption: enabled|disabled`、
  `voluntary_switching: enabled|disabled`、`cpu_concurrency: single|multi`、
  `task_concurrency: single|multi`。旧 `true|false` 只作为迁移期兼容输入。
- 首轮 guard contribution 推导如下：`RawSpinLock.LockIrqSave` 边界推导本地中断关闭、抢占关闭、主动切换关闭；`RawSpinLock.Action::Acquire` / `Release` 可作为普通 raw spin lock 的 guard 边界，只贡献该 `ResourceExclusiveContext` 的锁保护和 `obj_refs` 独占范围，不额外推导 irqsave、抢占关闭或主动切换关闭；`PreemptionControl.Disable` 边界推导抢占关闭和主动切换关闭；`LocalInterruptControl.SaveAndDisable` / `Disable` 边界只推导本地中断关闭；没有进入/退出事件的天然 guard 不自带运行时 effect，只由 `holds` 明确声明阶段边界保证的事实。
- 同一把锁可以被多个 resource exclusive context 的 guard 引用，用于建立不同受保护作用域。
- ResourceExclusiveContext 的锁 guard 如果边界事件带有 owner/current-task
  实参，例如 `Mutex.Transition::Lock(TaskRef)` / `Unlock(TaskRef)`，则同一个
  context 的 `guard.entered_by` 内只能出现一个 owner 实参集合，
  `guard.exited_by` 内也只能出现一个 owner 实参集合，且二者必须完全一致。
  同一把 mutex 在不同执行 owner 线上复用时，必须拆成多个 owner-specific
  context；不得在一个 guard 中同时列出 `Lock(BootTaskRef)` 和
  `Lock(KernelInitTaskRef)`，否则无法从规格上保证 unlock 释放的是同一 owner
  获取的锁。
- resource exclusive context 不需要 lifecycle state；进入上下文是一次由 guard 保护的独占执行尝试。
- 同一时刻至多一个执行流可以成功进入同一个 resource exclusive context。
- `within ContextName { ... }` 是唯一标准形态；`within` 不声明、不接收、不转发、不重命名实参。
- 规格编写时应强烈优先使用 `within ContextName { ... }` 表达 guard 作用域。只要被保护的词法 body
  可以用 `within` 准确表示，就不应把 `Guard.Enter -> protected drives -> Guard.Exit`
  平铺为同一个 `drives` 序列。例外情况可以存在，例如 guard 边界跨越无法用单个词法
  body 表达的控制流、进入/退出并不形成作用域，或当前工具尚不能表达必要的动态绑定；
  这些情况应在规格中说明原因。
- `within ContextName only-once { ... }` 表示该具体 lexical `within` 块声明自己在
  `ComputerProject.Transition::Preset` 可达调用图中只被进入一次。该标记必须由 model
  工具计数验证，验证失败即为规格错误；它不由 coding 或 impl 重新证明。当前
  formal source 暂不使用该标记驱动 guard 省略；它作为工具能力保留，等待后续
  guard 优化机制单独恢复。
- 外层 `drives` 中由 action result binding 产生的局部值按词法作用域对后续语句和嵌套 `within` 可见；需要别名时应在 `drives` 或上下文内部另建显式绑定，不得写成 `within ContextName(arg: value)`。
- `within` 块内只能直接驱动 `obj_refs` 中对象的 action/event，除非规格显式声明允许外部对象。
- context 成功退出后释放独占执行权；失败或 `Blocked` 时，外层 event 不得提交生命周期迁移。

事件或 action 使用无实参 `within` 声明上下文作用域。`within` 块内可以包含 `depends_on`、`drives`、`ensures` 和 `deferred`。进入/退出边界由 context 的 guard 声明，`within` 不再重复声明 `entered_by`/`exited_by`，也不把 guard 的进入/退出事件写入内部 `drives`。guard 推导出的上下文贡献是工具内部用于检查和渲染的归一化结果；源规格不应把这些结果作为与 guard 并列的第二套事实重复维护。

`KernelInitTask.Enable` 对应 `wake_up_new_task()` 的正式规格形态如下：

```text
state State::Ready {
    transitions {
        on Transition::Enable -> State::Online {
            within WakeUpNewTaskContext {
                depends_on {
                    task_state_new(KernelInitTask);
                    task_not_enqueued(KernelInitTask);
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
                    scheduler_select_runqueue_returns(Scheduler, KernelInitTaskRef, BootRunQueueRef);
                    task_runqueue_selected(Scheduler, KernelInitTaskRef, BootRunQueueRef);
                    task_cpu_ref_is(KernelInitTask, BootCPURef);
                    task_enqueued_on_runqueue(KernelInitTaskRef, BootRunQueueRef);
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

`within` 的语义是：先通过指定 context 的 guard 进入或确认该 context；进入成功后，在该作用域内执行块内的 `drives`；块内驱动全部成功后，`within` 的 `ensures` 成立，随后按 guard 退出或离开词法范围，外层 transition/action 才能继续提交自己的 `ensures`。`within` 不是普通参数传递，也不是对象所有权转移；它的标准形态始终是 `within ContextName { ... }`。外层 `drives` 中由 action result binding 产生的局部值在嵌套 `within` 中保持词法可见，因此 `selected_rq` 不需要也不允许作为 `EnqueueSelectedRunQueueContext` 的实参重复传入。当前 UP 路径通过 `runqueue_ref_targets(selected_rq, BootRunQueue)` 证明该 ref 指向 `BootRunQueue`，所以 context guard 暂时绑定 `BootRunQueueLock`；后续泛化时应从 `RunQueueRef` 解析目标 runqueue 及其 lock。

`only-once` 是 `within` 使用点上的可验证断言，不是 `Context` 类型属性。同一个
context 可以在一个地方被 `only-once` 使用，在另一个地方作为普通可复用上下文使用。
工具验证时从 `ComputerProject.Transition::Preset` 出发，沿 `drives` 和 `emits` 调用图统计每个标记
`only-once` 的 lexical block 的可达进入次数；计数不是 1 时必须报错。

当前策略是先建立基本 guard 规格和保守 lowering，不把 Effective Context 自动作为
省略 protocol guard 的依据。因此 formal source 暂时不应为了 guard 优化添加
`only-once` 使用点；已实现的 parser/model 支持和测试保留为后续优化基础。未来若
恢复 proof-only lowering，仍必须同时满足：外层 Effective Context 覆盖该 guard 的
高层语义、该 guard/call-site 显式允许优化、lexical block 通过 `only-once` 证明，
且没有 saved flags/token/owner/debug/memory-ordering 等运行时协议副作用消费者。

## SEM-CONTEXT-NESTING-001: Effective Context Composes Monotonically

上下文可以嵌套。当前执行流真正受到的运行约束不是某一个单独
`Context` 的名称，而是所有外层与内层上下文叠加后的结果。该结果称为
`Effective Context`。当前已经落地的是“通过锁人工定义边界”的资源独占上下文；
后续还需要正式规格化系统独占上下文，例如启动早期天然只有单执行流可达的
系统上下文，或者通过关闭中断、关闭抢占等机制建立的系统上下文。

嵌套的语义不是替换外层上下文，而是把外层 Effective Context 与内层
guard 推导出的 context contribution 叠加为新的 Effective Context。
Effective Context 决定当前流能访问哪些对象、能获得哪些层级的对象句柄，
以及能否主动切换、能否被抢占、本地中断入口是否关闭等运行约束。

源规格应优先声明 `guard` 和 `obj_refs`：运行时 guard 通过成对的
`entered_by`/`exited_by` 声明边界，由边界事件和引用目标类型推导该 context
对 Effective Context 的贡献；天然上下文 guard 通过纯 `holds` 声明阶段或词法
范围内已经成立的事实。`effects` 是工具内部可用于检查、
渲染或调试的归一化结果，不是 formal source 中必须人工重复维护的第二套事实。
当前需要表达和推导的最小维度包括：

- `local_interrupts`：本 CPU 本地中断入口是否关闭。
- `preemption`：当前执行流是否允许被普通抢占。
- `voluntary_switching`：当前执行流是否允许主动进入可能导致任务切换的普通
  `schedule()`、`cond_resched()`、`might_sleep()`、阻塞等待或 mutex 慢路径。
- `exclusive_refs`：当前作用域独占或受保护访问的对象引用集合。
- `cpu_concurrency`：是否存在多 CPU 并发执行风险。
- `task_concurrency`：是否存在普通任务并发或调度切换风险。

`handle_level` 是后续要加入的 effect 维度，用于表达当前作用域对对象可见的
句柄层级或 capability；首轮工具实现先不推导句柄层级。

纯 `guard.holds` 只应声明该天然上下文明确保证的事实。某个维度如果不由当前
guard 保证，就保持缺省；缺省不是 `true` 或 `false`，而是
neutral/inherited，表示该 context contribution 对该维度不作保证，叠加时
不改变外层已经存在的约束，也不能凭空生成新的证明能力。

嵌套检查必须满足单调加强规则：从外到内可以越来越强，但不能反向削弱外层已经建立的约束。也就是说，内层上下文可以进一步关闭中断、关闭抢占、扩大受保护对象集合或提升对象句柄层级；但不能在外层已经要求本地中断关闭、不可抢占或不可睡眠时，引入语义上要求本地中断开启、可抢占或可睡眠的上下文。

例如，在一个要求不可睡眠的自旋锁上下文中，嵌套一个需要阻塞等待的
Mutex guard，应被判定为上下文嵌套违例。原因是内层 guard contribution
与外层 Effective Context 冲突，不能形成合法的叠加结果。

多种上下文和上下文嵌套的正式规格化，是主规格中“组件化内核在不同上下文可以访问对象不同层级句柄”的具体化：guard contribution 栈给出当前流的访问能力，句柄层级由 Effective Context 推导，而不是由对象所有权或普通参数传递隐式决定。

迁移期工具仍兼容旧 `.spec` 手写 `effects` 块。该兼容只是过渡输入形式，
不是最终源规格语义；工具应从 guard schema 推导 context contribution。
若迁移期同时存在手写 `effects`，工具必须检查它不得弱于、偏离或重复矛盾于
guard 推导结果。

句柄层级推导、对象 transition/action 的上下文需求声明、系统天然独占上下文的来源证明等更丰富的
guard/effect 语义仍在后续扩展范围内。RCU 读侧上下文目前只有
`SchedInitPhase` 的 `BootIdleRcuReadSide` first slice：它记录 `init_idle()` 中
`__set_task_cpu()` 外层的 balanced `rcu_read_lock()`/`rcu_read_unlock()` guard，不表示完整
RCU reader nesting、preemptible-RCU accounting、quiescent-state 或 scheduler/RCU context-switch
语义。

## SEM-CPU-VIEW-MODEL-001: CPU View Is The Base Modeling View

正式规格以 `CPU视角` 为基础，不支持脱离具体执行 CPU 的“上帝式”全局全知视角。每个 `CPU视角` 描述的是：本 CPU 自身拥有或可直接访问的本地状态、本 CPU 可以驱动的事件/action，以及本 CPU 能观察到的环境事实。其它 CPU 的内部执行进展，对当前 CPU 来说只能通过同步对象、ack、共享对象状态、拓扑事实、IPI 可见结果等环境事实进入当前视角，而不是由当前 CPU 直接展开或控制。

CPU 的 live 寄存器组也是 `CPU视角` 的私有对象，包括通用寄存器组 GPRs 和控制状态寄存器 CSRs。每个 CPU 都拥有自己的 GPR/CSR 实例集合；一个 CPU 不能在自己的规格步骤中直接读写另一个 CPU 的 live registers，只能通过 trap frame、saved task context、IPI/同步结果或共享内存中已经发布的保存副本观察间接结果。RISC-V64 中 `tp` 属于本 CPU 的 GPR 视图，`sstatus`、`stvec`、`sie`、`sip`、`satp` 等属于本 CPU 的 CSR 视图；现有规格中 `Riscv64.tp`、`Riscv64.satp` 这类具名写法是 BP 迁移期的当前 CPU register alias，后续规范化时应收敛为 `CurrentCPU.cpu.Registers` 或等价 CPU-local register group 下的成员。

`BP视角` 和 `AP视角` 是 `CPU视角` 的两个具体分类。BP 是唯一且必须存在的启动 CPU，承担主要内核初始化职责；AP 是后续进入的 secondary CPU，复用共享类型语义和 BP 已建立的共享环境，但必须拥有自己的 `CurrentCPU`、本地中断控制、current-task 引用、GPR/CSR 寄存器组和 AP entry/ack 路径。

多个 CPU 视角共同可见、共同依赖或共同维护的公共事实集合，命名为 `共享全局视角`。它包括共享内存对象、全局 phase 边界、`CpuGroup`/topology、全局调度设施、同步对象和跨 CPU 可见状态等。`共享全局视角` 不是新的执行主体，也不是可以同时支配所有 CPU 私有步骤的全局控制视角；它只是各个 `CPU视角` 中公共可见环境的规格化名称。CPU 私有对象或引用，例如 `CurrentTaskRef`、本地中断状态、当前寄存器组和当前任务切换结果，必须留在对应 CPU 视角内表达。

## SEM-CURRENT-CPU-MODEL-001: CurrentCPU Is The CPU-Local Self Identity Entry

`RawSpinLock`、本地中断开关和抢占开关的正式建模必须建立在 `CurrentCPU` 上。`CurrentCPU` 表示每个 CPU 启动时天然拥有的“当前 CPU 自我身份入口”；它不是 `Context` 语义中的资源访问上下文，也不是 `within` 使用的资源独占上下文。

`CurrentCPU` 的首轮正式语义如下：

- 每个 CPU 启动时自动产生一个与自身唯一对应的 `CurrentCPU` 实例。
- `CurrentCPU` 独立于 `CpuGroup`，不挂在 `CpuGroup` 对象树下；BP 侧的 `CurrentCPU` 早于几乎所有内核对象存在。
- `CurrentCPU` 拥有与自身对应的统一 `CPUObject` 实例；`BootCPU` 是 logical id 为 `0`、带有 bootstrap role 的 CPU 实例，不是独立 CPU 类型。
- 每个 `CPUObject` 实例持有或发布自己的 `hartid`、`logical_id`、`possible`、`present`、`active` 和 `online` 事实。`hartid` 属于 CPU 实例属性，不应长期保存在 `CpuGroup` 普通字段中。
- `CpuGroup` 后续维护对这些 CPU 对象的引用、`CpuGroup.Cpu[logical_id]` 索引和拓扑组织关系，而不是天然拥有 CPU 本体。
- `CpuGroup` 可以维护 possible/present/online 集合视图；集合元素是 `CpuRef` 或等价 CPU 引用，目标仍是统一 `CPUObject` 实例，不引入拥有 CPU 本体的 `PossibleCpu` / `PossibleRunQueue` 之类中间对象。
- 内核启动最早期，当前执行 CPU 只通过自己的 `CurrentCPU` 认识自身和访问自己拥有的 CPU 对象；此时规格层尚不应假定已经存在 `BootCPU`。
- `CurrentCPU.Preset` 记录入口或固件交付的 hartid。
- `CurrentCPU.Setup` 记录或确认 logical CPU id，并把该事实发布到自身拥有的 CPU 对象。
- `CurrentCPU.Enable` 在 `CpuGroup` 建立后，把自身拥有的 CPU 对象注册/绑定到 `CpuGroup.Cpu[id]` 引用索引和对应集合视图。
- `BootCPU` 后续应从早期身份对象逐步退化为 `CurrentCPU.cpu` 所指 CPU 的 bootstrap role、alias 或描述性 fact；现有 `BootCPU` 对象可在迁移期保留，以避免一次性大范围重写。

secondary CPU 的边界必须单独区分。`CpuGroup` 可以先从 `PlatformCpuInfo`、DeviceTree 或 topology 事实中知道 possible secondary CPU，并维护对应 `CPUObject` 实例引用、logical-id 索引和 possible/present 集合视图；但在某个 AP 真实进入 secondary entry 之前，不应假定该 AP 的 live `CurrentCPU` 已经存在，也不应认为该 AP 已经拥有可操作的 CPU-local interrupt、current task slot 或 task preemption 控制链。BP 侧如果提前为 AP 准备 idle task、hotplug state、runqueue 元数据或同步 completion，这些对象应标记为 BP 侧为 future CPU 准备的描述/资源，不等同于 AP 自己的 `CurrentCPU` 已建立。AP 进入后，才由该 AP 自动产生自己的 `CurrentCPU`，拥有或接管对应 CPU 对象，并把该 CPU 对象注册/绑定回 `CpuGroup.Cpu[id]` 引用索引和集合视图。

完成上述绑定后，与当前锁建模相关的访问链应为：

```text
CurrentCPU
    -> cpu
    -> EventStream
    -> InterruptStream / ExceptionStream
    -> LocalInterruptControl
    -> CurrentTaskSlot
    -> current Task
    -> PreemptionControl

CpuGroup
    -> CpuRef[logical_id] -> CPUObject instance
    -> possible/present/online set views
```

其中 `EventStream` 属于具体 CPU，并在该 CPU 的 trap entry 中把异常和中断分流到自己的 `ExceptionStream` 与 `InterruptStream`；`LocalInterruptControl` 属于具体 CPU，`CurrentTaskSlot` 也属于具体 CPU 并保存当前任务引用；`PreemptionControl` 属于当前 task/thread_info。`preempt_disable()` 和 `preempt_enable()` 不应直接作用于被唤醒或被创建的任务，而应通过 `CurrentCPU -> cpu -> CurrentTaskSlot.current_task` 找到当前正在执行的任务后再驱动该任务的抢占控制事件。

当前 boot CPU 规格中，具名 `EventStream`、`InterruptStream` 和 `ExceptionStream` 是 boot CPU 的事件入口流迁移期实例。后续 AP 进入 secondary entry 时，每个 AP 必须建立自己的 live `CurrentCPU -> cpu -> EventStream` 链，再由该 `EventStream` 关联自己的 `InterruptStream`/`ExceptionStream`；BP 侧提前为 future CPU 准备的描述对象或 handler 模板不等同于 AP 自己的 live event stream。

`LocalInterruptControl` 应接管本 CPU local interrupt 总开关状态的直接控制能力，即 RISC-V 上的 `sstatus.SIE`。`InterruptStream` 不应直接修改这个总开关，也不应对外提供普通的 `local_irq_enable/disable` runtime transition；它只能在自身 lifecycle transition 中驱动 `CurrentCPU.cpu.LocalInterruptControl` 完成阶段边界所需的 enable/disable。`LocalInterruptControl` 则只提供可反复调用的 runtime transitions，例如 `Disable`、`Enable`、`SaveAndDisable(out flags)` 和 `Restore(flags)`；本地中断总开关打开/关闭是运行期状态变化，不是 `LocalInterruptControl` 自身 lifecycle transition。

`InterruptStream` 仍负责中断流的 source/pending 层，例如 RISC-V 的 `sie` source enable bits 和 `sip` pending bits，以及 cause 到 handler policy 的分发表。它可以直接在自身事件中清理或维护 `sie/sip` 这类分开关/挂起状态；这不等同于直接操作 `sstatus.SIE` 总开关。正式边界是：`sstatus.SIE` 只能由 `LocalInterruptControl` 直接改变，`InterruptStream` 可以驱动 `LocalInterruptControl`，也可以直接管理 `sie/sip`。

`RawSpinLock` 类型建模在该访问链成立后展开。`raw_spin_lock_irqsave()` 对应的事件应同时驱动三类效果：保存并关闭当前 CPU 本地中断、关闭当前任务抢占、获取锁本体。释放路径应先释放锁本体，再恢复本地中断状态并打开当前任务抢占。示例形态如下：

```text
RawSpinLock.Transition::LockIrqSave(current_cpu: CurrentCPU) {
    drives {
        current_cpu.cpu.LocalInterruptControl.Transition::SaveAndDisable(out flags);
        current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Transition::Disable;
        self.Action::Acquire;
    }
}

RawSpinLock.Transition::UnlockIrqRestore(current_cpu: CurrentCPU, flags: IrqFlags) {
    drives {
        self.Action::Release;
        current_cpu.cpu.LocalInterruptControl.Transition::Restore(flags);
        current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Transition::Enable;
    }
}
```

普通 `raw_spin_lock()` / `raw_spin_unlock()` 对应 `RawSpinLock.Action::Acquire` / `Release`。如果调用点已经位于外层 irqsave 或其它上下文中，内层普通 raw spin lock 应建模为单独的 `ResourceExclusiveContext`，其 guard 使用 `Action::Acquire` / `Release`，而不是伪装成第二个 irqsave guard，也不能把该锁边界降级为普通 predicate。

`KernelInitTask.Enable` 的 `within WakeUpNewTaskContext` 由 `KernelInitTaskPiLock.Transition::LockIrqSave(current_cpu: CurrentCPU)` 建立进入边界，并由对应的 `UnlockIrqRestore` 建立退出边界。`within` 块内部只保留受保护资源对象的 action/transition，例如设置 task runtime state、选择 runqueue 和入队任务。

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

## SEM-PROCESS-RESULT-001: Transition And Action Results Are Explicit

transition 和 `action` 都应具有显式返回结果。正式结果集合由 `ProcessResult` 语义定义，最小包括：

- `Success`：操作成功完成。
- `Failed(code)`：操作失败，可携带具体错误码或原因。

需要表达等待、重试或条件暂未满足时，可以使用：

- `Blocked(reason)`：本次操作没有提交成功结果，但不是语义错误。

对 transition 的结果约束：

- 只有 `Success` 才提交 `state_effect`，并使目标状态、目标扩展状态及该事件的 `ensures` 成立。
- `Blocked` 和 `Failed` 都不得使对象进入目标状态或目标扩展状态，也不得假定该事件的 `ensures` 成立。
- 对 `StateEffect::Conditional` 的 event，`Success` 后是否发生状态变化、变化到哪里，以 Type process 的条件迁移表为准。

对 `action` 的结果约束：

- `Success` 表示动作完成，但对象仍停留在 action 所属的被建模状态内。
- `Blocked` 和 `Failed` 表示动作未完成或失败，同样不推进被建模状态。

当前推导器把对象 lifecycle transition 视为成功路径上的静态推导；失败、阻塞、错误码和 Type process 结果尚未进入推导语法。后续扩展错误路径或运行期 process 推导时，必须保持上述提交语义。

## SEM-LIFECYCLE-OPERATIONAL-001: Lifecycle And Runtime Transitions Have Different Trigger Rules

生命周期 transition 用于建立或销毁对象生命周期状态，当前正式集合为 `Preset`、`Setup`、`Enable`、`Disable`、`Cleanup`。在同一对象生命周期轮次中，每个生命周期 transition 原则上至多成功触发一次。当前启动模型采用 forward-only 推进模式，重复触发生命周期 transition 应视为规格错误。

运行期 transition 用于对象进入某个可服务生命周期状态后的运行期状态迁移，例如锁、队列、映射、分配器、调度器和 I/O 对象中的 `lock`、`try_lock`、`unlock`、`enqueue`、`dequeue`、`map`、`unmap`、`alloc`、`free`、`read`、`write` 等。运行期 transition 可以在对象生命周期内反复触发；每次触发都必须符合该对象当前运行状态的迁移规则。

示例：普通非嵌套自旋锁中，`lock` 和 `try_lock` 都是运行期 transition，因为它们成功时都会把锁从 `Unlocked` 推进到 `Locked(owner=current)`。`lock` 在锁被别人持有时可以返回 `Blocked(occupied)`，在已经被自己持有时返回 `Failed(nested_lock)`；`try_lock` 在锁被别人持有时返回 `Failed(busy)`，在已经被自己持有时返回 `Failed(nested_lock)`。二者可以具有相同的成功源状态和目标状态，但它们是两个不同 transition。

## Non-Hard Guidance: Object Granularity Is Hierarchical

本节是建模建议，不是硬语义规则；`model`、`derive`、`view`、`render` 不得把本节作为 error 或 warning 的强制来源。

对象可以有不同粒度。当前建模层级中不再继续拆分的最小粒度对象可视为原子对象；除此之外的对象通常是复合对象，由更小粒度对象组合而成。复合对象状态通常由自身属性和子对象状态支撑，复合对象 transition通常由子对象 transition支撑。

在规格描述和对象实现中，建议优先使用足以表达当前语义的高粒度对象，以便逐级封装实现细节和复杂性。该建议不改变生命周期状态名、transition 名、迁移三元组和 transition 唯一性等硬规则；当现实实现受限时，可以展开较低粒度对象或采用临时承载方式，但应在说明或 coding 规格中记录原因。
