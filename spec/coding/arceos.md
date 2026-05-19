# ArceOS Coding 规格

本文记录以 ArceOS 作为参考实现时的补充约束。

## 适用范围

本文件说明在实现目标内核时如何参考 ArceOS。参考范围包括工程组织、RISC-V64 启动路径、地址空间、SBI、FDT、early console、allocator 和模块拆分经验。

当前目标不是复制 ArceOS，而是在 `spec/model` 的对象和阶段边界下吸收 ArceOS 中已经验证过的实现经验。

## 参考原则

- 模型对象边界优先于 ArceOS 现有模块边界。
- 可复用思想、接口形状和成熟实现路径，但不得因为 ArceOS 现有代码结构而改变模型状态迁移。
- 若 ArceOS 中某个实现步骤覆盖多个模型事件，目标内核应按模型事件拆分或显式记录合并理由。
- 若模型中一个对象需要参考 ArceOS 多处代码，应在实现任务中列出映射关系。

## 初步参考方向

- 启动入口：参考 ArceOS 的 RISC-V64 entry、链接布局和 early boot 组织方式。
- 地址空间：参考 ArceOS 的页表抽象和地址类型，但按 `TrampolineVm`、`EarlyVm`、`SwapperVm` 区分阶段边界。
- SBI：参考 ArceOS 的 SBI 调用封装，形成规格中的 `SBI` 能力视图。
- FDT：参考 ArceOS 的设备树解析入口，但保持 `RawDtb` 与 `EarlyDtb` 的阶段边界。
- 输出：参考 ArceOS early console 或 logging 机制，落实 `PrintkBuffer` 与 `EarlyCon` 的区别。
- 内存管理：参考 ArceOS 早期内存区段和 allocator 初始化经验，落实 `MemBlock` 的候选区段、保留区段和 enable 边界。

## 与模型不一致时的处理

当 ArceOS 参考代码与当前模型出现不一致时，按以下顺序处理：

1. 先判断模型是否已有明确语义。如果有，按模型实现。
2. 若 ArceOS 暴露出模型缺口，先记录缺口，再决定补模型还是写 coding 覆盖。
3. 若只是实现组织差异，允许代码采用不同结构，但必须保留模型事件可追踪性。

## 待补充

- `tgoskits` 中 ArceOS 代码的实际路径。
- 目标内核与 ArceOS 参考模块的映射表。
- 可直接复用、需要改写、禁止复用的代码分类。
