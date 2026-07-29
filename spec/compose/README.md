# Compose 规格

2026-07-29 KernelAddrSpace/translation controller 复核：`KernelAddrSpace` 与 `Vm` 在既有
`arceos_ex::objects`/`Context` 内拆为私有 module，四个 controller 的激活身份收归既有 canonical
`Cpu`；BootTask bind action 和 AP boot-data 扩展也只在既有 boot-init/SMP/arch 私有边界内流动。
不新增 crate、facade、feature、公开 API 或组件依赖。`spec/compose/main.spec` 增加三项组合 invariant，
防止后续把地址空间资源与控制面重新合并、复制 CPU owner 或发布 controller internals。

本目录记录 `Composition Phase` 的补充约束。

2026-07-27 tools2 Signal animation v3 复核：request/feedback/settle 因果时刻只改变独立 Python
`animate` 投影、离线 HTML 协议和 Svelte 播放器；tools2 v7 derive 的同步响应、异步 FIFO、Signal
identity/cause、失败传播与 event sequence 不变，也不新增或
调整内核 crate、Rust module、facade、feature、公开 API 或组件依赖。`spec/compose/main.spec` 已复核，
组合语义无需修改。

2026-07-28 CurrentTask 选择器复核：删除 CPU-local writable slot 只改变既有 objects/boot-init 私有实现
中的执行上下文解析与 RISC-V64 `tp` lowering；无状态 capability、Task/Flow/CPU 一致性检查和调度提交
仍由既有 crate、module 与 facade 承接，不新增公开 API 或组件依赖。`spec/compose/main.spec` 已复核，
组合语义无需修改。

2026-07-27 Linux RV64 Kernel 启动契约复核：新增的只读 `LinuxRiscv64KernelBootSpec` 仍由既有
`systems::kernel` metadata 和入口验证承接；物理 PMD 对齐及 `satp=0` 检查使用既有 Config/Lds、CSR
与 Kernel.Enable 接受边界，`sie/sip` 清零继续位于既有 BootInitFlow/InterruptType 入口汇编路径。
不新增 crate、module、facade、feature、公开 API 或组件依赖；`spec/compose/main.spec` 已复核，组合
语义无需修改。

2026-07-26 Kernel.Enable 分层迁移复核：晚移 `Kernel.Online` 只调整既有 `systems::kernel`、
BootInitFlow、scheduler、KernelInitFlow 与 payload 私有 module 之间的状态检查和 checkpoint 顺序；
不新增 crate、facade、feature、公开 API 或组件依赖，也不引入 continuation module 或 pending 状态。
`spec/compose/main.spec` 已复核，组合语义无需修改。

2026-07-25 lifecycle 继承与 CopyProcess current-task 投影复核：effective handler 的组成只改变规格
工具对既有 Type/instance 声明的解释；Rust `copy_process` 在既有 objects/boot-init 私有边界增加只读
`CurrentTask` capability 与 `TaskRef` 输入，不新增 crate、facade、feature、公开 API 或组件依赖。
`spec/compose/main.spec` 已复核，组合语义无需修改。

2026-07-24 tools2 Signal animation 复核：独立 Python `animate` 包、Svelte/TypeScript 源码、锁定的
frontend 工具链和纳入仓库的静态 JS/CSS bundle 只组合 tools2 离线 HTML 产物；不新增或调整内核
crate、Rust module、facade、feature、公开 API 或组件依赖。`spec/compose/main.spec` 已复核，内核
组合语义无需修改。

正式规格入口是 [`main.spec`](main.spec)。本文件只解释组合封装阶段的背景、范围和取舍。

2026-07-24 Task/TaskFlow 断点与 AP authority 复核：本轮在既有 `objects`、scheduler、RISC-V
switch 和 SMP bringup module 内，把寄存器现场统一收归 Task，并把 AP idle Task/Flow 改为按
logical-id 的既有对象族 lowering；不新增 crate、facade、feature、公开 API 或组件依赖。
`spec/compose/main.spec` 已复核，组合语义无需修改。

2026-07-23 静态输入与 Enable 启动链复核：Config/Lds 改为构建时初态 Online 输入，Computer、
Riscv64Platform、OpenSBI 改为初态 Ready 的 metadata-only system，并把形式启动链压缩为显式
Enable 交接。这些调整不新增 crate、module、facade、feature、公开 API 或真实启动入口；既有 OpenSBI、
汇编入口、Kernel Rust 入口和 checkpoint 顺序保持不变。`spec/compose/main.spec` 无需修改。

2026-07-23 tools2 声明结构事实复核：`has_slot` 的实例属性类型解析只发生在独立 Python derive 包内，
消费既有 model protocol 的字段/类型信息；不新增或调整内核 crate、Rust module、facade、feature、
公开 API 或组件依赖。`spec/compose/main.spec` 已复核，组合语义无需修改。

2026-07-25 单一系统树启动链复核：Computer/Riscv64Platform/OpenSBI 继续使用 metadata mapping，
Kernel 保留真实入口 lifecycle；退役的工程 metadata module 不形成组件接口。`Computer` 由
Riscv64Platform/OpenSBI/Kernel 静态组装，运行启动由显式异步
Computer -> Riscv64Platform -> OpenSBI -> Kernel Signal 链表达。静态 parent/assembly 不生成调用，
metadata mapping 也不替代现有 firmware、assembly 或 Kernel Rust 入口。`spec/compose/main.spec`
无需新增 crate、facade、feature 或公开 API。

2026-07-22 tools2 完整主模型与 effective parent 复核：本轮只统一规格模型消费者的层级解释、独立
Signal 工具协议和仓库内 CLI 入口，不新增或调整内核 crate、module、facade、feature、公开 API 或
组件依赖。`spec/compose/main.spec` 已复核，组合语义无需修改。

2026-07-21 Task / TaskFlow 内部 module 对齐审查：本轮在既有 objects crate 内把
`objects::task_flow::{TaskFlow, TaskFlowRef}` 建立为独立规范路径，并由 `objects::task` 仅承载 Task
core/identity。该路径调整不新增 crate、facade、build feature 或外部兼容 API，不保留旧 module
re-export；`spec/compose/main.spec` 已复核，外部 composition 语义无需修改。

2026-07-21 Task 类型级 lifecycle 与 instance 瘦身闭合审查：本轮把普通 Task lifecycle 统一 lower
到既有 Rust `Task` core，并把 KernelInit/Kthreadd 角色编排收回既有 boot-init phase module；
不改变 crate、module、facade、build feature 或组件发布边界。`spec/compose/main.spec` 已复核，
组合语义无需修改。

2026-07-22 BootTask/BootInitFlow 闭合审查：删除 crate 内临时 boot-init TaskFlow module/Context field，并把
启动期内部 module 统一为 `phases::boot_init`，用于承载 PhaseObject 生命周期。
这些都是 `arceos_ex` crate 内私有 lowering 调整，不新增 facade、feature、crate 或兼容 alias；
`spec/compose/main.spec` 已复核，组合语义无需修改。

2026-07-23 Task 执行权重构复核：`BootInitFlow` 继续以
`phases::boot_init::BootInitFlow` 私有 wrapper 内嵌统一 `objects::task_flow::TaskFlow` core，并在既有
`Context`/scheduler/objects module 内完成 Task OnCpu 与严格 continuation lowering；删除内部
`DispatchWindow`。CurrentTask 由 TaskFlow 执行上下文解析，不新增 crate、feature、facade 或
public compatibility API，`spec/compose/main.spec` 无需改变。

2026-07-22 TaskFlow 直接子阶段归属复核：Boot/Interrupt/SmpRuntime 目录继续作为 crate 内私有
namespace，删除其 wrapper lifecycle；payload 拆成两个私有 leaf module，KernelInitFlow commit
action 仍位于既有 objects/runtime 内部边界。该调整不新增 crate、feature、facade、公开 alias 或
组件依赖，`spec/compose/main.spec` 无需改变。

2026-07-24 启动 Flow 物理布局复核：crate 内新增私有 `flows` 代码层，按父 Flow 归组
`BootInitFlow`、`BootIdleFlow` 及其直属子阶段；删除 `phases::boot_init` 旧路径且不提供兼容 re-export。
该变化只调整 crate 内 canonical module ownership，不新增 crate、feature、facade、公开接口或组件依赖，
因此 `spec/compose/main.spec` 的组合语义无需改变。

`Composition Phase` 位于 `Object Coding Phase` 之后。它不重新定义模型对象、状态、事件、依赖和阶段顺序，而是在对象级编码实现已经满足规格语义的前提下，决定这些对象如何被组合、封装和发布。

## 阶段目标

`Composition Phase` 关注以下问题：

- 哪些对象级实现放入哪些 crate。
- crate 内部如何拆分 Rust module。
- crate 和 module 的公开接口如何设计。
- 哪些接口需要 adapter 或 facade。
- 构建工具如何选择目标内核、平台、应用和 feature。
- 如何让新实现尽量接近参考内核的组件形态，便于演化和维护。

该阶段不得改变对象级语义。若组合封装需求与模型 transition、状态迁移或依赖关系冲突，应回到 `spec/model` 或 `spec/coding/` 处理，而不是在封装层隐式改变行为。

## 与 Coding 的分工

`spec/coding/` 负责对象级编码规则：

- 对象由什么 Rust 类型、静态单例或启动上下文字段承载。
- 事件如何推进状态。
- 依赖和后置事实如何检查。
- checkpoint/hook 如何对应状态一致点。
- 架构、语言、安全和外部 crate 信任边界。

`spec/compose/` 负责组合封装规则：

- 对象级实现如何组织为 crate/module。
- 哪些 public API 需要保持与参考实现接近。
- 哪些内部实现可以重构，哪些接口需要兼容。
- overlay workspace、构建配置和应用选择如何接入。

代码物理位置不等于最终组件边界。早期实现可以先把对象级代码放入方便运行的 crate 中；后续整理时，应按本目录规则重新审视 crate/module 边界。

## ArceOS 组合原则

`arceos_ex` 的组合封装目标是成为 ArceOS 的规格化演化候选，而不是另起一套完全无关的工程结构。因此在 `Composition Phase` 中应遵循：

- 组件形态尽量沿用 ArceOS 的 crate 边界和命名习惯。
- crate 内部 module 的公开接口尽量接近 ArceOS。
- `ax-std`、`ax-api`、`ax-feat` 这类应用接口层第一轮不主动复制；优先通过构建工具把底层实现切换到 `_ex` 组件。
- 接口兼容是优先目标，但不是百分百硬约束。若规格语义、对象边界或演化路径要求改变接口，应记录原因。
- `os/arceos` 和已有 `components` 实现作为只读参考；`arceos_ex` 通过新增目录、overlay workspace 或新增 `_ex` 组件接入。
- 第一轮只参考并接入 ArceOS 的 `ax-std` Unikernel 路径；不参考、不适配也不调试 ArceOS C API 和 Rust std/Hermit API 路径。

组合封装阶段可以复用 ArceOS 的接口形状、目录习惯和成熟构建路径，但不得因为 ArceOS 现有模块边界而改变模型对象的状态迁移。

## arceos_ex 组件化试验计划

本节记录当前人与 AI 讨论形成的工程共识。它是可持续补充的协作备忘录，不是正式规格；其中已经稳定、需要强约束的部分再进入 [`main.spec`](main.spec)。

`arceos_ex` 采用与 ArceOS 并行的 `_ex` 组件体系。crate 包名必须带 `-ex` 后缀以避免和现有 ArceOS 组件冲突，例如 `ax-hal-ex`、`ax-runtime-ex`、`ax-alloc-ex`、`ax-mm-ex`、`ax-task-ex`、`ax-std-ex`、`ax-api-ex`、`ax-feat-ex`。目录仍按 ArceOS 习惯组织；`os/arceos_ex/modules/axhal`、`os/arceos_ex/modules/axruntime` 这类目录名可以不带 `_ex`，因为上级目录已经区分；新增到顶层 `components/` 的 ex 组件目录必须带 `_ex` 后缀，例如 `components/someboot_ex`。

组件源码内部可以通过 Cargo 依赖别名保持 ArceOS 风格的 Rust 路径，例如：

```toml
ax-hal = { package = "ax-hal-ex", path = "../axhal" }
ax-task = { package = "ax-task-ex", path = "../axtask" }
```

这样包名区分实现，代码仍可使用 `ax_hal::...`、`ax_task::...` 等接口形态。公开 API 应尽量与 ArceOS 对应组件兼容；真正不同的语义或边界必须记录原因。

主干启动链按以下组件衔接：

```text
someboot-ex -> axplat-dyn-ex -> axruntime-ex -> apps(payload)
```

在当前 tgoskits/ArceOS 构建逻辑中，`riscv64` 默认启用动态平台路径，即默认 `plat_dyn = true`。因此在 `riscv64` + `plat-dyn` 场景下，上述链条可视为 ArceOS 的组件级主启动链。其符号级控制流大致为：

```text
firmware/OpenSBI
  -> someboot::_head/kernel_entry
  -> someboot::prime_entry
  -> __someboot_main
  -> axplat-dyn::boot::main
  -> ax_plat::call_main
  -> __axplat_main
  -> axruntime::rust_main
  -> ax_app_entry
  -> app main
```

`someboot-ex`、`axplat-dyn-ex`、`axruntime-ex` 和 `apps(payload)` 是启动链上的四个组件级节点。`axplat-dyn-ex` 内部可以继续使用 `ax-plat` 的接口和宏机制完成桥接，例如 `ax_plat::call_main` 到 `axruntime-ex` 的入口映射。

LKM 子阶段在第一轮组件化中按以下规则归属：

- `入口前导期` 子阶段放入 `someboot-ex`，负责最低层入口、启动参数、linker/entry layout、BSS、栈和进入下一启动节点前的早期准备。
- `payload` 子阶段对应 `apps(payload)`，第一轮从 helloworld 起步，后续由 normal tests 逐步扩展。
- 除 `入口前导期` 和 `payload` 之外的其它 LKM 启动子阶段，第一轮统一归入 `axruntime-ex`，由它作为启动编排层承接并调度。
- `axplat-dyn-ex` 第一轮只作为 `someboot-ex` 到 `axruntime-ex` 的中间桥接和动态平台入口适配层，不放置 LKM 子阶段对象。

上述归属是启动链封装的初始策略，不表示所有对象永久属于 `axruntime-ex`。后续若某个子阶段本质上是在初始化资源对象，例如 allocator、task、mm、driver、IRQ 等，应逐步下沉到对应功能组件；`axruntime-ex` 保留调用编排，功能组件承接资源对象和公开接口 shim。

LKM 中的阶段对象应优先封装进上述沿启动线衔接的主干组件。LKM 中的资源对象按功能组件逐步封装，例如 `axtask-ex`、`ax-alloc-ex`、`ax-mm-ex` 等。功能组件应通过内部 shim 把公开接口映射到内部资源对象接口，避免应用侧或测试侧直接依赖 LKM 对象实现细节。

推荐实现顺序：

1. 先扩展 tgoskits 构建方式，新增 `cargo xtask arceos_ex ...` 目标，并使用独立 snapshot，例如 `.arceos_ex.toml`。
2. 建立 `os/arceos_ex` 和必要 `_ex` 组件骨架，使 helloworld 的 ex 依赖图可以解析。
3. 分析并实现主干组件前后顺序和衔接接口，先打通 `someboot-ex -> axplat-dyn-ex -> axruntime-ex -> payload`。
4. 把 LKM 阶段对象按主干链封装进对应组件。
5. 以测试为引导逐步封装资源对象；第一验收点是 helloworld，即 `cargo xtask arceos_ex qemu --arch riscv64`。
6. 后续 normal tests 按简单到复杂拆分成若干小组，引导 `ax-alloc-ex`、`ax-mm-ex`、`axtask-ex`、IRQ、FS、NET 等功能组件逐步成形。

测试源码本身不得因为 `arceos_ex` 目标而修改。`arceos_ex test` 应在构建层完成依赖选择或重定向，让测试中原本面向 ArceOS 的 `ax-std`、`ax-api`、`ax-feat` 等接口解析到 `_ex` 组件体系。若某个测试需要新公开接口，优先检查是否应该补齐兼容 shim；只有当模型或对象边界确实要求改变接口时，才记录例外。

### 调用链与功能组件推进计划

启动链打通之后，下一步重点转向调用链。当前需要区分两条链：

- 启动链：`someboot-ex -> axplat-dyn-ex -> axruntime-ex -> apps(payload)`，负责从固件入口推进到应用入口。
- 调用链：`apps(payload) -> ax-std-ex -> ax-api-ex -> 功能组件_ex -> shim -> LKM 对象`，负责应用和测试调用公开接口时如何落到对象实现。

`helloworld` 通过只说明最短 console 调用链已经可用，即 `ax-std-ex::println! -> ax-api-ex console -> ax-hal-ex console -> early console`。这不等价于 `ulib` 和 `api` 两层已经完整具备。后续补齐 `ax-std-ex`、`ax-api-ex` 和功能组件时，应把测试失败区分为三类：公开接口形状缺失、shim 映射缺失、底层 LKM 对象能力缺失。只有前两类适合直接通过组件封装解决；第三类需要回到对象能力本身评估。

测试支持排序应考虑 LKM 当前对象能力，而不只按测试名或 ArceOS 组件依赖排序。优先处理不强依赖新对象能力的兼容面，例如 `core`/`alloc` re-export、`println!` 宏兼容、`Duration`、`Vec`、`atomic` 等基础类型和宏；再处理已有对象可能支撑的 console、time、基础 task/sync 等接口；`fs`、`net`、`display`、`backtrace` 等依赖面更宽的测试应靠后，因为即使引入 `_ex` 组件壳，缺少对应 LKM 资源对象也无法真正通过。

当引入功能组件，例如 `axtask-ex`、`axmm-ex`、`axfs-ex`、`ax-alloc-ex` 等，应按三步推进：

1. 先引入接口兼容的 `_ex` 组件壳。crate 包名带 `-ex` 后缀，公开接口尽量贴近 ArceOS 对应组件，使 `ax-std-ex`、`ax-api-ex` 和测试依赖图可以解析到 `_ex` 组件体系。此阶段可以只提供最小实现或明确的 unsupported 路径，但不要修改测试源码。
2. 在组件内部增加 shim 层，把公开接口映射到 `components/arceos_objects_ex/` 内部已有对象接口。应用、测试和上层组件不应直接依赖对象内部实现；对象接口通过组件 shim 暴露。
3. 尝试把相关对象从 `components/arceos_objects_ex/` 迁移到对应 `_ex` 功能组件内部，使资源对象和功能组件边界逐步一致。该步骤是尽力目标，不强求一次完成；若因为依赖环、初始化顺序或对象共享冲突暂时做不到，应保留公共对象组件并记录原因。

## 输出 Facade

对象级输出路径应先由 `PrintkBuffer`、`EarlyCon` 和后续正式 `Console` 承担。`Object Coding Phase` 可以先引入启动期内部 `printk`/`println-like` 前端，用于输出 `arceos_ex` 启动 banner；应用侧 `axstd::println!` 是另一个前端入口，用于 `helloworld` 等 Unikernel payload。二者不是同一个入口，但应汇聚到同一条缓冲路径；`info!/debug!/warn!/error!` 和 panic 输出等更完整前端后续也应汇聚到该路径：

```text
frontend -> PrintkBuffer.write(...) -> ring buffer -> EarlyCon/Console drain
```

`axlog` 属于 ArceOS 风格的上层日志 facade。它可以在 `Composition Phase` 中提供日志级别过滤、格式化、时间戳、CPU/task 标识、颜色和 `LogIf` 适配，但不应反向改变对象级输出语义。对象级编码阶段可以先实现启动期内部输出前端和应用侧 `axstd::println!` 到 `PrintkBuffer -> EarlyCon(SBI)` 的最小闭环；后续再把 `axlog` 接到同一 `PrintkBuffer` 路径。

## 分配器 Facade

第一轮对象级实现默认不依赖堆分配。`EntrySuccessorPhase` 当前只要求推进 `PhysicalMemory`、`MemBlock`、`SwapperVM` 等早期内存对象；它不等价于已经建立 Rust 全局堆分配器。需要 `Vec`、`String`、`Box` 或其它堆对象时，必须先确认规格中已经引入并推进了 `KernelHeap` / `Allocator` 一类对象。

`ax-alloc` 属于 ArceOS 组件封装和兼容接入层需要评估的 facade。若现有 `ax-std`、`ax-api`、`ax-feat` 或 `arceos-rust` 依赖链临时拉入 `ax-alloc`，应把它记录为 overlay workspace 的兼容成本，而不是对象级规格已经要求的核心对象。后续只有在模型显式定义 allocator 生命周期后，才把 `GlobalAlloc` 初始化纳入正式启动流程。
