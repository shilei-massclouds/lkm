# arceos_ex coding index

本文是 `arceos_ex` 的 coding 阅读索引。coding 层以自然语言 `.md` 为权威来源；对象、状态、
迁移、依赖和阶段顺序来自 `spec/model`，本目录只说明它们如何映射到当前 Rust 实现。

长篇实现说明、命令和阶段性取舍位于
[`arceos_ex-implementation.md`](arceos_ex-implementation.md)。

## 阅读顺序

1. [`README.md`](README.md)、[`mapping.md`](mapping.md) 和
   [`phase-paradigm.md`](phase-paradigm.md)：coding 职责、通用规则和阶段专用映射。
2. [`projects/kernel.md`](projects/kernel.md)：KernelProject 映射。
3. [`systems/kernel.md`](systems/kernel.md)：Kernel 生命周期和顶层阶段树映射。
4. 编排阶段：BootInitFlow 直接叶阶段 -> 首次 PID 1 dispatch -> KernelInitFlow 直接叶阶段 ->
   payload commit。
5. 对应叶子阶段和普通对象主题。
6. [`arceos_ex-implementation.md`](arceos_ex-implementation.md)：当前实现证据和工程入口。

## System 与阶段索引

| Scope | Coding 映射 |
| --- | --- |
| Kernel system/startup | [`systems/kernel.md`](systems/kernel.md) |
| Boot leaf namespace | [`phases/boot.md`](phases/boot.md) |
| Boot `entry-prelude` | [`phases/boot/entry-prelude.md`](phases/boot/entry-prelude.md) |
| Boot `entry-successor` | [`phases/boot/entry-successor.md`](phases/boot/entry-successor.md) |
| Boot `core-prepare` | [`phases/boot/core-prepare.md`](phases/boot/core-prepare.md) |
| Boot `mm-core-init` | [`phases/boot/mm-core-init.md`](phases/boot/mm-core-init.md) |
| Boot `sched-init` | [`phases/boot/sched-init.md`](phases/boot/sched-init.md) |
| Interrupt leaf namespace | [`phases/interrupt.md`](phases/interrupt.md) |
| Interrupt `irq-time-init` | [`phases/interrupt/irq-time-init.md`](phases/interrupt/irq-time-init.md) |
| Interrupt `local-irq-enable` | [`phases/interrupt/local-irq-enable.md`](phases/interrupt/local-irq-enable.md) |
| Interrupt `irq-open-prepare` | [`phases/interrupt/irq-open-prepare.md`](phases/interrupt/irq-open-prepare.md) |
| Interrupt `process-prepare` | [`phases/interrupt/process-prepare.md`](phases/interrupt/process-prepare.md) |
| BootInitFlow orchestration | [`phases/boot-init.md`](phases/boot-init.md) |
| BootInitFlow `rest-init` | [`phases/boot-init/rest-init.md`](phases/boot-init/rest-init.md) |
| KernelInitFlow execution phases | [`phases/smp-runtime.md`](phases/smp-runtime.md) |
| SMP runtime `pre-smp-init` | [`phases/smp-runtime/pre-smp-init.md`](phases/smp-runtime/pre-smp-init.md) |
| SMP runtime `smp-bringup` | [`phases/smp-runtime/smp-bringup.md`](phases/smp-runtime/smp-bringup.md) |
| SMP runtime `runtime-core` | [`phases/smp-runtime/runtime-core.md`](phases/smp-runtime/runtime-core.md) |
| SMP runtime `initcall` | [`phases/smp-runtime/initcall.md`](phases/smp-runtime/initcall.md) |
| SMP runtime `rootfs` | [`phases/smp-runtime/rootfs.md`](phases/smp-runtime/rootfs.md) |
| SMP runtime `finalize` | [`phases/smp-runtime/finalize.md`](phases/smp-runtime/finalize.md) |
| Payload prepare/commit | [`phases/payload.md`](phases/payload.md) |

## Object 索引

| Scope | Coding 映射 |
| --- | --- |
| Model object 文件完整覆盖 | [`objects/README.md`](objects/README.md) |
| Device tree | [`objects/device-tree.md`](objects/device-tree.md) |
| Effective context | [`objects/effective-context.md`](objects/effective-context.md) |
| Completion | [`objects/completion.md`](objects/completion.md) |
| Bio / BufferHead | [`objects/bio.md`](objects/bio.md) |
| Block device | [`objects/block-device.md`](objects/block-device.md) |
| Virtio core/MMIO/ring/RNG | [`objects/virtio.md`](objects/virtio.md) |
| Virtio block | [`objects/virtio-blk.md`](objects/virtio-blk.md) |
| Ext2 | [`objects/ext2.md`](objects/ext2.md) |
| VFS / devfs | [`objects/vfs.md`](objects/vfs.md) |
| User boot / files / syscall | [`objects/user-boot.md`](objects/user-boot.md) |
| Rootfs image construction | [`projects/rootfs-image.md`](projects/rootfs-image.md) |
| Rootfs/user acceptance testing | [`../testing/rootfs.md`](../testing/rootfs.md) |

上述 `.md` 是唯一 coding 权威来源；稳定 legacy rule ID 已归并到对应主题文件，coding `.spec`
已退场且不得重新引入。审计结果见
[`../../docs/roadmap/phase-paradigm-audit.md`](../../docs/roadmap/phase-paradigm-audit.md)。
