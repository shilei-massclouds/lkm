# 模型到代码映射规格

本文记录 `spec/model` 中的模型元素映射到 Rust 内核代码时的默认规则。该文件会随着 `arceos_ex` 实践持续细化。

## 正式规格入口

本文件是说明性文档。对象级代码生成和实现阶段必须遵守的硬性要求已经移入正式规格 [`main.spec`](main.spec)，并由 [`mapping.spec`](mapping.spec) 承载。

后续说明、例子和建议不得降低 `mapping.spec` 中的硬约束。若无法满足正式规格中的任一硬约束，必须停止实现并报告规格缺口、工具缺口或实现阻塞。

## 规则强度

正式 coding 规格使用以下规则强度：

- `MUST`：硬约束。违反时必须停止实现，直到模型、coding 规格、工具或实现被修正。
- `SHOULD`：强建议。实现默认应遵循；若偏离，必须记录原因、生效范围和未来收敛路径。
- `MAY`：允许项。可以采用，但后续代码不得把它当作隐含依赖，除非先提升为 `SHOULD` 或 `MUST`。
- `NOTE`：说明项。只解释背景或取舍，不形成直接实现义务，也不能覆盖 `MUST` 或 `SHOULD`。

## 基本原则

- 首要硬约束：实现必须严格遵循模型规格推导出的对象状态、事件顺序和依赖边界。不得为了减少汇编、提前进入 Rust、复用既有工程路径或其它实现便利，提前执行尚未触发事件的对象职责；也不得先执行动作、后补状态或 checkpoint。若当前实现无法满足该约束，必须停止并报告规格或实现缺口。
- 模型规格是实现约束来源，不是事后解释材料。
- 代码应保留模型对象、事件、状态和阶段边界的可追踪性。
- Object Coding Phase 只处理对象级语义，包括对象状态、事件推进、依赖检查和 invariant 检查；crate 边界、公开 facade 和与既有组件体系兼容属于后续 Composition Phase。
- 若某条映射规则与具体实现发生冲突，应先记录冲突，再决定是补充 coding 规格、调整模型，还是停止实现并报告。
- 映射规则应服务于未来与 Linux 等参考内核的状态差分。来自同一规格的不同内核实现，应能在相同状态一致点上采集和比较状态。

## 对象粒度与层次

对象存在不同粒度。当前建模层级中不再继续拆分的最小粒度对象是原子对象；除此之外，对象通常是由更小粒度对象组合而成的复合对象。复合对象的状态通常由自身属性和子对象状态支撑，复合对象的生命周期 transition通常通过驱动子对象 transition来完成。

映射到代码时，高粒度对象通常对应更高抽象层次。若一个高粒度对象足以表达当前规格需要的状态、依赖和事件，应优先以该高粒度对象作为实现边界，再把具体细节封装进子对象、私有字段或内部 helper 中。这样有利于把复杂性逐级收敛到局部实现，减少上层阶段编排直接接触底层细节的机会。

这一条是建议性原则，不是硬约束。现实代码可能因为启动早期汇编边界、裸机地址空间限制、性能路径、历史接口、工具能力或实验阶段目标，暂时无法让对象边界完全匹配理想层次。出现这种情况时，应记录取舍原因，并保留未来向更合适复合对象边界收敛的可能。

## Object Coding Phase 源码映射

在对象级实现阶段，源码组织应首先服务于模型对象和事件的可追踪性，而不是服务于最终 crate/component 发布边界。

`startup-timeline` 对应 `main.rs`。`main.rs` 是最顶层启动时间线的承载点，负责建立最小入口边界，并组织一级 Phase 对象的过程调用。它不应吸收普通资源对象的状态和事件实现；这些实现应下沉到 `objects/` 或相应 Phase 过程调用的对象方法中。

Phase 对象对应主动的过程式代码。各级 Phase 对象应按规格中的包含关系放入 `phases/` 下的相应层次，例如 `BootPhase`、`InterruptPhase`、`EntryPreludePhase`、`EntrySuccessorPhase` 分别形成清晰的过程边界。Phase 源码只生成函数和 module，不生成资源对象式 `struct + impl`，也不生成普通对象式 `Lifecycle` 状态机。

资源对象和其它非 Phase 对象对应面向对象风格的 Rust 代码，默认形态是 `struct + impl methods`。原则上每个规格对象应有一个独立 `.rs` 文件；文件数量增加后，应优先按对象类别建立目录层级，例如 CPU、内存、启动参数、平台、输出、地址空间等，而不是按 Phase 分类。Phase 过程可以持有上下文或对象集合，但这种 context carrier 不等同于规格中的资源对象，不能替代资源对象自身的状态和transition 边界。

## Context 映射

`SHOULD`：对象级实现应维护一个全局 `Context`，用于承载启动过程中需要长期存活的资源对象。Phase 模块通过 `ctx` 或 `context` 参数/局部变量使用该上下文，并按模型顺序驱动资源对象 transition。

`SHOULD`：`Context` 的字段应按资源对象类别组织，并逐步与 `objects/` 目录层级一致。按 Phase 聚合资源对象只能作为过渡方案，必须记录原因和未来拆分方向。

`SHOULD`：全局上下文访问入口应集中维护，例如 `crate::context::context()` 和 `crate::context::context_ref()`。Phase 文件不应各自定义本地 `objects()`、`context()` 或类似访问函数。

`SHOULD`：承载 `Context` 的函数参数和局部变量应命名为 `ctx` 或 `context`，不应命名为 `objects`，以免与模型中的 `object` 概念混淆。

## 阶段链式映射规则

基于[阶段范式](../charter/phase-paradigm.md)，模型中的阶段树在 impl 中映射为平坦的函数调用链。本条规则取代下文"Phase 对象"节中的旧生成规则。

### 基本映射

1. **叶子子阶段是 impl 中唯一出现实体的层**。编排层（Kernel、BootPhase、InterruptPhase、UpMultitaskPhase、SmpRuntimePhase、PayloadPhase）不生成独立函数或模块；其 `drives` 语义坍缩为子阶段间的调用顺序。

2. **每个子阶段的每个迁移对应一个函数**：`preset()`、`setup()`、`enable()`。函数体遵循统一结构：

   ```
   fn xxx_preset/setup/enable() {
       // 1. depends_on → 断言检查
       // 2. drives → 调用被驱动对象的迁移函数
       // 3. ensures → 断言 + 推进自身状态
       // 4. emits → 调用本子阶段下一迁移函数
   }
   ```

3. **子阶段间串接**：前一个子阶段的 `enable()` 末尾（emits 位置）调用后一个子阶段的 `preset()`。若本子阶段是本层最后一个，则 `enable()` 末尾调用下一编排层首个子阶段的 `preset()`。

4. **编排层 coding 文件**只记录子阶段的串接顺序，不生成函数。例如 `coding/phases/boot.md` 描述 `EntryPreludePhase → EntrySuccessorPhase → CorePreparePhase → MmCoreInitPhase → SchedInitPhase` 的串接关系。

5. **叶子子阶段 coding 文件**详细描述每个迁移的 `depends_on`/`drives`/`ensures`/`emits` 到 impl 的具体映射。

### 整体调用形状

```
EntryPreludePhase.preset()   ← 由启动入口调用
  → .setup()
    → .enable()
      → EntrySuccessorPhase.preset()
        → .setup()
          → .enable()
            → CorePreparePhase.preset()
              → ...
                → PayloadPhase.preset()
                  → .setup()
                    → .enable()    → 不返回
```

## Phase 对象（旧规则，已被阶段链式映射规则替代）

Phase 对象表示阶段或子阶段的编排边界，例如 `PreparePhase`、`BootPhase`、`InterruptPhase`、`EntryPreludePhase`、`EntrySuccessorPhase`、`PayloadPhase`。

Phase 代码生成以模型中的阶段树为输入，默认采用深度优先遍历。生成器从顶层启动对象进入第一个阶段；若该阶段不属于当前内核代码生成范围，则只生成必要的事实采纳、检查或发布边界，然后回到父层级继续处理下一个阶段。若该阶段属于当前内核代码生成范围且包含子 Phase，则递归处理其子 Phase；若该阶段没有子 Phase，则生成该叶子 Phase 的主体 `setup()` 过程。

父 Phase 在模型中声明和组织直接子 Phase 的顺序，这一顺序是 coding 生成 `handoff()` 链的依据。父 Phase 本身不应被机械生成成"直接逐个调用子 Phase `setup()`"的串行过程；运行时控制流应由当前 Phase 的 `handoff()` 显式连接到同层下一个 Phase 的 `setup()`。当某个 Phase 是本层级最后一个子 Phase 时，它的 `handoff()` 回到父 Phase 的完成确认过程。

非叶子 Phase 的代码职责主要是边界确认：在所有子 Phase 通过 `handoff()` 链完成后，执行自身 `depends_on`、`ensures`、`invariant` 对应的检查和 checkpoint，并将父 Phase 推进到对应完成边界。叶子 Phase 的代码职责主要是按本 Phase 规格中的 `drives` 顺序推进普通对象 transition，并在完成后调用自身 `handoff()`。

特殊 Phase 可以由 coding 规格或对象规格显式覆盖默认生成方式。覆盖必须说明生效范围、原因和仍需保留的模型边界。例如入口前导期的 `setup()` 可以从架构入口符号开始，并由必要汇编和 Rust 续段共同组成；这种覆盖不改变 Phase 仍需提供 `setup()`、`handoff()`、边界检查和 checkpoint 的要求。

Phase 对象在源码中不得对应资源对象式 Rust `struct`。默认实现方式是函数和 module，例如：

- `entry_prelude_phase_setup(...)`
- `entry_prelude_phase_handoff(...)`
- `entry_successor_phase_setup(...)`
- `entry_successor_phase_handoff(...)`
- `boot_phase_setup(...)`

每个 Phase 对象在 coding 阶段默认生成 `setup()` 和 `handoff()` 两个过程函数。`handoff()` 是 Phase coding 中对模型 `cleanup` 的本地别名，表达"本阶段退出服务并移交控制权"。若模型中显式定义了该 Phase 的 `cleanup`，则 `handoff()` 的前半段必须实现对应动作、检查和 checkpoint；若模型没有显式 `cleanup`，则 `handoff()` 只负责移交执行权。

Phase 的 `setup()` 负责本阶段主体推进。除准备期等明确例外外，`setup()` 成功完成本阶段目标后，最后一步必须调用本 Phase 的 `handoff()`。同一层级内，前一个 Phase 的 `handoff()` 调用下一个兄弟 Phase 的 `setup()`；若当前 Phase 是本层级最后一个阶段，则调用父 Phase 的 `setup()` 或父 Phase 的完成确认过程。父 Phase 是子 Phase 顺序的组织者和规格来源，但 coding 中不应简单生成"父 Phase 直接逐个调用子 Phase"的控制流。

Phase 过程的主要职责是按规格中的 `drives` 顺序推进普通对象的生命周期 transition，并在阶段边界调用模型边界检查函数。Phase 自身的 `depends_on`、`ensures` 和 `invariant` 应转化为显式 checkpoint/check 函数；这些函数先检查对应事实，成功后才发出 trace checkpoint。Phase checkpoint 是模型状态边界函数，不只是日志 hook。

为了支持阶段 invariant、父阶段完成确认和未来状态差分，可以为每个 Phase 设置轻量的全局状态记录变量。该变量只记录 `Base`、`Ready`、`Online`、`Destroyed` 等模型边界状态，不负责事件合法性推进，也不替代资源对象的 `Lifecycle`。Phase 的事件唯一性和顺序约束由生成出的 `setup()/handoff()` 调用结构、检查器和测试共同保证。

`PreparePhase` 表示入口前已经形成的准备边界，当前 coding 中作为明确例外处理。它可以生成准备事实的检查或发布函数，但不强制生成普通 Phase 的 `setup()/handoff()` 链。`PreparePhase.Enable` 的代码映射另行讨论，不应影响普通 Phase 的生成规则。

### ArceOS/Unikernel 引导边界

映射到 `arceos_ex` 时，`ax-hal-ex` 只应承载最低层入口前导路径。它负责从 `_start` 开始建立进入 Rust 代码所需的最小执行条件，并推进到
`EntryPreludePhase.Ready`。

`EntryPreludePhase.Ready` 之后应交由 `ax-runtime-ex` 接管。`EntrySuccessorPhase` 是
`ax-runtime-ex` 引导过程的第一部分，而不是 `ax-hal-ex` 的长期编排职责。`ax-runtime-ex`
可以调用 `ax-hal-ex`、平台 crate 和其它组件提供的对象 transition函数，但阶段编排边界应保留在 runtime 侧。

`ax-runtime-ex` 覆盖从 `EntrySuccessorPhase` 开始到 `PayloadPhase` 为止的大部分内核引导过程。当前正式模型中，`InterruptPhase` 位于
`BootPhase` 之后、`PayloadPhase` 之前；后续新增的内核初始化阶段、服务初始化或运行形态切换，应默认插入在
`EntrySuccessorPhase` 与 `PayloadPhase` 之间的正式阶段树位置，并继续保留模型 checkpoint。

在 ArceOS Unikernel 形态下，`PayloadPhase` 负责把启动编排链移交给 selected payload，而不把该入口固定为传统操作系统的普通用户态进程入口。不同 Unikernel 应用、测试应用或未来的宏内核引导应用，都可以作为 selected payload。未来若增加宏内核形态，相关 payload 可以在 `PayloadPhase.Enable` 后完成切换到用户态并启动首个用户态应用的最后步骤。`PayloadPhase.Enable` 的运行期约定是不返回：payload 要么进入服务循环，要么最终停机。

## 普通对象

除 Phase 对象外，模型对象原则上应对应 Rust `struct`、静态单例或启动上下文中的结构化字段。

资源对象的事件应优先实现为该对象 `impl` 上的方法。若启动早期限制导致对象暂时只能由静态单例、裸指针范围或上下文字段承载，也应保留明确的对象命名和transition 函数边界，并记录后续收敛为独立对象文件的计划。

普通对象的代码实体应承载以下信息中的一部分或全部：

- 当前生命周期状态或可推导的 checkpoint。
- 规格中定义的关键属性。
- `depends_on` 所需的前置事实。
- `ensures` 和 `invariant` 需要记录或验证的事实。

如果某个普通对象暂时无法形成独立 struct，应记录原因，并说明它由哪个父对象、上下文对象或模块字段承载。

## 事件映射

模型 transition默认映射为明确命名的 Rust 函数或方法：

- `Preset` -> `preset`
- `Setup` -> `setup`
- `Enable` -> `enable`
- `Cleanup` -> `cleanup`

Phase 对象的事件默认映射为过程函数，例如 `entry_prelude_phase_setup(...)`。资源对象的事件默认映射为对象方法，例如 `raw_dtb.setup(...)` 或 `early_vm.enable(...)`。

transition 函数应尽量只推进一个对象的一次生命周期迁移。若某段底层实现天然覆盖多个模型 transition，应在上层显式拆分transition 边界，或记录不能拆分的原因。

生命周期 transition和后续操作事件都应具有显式返回结果。代码实现可以使用适合所在层级的具体类型，但语义上至少应能区分：

- `Success`：transition 成功完成，状态迁移已提交。
- `Blocked(reason)`：当前条件暂未满足，状态迁移未提交。
- `Failed(code)`：事件失败，状态迁移未提交。

对于当前启动路径中的生命周期 transition，第一轮可以先用 `bool` 或等价最小结果承载 `Success/Failed`；一旦需要区分等待、重试、错误码或状态差分，应升级为结构化结果类型。无论具体返回类型如何，只有成功返回才能记录目标状态、发出完成 checkpoint 或使对应 `ensures` 对后续推导成立。

## Action 映射

`action` 映射为依附于对象某一既有状态的 Rust 函数或方法。它可以有副作用，也可以失败或阻塞，但成功时不推进当前被建模状态。

典型 action 包括：

- `Console.Online.write(...)`
- `EarlyIoremap.Ready.map(...) / unmap(...)`
- `SBI.Ready.console_putchar(...) / system_reset(...)`

如果某个操作成功时会推进被建模状态，应建模为 event，而不是 action。例如普通非嵌套自旋锁的 `lock`、`try_lock`、`unlock` 都是操作事件；它们可以反复触发，但每次成功都会提交锁运行状态迁移。

## 状态与检查点

状态可以通过以下方式表示：

- `enum` 状态字段。
- typestate 类型。
- debug/runtime checkpoint。
- 编译期或运行期断言。
- 供外部采集的 trace 点。

第一轮不要求所有状态都以运行期字段长期保存，但关键状态一致点必须能在代码中定位和采集。当前至少应保留以下类别的 checkpoint：

- 阶段和子阶段完成点。
- 普通对象生命周期迁移完成点。
- 未来可能与 Linux 参考状态做差分的状态点。
- 规格中 `Ready`、`Online`、`Destroyed` 对后续对象形成依赖的状态点。

第一轮可以先预留 checkpoint 接口，不要求立即输出完整差分数据。最小运行目标仍是通过 SBI early console 运行默认 smoke payload 并关机；独立 `APP=hello` 仍保留为最小输出路径示例。

checkpoint 在代码中应实现为 hook，而不是普通日志调用。默认 hook 为空实现；具体工程可以通过编译或链接选项插入不同处理机制，例如 SBI 单字符输出、内存 trace buffer、QEMU 调试出口或未来的状态差分采集器。hook 不改变对象状态，不参与事件推进，也不能成为规格依赖。

checkpoint hook 只定义观察时机。结构化观察内容应来自对象或 provider 暴露的长期 observation facts，例如生命周期状态、
计数器、上下文标志、同步边界事实或最近一次 action snapshot。handler 可以读取这些事实并输出、分类或停机，
但不得把 handler 的存在作为对象 transition 的前置条件，也不得把 handler 内部临时状态当作模型事实。

失败诊断和 checkpoint handler 是不同机制。`failure_diagnostic` 属于失败路径上的错误 payload：在检测到
predicate/check 失败时采集必要对象事实，随错误传播，并在最终输出阶段打印。它不是新增 checkpoint，也不是注册在
某个失败 checkpoint 上的 handler。成功路径不得因为 failure diagnostic 支持而改变 checkpoint 序列。

checkpoint announce 是独立观测路径，不属于 `EarlyCon` 或正式 `Console`。在入口前导期最早阶段，允许使用极小的 SBI 字符输出后端，只输出稳定 checkpoint id 对应的单个字符。这样可以避免字符串地址、缓冲区地址、allocator、FixMap、线性映射或 console 初始化状态对 announce 的影响。完整名称和语义应由静态映射表维护，例如 `EarlyVm.Ready -> 'D'`；字符仅用于早期烟雾测试和定位。

如果某个 checkpoint 位于页表切换前后，hook 实现必须保证自身代码地址在当前地址空间可执行，且不得读取尚未映射的数据。进入 `EarlyVm` 或更晚阶段后，可以切换到更丰富的 announce/observer 后端，但仍应保持与 early console/console 路径隔离。

## 依赖与后置事实

`depends_on` 应映射为函数前置检查、类型约束、构建期检查或启动断言。

`ensures` 和 `invariant` 应映射为后置状态记录、防御性检查、断言、测试断言或可观测 trace 点。invariant 检查不得只停留在注释层面；在可执行路径中应至少有一种对应机制，例如transition 前置检查、transition 后置检查、`assert!`、`debug_assert!`、构建期检查或单元测试。若 invariant 失败，事件不得提交目标状态。

对于暂时无法自动验证的事实，不得静默跳过。应选择以下处理之一：

- 停止实现并报告规格或实现缺口。
- 将其记录为明确的 coding 假设，并说明生效范围。
- 补充模型或 coding 规格后再继续实现。

## 汇编边界

入口前导期允许使用必要汇编，但应尽量薄。汇编代码默认只承担 Rust 难以可靠表达的最小职责：

- 接收并保存启动 ABI 传入的寄存器参数。
- 建立最初可用的启动栈。
- 执行必须在 Rust 入口前完成的极小 CSR 或地址控制操作。
- 跳转到 Rust 入口。

对象推进、状态记录、DTB 检查、页表构造、SBI 能力视图建立和阶段编排应尽量进入 Rust 代码。若必须留在汇编中，应在对应对象 transition中记录该实现边界。

## 未来差分

`arceos_ex` 将来可能与 Linux 或其它由同一规格指导的内核进行状态差分。当前实现虽然不要求完成差分工具，但代码结构应避免破坏这一可能性：

- 不应把多个模型状态点合并到无法区分的一段不可观测代码中。
- 不应让关键事实只存在于局部临时变量且无法采集。
- 不应用平台硬编码替代模型中的对象事实。
- 关键 checkpoint 的命名应尽量沿用模型对象和状态名称。

### Linux checkpoint mapping-only 规则

Linux checkpoint mapping artifact 只记录可审阅的静态源码锚点。默认参考树是只读
`../linux-6.12`；mapping pass 不应修改 Linux 源码、不应添加 probe，也不应依赖
runtime 采集结果来证明锚点。

映射规则只能声明当前能从 Linux 源码复核的边界。若 Linux 没有与 `arceos_ex` 普通对象
一一对应的对象边界，应使用 `range` 或 `medium` confidence 表达部分对齐；若无法从源码
稳定证明，checkpoint 必须保持 `unmapped`，并在 notes 中记录具体原因。用户态启动路径允许
`UserBoot` 与运行期 `UserExec` 复用同一 Linux exec/binfmt/riscv return 锚点，但 notes
必须区分这是 boot-time init exec 视图还是运行期 exec syscall 视图。

Finalize 末端的 deferred/trimmed checkpoint 只有在 Linux `kernel_init()` 中存在直接可复核
call site 或状态赋值时才可映射。纯 deferred 语义或缺少本地 Linux 对象边界的 checkpoint
应继续保留 `unmapped`。

若单个 Linux call-site 同时承载 `arceos_ex` 的阶段边界和对象级事实，mapping artifact 可以
让多个 checkpoint 复用同一锚点，但每条 notes 必须区分该条记录声明的是 phase boundary、
object fact 还是 deferred boundary。Runtime Core 窗口中的 `sched_init_smp()` 和
`page_alloc_init_late()` 属于这种情况：共享锚点不表示 Linux 暴露了多个独立对象，只表示同一
源码边界可静态复核不同的 `arceos_ex` 语义视图。

## 待补充

- `arceos_ex` 第一轮对象到 crate/module/struct 的具体映射表。
- checkpoint 输出格式。
- 与 Linux 状态差分的采集接口。
- typestate 与运行期状态字段的选择规则。

<!-- formal-predicate-notes:spec/coding/mapping.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/mapping.spec` 的长注释迁移而来。`*.spec` 文件只保留 rule ID、type 分组、MUST/SHOULD/MAY/NOTE 层级和最短标签；解释、背景、参考路径、阶段性取舍与例子在这里维护。

### CodingRuleLevels

#### MUST

A mandatory rule. Violating a MUST rule blocks implementation until
the model, coding spec, tool or implementation is corrected.

#### SHOULD

A strong recommendation. Generated or handwritten code is expected to
follow it by default. A deviation is allowed only with an explicit
recorded reason, scope and future convergence path.

#### MAY

An optional technique. It is permitted when useful but must not be
required by later code unless promoted to SHOULD or MUST.

#### NOTE

Explanatory guidance. It carries no direct implementation obligation
and cannot override MUST or SHOULD rules.

### CodingMappingMust

#### Model priority

Implementation must strictly follow objects, states, transitions,
dependencies, drive order, phase boundaries and proof obligations
derived from spec/model. Implementation convenience, directory habit,
reused code, early Rust entry, reduced assembly or build-tool limits
are not valid reasons to diverge from model semantics.

#### Map before coding

Code generation or modification must first identify the model object
to source-file mapping. Code may only be written to the mapped file or
its explicit submodules unless a recorded exception exists.

#### Object ownership

The main implementation of a model object must live in that object's
mapped file. Phase setup/handoff/checkpoint code belongs to the Phase
file; resource-object state and event methods belong to the resource
object file.

#### Phase tree path

Phase file paths must reflect the model parent/child phase tree.

#### DFS generation

Phase code must be generated from the phase tree using depth-first
traversal. Runtime control between sibling phases is connected by
handoff() -> next.setup(), not by flattening all child phase bodies
into the parent phase.

#### Phase shape

Phase objects must not be implemented as resource-object style
struct + impl lifecycle state machines. They are process modules with
setup(), handoff(), boundary checks and checkpoints.

#### Transition boundaries

Each model transition must keep a locatable code boundary. If low-level
code must complete adjacent transitions without a hardware-visible gap, the
mapped source must still preserve transition functions, state adoption,
checks and checkpoint boundaries.

#### Checks before checkpoints

depends_on, ensures and invariant facts must be checked before the
target state is committed, a checkpoint is emitted or later code uses
the fact as an established dependency.

#### Checkpoint ownership

A checkpoint may only be emitted by the mapped implementation of the
object event or phase boundary whose fact it reports. A lower-level
phase, resource object, assembly entry point or continuation must not
emit synthetic checkpoints for parent phases, sibling phases,
preparation phases or other objects merely to make the runtime trace
visually match the derived trace.

#### Checkpoint as observation timing

A checkpoint is a stable observation timing boundary. It may trigger
trace backends or observer handlers, but it is not ordinary logging,
does not advance object state by itself, and must not become a
dependency of the transition whose boundary it observes.

#### Observation facts

Structured observation content must live on the owning object,
provider or explicit context object as long-term facts. Handlers may
read, classify or emit those facts; they must not manufacture model
facts through handler-local debug state.

#### Failure diagnostic separation

Failure diagnostics are structured error payloads collected on a
failing predicate/check path and emitted when the error is reported.
They are not additional checkpoints and are not checkpoint handlers
registered on a failure point. Supporting them must not change the
successful checkpoint sequence.

#### Linker script mapping

A generated or maintained linker script is the coding artifact that
realizes the model Lds object. It must be driven by the PreparePhase
Lds attributes and by the Config attributes that Lds depends on, such
as kernel addresses, page size, section alignment, head-text layout
and boot-stack size. Hard-coded linker constants are only permitted
as recorded transitional exceptions with the corresponding model
Lds/Config source named.

#### Explicit exceptions

Architecture, linker, Rust-language or boot-ABI constraints that force
code outside the default mapped file must be recorded with reason,
scope and the preserved model boundary.

#### Verification gates

Changes touching model semantics, phase call chains, object states or
entry paths must pass the configured build/verify/run gates before
they are treated as complete.

### CodingMappingShould

#### Global context

Object Coding Phase should maintain long-lived resource objects in a
single implementation Context rather than in phase-local object
carriers. Phase modules should borrow this Context and use it to
drive resource-object transitions.

#### Context parameter names

Function parameters and local variables that carry the implementation
Context should be named ctx or context. They should not be named
objects, because objects has a formal model meaning.

#### Context resource layout

Context fields should be grouped by resource-object category, matching
the source directory hierarchy as it evolves. They should not be
grouped by Phase except as an explicitly recorded transitional step.

#### Context accessor placement

Context accessors should be centralized, for example as
crate::context::context() and crate::context::context_ref(). Phase
files should not define their own local objects()/context() accessors.

### LinuxCheckpointMappingRules

#### Static source only

Linux checkpoint mapping artifacts record reviewable anchors in the
read-only Linux reference tree. The mapping pass must not modify
Linux source, add probes, or depend on runtime collection to justify
a mapping.

#### Unproven stays unmapped

If a checkpoint cannot be tied to a stable Linux source boundary from
the reference tree, it must remain unmapped and the notes must record
why no reliable boundary was claimed.

#### Partial boundaries

When Linux lacks a local object boundary matching the arceos_ex
checkpoint, the mapping should use a range or medium confidence to
make the partial alignment explicit.

#### UserBoot versus UserExec

Boot-time init exec checkpoints may reuse Linux exec/binfmt/return
anchors that also describe runtime exec syscall checkpoints, but the
notes must distinguish the boot-time init exec view from the runtime
exec syscall view.

#### Shared call-site semantics

Multiple arceos_ex checkpoints may reuse the same Linux call site
when a single Linux boundary is the reviewable anchor for a phase
boundary and one or more object facts. Each mapping note must name
the semantic view being claimed, such as phase boundary, object fact
or deferred boundary, so the artifact does not imply distinct Linux
objects where Linux exposes only one local call-site boundary.

do_basic_setup() object-level checkpoint mappings may reuse direct
call sites from the same Linux function, including do_initcalls().
Their notes must distinguish cpuset/cgroup trimmed no-op position
reservation, driver core deferred boundary, procfs IRQ view deferred
boundary, constructor table dispatch, initcall table dispatcher, and
the do_basic_setup() end boundary before the KUnit handoff.

<!-- formal-predicate-notes:spec/coding/mapping.spec END -->
