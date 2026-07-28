# 概念体系迁移

## 文档定位

本文约束项目从现有的对象/事件模型逐步迁移到系统/信号模型的过程。迁移后的目标概念由
[系统与信号模型](system-signal.md)定义；本文只定义迁移期间的术语映射、兼容边界、推进顺序、
deferred 项和完成条件。

当前 `.spec` 语言仍以 [`spec/model/SEMANTICS.md`](../model/SEMANTICS.md) 为硬语义来源。在该文件、
模型工具和测试完成对应更新前，不得把本文描述的目标语法直接写入 `.spec`，也不得由工具擅自把
旧语法解释成尚未落地的新语义。

## 迁移目标

概念体系迁移完成后遵守以下原则：

1. 每个具有独立身份和功能边界的对象都是系统。`Object` 不再表示一种与 `System` 对立的实体；
   若 DSL 继续保留 `object` 声明，它只表示具名系统实例的声明形式。
2. `Signal` 取代 `Event`，成为系统之间交互以及触发系统响应的统一概念和正式名称。
3. 信号与响应过程严格分离。信号描述源系统向目标系统发送的交互；`Transition` 和 `Action`
   描述目标系统收到信号后的处理过程。
4. `drives` 和 `emits` 都表示发送信号：`drives` 同步发送并等待目标响应完成，`emits` 异步发送且
   源系统不等待目标完成。
5. 所有系统都采用被动响应模式。任务、CPU flow 和其它持续执行过程仍是某个启动信号或运行时
   信号所引起的响应链，不再通过“主动对象”或“主动模式”建立另一套推进语义。
6. 系统边界具有层级相对性。发送给子系统的信号只触发该目标子系统，不自动触发其父系统；父系统
   是否收到信号只由信号的目标决定。

## 目标语义映射

| 现有概念或写法 | 目标概念 | 迁移说明 |
| --- | --- | --- |
| `Event` | `Signal` | 逐步替换概念和名称；迁移完成后不再用 Event 表示系统交互。 |
| 独立 `Object` | `System` | 对象身份、状态和边界按系统理解；是否保留 `object` 关键字另行决定。 |
| 迁移事件、动作事件 | Signal + 响应过程 | 信号不是 transition/action；目标系统收到信号后才执行相应过程。 |
| completion event | completion signal | transition 完成后可异步发出后续信号。 |
| `drives Object.Transition::X` | 同步发送目标信号 | 当前写法是把信号和响应过程折叠在一起的兼容表达。 |
| `emits Object.Transition::X` | 异步发送目标信号 | 当前写法是把信号和响应过程折叠在一起的兼容表达。 |
| 主动/被动对象分类 | 统一的被动系统 | `start` 等业务名称不自动代表主动模式。 |

目标模型中，源系统在 `drives` 或 `emits` 后引用的是信号，而不是目标系统的 transition/action。
目标系统使用 `on Signal { ... }` 定义响应，并在响应中执行会改变状态的 `Transition` 或不改变被建模
状态的 `Action`。信号声明、处理绑定和参数传递的最终 DSL 语法尚未确定，本文中的写法只表达概念
关系，不是当前可直接使用的 `.spec` 语法。

## 系统层级与信号归属

外部信号和内部信号必须相对于某个被分析系统进行判断：

1. 目标是当前系统的信号，触发当前系统定义的响应过程。
2. 目标是当前系统内部子系统的信号，只触发该子系统，是当前系统内部的交互。
3. 子系统收到信号不会沿 parent 关系向上冒泡，也不能隐式触发父系统的 transition/action。
4. 父系统响应信号期间可以同步或异步地向子系统发出新信号，从而形成从外到内的响应链。
5. 子系统响应期间也可以向自身、同级系统、父系统或其它可达系统发出信号，但必须显式声明目标。

以 `Kernel` 为例，启动信号和中断信号的目标是 `Kernel`，可以触发内核系统层面的响应；异常、
syscall、调度和任务切换等信号以相应内部子系统为目标，不因发生在内核内部而自动触发 `Kernel`。

## 同步与异步发送

`drives` 和 `emits` 的共同点是源系统向明确的目标系统发送 Signal，区别是源系统是否等待：

- `drives`：同步发送。目标响应完成后，源系统才继续当前响应过程；目标在响应中形成的同步调用链
  也属于等待范围。
- `emits`：异步发送。源系统完成发送后不等待目标响应，目标系统按照异步交付规则处理信号。

现有 model 把 `emits` 定义为同一推导链内的 post-commit completion event。迁移到异步 Signal 前，
必须先在 formal semantics 中确定发送成功点、排队与交付顺序、失败反馈、目标不可响应时的处理、
eventual delivery 要求，以及异步响应是否建立新推导链。上述问题没有确定前，不得只做名称替换就
宣称已经实现异步语义。

## 被动响应模式

目标概念体系不再保留主动模式。系统只有收到信号后才能执行响应过程和改变状态；响应过程中可以
继续发送信号，形成持续或分叉的连锁反应。任务运行、CPU flow、调度、idle loop 和应用执行均按
这种因果链理解，不以“能够持续执行”为理由引入无需信号触发的主动系统。

迁移期间，旧文档中的“主动对象”“主动模式”以及以 `start`/`enable` 区分主动与被动对象的说明
属于待迁移内容。删除这些表述前必须检查其是否还承载执行主体、控制权交接或不返回入口等其它
有效语义，不能机械删除相关过程和边界。

## 迁移期兼容规则

1. 新增或实质修改的 charter 内容应使用 System、Signal 和被动响应模式；引用旧模型时可以同时
   标出当前兼容写法。
2. 尚未修改的旧 charter/model/coding 内容允许暂时保留 Event、Object 和主动对象等术语，但不得
   以这些遗留写法反向否定已确定的目标概念。
3. `.spec` 必须继续遵守当前 `SEMANTICS.md` 和 parser 支持的语法，直到 formal semantics、工具和
   测试在同一变更链中完成升级。
4. 当前 `drives/emits Object.Transition::X` 只能在概念审计中视为“Signal 与响应过程折叠”的遗留
   表达；工具不得在没有新硬语义时自动改变其执行方式。
5. 不进行无边界的全仓机械替换。每次迁移必须先确认术语所在层级、信号源和目标、同步方式、响应
   过程以及父子系统关系。
6. `TrapType` 等既有正式对象名的更名属于模型接口变更，默认必须按 `charter-first` 单独处理；
   用户显式触发 `model-first` 时适用 [`spec/guidance`](../guidance/README.md) 的确认门禁，但不改变
   最终权威层级。
7. 通用交互概念 `Signal` 与 Linux/POSIX 进程信号机制必须保持可区分。`SignalCore`、
   `SignalRuntime` 等名称表示进程信号领域对象，不等同于通用 System Signal。

## 推进阶段

### 第一阶段：Charter 收敛

- 建立系统与信号的目标概念和本文的迁移约束。
- 在被修改的专题 charter 中明确系统层级、信号源/目标及内部/外部信号。
- 修正 `new_charter.md` 中把应用视为 Kernel 外部环境的描述，将应用纳入内核内部系统。
- 盘点 Event、主动模式、对象/系统边界以及 transition/action 被称为信号的遗留表述。

### 第二阶段：Formal semantics 设计

- 在 `spec/model/SEMANTICS.md` 中正式定义 System、Signal、`on Signal`、同步 `drives` 和异步
  `emits`。
- 确定信号身份、类型、附加信息、源/目标约束和响应过程绑定规则。
- 闭合异步发送、交付、失败、推导链和可观测顺序语义。
- 给出旧 `.spec` 写法到新语法的兼容与废止规则。

### 第三阶段：DSL 与工具迁移

- 按 formal semantics 更新 parser、AST、model checker、derive、view、render 和测试。
- 在兼容期同时识别旧写法和新写法，并对新引入的旧写法给出诊断。
- 工具能够区分 Signal、Transition 和 Action，不再通过名称猜测三者关系。

### 第四阶段：Model 与下游规格迁移

- 按系统边界逐组迁移 `.spec`，显式建立 Signal 及其响应过程。
- 更新 phase paradigm、coding、compose、testing 和 checkpoint 术语及映射。
- 决定 `任务子系统`、`中断子系统` 是新增聚合系统，还是由现有 phase/stream/object 重构形成。

### 第五阶段：遗留清理

- 删除 Event 作为系统交互概念的兼容语义。
- 清理主动模式和主动对象分类，同时保留已确认有效的执行主体与控制权边界。
- 删除旧式 Signal/response 折叠语法及对应工具兼容路径。
- 将已稳定的硬规则保留在 `SEMANTICS.md`，本文转为历史迁移记录或归档。

## Deferred 决策

以下事项已经识别，但不得在缺少专门设计时猜测处理：

1. Signal 声明、引用、泛型附加信息和 `on Signal` 响应绑定的最终 DSL 语法。
2. 同一个 Signal 类型是否允许在不同调用点选择同步或异步发送，以及该属性属于类型还是发送实例。
3. 异步 Signal 的队列、优先级、并发、失败反馈、取消和 eventual delivery 语义。
4. DSL 是否保留 `object` 作为 System 实例声明关键字，是否需要通用 `System` type。
5. `任务子系统`、`中断子系统` 与现有 `Task`、`Scheduler`、interrupt leaf phases、`TrapType`、
   `InterruptType` 和 IRQ 对象的最终映射。
6. `TrapType`、event checkpoint、trace event 等既有名称迁移后的正式命名。

## 完成条件

只有同时满足以下条件，概念体系迁移才算完成：

1. charter 中除历史说明外，不再以 Event 表示系统交互，不再依赖主动模式解释系统推进。
2. `SEMANTICS.md` 已正式定义 Signal 与响应过程的区别，以及 `drives`/`emits` 的同步/异步语义。
3. DSL 和全部 model 工具支持并检查 Signal、目标 System 和 `on Signal` 响应绑定。
4. 正式 model 已完成迁移，不再把 Signal 与 `Object.Transition::X`/`Object.Action::X` 折叠表达。
5. Kernel 外部信号与内部信号边界已经在 charter、model 和 coding 中保持一致。
6. `任务子系统`、`中断子系统` 等 deferred 系统边界已经作出决定并落实，或明确移出目标范围。
7. 旧语法兼容路径及其测试已经删除，长期硬规则已经归入 `SEMANTICS.md`。
