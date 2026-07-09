# arceos_ex coding index

本文是 `arceos_ex` coding 规格的兼容入口和阅读索引。正式硬约束入口仍是 [`arceos_ex.spec`](arceos_ex.spec)，该文件只 include 拆分后的 system、phase 和 object formal 文件。

当前目标仍是让 `arceos_ex` 作为规格驱动的对象级内核原型运行，并完成 Boot、Interrupt、UpMultitask、SmpRuntime 到 Payload 的启动链路。长篇对象级实现说明、命令和工程边界保留在 [`arceos_ex-implementation.md`](arceos_ex-implementation.md)。

## 阅读顺序

1. [`main.spec`](main.spec)：coding 目录正式入口。
2. [`arceos_ex.spec`](arceos_ex.spec)：`arceos_ex` 兼容入口，按拆分后的 formal 文件 include。
3. [`systems/kernel.spec`](systems/kernel.spec) / [`systems/kernel.md`](systems/kernel.md)：Kernel system 与 startup/payload handoff 约束。
4. Phase topic：按 Boot -> Interrupt -> UpMultitask -> SmpRuntime 阅读对应 `phases/` 文件。
5. Object topic：按需要阅读 `objects/` 下的 reusable object/subsystem 约束。
6. [`arceos_ex-implementation.md`](arceos_ex-implementation.md)：当前实现说明、命令、阶段性取舍和专题方案。

## Formal topic index

| Scope | Formal | Notes |
| --- | --- | --- |
| Kernel system/startup | [`systems/kernel.spec`](systems/kernel.spec) | [`systems/kernel.md`](systems/kernel.md) |
| Boot phase: `entry-prelude` | [`phases/boot/entry-prelude.spec`](phases/boot/entry-prelude.spec) | [`phases/boot/entry-prelude.md`](phases/boot/entry-prelude.md) |
| Boot phase: `entry-successor` | [`phases/boot/entry-successor.spec`](phases/boot/entry-successor.spec) | [`phases/boot/entry-successor.md`](phases/boot/entry-successor.md) |
| Boot phase: `core-prepare` | [`phases/boot/core-prepare.spec`](phases/boot/core-prepare.spec) | [`phases/boot/core-prepare.md`](phases/boot/core-prepare.md) |
| Boot phase: `mm-core-init` | [`phases/boot/mm-core-init.spec`](phases/boot/mm-core-init.spec) | [`phases/boot/mm-core-init.md`](phases/boot/mm-core-init.md) |
| Interrupt phase: `irq-time-init` | [`phases/interrupt/irq-time-init.spec`](phases/interrupt/irq-time-init.spec) | [`phases/interrupt/irq-time-init.md`](phases/interrupt/irq-time-init.md) |
| Interrupt phase: `local-irq-enable` | [`phases/interrupt/local-irq-enable.spec`](phases/interrupt/local-irq-enable.spec) | [`phases/interrupt/local-irq-enable.md`](phases/interrupt/local-irq-enable.md) |
| Interrupt phase: `irq-open-prepare` | [`phases/interrupt/irq-open-prepare.spec`](phases/interrupt/irq-open-prepare.spec) | [`phases/interrupt/irq-open-prepare.md`](phases/interrupt/irq-open-prepare.md) |
| Interrupt phase: `process-prepare` | [`phases/interrupt/process-prepare.spec`](phases/interrupt/process-prepare.spec) | [`phases/interrupt/process-prepare.md`](phases/interrupt/process-prepare.md) |
| Up-multitask phase: `rest-init` | [`phases/up-multitask/rest-init.spec`](phases/up-multitask/rest-init.spec) | [`phases/up-multitask/rest-init.md`](phases/up-multitask/rest-init.md) |
| SMP runtime phase: `pre-smp-init` | [`phases/smp-runtime/pre-smp-init.spec`](phases/smp-runtime/pre-smp-init.spec) | [`phases/smp-runtime/pre-smp-init.md`](phases/smp-runtime/pre-smp-init.md) |
| SMP runtime phase: `smp-bringup` | [`phases/smp-runtime/smp-bringup.spec`](phases/smp-runtime/smp-bringup.spec) | [`phases/smp-runtime/smp-bringup.md`](phases/smp-runtime/smp-bringup.md) |
| SMP runtime phase: `runtime-core` | [`phases/smp-runtime/runtime-core.spec`](phases/smp-runtime/runtime-core.spec) | [`phases/smp-runtime/runtime-core.md`](phases/smp-runtime/runtime-core.md) |
| SMP runtime phase: `initcall` | [`phases/smp-runtime/initcall.spec`](phases/smp-runtime/initcall.spec) | [`phases/smp-runtime/initcall.md`](phases/smp-runtime/initcall.md) |
| SMP runtime phase: `rootfs` | [`phases/smp-runtime/rootfs.spec`](phases/smp-runtime/rootfs.spec) | [`phases/smp-runtime/rootfs.md`](phases/smp-runtime/rootfs.md) |
| SMP runtime phase: `finalize` | [`phases/smp-runtime/finalize.spec`](phases/smp-runtime/finalize.spec) | [`phases/smp-runtime/finalize.md`](phases/smp-runtime/finalize.md) |
| Object/subsystem: `device-tree MUST` | [`objects/device-tree-must.spec`](objects/device-tree-must.spec) | [`objects/device-tree-must.md`](objects/device-tree-must.md) |
| Object/subsystem: `device-tree SHOULD` | [`objects/device-tree-should.spec`](objects/device-tree-should.spec) | [`objects/device-tree-should.md`](objects/device-tree-should.md) |
| Object/subsystem: `effective-context` | [`objects/effective-context.spec`](objects/effective-context.spec) | [`objects/effective-context.md`](objects/effective-context.md) |
| Object/subsystem: `completion` | [`objects/completion.spec`](objects/completion.spec) | [`objects/completion.md`](objects/completion.md) |
| Object/subsystem: `block-io` | [`objects/block-io.spec`](objects/block-io.spec) | [`objects/block-io.md`](objects/block-io.md) |

<!-- formal-predicate-notes:spec/coding/arceos_ex.spec START -->

## Formal predicate notes

`arceos_ex.spec` 现在是 include-only 兼容入口；predicate notes 不再集中维护在本文件。请按上表进入对应 topic `.md`，或从 [`arceos_ex.spec`](arceos_ex.spec) 的 include 顺序跳转。

<!-- formal-predicate-notes:spec/coding/arceos_ex.spec END -->
