# Model Semantics Hard Rules

本文档记录 `.spec` 模型语言的硬语义规则。修改规格、模型构建器、推导器、检查器、视图或渲染工具前，应先阅读本文档；工具实现不得用后续阶段的“消歧”或展示逻辑绕过这些规则。

## SEM-NAME-001: State And Event Names Are Controlled

状态名和事件名必须来自受控集合。规格不得临时发明新的生命周期名称来表达局部语义；如果确实需要新增名称，必须先修改本文档、`model` 阶段检查器和对应测试。

允许的状态名：

- `Base`
- `Prepared`
- `Ready`
- `Online`
- `Destroyed`

允许的事件名：

- `Preset`
- `Setup`
- `Enable`
- `Cleanup`

语义约定：

- `Base` 的别名包括：初始态、基态、尚未建立。
- `Prepared` 的别名包括：预置态、前置条件已建立。
- `Ready` 的别名包括：就绪态、主要构建已完成。
- `Online` 的别名包括：在线态、已启用、可服务。
- `Destroyed` 的别名包括：已销毁、已退出服务、已清理、已预留、`reserved`。`Reserved` 不是正式状态名。
- `Preset` 表示建立进入主要构建流程前的早期前置条件，通常推进到 `Prepared` 或 `Ready`。
- `Setup` 表示完成对象的主要构建，使对象进入 `Ready`。
- `Enable` 表示让已经构建完成的对象进入服务状态，通常推进到 `Online`。别名包括：启用、上线、进入服务、保护、`guard`。当语义是建立栈 canary 这类保护性运行约束时，仍使用 `Enable` 作为正式事件名。
- `Cleanup` 表示对象退出服务或释放阶段性抽象，通常推进到 `Destroyed`。

检查点：

- `model` 阶段必须检查对象 `initial_state`、状态声明名、事件声明名和事件目标状态名。
- 任何不在受控集合内的名称必须报 `error`。
- `derive`、`view`、`render` 不得通过名称猜测或展示修正来补偿非法名称。

## SEM-TRANSITION-001: Lifecycle Transitions Are Controlled

当前模型只能使用预先定义的状态迁移三元组 `(source_state, event, target_state)`。别名不参与迁移定义；`.spec` 中必须使用正式状态名和事件名。

允许的迁移：

- `Base --Preset--> Prepared`
- `Base --Preset--> Ready`
- `Base --Setup--> Ready`
- `Prepared --Setup--> Ready`
- `Prepared --Enable--> Online`
- `Ready --Enable--> Online`
- `Ready --Cleanup--> Destroyed`
- `Online --Cleanup--> Destroyed`

检查点：

- `model` 阶段必须检查每个事件声明的源状态、事件名和目标状态三元组。
- 任何不在允许迁移集合内的三元组必须报 `error`。
- 新增迁移必须先修改本文档、`model` 阶段检查器和对应测试。

## SEM-UNIQUE-001: Forward-Only Model Forbids Duplicate States And Events

同一个 `object` 内，`Event::X` 只能定义一次。事件身份是 `(Object, Event)`，不是 `(Object, SourceState, Event)`。

原因：

- `drives` 引用形式是 `Object.Event::X`，不包含源状态。
- 如果同一对象内允许多个 `Event::X`，引用目标会变得不唯一。
- 当前状态只决定事件是否可触发，不参与事件命名。

检查点：

- `model` 阶段必须扫描同一对象的所有 `state.events`。
- 发现重复 `Event::X` 时必须报 `error`，并指向重复定义位置。
- `derive`、`view`、`render` 不得通过 `source_state -> target_state` 为重复事件消歧。

例外：

- 只有将来显式引入“可重复事件/可重入事件”的对象类型或事件语义后，才能放宽本规则；默认所有对象都禁止重复事件名。
