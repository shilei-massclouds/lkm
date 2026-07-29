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
- `OnCpu`（仅限 `Task` 类型及其实例）
- `Offline`
- `Destroyed`

允许的生命周期 transition 名：

- `Preset`
- `Setup`
- `Enable`
- `Disable`
- `Cleanup`
- `Continue`（仅限 `Task`）
- `Suspend`（仅限 `Task`）

语义约定：

- `Base` 的别名包括：初始态、基态、尚未建立。
- `Prepared` 的别名包括：预置态、前置条件已建立。
- `Ready` 的别名包括：就绪态、主要构建已完成。
- `Online` 的别名包括：在线态、已启用、可服务。
- `OnCpu` 表示 Task 当前实际占有 CPU；它不是通用对象或 Phase 状态。
- `Offline` 的别名包括：离线态、已退出主要服务、资源已交接、`handoff`。`Handoff` 不是正式状态名。
- `Destroyed` 的别名包括：已销毁、已退出服务、已清理、已预留、`reserved`。`Reserved` 不是正式状态名。
- `Preset` 表示建立进入主要构建流程前的早期前置条件，通常推进到 `Prepared` 或 `Ready`。
- `Setup` 表示完成对象的主要构建，使对象进入 `Ready`。
- `Enable` 表示让已经构建完成的对象进入服务状态，通常推进到 `Online`。别名包括：启用、上线、进入服务、保护、`guard`。当语义是建立栈边界保护或 guard 这类运行约束时，仍使用 `Enable` 作为正式 transition 名；单纯刷新对象属性的动作不因此升级为生命周期 transition。
- `Disable` 表示对象退出主要服务路径或完成资源所有权交接，但对象元数据仍保留给诊断、引用收尾或后续销毁。
- `Cleanup` 表示对象退出服务或释放阶段性抽象，通常推进到 `Destroyed`。
- `Continue` / `Suspend` 只表达 Task 的 `Online <-> OnCpu` 调度往返；Task 的 terminal
  handoff 可从 `OnCpu` 直接 `Disable` 到 `Offline`，不得先制造不可恢复的 Online 断点。

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
- `Online --Continue--> OnCpu`（仅限 `Task`）
- `OnCpu --Suspend--> Online`（仅限 `Task`）
- `OnCpu --Disable--> Offline`（仅限 `Task` 的 terminal handoff）

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

## SEM-DECLARE-001: Drives May Declare Fresh Runtime Instances

`declare name of Type;` 是 transition/action 及其嵌套 `within` 的 `drives` 中唯一的运行期
instance 声明语法。它是 source-ordered executable statement，不是顶层 declaration，也不是
静态 object 的别名。

执行规则：

- derive 执行到声明语句时创建一个 declared Type 的 fresh instance；初始 lifecycle state 必须是
  `State::Base`。声明本身不调用 `Preset`，也不建立任何 transition commit。
- 同一 declaration site 每次执行都创建不同 runtime identity，实例间不得共享 state、facts、
  transition commits 或 trace node。失败路径不隐式 rollback、Disable 或 Cleanup 已创建实例。
- `declare` 不建立 parent、owner、active binding 或集合成员关系；这些关系只能由后续显式
  process commit 和正式 fact 建立。
- Type 必须存在。动态 receiver 的 transition/action 由 declared Type 及其父 Type process 解析；
  process 不存在、参数不匹配或 lifecycle 迁移非法都必须报错。

Alias 规则：

- declaration alias 与 `let` 结果 alias、owner process 参数共享词法 SSA namespace。
- 禁止重复声明、shadow、use-before-declare，以及与当前位置可见的静态 object 重名。
- alias 从声明位置之后开始，对当前 `drives` 后续 statement 和随后进入的嵌套 `within` 可见。
  嵌套 scope 内声明的 alias 不反向泄漏到父 scope，也不泄漏到 sibling scope。
- callee 不捕获 caller alias；runtime instance 只能通过显式 typed process 参数传给 callee。
- alias 离开 scope 不销毁 instance。实例可通过 ownership/集合 facts、Action result 返回的 typed
  `Ref` 或后续 process 参数持久化并重新定位；alias 绝不是全局 object name。

静态模型和 JSON 规则：

- `ObjectModel.objects` 只包含源码具名 objects。declaration site 保存在 owner process 的有序
  drive statements 中；runtime instance 只存在于 derive/trace 输出。
- AST/model JSON 中 declaration statement 必须显式包含 `kind = "declare"`、`alias`、
  `declared_type`、owner process、owner-local source ordinal 和诊断 span。普通 process call、`let`
  binding、static object 与 declaration site 不得复用同一 JSON shape 来隐式消歧。
- runtime JSON 必须显式区分 `declaration_site`、`runtime_instance_id`、`alias`、`declared_type`
  和可选 `static_object`；运行期实例的 `static_object` 必须为空。

稳定 identity 规则：

- runtime identity 由“根调用路径 + owner process + declaration site 的 owner-local source ordinal +
  alias + 该调用路径下 occurrence”组成。
- source ordinal 来自 owner process 的有序 executable statement/declaration-site 序列，不使用
  物理源码行号；源码行号只用于诊断。因此无关空行或源码行移动不得改变 identity。
- occurrence 对同一根调用路径和 declaration site 单调区分每次执行。trace/view/render 不得把
  多个 occurrence 折叠为一个 alias node。
- static view/render 展示 declaration site；derive/trace view 展示实际 runtime instances 及各自
  state/commit。任何展示层都不得把 runtime instance 写回 static object inventory。

首版不新增循环、并发声明调度或通用垃圾回收语义；多 instance 来自同一 declaration site 在
不同推导调用中的重复执行。

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

所有 `emits` 都是严格 completion event。目标 transition 若因 source state 或
`depends_on` 不满足而不能接受，或者异步 handler 后续失败，整个根执行必须失败。已经发送的事件
不得静默忽略、排队等待未来状态或重试。

`emits` 的约束：

- `emits` 条目可以写成 `Transition::Name`、`Action::Name` 或带 receiver 的相同形式。
- receiver 也可以是具有静态类型的 association path，例如
  `self.initial_flow.Transition::Preset`。当 association path 穿过 `TaskRef` 时，
  先按该引用当前绑定的 `Task` 解引用，再访问 Task association。每一级 association 和最终
  transition 都必须能由静态类型唯一解析；动态 receiver 不能绕过类型检查。
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
- `emits` 覆盖 lifecycle transition、显式 state-changing runtime Transition 与 Action Signal；
  后两者不改变 `emits` 必须在 source commit 后才投递的规则。

完整模型的默认推导入口是唯一的外部 `Human` 编排。Human 先同步 drives
`Computer.Transition::Preset`，成功后同步 drives `Computer.Transition::Setup`；两者都完成后再异步
emits `Computer.Transition::Enable`。`external` 不是模型对象：没有 lifecycle、parent 或 handler，
也不进入对象树。任一同步 drives 失败立即短路，后续 drives 和 emits 均不发送；Computer 的三个
handler 不相互发送 Signal。后续运行启动固定为
`Computer.Enable -> Riscv64Platform.Enable -> OpenSBI.Enable -> Kernel.Enable` 的 completion-event
链；Kernel.Enable 再把 `BootInitFlow.Preset` 作为其 Ready→Online 迁移内的第一个下层 `drives`。
`Config` 与 `Lds` 初态 Ready，并由 Kernel.Setup 依次驱动 Enable。所有实际 Signal
仍必须由显式 `emits` 或 `drives` 产生，不得依赖工具隐式补齐。

## SEM-SYSTEM-PARENT-001: Effective Parent Is Shared Across Model Consumers

模型构建器必须先建立类型继承，再按下列顺序归一化每个静态系统的 effective parent：

1. 源声明存在显式 `parent` 时原样采用，后续规则不得覆盖。
2. 若组合模型包含具名 `Kernel`，无显式 parent 的系统采用 `Kernel`；具名 `Computer` 是系统树显式
   根，不应用该默认。系统树为 `Computer -> {Riscv64Platform, OpenSBI, Kernel}`；启动 CPU 的局部层级为
   `CurrentCPU -> BootCPU -> BootCpuRegisters`。
3. 不含 `Kernel` 的独立 fixture 保留声明形成的根集合，不建立不存在的默认 parent。

归一化后必须检查 unknown parent、self parent 和传递环，并把 effective parent 写入 model protocol。
老 `tools/model`、tools2 model、文本树、静态 trace/SVG 和 Signal hierarchy coordinate 都必须消费该字段，
不得各自重建另一套 parent 解释。parent 不产生隐式 Signal 传播或 process 继承。

## SEM-SIGNAL-TOOLS2-001: Full-Model Signal Derivation Is An Isolated Compatibility Semantics

本节的完整 Signal envelope 语义只约束 `tools2/`；受支持的旧 `tools/` 路径必须用自身既有协议解析、
建模、推导和展示同一 external 默认编排与显式单 Signal 边界，但不导入 tools2 实现。
`tools2` 的 `lkm.spec.*` 中间协议统一使用 version `5` 并带 `producer: "tools2"`；每个阶段必须拒绝
producer 或 version 不匹配的输入，不能把 tools2 version 1/2/3/4、老工具 JSON 或旧 snapshot 当成兼容输入。

### Normalization and handling

当前折叠调用 `Receiver.Transition::Name(args)` 或 `Receiver.Action::Name(args)` 规范化为一个 Signal
envelope：source 是当前响应 owner（根请求默认 `Human`），target 是解析后的 Receiver，name 是
`Name`，payload 是按 handler 形参完成类型检查和规范化的命名值。根 CLI 的
`--signal Target.Name` 直接建立相同 envelope；根 Signal 不允许用 process kind 替代 target/name。
tools2 外部请求中的 `Target.Startup` 是 `Target.Preset` 的保留别名。shortcut、driver 和 derive API/CLI
必须在处理请求前把它规范化为 `Preset`；模型调用、handler 查找、Signal record、ID、事件和 canonical
JSON 只保留 `Preset`。该别名不扩展受控 transition 集合，DSL 仍必须使用 `Transition::Preset`。

目标对象中同名 Transition 或 Action 是兼容 handler；其中同名 lifecycle Transition 按
`SEM-LIFECYCLE-INHERIT-001` 构造唯一 effective handler，普通 Action 保持其已有的类型特化解析。
只有 `lifecycle_override: true` 才完整替换传递 lifecycle 贡献。effective handler 与 owner state 的
`depends_on` 均满足时才接受 Signal；没有 handler、同名 handler 歧义、owner state 不匹配或任一前提
不成立均得到 `rejected`。参数缺失、未知参数、重复参数、值类型不匹配、非法引用、规格矛盾或
invariant 失败属于 `failed`，不能降级为 rejected。Transition handler 成功时提交 target state；
Action handler 不改变 lifecycle state，只提交其 `ensures` 事实。

完整模型事实语义支持嵌套泛型类型、属性、owned/association/reference、predicate/function、context/
lock、Type process、对象 override、局部 action result binding、`within`、`deferred`/`trimmed` evidence
以及当前主模型中的 state/fact/reference 表达式。scenario 可以在模型初态上覆盖 state、fact 和
reference。初态 state invariant 建立带来源的初始事实；有 body 的 predicate 按实参替换后在当前快照
求值。`has_slot(value, SlotKind::Member)` 还可以消费 model protocol 中的声明结构：derive 必须先把
`value` 的实例属性路径解析到声明类型，再在该类型及其基类型的 `slots` 字段中查找 `Member` 对应的
槽位。只有实际声明存在时条件才成立；实现不得把 `has_slot(TypeName, ...)` 的事实按字符串替换成
`has_slot(instance.field, ...)`，也不得对特定对象或枚举成员硬编码。条件事件应把这种证明标记为
`model_structure`，与来自当前快照的精确事实区分。未知新语法必须在 parse/model 诊断中以
`unsupported` 和 source span 报告，不能静默删除、按名称猜测或留给 view/render 修正。

显式根 Signal 没有 scenario 时从模型初态接收。完整主模型的初态包含 Online 的静态
`Riscv64`/`SbiSpec`/`BootArgs`、Ready 的 `Config`/`Lds`，以及 Base 的 Computer 和三个直接子 System。Ready 只表示
本实例已构造且可启动，不表示已经运行。省略根 Signal 时执行模型中唯一的 `external Human` 编排；
`Kernel.Enable` 或其它
后续 Signal 若因前期状态或事实未建立而被拒绝，必须直接得到列出缺项和
完整因果链的 `failed`。derive 不得隐式回溯 emitter、执行上游 transition 或合成前置快照；调用者从
后续边界继续时必须显式提供真实边界的 snapshot/scenario。

快捷入口未给 `-t/--trigger` 时从初态执行唯一的 `external Human` 编排；第一个真实 Signal 是
`Human -> Computer.Preset`，但不存在 `Human.Startup` envelope。快捷入口、driver 和 derive 的 source
默认均为 `Human`，显式 `--source` 优先；底层 driver/derive 省略 `--signal` 时采用相同外部编排。
显式 `--signal` 只建立该单个根 Signal，不补齐外部编排的前序或后继 lifecycle。

`Startup -> Preset` 规范化规则不因运行系统改为 Enable 启动而改变；`Startup` 不是 `Enable` 的别名。
因此 `Kernel.Startup` 表示设计期 `Kernel.Preset`，不能替代真实固件交接 `Kernel.Enable`。

### Ordering, rejection and completion

`drives` 是 source-ordered 逻辑同步 Signal：目标响应（包括它的同步子响应）完成后，源 handler 的
逻辑过程才继续；实现可以用跨栈切换、跨 Task continuation 或 checkpoint 边界承载，不要求目标响应
与 source 位于同一物理调用栈。
任一 drives 被拒绝，根推导为 `failed`。`drives` statement 写成 `A || B` 时表示有序备选：工具在同一到达前快照上按源码顺序检查 receiver、handler、payload、source state 和 `depends_on`，只发送首个可接受候选。未选候选只产生 choice-check 证据，不分配 Signal ID；若没有候选可接受，当前 drives 失败并报告每个候选的拒绝原因。候选一旦发送并开始响应，后续同步子响应失败不得回滚并改选另一个候选。
条件或 invariant 中的 `P || Q` 是短路逻辑析取，并与 drives 的有序 Signal 备选保持不同的规范化节点；它只读取当前快照，不发送 Signal。

`emits` 只有在源 handler 的 state/fact 提交并通过 invariant 后，才按 source order 追加到全局 FIFO。
源响应完成后由调度器逐项取出；每次 enqueue/dequeue 和调度选择都必须写入 trace。emits 被拒绝或
异步处理失败使根推导为 `failed`。条件不满足不得产生
`pending`；当前工具不建立等待队列、continuation 或未来 Signal 恢复。

### Pre-send until boundary

derive/driver/快捷入口都接受可选 `-u/--until Target.Name`。until request 与根请求
使用相同的 `Startup` 到 `Preset` 规范化。未给 until 时保持普通完整闭包语义；给出 until 时，工具按
规范化后的 target/name 精确匹配第一个实际将发送的 Signal。

匹配点必须在动态 receiver、实际 target、source-ordered choice 的唯一被选候选、payload 原始实参和
hierarchy coordinate 已确定之后，但在下列任何动作之前：分配 Signal ID、记录 `signal_sent`、把 emits
追加到 FIFO、记录接收、检查该目标的预算/handler/state/depends_on、绑定 payload 或执行 handler。
因此目标 Signal 在产物中没有 envelope、ID、sent/enqueue/receive/handler 事件。目标是根 Signal 时，
匹配发生在根 envelope 创建前，boundary snapshot 等于模型初态或显式 scenario。

到达边界后根 verdict 为 `reached`，除非在此之前已经 failed 或产生 truncated frontier；failed 优先于
bounded，bounded 又优先于 reached。boundary 必须记录规范化目标、发送方、delivery/coordinate/cause
构成的发送位置、调用 span（根请求为 null）和边界时的稳定 snapshot。正在执行但尚未提交的祖先响应
标为 `stopped`；已经提交的响应保持 `completed`；已创建、已入 FIFO 但尚未处理的 Signal 标为
`stopped`。停止后不再执行余下 body、提交 effect、创建后续 emits 或清空 FIFO，不产生 `pending`，也
不保存隐式 continuation。

`reached` 是 check 成功结果并返回 0。完整闭包自然结束仍未到达 until 发送点时，根 verdict 为
`until_signal_not_reached`，check 返回 1。failed、bounded 和 until_signal_not_reached 都不得写
snapshot。until 目标自身的预算、接收条件和 handler 不参与判断，因为该 Signal 尚未发送。

`within` 按 source order 进入/退出已建模 context，并把有效 context 写入事件与 Signal record；局部 action
result binding 对后续 statement 和嵌套 within 可见。受控提交只更新 lifecycle state、已声明 mutable
reference/association 和已证明 fact。结构化 deferred/trimmed 只保留 inventory 并在 owner 可达时验证
evidence，不生成 `blocked`。无限预算下若相同因果请求在完全相同快照再次出现，derive 必须以带调用
位置的 `causal_cycle_without_snapshot_progress` failed 结束。

Signal/响应 outcome 使用：envelope 接收判定的 `rejected`、响应结束的
`completed`、截至传播的 `stopped`、预算 frontier 的 `truncated` 和根推导的
`complete`/`reached`/`until_signal_not_reached`/`failed`/`bounded`。`pending` 是
保留词但当前工具不可产生。`blocked` 不属于 tools2 核心协议，只能出现在老工具兼容报告。

严格拒绝、类型/规格/invariant 错误必须保留根 Signal 到失败点的完整 `cause_id` 链。失败产物保存
initial snapshot、每个成功响应边界和 last stable snapshot；已经提交的边界不回滚。derive 只要成功
写出结构化诊断 JSON 就正常退出；check 对 `complete` 和 `reached` 返回成功，对
`until_signal_not_reached`、`failed` 和 `bounded` 返回失败。

### Hierarchical propagation budget

每个 Signal envelope 都保存相对根目标的 hierarchy coordinate `(depth, breadth)`。根目标是 `(0, 0)`；
预算由 `--max-depth` 和 `--max-breadth` 独立限制；底层 derive/driver 默认都是 `3`，快捷入口默认
`all/all`，显式非负整数表示上限，`all` 表示无限。预算在执行目标 handler 前检查；越界 Signal不执行，记录 `truncated` frontier，根结果为
`bounded`。failed 优先于 bounded。

坐标从当前 source target 到下一个 target 按 parent 树更新：

- self Signal 不消耗预算；向后代沿每条 parent 边增加 depth，breadth 在换层后重置为 `0`。
- 向祖先沿 parent 边回退 depth，breadth 在换层后重置为 `0`。
- 同一 parent 下换到 sibling 增加一次 breadth，depth 不变。
- 跨分支先退到最近公共祖先，再横移一次进入目标分支，最后沿 parent 边下钻；最终 depth 等于目标
  相对根目标的 parent 坐标，横移计入该层 breadth，下钻到新层时 breadth 重置。
- breadth 沿每条因果链独立继承；不同 drives 子调用或 FIFO entries 不争抢全局额度。

目标不在根目标所在 parent 树、parent 缺失或 parent 成环属于模型 `failed`，不能用预算截断掩盖。

### Required derivation records and resumability

`derive.json` 至少包含 root request、可选 until request/reached boundary、model fingerprint、initial snapshot、last stable snapshot、根 verdict、
Signal envelopes、resolved handler、typed payload、`drives`/`emits` delivery、hierarchy coordinate、全局事件
顺序、before/after snapshot、cause/response parent、outcome、拒绝/失败原因和 truncated frontier。Signal
ID 由根请求和稳定因果路径/同级 ordinal 生成，不使用临时目录、进程号或物理行号；同一输入重复运行
必须得到相同 ID、顺序和 JSON。

`complete` 和 `reached` 可由 driver 写出 `--snapshot-out`，其内容可作为后续 `--scenario` 的 snapshot
基础；reached snapshot 必须携带 boundary provenance。`failed`、`bounded` 和
`until_signal_not_reached` 都不得导出可续跑 snapshot。`view.json` 只能整理 derive 的结构化字段，不能重新
执行 guard、改变顺序或从 label 猜 outcome。默认 text renderer 按 Signal 创建顺序输出简化系统传播
图，不得按 hierarchy depth 重排；每行使用 Signal 的 `coordinate.depth` 做两空格层级缩进，存在负
depth 时以本次 trace 的最小 depth 整体平移。已解析 Transition 显示 target 在 before/after snapshot
中的实际状态，Action 不显示状态；停止或拒绝且未提交的 Transition 因此显示相同状态，未解析 handler
不得根据 Signal 名称或兼容 kind 猜测类型。文本显示层把 canonical `Preset` 拼作 `Startup`，结构化
协议仍只保存 `Preset`。非 `completed` Signal 在本行显示 outcome/reason；首行保留 verdict，reached
显示发送前 boundary，failed 保留 failure chain/reason，空 Signal trace 也必须是合法输出。

只有环境变量 `VERBOSE` 严格等于 `1` 时，text renderer 才输出既有详细视图，展示因果层级、同步等待、
异步 FIFO、payload、状态/事实/reference 变化、until request/reached boundary、stopped 传播、拒绝
原因、预算 frontier、source span 和完整失败链，并保留 canonical `Preset` 拼写。未设置、空值、`0`
或其它值均选择简化视图。这个渲染选择不得改变 derive/view JSON、snapshot、Signal 顺序、verdict 或
退出码。

## SEM-TYPE-PROCESS-001: Type Processes Define Reusable Runtime Semantics

`type` 定义可复用对象类型的共同属性、owned 子对象、标准生命周期 process、运行期 process、扩展状态和约束。`object X: SomeType` 表示 `X` 是 `SomeType` 的一个具名实例；实例绑定并继承 `SomeType` 上定义的 Type process。也就是说，`X.Transition::Setup`、`X.Transition::Enable` 或 `X.Action::Done` 若来自 Type 定义，语义上是“对实例 X 执行 Type process”；其中 `Transition::Setup` / `Transition::Enable` 是 lifecycle transition 的迁移期语法，不是外部事件。

Type body 中的 `key: ValueType;` 是 Type 属性声明，必须被 parse/model 工具保留。`owned { field: ChildType; }` 声明 Type 实例拥有的子对象或内嵌资源；owned 子对象随宿主实例建立实例身份，不是外部引用，也不表示普通参数传递。`associations { field: TargetType; }` 声明必须绑定的 typed association；object 使用 `associations { field = TargetObject; }` 绑定静态实例，动态实例可以由声明它的 transition 参数完成绑定。association 默认在首次绑定后不可改写；只有显式声明为 `mutable field: TargetType` 的 association 可以在 process 的 `updates { self.field = value; }` 中提交新 target。association 不取得 target ownership，也不等价于历史集合或当前 active binding。

- `lifecycle { ... }` 定义 Type 实例的标准生命周期 transition process，名称应使用受控生命周期 transition 名。
- `processes { ... }` 定义实例进入可服务状态后的运行期 process；其中 `Transition::Name` 是迁移期语法，若 `state_effect` 不是 `None`，语义上是运行期 transition；`Action::Name` 是 action。
- `owned { ... }` 定义实例拥有的内嵌资源或子对象；Type process 可以通过 `self.field.Transition::Name` 或 `self.field.Action::Name` 驱动它们。
- `associations { ... }` 定义必须由实例绑定、且能被静态类型检查的对象关系；
  association path 可以作为 fact 参数或 process/event receiver。
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

Type/Instance 的基本原则是：可复用 Type 已定义的行为、状态效果和通用 facts，必须落在 Type 上；实例只负责把该 Type 行为绑定到具体场景，并补充场景级语义。实例不得复制 Type 内部 bookkeeping，例如 `Completion` 实例不应重新手写 token、wait queue 或 wake-one 的通用结果；这些结果应来自被驱动的 `Completion` Type process。实例 wrapper transition 只声明自己的场景贡献；工具必须把继承贡献组成 effective handler，不能要求 wrapper 重写 Type 契约。

## SEM-LIFECYCLE-INHERIT-001: Lifecycle Handlers Accumulate Base To Derived

同名 lifecycle Transition 的有效契约由所有声明贡献累积组成。贡献顺序固定为最远基类型到
最近派生类型，最后是具体 instance；空壳基类型没有贡献时不产生隐式 lifecycle。所有贡献保留
owner、handler span 和每个 body member/entry span。

- handler 的参数签名、source/target state 与非 `None` `state_effect` 必须兼容；冲突在 model 阶段
  报错。
- `depends_on`、`ensures`、invariant 与其它 facts 累积；所有前置条件共同成立，`self` 绑定实际实例。
- `drives`、`within`、updates 等执行 member 按贡献顺序及各自源码顺序执行。
- `emits` 从执行 body 分离，只有整个 transition 状态提交且所有 target invariant 成立后，才按相同
  贡献/源码顺序入队。
- 派生类型或实例重复父级的规范化 condition、effect、drive 或 emit 是 model error。工具不得静默
  去重、执行重复副作用或用最近声明遮蔽父级。

`lifecycle_override: true` 是唯一完整替换机制。它要求 instance 同时声明完整 initial state 和 state
graph，并完全跳过传递类型贡献；局部 override 或通过缺省条目削弱继承契约不合法。老工具与 tools2
必须对同一 effective handler 得到相同的 guard/fact/action/emit 顺序。

没有显式拆分 `type T` 和 `object X: T` 的对象，规格语义上可以视为匿名 singleton type 的唯一实例：对象自身同时承载唯一实例身份和该实例的专属行为定义。该形式只适用于确实只有一个实例且短期没有复用需求的对象；一旦需要多个实例、对象引用泛化、owned 子对象复用或跨实例不变量，必须拆出显式 Type，让实例通过 `object X: T` 绑定。

`Completion` 是当前第一个正式 Type process 示例。Linux `struct completion` 内嵌 `swait_queue_head wait`，因此规格中 `Completion` 拥有 `SimpleWaitQueue`，不是引用外部 wait queue。`Completion.Setup` 驱动 owned wait queue 的 setup；`Complete`/`CompleteAll` 驱动 wait queue 的 wake action；`Wait` 驱动 wait queue 的 prepare/finish wait action；`Reinit` 只重置 completion token 状态并保留 wait queue。`Complete`、`CompleteAll`、`Wait`、`TryWait`、`Reinit` 是 `StateEffect::Conditional` 的运行期 transition；`Done` 是 `StateEffect::None` 的只读 action。

Completion 也说明了 transition/action factoring 的边界：`Completion.Setup` 可以调用 `SimpleWaitQueue.Setup`，`Completion.Complete` 可以调用 `SimpleWaitQueue.WakeOne` action，因为这些子动作本身不推进 Completion 的扩展状态；但 `Completion.Complete` 仍不能改成 action，因为它会把 `CompletionExtState::Pending` 推进到 `CompletionExtState::Completed`，或在其它扩展状态下按条件迁移表提交结果。

`Task` 是唯一的 task_struct-like 载体类型；运行时可以同时存在多个彼此独立的 `Task` 实例。`BootTask`、`KernelInitTask` 与 `KthreaddTask` 是静态具名实例；每次 fork/clone 则通过 `declare` 创建 fresh 动态 `Task`。boot idle 只是 `BootTask` 的 Flow handoff，不产生第二个 Task。PID 1 的用户态身份、地址空间、文件、凭据、信号与 trap frame 直接关联稳定实例 `KernelInitTask`，不经过 persona wrapper。`UserTaskSet` 表示一般用户 Task 集合；每次 fork/clone 都向集合加入 fresh Task/TaskRef pair，该 Task 具有独立 PID 和 lifecycle。动态 child 不获得全局具名 alias。`Task.Online` 只表示该 carrier 已完成 wake-up/runqueue publication、可以参与调度；它不表示该 Task 此刻占有 CPU，也不表示其 initial Flow 已经启动。

`Task` 保存调度身份并物理拥有 `TaskThreadContext`；每个 `TaskFlow` 实例只保存自己的
lifecycle state 和 continuation 身份。`TaskThreadContext` 的架构寄存器区固定为
`ra/sp/s0..s11`，同时保存正交的 breakpoint validity、所绑定的带 generation
`TaskFlowRef` 以及真实 save/restore 计数。由 effective TaskFlow 解析的 CurrentTask 与 CPU identity
不属于可恢复寄存器现场。`TaskFlow` 是 `PhaseObject` 的传递子类型，并用
`parent: Task` 类型约束要求每个实例的 parent 都是其 owner Task。Task 声明
`initial_flow: TaskFlow` typed association：它只记录创建该 Task 时绑定的 Flow，不随
exec 或 idle handoff 改写。静态绑定是 `BootTask -> BootInitFlow`、
`KernelInitTask -> KernelInitFlow`、`KthreaddTask -> KthreaddFlow`；动态 clone child
在声明点绑定本次 fresh `fork_flow`。Flow parent、Task initial-flow association、
`task_owns_flow` 与 `task_flow_owner_is` 必须指向同一 Task/Flow pair，否则模型不成立。
`BootInitFlow` 与 `KernelInitFlow` 都是直接的具体 `TaskFlow` 实例，不引入
`BootInitPhase`、`KernelInitPhase` 或 `KernelInitFlowType`。`KthreaddFlowType` 的实例是
`KthreaddFlow`，`BootIdleFlowType` 的实例是 `BootIdleFlow`；所有用户应用映像共用
`UserAppFlow` 类型，但每次 exec 或 fork continuation 都创建独立实例。

Flow 关系分为四类，不能混用：`initial_flow` association 只记录创建时 Flow，`task_owns_flow(task, flow)` 记录 Task 曾经拥有的历史集合，`task_active_flow_is(task, flow)` 记录当前可恢复 continuation，`task_flow_handoff(task, from, to)` 记录 handoff 历史。每个 Flow 只有一个 owner；一个 Task 可以按 exec 顺序拥有多个 Flow，但任一时刻最多一个 Flow Online。`BootInitFlow` 是 BootTask 的 initial TaskFlow，`BootIdleFlow` 是其后继 idle TaskFlow；正式 replacement handoff 包括稳定 `KernelInitTask: KernelInitFlow -> fresh Pid1UserAppFlow`（首个 exec 的专用实例），以及每个动态 child 从 declared fork-continuation Flow 到每次 exec 新声明 Flow。不同 Task 绝不共享同一 `UserAppFlow` 实例。

`Task.Online` 只表示该 carrier 已发布且可由 Scheduler 派发，不承诺普通 runnable queue
membership；idle task 可以只由 `rq->idle/rq->curr` 表达。对普通 Task，Online 是唯一允许
`TaskBreakpointState::Valid` 的状态；该断点必须绑定一个通过 slot/generation 校验且属于该 Task
的 `TaskFlowRef`。`Task.OnCpu` 表示 CPU 的唯一 current/执行权 carrier，但是否允许执行由正交的
`TaskExecutionAuthority::{None, Reserved, Live}` 表达：普通可恢复执行必须是
`OnCpu/Live/Invalid`，而不是仅凭 OnCpu。TaskFlow 的每个 lifecycle transition 和执行期 action
都必须显式依赖 parent Task 为 OnCpu 且 authority 为 Live；仅有 Task.Online、OnCpu reservation
或 runqueue membership 均不足以执行 Flow。`TaskFlow` 的类型继承只保证 parent 类型约束和已声明
process 适用于其子类型，不引入另一套 guard。fresh dynamic Flow 在 Base 中建立
owner/parent/entry-source 的 structural `Bind` 不是 lifecycle 推进或执行期 action，因此可以先于
parent 首次 dispatch。

`BootTask` 的模型初态是 `OnCpu/Live/Invalid`，表示固件/架构入口已经交付 boot CPU；其首次执行
不经过 Scheduler。Kernel 接受 OpenSBI Enable 后保持 Ready，并在同一 Enable 过程中直接驱动严格
`BootInitFlow.Preset`；BootInitFlow、首次调度和 KernelInitFlow 都是该迁移的过程细化，不是提交后的
异步后继事件。只有应用环境准备完成并提交 Kernel Online 后，才异步发出 payload handoff action。
普通 Task 的 Setup 只准备寄存器字节并把断点置为 Prepared；Enable 才把它绑定到 initial Flow，
发布 `Online/None/Valid`。Scheduler 在物理切换后的 next 栈 finish 点消费该断点并提交
`OnCpu/Live/Invalid`，再通过 `DispatchContinuation` 的
`drives initial.Preset || active.Continue` 选择唯一可接受 handler：initial Flow 仍为 Base 时严格
启动它，否则恢复唯一 active Flow。Suspend 把当前 active Flow 的现场保存回 Task 并重新发布
`Online/None/Valid`。没有候选、候选歧义、stale generation 或处理失败都使根执行失败。
Scheduler 是普通 Task Continue 与 Suspend 的唯一发送者。

`UserAppFlow` 的统一 lifecycle 是：Base 中的结构 `Bind` 建立唯一 owner/parent、入口来源和 fresh/独占关系但不推进 lifecycle；Preset 启动已绑定 Flow；Setup 准备 exec 映像或 fork continuation 的执行上下文；Enable 成为 owner 唯一 Online Flow 并跨入用户应用黑盒；Disable 处理 exit、exit_group 或 successful-exec replacement；Cleanup 释放实例并保证它不再 active。用户应用内部不声明 action 或 transition；syscall、trap、files 和其它内核资源操作仍属于相应内核对象。successful exec 不替换 Task：新 Flow 先 Bind/Preset/Setup，旧 Flow 再 Disable，随后提交 active binding handoff、新 Flow Enable，最后旧 Flow Cleanup。

普通 Task 退出必须先 Disable/Cleanup 当前 Flow；每次成功 exec 还必须 Cleanup 被替换的旧 Flow
并记录 prior-owned Flow 已 Destroyed。Flow 清理后的 terminal handoff 从
`OnCpu/Live/Invalid` 直接执行 Task Disable 到 `Offline/None/Invalid`，再由 next 栈执行 Cleanup；
禁止为了退出而制造一个没有可恢复 continuation 的 Online 断点。`Task.Cleanup` 要求所有 owned Flow
均已 Destroyed，因此任一 owned Flow 仍存活时 Task 不得进入 Destroyed。动态声明、owned Flow facts
与泛型实例 lifecycle 调用是本模型的正式能力；BootTask 没有退出 transition。

`TaskRuntimeState` 是 `Task` 的扩展运行态，不是对象 lifecycle state。因此，设置任务运行态应建模为 `Task.Transition::SetRuntimeState(state: TaskRuntimeState)` 这样的运行期 transition，而不是 `Action::SetTaskState`。当前实现先使用简单的 `StateEffect::Conditional` 和普通 fact 表达运行态提交；后续引入状态机模型后，每次进入特定 `TaskRuntimeState` 时应执行 transition guard、leave-state check 和 enter-state consistency check，例如确认调度实体、runqueue 选择、锁/抢占/中断上下文和跨对象不变量。

`task_state_running(task)` 是 `task_runtime_state_is(task, TaskRuntimeState::Running)` 的派生别名。调用 `Task.Transition::SetRuntimeState(TaskRuntimeState::Running)` 后，Type process ensures 先提交运行态事实，再由 derive 自动推出该别名；阶段规格不应在 `Enable.ensures` 中重复手写这个别名。

`Task.Action::PinToBootCpu(cpu_ref: CpuRef)` 是状态内 action，用于提交 task 的亲和性约束属性，不推进 task lifecycle，也不改变 `TaskRuntimeState`。在 `rest_init()` 中，调用点写为 `KernelInitTask.Action::PinToBootCpu(BootCPURef)`：receiver 已经确定目标 task，`BootCPURef` 是 `BootCPU` 发布的 CPU 引用。该 action 只提交两类属性事实：设置 `PF_NO_SETAFFINITY` 等价的 task flag，以及把 task cpumask 限制到 boot CPU。Linux 源码中的 `find_task_by_pid_ns(pid, &init_pid_ns)` 是用局部 pid 重新取回 task 指针的实现路径，不作为正式参数或 drives；规格层已经持有 `KernelInitTask` receiver。该源码路径由 `rcu_read_lock()/unlock()` 定界，当前保留为 deferred 上下文建模问题：它是否属于资源独占上下文，还是应建模为独立的 RCU/读侧上下文，后续讨论。

正式规格必须区分对象和对象引用。对象拥有 lifecycle/runtime state、facts 和 invariants；引用是在上下文中访问对象的类型化能力。`TaskRef`、`RunQueueRef` 与 `CpuRef` 分别绑定对应目标。`CurrentTask` 是 effective TaskFlow parent 的保留选择器，`CurrentTaskRef` 是 `ref(CurrentTask)`；二者都不是对象或 snapshot state。`CurrentRunQueueRef` 仍是所选 CPU 的私有投影，而 `CurrentCPU` 是 effective `TaskFlow.cpu_ref` 的解引用结果。action 返回引用时，调用方使用 SSA 风格 `let` 绑定；后续可用 typed reference receiver 分发到目标对象。CPU 归属只保存在 TaskFlow：入口和 scheduler commit 写 `TaskFlow.Action::AssignCpuRef`，Task 不保存同义字段。

Ref receiver 的正式分发规则是：若 `R` 是 `XXXRef` 类型的引用值，且 `XXXRef` 的目标对象类型 `XXX` 声明了 `Transition::E` 或 `Action::A`，则 `R.Transition::E(...)` / `R.Action::A(...)` 表示通过引用对目标对象执行 `XXX` 类型定义的 process；process 内部的 `self` 绑定到引用当前指向的目标对象。引用类型自身的 structural process，例如 `TaskRef.Action::Bind(task)`，只用于建立普通引用，不得用于改写 CurrentTaskRef。typed association path 允许引用目标的 association 透明访问，例如 Task 的 `initial_flow` 与 `active_flow`。普通 attribute 与 owned child 的通用 `Ref.attr` / `Ref.child` 仍未开放；其它引用关系继续使用 `task_ref_targets(...)`、`runqueue_ref_targets(...)`、`runqueue_ref_cpu_is(...)` 等 fact 承载。

`CurrentTask := effective_task_flow.parent`，`CurrentTaskRef := ref(CurrentTask)`。解析必须同时证明
Flow parent/owner 一致、目标 Task 是唯一 `OnCpu/Live` 执行主体、Task.active_flow 等于 effective
TaskFlow 且 TaskRef generation 有效。缺失上下文、非活跃 Flow、错误 owner、非 `OnCpu/Live` 或
悬空引用一律拒绝。同步 drives 子树继承 effective Flow；异步 emits 在接收方重新解析。不存在
可写 current-task 槽、selector action 或另一份权威 snapshot state。

`CurrentRunQueueRef` 的正式语义是 CPU 视角私有的 current-runqueue 引用。调度路径先从 effective Flow 的 CpuRef 解析 `CurrentCPU`，再通过 `CpuGroup.cpus[id].RunQueue` 解析当前 runqueue。BP 路径落到 `BootRunQueue`，是因为 active Flow 指向 CPU0；AP 路径同样由各自 Flow 的 CpuRef 决定。

`SchedulerObject.Action::SelectRunQueue(task_ref: TaskRef) -> RunQueueRef` 只选择目标 runqueue，不推进 Scheduler lifecycle 或提交成员关系。所选 runqueue 的 CpuRef 必须在 enqueue 前写到目标可恢复 Flow，并在真正迁移时由 scheduler commit 再确认；Task 不接收 CPU assignment。`RunQueueRef` 与 `CurrentRunQueueRef` 保持类型分离。

`SchedulerObject.Action::Schedule` 是 `Scheduler.Online` 后的调度分界 action。

### Replicated PhaseObject family

当 phase 规格明确声明对象按 target key（例如 secondary CPU `logical_id`）replicate 时，model
中的单个 `PhaseObject` 表示该 key 集合上的实例族。每个 key 独立拥有对象状态；对象的
`depends_on`、`ensures`、invariant 和同对象 `emits` 都按该 key 解释。

父 transition 中针对 replicated siblings 的 source-ordered `drives` 是 pointwise order：同一
key 上前一 sibling Online 后才能推进后一 sibling，不同 key 之间允许交错。Task/TaskFlow family
也遵循同一 key 规则；`ApIdleTask[id]`、`ApIdleFlow[id]`、HSM Startup signal 和三段 AP phase 必须
共享 logical-id key，任何 signal 都不能被其它 key 的 Flow 消费。family 级
`state == State::Online` 表示目标集合中所有实例 Online，只提供聚合完成事实，不隐含跨 key 的
阶段屏障，也不把实例 transition 的 owner 转移给聚合读取者。当前语法不增加 indexed-object
表达式；具体 family key 和聚合解释必须由对应 charter/model 注释与 coding 映射共同固定。
它不推进 `Scheduler` lifecycle state，但会提交一次调度边界的运行期事实。
`schedule_preempt_disabled()` 仍由调用方展开为三段式：
先退出调用方继承的 preempt-disabled guard，再调用
`Scheduler.Action::Schedule`，最后进入新的 boot-idle preempt-disabled
上下文。`Schedule` 自身内部则建模 `schedule()`/`__schedule()` 的最小边界：
先由 `PreemptionControl.Disable` 边界建立 schedule-owned 不可抢占上下文，再由
`InterruptType.SaveAndDisable` 边界关闭本 CPU 本地中断，然后在 runqueue lock context 中
先从 effective TaskFlow 解析 `CurrentTaskRef`，再执行
`let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef)`，
最后进入 `SchedulerObject.Action::SwitchTo(CurrentTaskRef, next)`。

`SwitchTo` 对应 `prepare_task_switch()`、物理 switch 与 next 栈 `finish_task_switch()` 的三段边界。prepare 只验证
prev 是 `OnCpu/Live/Invalid` 且存在 active Flow，并验证 next 是
`Online/None/Valid` 且其 FlowRef slot/generation/owner 均有效；不得预提交 lifecycle。物理 switch
保存 prev Task 的 context 并恢复 next context。next 栈上的 finish 是普通 Signal 不可观察中间态的
原子提交：prev 失去执行权、next 成为 `OnCpu/Live/Invalid`、next active Flow 与 CurrentTask
解析结果同时切换，然后严格选择 initial
Flow Startup 或 active Flow Continue。terminal prev 走 `OnCpu --Disable--> Offline --Cleanup-->
Destroyed`，context 保持 Invalid。`prev == next` 是严格 identity no-op，不发出 Suspend/Continue、
不保存/恢复现场、不改变 lifecycle、authority、breakpoint 或计数。MM、FPU/vector、`last` 返回值、
完整 prepare/finish hooks、worker 与 scheduler-class 细节继续按对象展开。

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
InterruptType guard 产生本 CPU 本地中断关闭效果。嵌套检查和上下文强度
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
- 首轮 guard contribution 推导如下：`RawSpinLock.LockIrqSave` 边界推导本地中断关闭、抢占关闭、主动切换关闭；`RawSpinLock.Action::Acquire` / `Release` 可作为普通 raw spin lock 的 guard 边界，只贡献该 `ResourceExclusiveContext` 的锁保护和 `obj_refs` 独占范围，不额外推导 irqsave、抢占关闭或主动切换关闭；`PreemptionControl.Disable` 边界推导抢占关闭和主动切换关闭；`InterruptType.SaveAndDisable` / `Disable` 边界只推导本地中断关闭；没有进入/退出事件的天然 guard 不自带运行时 effect，只由 `holds` 明确声明阶段边界保证的事实。
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
  `Computer.Transition::Preset` 可达调用图中只被进入一次。该标记必须由 model
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
                    KernelInitFlow.Action::AssignCpuRef(BootCPURef);
                }

                within EnqueueSelectedRunQueueContext {
                    depends_on {
                        runqueue_ref_targets(selected_rq, BootRunQueue);
                        runqueue_ref_cpu_is(selected_rq, BootCPURef);
                        task_flow_cpu_ref_is(KernelInitFlow, BootCPURef);
                    }

                    drives {
                        selected_rq.Transition::EnqueueTask(KernelInitTaskRef);
                    }

                    ensures {
                        raw_spinlock_irqsave_entered(BootRunQueueLock, CurrentCPU);
                        raw_spinlock_irqrestore_exited(BootRunQueueLock, CurrentCPU);
                        runqueue_contains_task(BootRunQueue, KernelInitTaskRef);
                    }
                }

                ensures {
                    scheduler_select_runqueue_returns(Scheduler, KernelInitTaskRef, BootRunQueueRef);
                    task_runqueue_selected(Scheduler, KernelInitTaskRef, BootRunQueueRef);
                    task_flow_cpu_ref_is(KernelInitFlow, BootCPURef);
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
工具验证时从 `Computer.Transition::Preset` 出发，沿 `drives` 和 `emits` 调用图统计每个标记
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

CPU 的 live 寄存器组也是 `CPU视角` 的私有对象，包括通用寄存器组 GPRs 和控制状态寄存器 CSRs。一个 CPU 不能在自己的规格步骤中直接读写另一个 CPU 的 live registers，只能通过 trap frame、saved task context、IPI/同步结果或共享内存中已经发布的保存副本观察间接结果。RISC-V64 中 `tp` 属于本 CPU 的 GPR 视图，`sstatus`、`stvec`、`sie`、`sip`、`satp` 等属于本 CPU 的 CSR 视图。当前只为启动 CPU 建模 `BootCpuRegisters`，并只保留入口所需的 `a0/a1/sp/tp/gp` 与现有 supervisor CSR 子集；它是 `BootCPU` 的私有子对象，不扩展为完整寄存器文件，也不推广到所有 `CPUObject`。AP 的寄存器对象留待 SMP 规格扩展。

`CPU.Action::DisableFpuVectorExecution` 是 CPU-local、无 lifecycle state effect 的 action。它关闭目标 CPU
当前的浮点与向量执行状态，并建立三类策略事实：内核态默认关闭、临时启用必须位于明确受控的执行区间
且退出后恢复关闭、用户态启用服从任务需要和系统策略。BootInitFlow 只通过 `CurrentCPU` 驱动该 action；
CpuGroup 不拥有或复制这些执行状态。

`BP视角` 和 `AP视角` 是 `CPU视角` 的两个具体分类。BP 是唯一且必须存在的启动 CPU，承担主要内核初始化职责；AP 是后续进入的 secondary CPU，复用共享类型语义和 BP 已建立的共享环境，但必须拥有自己的 effective TaskFlow、`CurrentTask`/`CurrentCPU` 解析、本地中断控制、GPR/CSR 寄存器组和 AP entry/ack 路径。

多个 CPU 视角共同可见、共同依赖或共同维护的公共事实集合，命名为 `共享全局视角`。它包括共享内存对象、全局 phase 边界、`CpuGroup`/topology、全局调度设施、同步对象和跨 CPU 可见状态等。`共享全局视角` 不是新的执行主体，也不是可以同时支配所有 CPU 私有步骤的全局控制视角；它只是各个 `CPU视角` 中公共可见环境的规格化名称。本地中断状态、当前寄存器组和当前任务切换结果必须留在对应 CPU 视角内表达；CurrentTaskRef 始终由该执行上下文的 effective Flow 派生。

## SEM-CURRENT-TASK-MODEL-001: CurrentTask Is A Flow-Scoped Selector

`CurrentTask` and `CurrentTaskRef` have no declaration, lifecycle, child ownership, writable action, or
snapshot entry. A successful resolution records the canonical Task together with the effective Flow and
generation-checked TaskRef. Synchronous descendants preserve that source; asynchronous receivers do not.

## SEM-CURRENT-CPU-MODEL-001: CurrentCPU Is A Flow-Scoped Selector

`CpuGroup.cpus[logic_id]` is the sole canonical CPU instance collection. `CurrentCPU` is not an object, owner, instance, or lifecycle. It resolves as `dereference(effective_task_flow.cpu_ref)` and is valid only while that Flow has execution authority. `CpuRef` is a typed stable reference to a published indexed element; dereference of a missing element is an error.

Only entry and scheduler-commit boundaries may write `TaskFlow.cpu_ref`; Task has no synonymous CPU assignment. A Flow retains the assigned or last CPU while not OnCpu, migration changes it at commit, and Flow handoff copies it before the successor becomes active. A synchronous `drives` subtree inherits the effective Flow and may use `CurrentCPU`; an asynchronous `emits` edge does not. Trace records the canonical `CpuGroup.cpus[i]` target and source Flow/CpuRef.

`CpuGroup.Preset` atomically declares CPU0 and advances both child and parent to Prepared before Kernel Enable. Kernel acceptance binds CPU0s CpuRef to BootInitFlow; BootInitFlow.Preset resolves CurrentCPU, records the entry hartid and advances CPU0 to Ready. Later platform validation makes it Online. CpuGroup.Setup atomically creates AP elements and publishes topology. possible/present/active/online sets derive from CPU states and may be cached only as rebuildable bitmaps.

CPU-local interrupt control, registers and scheduler-local children belong below each CPU element. CurrentTask does not. A stateless `CurrentCpu` implementation capability borrows the selected element; Context stores CpuGroup, not an independent current/boot CPU. The raw-spin-lock chain uses that borrowed CPU local state and the Flow-selected current task. AP `CurrentCPU` becomes resolvable only after an AP Flow receives execution authority and its CpuRef; possible/present membership alone is insufficient.

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
