# 系统与信号模型

本文定义项目迁移完成后采用的目标概念体系。现有 charter、model DSL 和工具仍处于逐步迁移过程，
迁移顺序和兼容约束见[概念体系迁移](concept-migration.md)。

## 系统System

系统是具有独立功能的抽象，存在于外部环境中，系统与外部环境之间有明确定义的边界，二者仅能在边界上通过信号进行交互。

此前规格中常用的独立对象，本质上都是系统。对象不再表示一种与系统对立的实体；迁移期间保留的
`object` 可以继续作为具名系统实例的声明形式。系统、对象、子系统和超系统的称呼差异，主要反映
当前分析层级和建模粒度，不改变其作为系统的基本语义。

系统功能是基于以下两方面因素的对外表现：

1. 系统的静态构成和构成元素的功能；
2. 构成元素之间的动态交互与相互作用。

系统是嵌套结构，系统可以由更小粒度的子系统构成，同时，外部环境实际是系统的超系统，这个超系统由系统和其它同级系统构成。所以系统、子系统和超系统只是相对而言，它们都是系统。

系统边界由构成它的部分子系统的边界构成，不存在独属于本系统的边界。

系统与外部环境的边界，本质上是它与其它系统之间的边界。

系统层级不改变信号的明确目标。发送给某个子系统的信号只触发该子系统，不会因为 parent/child
关系自动向上冒泡并触发父系统；父系统是否收到信号，只由信号目标是否为父系统决定。

完整内核模型采用两棵稳定、可复核且彼此独立的有效层级树。源规格中的显式 `parent` 始终优先；当
组合后的模型包含 `Kernel` 时，没有显式 parent 且不是 `ProjectObject` 或其子类型的静态系统仍默认
属于 `Kernel`，但具名 `Computer` 是系统树显式根，绝不能应用该默认。

工程树是 `ComputerProject -> {HardwareProject, FirmwareProject, KernelProject}`；系统树是
`Computer -> {Riscv64Platform, OpenSBI, Kernel}`，且 `BootHartContext.parent = Riscv64Platform`。
工程产物通过显式 parent 归属对应 Project，运行系统不挂在 Project 下。不包含 `Kernel` 的独立规格
片段保持自己的根，工具不得为了凑齐主模型层级而虚构 Kernel。
有效层级只用于结构展示、Signal 坐标和预算；它仍不产生隐式冒泡、广播或 handler 继承。未知
parent、自引用和 parent 环都是模型错误，所有静态 view 与 Signal 工具必须消费同一份归一化层级。

## 信号Signal

系统与外部环境交互本质上是与其它系统的交互，必须通过发送和接收信号的方式，在边界发生。

`Signal` 将逐步取代 `Event`，成为这类交互的统一概念和正式名称。迁移完成后，不再使用 `Event`
表示系统交互或系统响应的触发原因。

信号是驱动系统发生状态变化和对外反馈的唯一原因。

信号由源、目标、类型和附加信息构成。源和目标都是系统。

何时、发送何种类型的信号以及发送给谁，这些由源系统自身的行为定义。

目标系统在自身边界上定义可以接受何种类型的信号，并通过 `on Signal { ... }` 定义如何响应处理
信号。信号本身不是处理过程：可能导致系统状态变化的响应过程是迁移 `Transition`，不导致被建模
状态变化的响应过程是动作 `Action`。`Signal`、`Transition` 和 `Action` 必须保持不同身份，即使
迁移期间旧语法暂时使用相同名称或把它们折叠表达。

信号处理过程中，系统可以向自己或其它系统再次发出信号，如此一个信号可以导致连锁反应。

系统通过 `drives` 或 `emits` 发送信号，二者后面跟随的目标是信号，而不是目标系统的
`Transition`/`Action`。目标系统收到信号后，再执行它为该信号定义的响应过程：

1. `drives` 同步发送信号：源系统阻塞等待目标系统完成对信号的处理过程；
2. `emits` 异步发送信号：源系统只负责向目标系统发出信号，不等待目标完成处理。

上述是目标语义；当前 model 中 `drives/emits Object.Transition::X` 仍是有效的兼容写法，不能在
formal semantics 和工具更新前直接按新语法解释。异步发送的交付、排序、失败和推导链规则也必须
在 formal semantics 中闭合后才可实施。

每种类型信号可以定义自己的附加信息，随着信号发送。目标系统决定如何处理附加信息。

## Signal 响应与完整主模型工具边界

一次 Signal 响应从目标系统接收具名 Signal 开始，到该系统的 handler 完成、拒绝或因推导错误失败
为止。Signal envelope 与响应过程是不同对象：envelope 保存稳定身份、源、目标、名称、类型化
payload、严格性和因果关系；handler 才是目标系统内部执行的 `Transition` 或 `Action`。Transition
handler 可以提交生命周期状态，Action handler 只能提交规格允许的事实，不能借 Signal 调用伪造状态
迁移。

Signal 推导工具采用下列兼容边界：

- 当前 `Target.Transition::Name(...)` 和 `Target.Action::Name(...)` 规范化为发往 `Target` 的隐式
  `Name` Signal，原调用参数成为 payload；目标暂以同名 Transition/Action 作为兼容 handler。
- `drives` 同步发送并等待目标响应结束；`emits` 只在源响应提交后投递，并按全局 FIFO 处理。
- `drives` 中的 `A || B` 是按源码顺序选择首个当前可接受的 Signal handler；未被选择的候选不发送 Signal，也不制造 rejected/failed 记录。
- tools2 外部 Signal 名称中的 `Startup` 是 `Preset` 的保留别名；别名必须在 Signal identity、handler
  查找和截至匹配前规范化，正式 DSL 和 handler 仍只使用 `Transition::Preset`。
- 有效 parent 只定义推导传播的层级坐标和预算，不产生隐式冒泡、广播或 handler 继承。
- 当前里程碑不引入显式 `signal`/`on Signal` 语法，不把 handler 改名为 `OnName`，也不实现等待未来
  Signal 的 continuation；这些都必须作为后续独立模型变更完成。

响应结果必须使用明确分类：

- `rejected`：目标在当前稳定快照不接受 Signal，包括没有 handler、handler 歧义、状态不匹配或
  接收条件不成立。
- `discarded`：lossy Signal 被拒绝后立即丢弃，不重试，也不等待未来状态。
- `failed`：规格、类型、推导或 invariant 失败，或者 strict Signal 被拒绝导致根请求失败。
- `truncated`：下一次传播超出显式预算，因此不执行 frontier Signal。
- `completed`：响应和其同步子响应完成；其异步 Signal 已按规则进入 FIFO。
- `stopped`：发送前截至边界已到达，当前已接受但尚未提交的祖先响应或已经存在但尚未处理的 FIFO
  Signal 不再继续；这不是可恢复 continuation。
- `pending`：Signal 已接受但等待未来 Signal 才能继续。该概念保留，但当前工具不得产生它。

条件不成立只表示本次接收被拒绝，不得猜测为临时等待。严格拒绝的诊断必须保留从根 Signal 到拒绝
点的完整因果链；lossy 丢弃和预算截断也必须是 trace 中可见的事实，而不是展示层推断。

`tools2/` 是验证上述目标语义的独立工具链。本轮里程碑要求它完整加载 `spec/model/main.spec`，并能从
真实 Signal 到达边界推导其可达闭包。完整模型只有 `ComputerProject.Preset` 是无需预制条件的起点；
根请求没有显式 snapshot/scenario 时必须严格使用模型初态。`Kernel.Preset` 或其它后续 Signal 若因
前期状态或事实尚未建立而被拒绝，报告具体缺项和完整失败链是正确结果。工具不得隐式回溯 emitter、
运行上游 transition 或合成到达时快照；调用者从后续边界继续时必须显式提供 snapshot/scenario。

为了从真实上游推导得到后续 Signal 的到达前场景，tools2 必须支持发送前截至：调用者指定一个规范化
后的 `Target.Signal`，推导在第一个实际将发送该 Signal 的位置停止。匹配发生在动态 receiver、目标和
有序候选已经确定之后，但必须早于 Signal ID 分配、`signal_sent`、入队、接收和 handler 执行。边界
快照是该发送动作之前的最后稳定状态；目标若是根 Signal，则结果就是模型/scenario 初态。到达边界是
成功结果 `reached`，可导出带 boundary provenance 的 snapshot；未提交祖先响应和已有未处理 FIFO
Signal 变为 `stopped`，不得产生 `pending` 或隐式 continuation。若推导先 failed 或被预算截断，则其
结果优先且不得导出 snapshot；若完整闭包结束仍未遇到目标发送点，则结果为
`until_signal_not_reached`，同样不得导出 snapshot。

tools2 可以复用老工具的阶段名称和 CLI 外壳，但不导入 `tools/` 的实现或中间协议代码；两套工具
通过路径、独立 Python import path、producer 和 schema version 隔离。公开快捷入口是
`tools2/bin/pyveri`，默认主模型和无限 depth/breadth 预算；底层阶段 driver 仍保留通用 `3/3` 默认。
快捷入口的默认请求是 `Human -> ComputerProject.Preset`，`-t/--trigger` 可以覆盖目标，
`-u/--until` 可以指定发送前截至；底层 driver/derive 的 source 默认同为 `Human`，但底层
`--signal` 保持必填。tools2 协议统一为 version 3 并拒绝 version 1、version 2、老工具协议和旧
snapshot。老 `tools/` 继续承担默认
`make test` 和静态 trace/SVG；老工具的替换或退役、显式 Signal DSL 和交互 HTML 都需要后续另行确认。

主模型的启动创建顺序固定为三个子 Project Preset、三个子 Project Setup、Computer assembly、
`Computer -> Riscv64Platform -> OpenSBI -> Kernel`。后三段交接由异步 `emits` 形成全局 FIFO，不得按
hierarchy depth 重排。`ComputerProject.Online` 只表示已经把启动交给 Computer；它不等待异步下游
Online。

tools2 的默认文本视图服务于快速阅读 Signal 在系统层级间的传播：按结构化 Signal 的创建顺序逐行
展示 source、Signal、target，按目标相对根系统的 hierarchy depth 使用两空格缩进，并只为已解析的
Transition 展示目标系统实际 before/after 状态。若传播走向根目标的祖先，整体缩进必须平移到最小
depth，使最上层祖先保持在最左侧。文本把 canonical `Preset` 显示为用户别名 `Startup`，但不得改变
任何结构化产物。非 completed 结果、发送前 boundary 和失败原因必须在简化视图中直接可见；完整的
协议诊断仍由 `VERBOSE=1` 选择原详细文本视图。该选择只影响文本渲染，不能改变推导、校验、Signal
顺序、JSON、snapshot、verdict、退出码或正式模型。

## 通用分析方法

按照信号的传导顺序分析和建立系统模型，具体顺序：

信号 -> 系统边界 -> 系统内部处理 -> （可能发出）信号。

系统边界重点分析能够响应的信号类型。

系统内部重点分析处理流程，以及期间可能向其它系统发出何种信号。

系统边界和系统内部都可能由更小粒度的子系统构成，每级子系统都遵循上述顺序分析。

信号在系统之间，按照影响的先后顺序，形成从外到内、自顶向下的传导链和响应链。

所有系统都采用被动响应模式。任务、CPU flow、调度和应用执行同样属于信号引起的响应链；系统不
因响应过程能够持续执行而成为主动系统，也不再为主动模式建立另一套状态推进规则。

## 子系统建模原则

系统是否需要分解为更小粒度的子系统，以及子系统的划分边界如何确定，主要从信号处理的复杂程度考虑。对于复杂性高的信号处理过程，要考虑转化为多个子系统之间的协作过程。

信号包括启动信号和运行时信号。主要从运行时信号角度，考虑信号处理复杂性和子系统划分与建模问题；对启动信号的响应处理，主要考虑如何把已经划分的子系统的启动串联起来。
