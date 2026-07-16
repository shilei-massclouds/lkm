# BootPhase 编码指引

BootPhase 是 [Kernel 系统编码](../systems/kernel.md)的第二个直接子阶段。model 来源为
[`spec/model/phases/boot/phase.spec`](../../model/phases/boot/phase.spec)，实现落点为
`impl/arceos_ex/src/phases/boot/mod.rs`。本阶段拥有四个子阶段的驱动顺序；子阶段 Online 后必须
返回下述父 continuation，同级阶段之间不存在直接调用边。

## 入口 lowering

`_start` 只接受 `Kernel.Preset` 和 `EntryPreludePhase.Preset`；announce head 依次输出
`R -> A`。BSS 清零后进入 Rust 时，按 Prepare、Kernel、EntryPrelude 顺序 adoption：

- Prepare 提交自己的既有状态；
- Kernel 检查精确 Base 和准备条件；
- EntryPrelude 检查精确 Base 和入口依赖。

EntryPrelude Online 并提交 Kernel.Prepared 后，Kernel.Setup 调用普通 Rust `boot::preset()`。
该入口精确检查 Boot 为 Base、Kernel 保持 Prepared、EntryPrelude Online 和 `sstatus.SIE == 0`，
然后发出 `BootPhase.Started`。旧早期字节 `B` 仅为日志解析兼容保留，head 不再发出。

## Transition 映射

| Transition | Source -> target | Start / continuation | depends / drives / ensures | 提交、checkpoint 与后续 |
| --- | --- | --- | --- | --- |
| Preset | Base -> Prepared | 普通 Rust `preset()` | 精确检查 Base、Kernel.Prepared、EntryPrelude Online 和 SIE 关闭；无 child drives | 发出 `BootPhase.Started`，提交 Prepared，发出 `BootPhase.Prepared`，调用 Boot.Setup |
| Setup | Prepared -> Ready | `setup()`；`setup_after_entry_successor()` 恢复 | 精确检查 Prepared；在 `SingleTaskContext` 驱动 `EntrySuccessorPhase.Preset`；恢复时要求 EntrySuccessor Online | 提交 Ready，发出 `BootPhase.Ready`，调用 Boot.Enable |
| Enable | Ready -> Online | `enable()` 和三个子 continuation | 精确检查 Ready；依次驱动 CorePrepare、MmCoreInit、SchedInit 的 Preset，每次恢复检查刚完成子阶段 Online | 最后提交 Online，发出 `BootPhase.Online`，返回 `Kernel.Setup` 的 Boot completion continuation |

## Enable continuation

```text
Boot.Enable
  -> CorePrepare.Preset
  -> enable_after_core_prepare()
  -> MmCoreInit.Preset
  -> enable_after_mm_core_init()
  -> SchedInit.Preset
  -> enable_after_sched_init()
  -> Boot.Online
  -> Kernel.setup_after_boot()
```

`setup_after_entry_successor()`、`enable_after_core_prepare()`、`enable_after_mm_core_init()` 和
`enable_after_sched_init()` 都属于
Boot transition；叶子阶段只在 Online 后返回对应 continuation。正常路径不得使用泛化
`handoff()`，也不得让叶子阶段直接启动 sibling。

## 状态与 checkpoint

`BOOT_PHASE_STATE` 持久记录 Base、Prepared、Ready、Online。每个 transition start 检查精确
source state；每次提交后读回 target 并检查对应子阶段完成事实，然后发出同名 checkpoint。
`is_online()` 只在 Boot 和四个直接子阶段都精确 Online 时返回 true；它不重复检查 Kernel
直接拥有的 EntryPreludePhase。

| 观察点 | owner state | 落点 |
| --- | --- | --- |
| `BootPhase.Started` | Base | 普通 Rust `preset()` 接受后、提交前 |
| `BootPhase.Prepared` | Prepared | `preset()` |
| `BootPhase.Ready` | Ready | `setup_after_entry_successor()` |
| `BootPhase.Online` | Online | `enable_after_sched_init()` |

子阶段专题为：

- [boot/entry-successor.md](boot/entry-successor.md)
- [boot/core-prepare.md](boot/core-prepare.md)
- [boot/mm-core-init.md](boot/mm-core-init.md)
- [boot/sched-init.md](boot/sched-init.md)

EntryPrelude 的源码与 coding 文件暂保留在现有 `boot/` 物理路径，但阶段 parent 与 continuation
所有权以 Kernel 为准。
