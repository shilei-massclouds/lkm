# arceos_ex coding index

本文是 `arceos_ex` 的 coding 阅读索引。coding 层以自然语言 `.md` 为权威来源；对象、状态、
迁移、依赖和阶段顺序来自 `spec/model`，本目录只说明它们如何映射到当前 Rust 实现。

长篇实现说明、命令和阶段性取舍位于
[`arceos_ex-implementation.md`](arceos_ex-implementation.md)。

## 阅读顺序

1. [`README.md`](README.md) 和 [`mapping.md`](mapping.md)：coding 职责及通用映射规则。
2. [`projects/kernel.md`](projects/kernel.md)：KernelProject 映射。
3. [`systems/kernel.md`](systems/kernel.md)：Kernel 生命周期和顶层阶段树映射。
4. 编排阶段：Boot -> Interrupt -> UpMultitask -> SmpRuntime -> Payload。
5. 对应叶子阶段和普通对象主题。
6. [`arceos_ex-implementation.md`](arceos_ex-implementation.md)：当前实现证据和工程入口。

## System 与阶段索引

| Scope | Coding 映射 |
| --- | --- |
| Kernel system/startup | [`systems/kernel.md`](systems/kernel.md) |
| Boot orchestration | [`phases/boot.md`](phases/boot.md) |
| Boot `entry-prelude` | [`phases/boot/entry-prelude.md`](phases/boot/entry-prelude.md) |
| Boot `entry-successor` | [`phases/boot/entry-successor.md`](phases/boot/entry-successor.md) |
| Boot `core-prepare` | [`phases/boot/core-prepare.md`](phases/boot/core-prepare.md) |
| Boot `mm-core-init` | [`phases/boot/mm-core-init.md`](phases/boot/mm-core-init.md) |
| Boot `sched-init` | [`phases/boot/sched-init.md`](phases/boot/sched-init.md) |
| Interrupt orchestration | [`phases/interrupt.md`](phases/interrupt.md) |
| Interrupt `irq-time-init` | [`phases/interrupt/irq-time-init.md`](phases/interrupt/irq-time-init.md) |
| Interrupt `local-irq-enable` | [`phases/interrupt/local-irq-enable.md`](phases/interrupt/local-irq-enable.md) |
| Interrupt `irq-open-prepare` | [`phases/interrupt/irq-open-prepare.md`](phases/interrupt/irq-open-prepare.md) |
| Interrupt `process-prepare` | [`phases/interrupt/process-prepare.md`](phases/interrupt/process-prepare.md) |
| UpMultitask orchestration | [`phases/up-multitask.md`](phases/up-multitask.md) |
| UpMultitask `rest-init` | [`phases/up-multitask/rest-init.md`](phases/up-multitask/rest-init.md) |
| SMP runtime orchestration | [`phases/smp-runtime.md`](phases/smp-runtime.md) |
| SMP runtime `pre-smp-init` | [`phases/smp-runtime/pre-smp-init.md`](phases/smp-runtime/pre-smp-init.md) |
| SMP runtime `smp-bringup` | [`phases/smp-runtime/smp-bringup.md`](phases/smp-runtime/smp-bringup.md) |
| SMP runtime `runtime-core` | [`phases/smp-runtime/runtime-core.md`](phases/smp-runtime/runtime-core.md) |
| SMP runtime `initcall` | [`phases/smp-runtime/initcall.md`](phases/smp-runtime/initcall.md) |
| SMP runtime `rootfs` | [`phases/smp-runtime/rootfs.md`](phases/smp-runtime/rootfs.md) |
| SMP runtime `finalize` | [`phases/smp-runtime/finalize.md`](phases/smp-runtime/finalize.md) |
| Payload | [`phases/payload.md`](phases/payload.md) |

## Object 索引

| Scope | Coding 映射 |
| --- | --- |
| Device tree MUST | [`objects/device-tree-must.md`](objects/device-tree-must.md) |
| Device tree SHOULD | [`objects/device-tree-should.md`](objects/device-tree-should.md) |
| Effective context | [`objects/effective-context.md`](objects/effective-context.md) |
| Completion | [`objects/completion.md`](objects/completion.md) |
| Block I/O | [`objects/block-io.md`](objects/block-io.md) |

现存同主题 `.spec` 是待迁移遗留文件，不是阅读入口，也不得覆盖上述 `.md`。迁移状态见
[`../../docs/roadmap/phase-paradigm-audit.md`](../../docs/roadmap/phase-paradigm-audit.md)。
