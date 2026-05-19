# ArceOS Coding 规格

本文记录以 ArceOS 作为参考实现时的补充约束。

## 适用范围

本文件说明在实现目标内核时如何参考 ArceOS。参考范围包括工程组织、RISC-V64 启动路径、地址空间、SBI、FDT、early console、allocator 和模块拆分经验。

当前目标不是复制 ArceOS，而是在 `spec/model` 的对象和阶段边界下吸收 ArceOS 中已经验证过的实现经验。

`arceos_ex` 的演化约束分为两个层面：

- 代码实现层面：ArceOS 代码只是参考。能满足规格且改造成本合理的代码可以复用、复制或改写；不适合的代码应新写，不能无脑复用。
- 框架与接口层面：应尽量保持 ArceOS 的组件化形态。组件仍以 crate 为边界，crate 内部仍以 Rust module 组织；组件构成、组件公开接口和 module 公开接口应尽量接近 ArceOS。接口兼容是优先目标，但不是百分百硬约束；只有规格要求、实现边界或演化路径不允许时，才改变接口，并记录原因。

## 参考原则

- 模型对象边界优先于 ArceOS 现有模块边界。
- 可复用思想、接口形状和成熟实现路径，但不得因为 ArceOS 现有代码结构而改变模型状态迁移。
- 若 ArceOS 中某个实现步骤覆盖多个模型事件，目标内核应按模型事件拆分或显式记录合并理由。
- 若模型中一个对象需要参考 ArceOS 多处代码，应在实现任务中列出映射关系。
- Linux 参考主要用于理解机制和验证边界。若规格中已有 Linux 机制提示，优先使用规格；若规格缺少实现细节，可参考本地 Linux 源码 `~/gitStudy/linux-6.12.37/`，但必须转化为 ArceOS 组件和接口形式，不能直接照搬 Linux 结构。
- `os/arceos` 和已有 `components` 实现视为只读参考；`arceos_ex` 通过新增目录或新增 `_ex` 组件实现。
- 新增 crate 名称优先使用与 ArceOS 对应 crate 相同的语义名并加 `_ex` 后缀，避免 workspace 包名冲突。

## 初步参考方向

- 启动入口：参考 ArceOS 的 RISC-V64 entry、链接布局和 early boot 组织方式。
- 地址空间：参考 ArceOS 的页表抽象和地址类型，但按 `TrampolineVm`、`EarlyVm`、`SwapperVm` 区分阶段边界。
- SBI：参考 ArceOS 的 SBI 调用封装，形成规格中的 `SBI` 能力视图。
- FDT：参考 ArceOS 的设备树解析入口，但保持 `RawDtb` 与 `EarlyDtb` 的阶段边界。
- 输出：参考 ArceOS early console 或 logging 机制，落实 `PrintkBuffer` 与 `EarlyCon` 的区别。
- 内存管理：参考 ArceOS 早期内存区段和 allocator 初始化经验，落实 `MemBlock` 的候选区段、保留区段和 enable 边界。

## arceos_ex 第一轮形态

第一轮以 `helloworld` Unikernel 为目标，但该最小应用仍必须支撑 `EntryPreludePhase` 和 `EntrySuccessorPhase` 两个子阶段。实现边界达到 `EntrySuccessorPhase.Ready` 后，才能认为第一轮启动模型闭环完成。

建议的组织原则：

- `os/arceos_ex` 镜像 ArceOS 的 OS 侧目录习惯，例如保留 `modules/`、`examples/` 等组织方式。
- 内核核心组件可新增为 `ax-hal-ex`、`ax-runtime-ex` 等 crate，并在目录上尽量贴近 ArceOS 原有模块层次。
- ArceOS 现有 `examples/` 和 `test-suit/arceos/` 下的 Unikernel 应用应尽量复用，不默认复制到 `os/arceos_ex/examples`。当前仓库中 `helloworld` 已存在于 `os/arceos/examples/helloworld` 和 `test-suit/arceos/std/qemu-smp1/helloworld`。
- `arceos_ex` 第一轮通过 `ax-std` 正式接入现有 Unikernel 应用，而不是让应用直接依赖 `ax-runtime-ex`。
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
3. 若只是实现组织差异，允许代码采用不同结构，但必须保留模型事件可追踪性。

## 待补充

- `tgoskits` 中 ArceOS 代码的实际路径。
- 目标内核与 ArceOS 参考模块的映射表。
- 可直接复用、需要改写、禁止复用的代码分类。
- `ax-std` / `ax-api` / `ax-feat` 是否需要 `_ex` facade 的最终判断。
