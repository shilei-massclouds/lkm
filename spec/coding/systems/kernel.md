# Kernel 系统编码指引

`systems/kernel` 是 Kernel 生命周期的编排层。它协调各子阶段的有序 Preset/Setup/Enable，但不直接实现子阶段内部逻辑。

## 生命周期边界

Kernel 状态机：`Base --[Preset]--> Prepared --[Setup]--> Ready --[Enable]--> Online`。

`impl/arceos_ex/src/systems/kernel.rs` 中的三个公开入口点：

| 入口 | 不变量检查 | 下一动作 | 状态迁移 |
|---|---|---|---|
| `preset_after_boot()` | Prepare.Online, Boot.Ready | 驱动 InterruptPhase | Base → Prepared |
| `setup_after_interrupt()` | Prepare.Online, Boot.Ready, Interrupt.Ready | 驱动 UpMultitaskPhase | Prepared → Ready |
| `enable_after_smp_runtime()` | Prepare.Online, Boot.Ready, Interrupt.Ready, UpMultitask.Ready, SmpRuntime.Ready | 驱动 PayloadPhase | Ready → Online |

`enable_after_smp_runtime()` 启动 PayloadPhase 后，`mark_online()` 额外要求 PayloadPhase.Online。

## 范围边界

`systems/kernel` 只负责生命周期迁移的编排。子层关注点（ELF 加载、系统调用分发、地址空间设置、信号运行时、TTY、文件系统、凭据等）由各自的阶段或对象模块负责。这些主题的编码规则位于对应的阶段/对象编码文件中，不在此处。

## 形式谓词

本层不定义形式谓词。以上生命周期和范围规则是对 Kernel 系统的完整编码指引。
