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

完整内核模型采用一棵稳定、可复核的有效层级树。源规格中的显式 `parent` 始终优先；当组合后的模型
包含 `Kernel` 时，没有显式 parent 的静态系统仍默认属于 `Kernel`，但具名 `Computer` 是显式根，绝
不能应用该默认。

系统树是 `Computer -> {Riscv64Platform, OpenSBI, Kernel}`。启动 CPU 的局部层级片段是
`CpuGroup.cpus[0] -> BootCpuRegisters`；`BootCPU` 只是该元素的角色别名，寄存器对象不属于 `Riscv64Platform`。
外部规格和构造输入通过显式 parent 归属消费它们的 System：`Riscv64 -> Riscv64Platform`、
`SbiSpec/BootArgs -> OpenSBI`、`LinuxRiscv64KernelBootSpec/Config/Lds -> Kernel`。不包含 `Kernel` 的独立规格片段保持自己的根，
工具不得为了凑齐主模型层级而虚构 Kernel。
有效层级只用于结构展示、Signal 坐标和预算；它仍不产生隐式冒泡、广播或 handler 继承。未知
parent、自引用和 parent 环都是模型错误，所有静态 view 与 Signal 工具必须消费同一份归一化层级。

### SystemObject 四态

`SystemObject` 统一采用 `Base -> Prepared -> Ready -> Online` 四态，含义由系统自身而不是下游闭包
定义：

- `Base`：当前系统实例尚未完成规格采纳。
- `Prepared`：当前系统实例的规格已经建立或采纳，尚未完成构造。
- `Ready`：当前系统实例已经构造完成，可以接受本层 Enable 服务请求。
- `Online`：当前系统实例已经完整履行自身 charter 定义的 Enable 服务契约并原子提交完成；具体完成
  条件由该 System 的 charter 定义，不能从通用四态名称推断。

System.Online 不自动等于全部下游闭包成功，也不自动排除内部初始化或应用环境准备；是否把某段下层
过程纳入本层 Enable，取决于该 System 的服务边界。System 在提交 Online 后通过 `emits` 发出的后继
Signal 即使随后失败，已经提交的 Online 也不得回滚；严格 Signal 的失败仍沿因果链使根 verdict 为
failed。

### 分层生命周期抽象

上层 lifecycle Transition 可以由下层 Flow、Phase、调度、上下文切换和 checkpoint 共同实现。各层
必须遵循同一套提交规则：

1. Transition 从接受到完成期间保持源 lifecycle state；只有服务契约全部完成时才原子提交目标状态。
2. 下层 Flow、Phase、上下文切换和 checkpoint 是上层 Transition 的过程细化，不因跨函数、跨栈或跨
   Task 执行而自动成为上层 Transition 之后的独立生命周期。
3. `drives` 表示源响应对目标下层过程的逻辑完成依赖；它不要求物理调用栈同步，也不禁止实现用真实
   调度和上下文切换承载该依赖。
4. `emits` 只用于源 Transition 已提交后的后继事件。后继处理失败会使根 verdict 为 failed，但不得
   回滚已经提交的源状态。
5. 实现耗时、异步硬件执行或跨 Task continuation 本身都不是新增 `Starting`、`Pending` 等 lifecycle
   状态的理由；当前四态保持不变，过程进度由下层状态和 checkpoint 表达。

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

一次 `drives` 或同步根请求包含两次有方向的信息交互：Signal request 从 source 到 target，目标响应
完成、拒绝或失败后，feedback 从 target 返回 source，解除源的逻辑等待。feedback 属于同一个 Signal
envelope 的响应，不创建第二个 Signal，也不改变既有 identity、cause、FIFO 或失败传播语义。
`emits` 只有 source 到 target 的 request；目标 handler 仍会完成并提交自己的状态或事实，但该 settle
是目标内部处理事实，不是返回发送源的 feedback。全局推导仍必须观察 emits 的拒绝或失败并决定根
verdict，这种全局严格失败传播不表示发送源等待或接收了反馈。

上述是目标语义；当前 model 中 `drives/emits Object.Transition::X` 仍是有效的兼容写法，不能在
formal semantics 和工具更新前直接按新语法解释。异步发送的交付、排序、失败和推导链规则也必须
在 formal semantics 中闭合后才可实施。

每种类型信号可以定义自己的附加信息，随着信号发送。目标系统决定如何处理附加信息。

## Signal 响应与完整主模型工具边界

一次 Signal 响应从目标系统接收具名 Signal 开始，到该系统的 handler 完成、拒绝或因推导错误失败
为止。Signal envelope 与响应过程是不同对象：envelope 保存稳定身份、源、目标、名称、类型化
payload 和因果关系；handler 才是目标系统内部执行的 `Transition` 或 `Action`。所有 Signal 都是
不可丢弃的严格 Signal。Transition
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
- `failed`：规格、类型、推导或 invariant 失败，或者任一已发送 Signal 被拒绝导致根请求失败。
- `truncated`：下一次传播超出显式预算，因此不执行 frontier Signal。
- `completed`：响应和其同步子响应完成；其异步 Signal 已按规则进入 FIFO。
- `stopped`：发送前截至边界已到达，当前已接受但尚未提交的祖先响应或已经存在但尚未处理的 FIFO
  Signal 不再继续；这不是可恢复 continuation。
- `pending`：Signal 已接受但等待未来 Signal 才能继续。该概念保留，但当前工具不得产生它。

条件不成立只表示本次接收被拒绝，不得猜测为临时等待。拒绝诊断必须保留从根 Signal 到拒绝点的
完整因果链；异步 `emits` 的目标处理失败也必须传播为最终根执行失败，不得静默忽略、排队重试或
降级为其它 outcome。预算截断必须是 trace 中可见的事实，而不是展示层推断。

`tools2/` 是验证上述目标语义的独立工具链。本轮里程碑要求它完整加载 `spec/model/main.spec`，并能从
真实 Signal 到达边界推导其可达闭包。完整模型声明唯一的外部启动编排：

```text
external Human {
    drives {
        Computer.Transition::Preset;
        Computer.Transition::Setup;
    }
    emits {
        Computer.Transition::Enable;
    }
}
```

`external` 声明的主体不是 model object：它无 lifecycle、无 parent，也不参与对象数量和根树归一化。
同一声明中的 `drives` 严格同步按源码顺序执行，任一步失败立即短路；全部 drives 成功后，`emits` 才
按源码顺序进入全局 FIFO。完整模型只允许一个外部编排声明，默认推导执行该编排；显式单 Signal
请求仍只发送指定 Signal，不隐式补齐编排中的前序或后继 lifecycle。

底层 driver/derive 根请求没有显式 snapshot/scenario 时必须严格使用模型初态。`Kernel.Preset` 或其它
底层 driver/derive 根请求没有显式 snapshot/scenario 时必须严格使用模型初态。`Kernel.Preset` 或其它
后续 Signal 若因前期状态或事实尚未建立而被拒绝，报告具体缺项和完整失败链是正确结果。工具不得
隐式回溯 emitter、运行上游 transition 或合成到达时快照；调用者从后续边界继续时必须提供外部
snapshot/scenario。仓库可以提交从真实上游发送前截至推导得到的 canonical snapshot，并由公开快捷
入口把它作为后续 Signal 的默认外部输入；这仍是显式、可审计的场景选择，不改变 derive 的初态或
可达性语义，也不授权 derive 合成或回溯状态。

模型声明本身也是稳定快照可使用的结构事实。若 `has_slot(instance.field, SlotKind::Member)` 的第一个
实参能沿对象字段类型解析到含 `slots` 块的声明类型，且该类型或其基类型声明了与 `Member` 对应的槽位，
tools2 必须据此证明条件成立。证明必须从 model 中的字段类型和槽位声明通用推导，不能把类型占位符
事实误当成实例路径事实，也不能硬编码 `Config.fixmap`、`FixMapSlot::Fdt` 或某个主模型对象。不存在的
槽位仍应使本次 Signal 被拒绝；结构证明不得合成 lifecycle state、运行时 fact 或上游 Signal。

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
快捷入口省略 `-t/--trigger` 时执行唯一外部编排；`-t/--trigger` 可以覆盖为一个指定 Signal，
`-u/--until` 可以指定发送前截至；底层 driver/derive 的 source 默认同为 `Human`，省略
`--signal` 时同样执行唯一外部编排。仅当调用者显式给出 `-t/--trigger` 且没有给出
`-s/--scenario` 时，快捷入口按
规范化后的 Signal 查找 `tools2/scenarios/<CanonicalSignal>.snapshot.json`；显式 scenario 优先，
`Startup` 与 `Preset` 因而选择同一 canonical 文件。缺少默认文件或规范化名称不能安全落在 scenarios
目录内时，快捷入口必须在推导前以用户错误退出；省略 `-t` 时仍从模型初态执行 Human 外部编排，
包括只给出 `-u Kernel.Startup` 的发送前截至命令。tools2 协议统一为
version 5，移除 `lossy` 字段与 `discarded` outcome，并拒绝
version 1、version 2、version 3、version 4、老工具协议和旧 snapshot。动画封装协议为 version 3，
按 v5 event sequence 发布每个 Signal 的 request 及其 feedback、settle 或 terminal 因果时刻。受支持的旧 `tools/`
parse/model/derive/check/view/render 路径必须解析同一 `external` 声明并遵守
同一默认编排与显式单 Signal 边界；它们保留各自现有的中间协议版本，且不得导入 tools2 实现。

主模型的启动创建顺序固定为 Human 同步 drives `Computer.Preset`；该 handler 依次同步驱动三个直接
子 System 的 Preset。成功后 Human 同步 drives `Computer.Setup`；该 handler 再依次同步驱动三者
Setup 并建立 assembly fact。两步都成功后 Human 异步 emits `Computer.Enable`，随后经
`Computer -> Riscv64Platform -> OpenSBI -> Kernel` 交接。System 的自 Setup/Enable 以及这三段 System
交接中只有后者由异步 `emits` 形成全局 FIFO；Computer 的三个 handler 不相互发送 Signal。FIFO 不得
按 hierarchy depth 重排；Kernel 再在自身 Enable 响应中以
`drives` 完成 BootInitFlow、首次调度和 KernelInitFlow。每个 System.Online 表示该实例已经完成自身
charter 定义的 Enable 契约，不等待提交后 `emits` 的异步下游成功。

Human 创建的三个 Computer Signal 的 source 都是 `Human`；两次同步 Signal 的 delivery 是 `drives`，
Enable 的 delivery 是 `emits`。它们不是某个伪 `Human.Startup` Signal 的子 Signal，因而没有共同
`cause_id`；`root_request` 继续指向第一个真实 Signal `Computer.Preset`。

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
