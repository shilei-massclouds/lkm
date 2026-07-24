# ArceOS Coding 规格

本文记录以 ArceOS 作为参考实现时的补充约束。

## 适用范围

本文件说明在对象级编码阶段如何参考 ArceOS。参考范围包括 RISC-V64 启动路径、地址空间、SBI、FDT、early console、早期内存区段和模块拆分经验中与模型对象实现直接相关的部分。

当前目标不是复制 ArceOS，而是在 `spec/model` 的对象和阶段边界下吸收 ArceOS 中已经验证过的实现经验。

`arceos_ex` 的演化约束分为两个阶段：

- `Object Coding Phase`：先完成对象级编码实现。ArceOS 代码只是参考；能满足规格且改造成本合理的代码可以复用、复制或改写；不适合的代码应新写，不能无脑复用。
- `Composition Phase`：再进行组合封装。组件构成、crate/module 公开接口、adapter、overlay workspace 和与 ArceOS 组件体系兼容相关的规则，记录在 `../compose/README.md`。

## 参考原则

- 模型对象边界优先于 ArceOS 现有模块边界。
- 可复用思想、接口形状和成熟实现路径，但不得因为 ArceOS 现有代码结构而改变模型状态迁移。
- 若 ArceOS 中某个实现步骤覆盖多个模型 transition，目标内核应按模型 transition拆分或显式记录合并理由。
- 若模型中一个对象需要参考 ArceOS 多处代码，应在实现任务中列出映射关系。
- Linux 参考主要用于理解机制、阶段语义和验证边界。若规格中已有 Linux 机制提示，优先使用规格；若规格缺少实现细节，可参考本地 Linux 源码 `../linux-6.12/`。
- 具体编码实现方式优先参考 Rust 内核实现路径，尤其是本项目已有的 `tgoskits/os/arceos_ex` 实验代码和 ArceOS 当前 RISC-V64 启动代码；需要从 Linux 获得的机制必须转化为适合 Rust、ArceOS 风格组件和接口的实现形式，不能直接照搬 Linux 的源码结构、宏体系或汇编组织。
- `os/arceos` 和已有 `components` 实现视为只读参考；`arceos_ex` 通过新增目录或新增 `_ex` 组件实现。
- 新增 crate 名称、公开接口兼容和 workspace 接入属于 `Composition Phase` 决策；对象级编码阶段只在需要承载代码时采用临时落点，不把临时落点视为最终组件边界。

## 初步参考方向

- 启动入口：参考 ArceOS 的 RISC-V64 entry、链接布局和 early boot 组织方式。
- 地址空间：参考 ArceOS 的页表抽象和地址类型，但按 `TrampolineVm`、`EarlyVm`、`SwapperVm` 区分阶段边界。
- SBI：参考 ArceOS 的 SBI 调用封装，形成规格中的 `SBI` 能力视图。
- FDT：参考 ArceOS 的设备树解析入口，但保持 `RawDtb` 与 `EarlyDtb` 的阶段边界。
- 输出：参考 ArceOS early console 或 logging 机制，落实 `PrintkBuffer` 与 `EarlyCon` 的对象级区别。对象级编码阶段先建立启动期内部 `printk`/`println-like` 前端到 `PrintkBuffer.write(...) -> ring buffer -> EarlyCon.drain(...)` 的路径；应用侧 `axstd::println!` 是另一个前端入口，也应汇聚到同一 `PrintkBuffer` 后端路径。两类 `println!` 入口不应混同；`axlog` 这类上层日志 facade 属于组合封装阶段，不作为当前模型要求的核心对象。
- 内存管理：参考 ArceOS 早期内存区段经验，落实 `MemBlock` 的候选区段、保留区段和 enable 边界。第一轮对象级实现默认采用 no-alloc 路径；除非模型规格显式引入 `KernelHeap` / `Allocator` 一类对象并定义其生命周期，否则不得把全局分配器初始化作为当前入口后继期的核心步骤。

## arceos_ex 第一轮形态

第一轮以 `helloworld` Unikernel 为目标，但该最小应用仍必须支撑 BootInitFlow、KernelInitFlow 的
直接叶阶段和最终 selected payload commit。进入已绑定的 no-return payload 后，当前启动模型才闭环。

在 `arceos_ex` 中，`ax-hal-ex` 与 `ax-runtime-ex` 的引导责任应按阶段边界划分。`ax-hal-ex` 负责 `_start` 到
BootInitFlow.Preset 完成前的最低层入口前导路径；`BootInitFlow.Prepared` 之后由 `ax-runtime-ex` 接管。
`EntrySuccessorPhase` 是 `ax-runtime-ex` 引导过程的第一部分，之后逐步增加的内核初始化过程也属于
`ax-runtime-ex` 主引导链：BootInitFlow 直接完成 boot/interrupt/rest-init，首次 PID 1 dispatch 后
KernelInitFlow 直接完成 SMP/runtime 与 payload prepare，再由 commit action 移交 selected payload。

这里的 Unikernel app 是内核形态的引领入口。`helloworld` 是默认最小 payload，许多测试也可以作为 payload 运行。
未来宏内核形态通过 UserBoot commit action 完成用户 Flow replacement。无论 payload 是内核态服务
循环、测试后停机还是用户态入口，其最终 entry 都不返回启动编排链。

建议的组织原则：

- 当前第一轮实现可以为了运行闭环临时交织对象级代码和 ArceOS 接入代码，但文档解释和后续整理应按 `Object Coding Phase` 与 `Composition Phase` 分开处理。
- `os/arceos_ex` 镜像 ArceOS 的 OS 侧目录习惯，例如保留 `modules/`、`examples/` 等组织方式。
- 内核核心组件可新增为 `ax-hal-ex`、`ax-runtime-ex` 等 crate，并在目录上尽量贴近 ArceOS 原有模块层次。
- ArceOS 现有 `examples/` 和 `test-suit/arceos/` 下的 Unikernel 应用应尽量复用，不默认复制到 `os/arceos_ex/examples`。当前仓库中 `helloworld` 已存在于 `os/arceos/examples/helloworld` 和 `test-suit/arceos/std/qemu-smp1/helloworld`。
- `arceos_ex` 第一轮通过 `ax-std` 正式接入现有 Unikernel 应用，而不是让应用直接依赖 `ax-runtime-ex`。
- 第一轮只参考并接入 ArceOS 的 `ax-std` Unikernel 路径；不参考、不适配也不调试 ArceOS C API 和 Rust std/Hermit API 路径。应用输出应通过 `ax-std` 的 `println!` 进入内核统一输出路径，而不是通过 POSIX fd 或 Hermit syscall 层。
- `ulib/axstd`、`api/ax-api`、`api/ax-feat` 这类应用接口和特性接口第一轮不主动复制；目标是让它们的公开接口保持不变，并由构建工具把底层内核实现切换到 `_ex` 组件。
- 如果现有 `ax-std` / `ax-api` / `ax-feat` 的依赖链固定指向原 `ax-hal`、`ax-runtime`，导致 `arceos_ex` 无法接入 `_ex` runtime，则先由 `xtask` 生成 overlay workspace 并在其中替换 Cargo 依赖映射；只有在该方式不足时，才引入同接口的 `_ex` facade。
- 构建和运行应直接接入 tgoskits 现有 `cargo xtask` 工具体系。第一轮新增 `cargo xtask arceos-ex ...` 子命令，复用 ArceOS 的测试选择机制运行 `helloworld`；长期由 `xtask` 管理不同内核 profile 的 overlay workspace 或正式多 workspace manifest。

第一轮建议新增的核心实现 crate 包括：

- `ax-hal-ex`
- `ax-runtime-ex`
- `ax-plat-riscv64-generic`
- 必要时增加最小支撑 crate，但不得复制 `ax-std`、`ax-api`、`ax-feat` 的公开接口层，除非 `xtask` 依赖映射方案无法满足接入。

## 与模型不一致时的处理

当 ArceOS 参考代码与当前模型出现不一致时，按以下顺序处理：

1. 先判断模型是否已有明确语义。如果有，按模型实现。
2. 若 ArceOS 暴露出模型缺口，先记录缺口，再决定补模型还是写 coding 覆盖。
3. 若只是实现组织差异，允许代码采用不同结构，但必须保留模型 transition可追踪性。

## 待补充

- `tgoskits` 中 ArceOS 代码的实际路径。
- 目标内核与 ArceOS 参考模块的映射表。
- 可直接复用、需要改写、禁止复用的代码分类。
- `ax-std` / `ax-api` / `ax-feat` 是否需要 `_ex` facade 的最终判断。
