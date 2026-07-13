# BootPhase 编码指引

BootPhase 是 [Kernel 系统编码](../systems/kernel.md)的首个直接子阶段。model 来源为
[`spec/model/phases/boot/phase.spec`](../../model/phases/boot/phase.spec)，实现落点为
`impl/arceos_ex/src/phases/boot/mod.rs`。本阶段拥有五个子阶段的驱动顺序；子阶段 Online 后必须
返回下述父 continuation，同级阶段之间不存在直接调用边。

## 入口 lowering

`_start` 同时是 `Kernel.Preset`、`BootPhase.Preset` 和 `EntryPreludePhase.Preset` 的最早可执行
边界。announce head 依次输出 `R -> B -> A`，分别观察三个 Preset 被接受；三者发出时状态都仍
为 Base。BSS 清零后进入 Rust 时，按 Prepare、Kernel、Boot、EntryPrelude 顺序 adoption：

- Prepare 提交自己的既有状态；
- Kernel 检查精确 Base 和准备条件；
- Boot 检查精确 Base，并在最早 Rust 边界确认 `sstatus.SIE == 0`；
- EntryPrelude 检查精确 Base 和入口依赖。

`SingleTaskContext` 是 Rust 入口前已经成立的天然启动上下文，汇编阶段无法构造 Rust guard。
因此只延迟 adoption，不省略模型上下文，也不在 Rust 重复发出任何 Started checkpoint。

## Transition 映射

| Transition | Source -> target | Start / continuation | depends / drives / ensures | 提交、checkpoint 与后续 |
| --- | --- | --- | --- | --- |
| Preset | Base -> Prepared | `_start` 接受；`preset_after_entry_prelude()` 恢复 | adoption 检查 Boot 为 Base、SIE 关闭；在 `SingleTaskContext` 驱动 `EntryPreludePhase.Preset`；恢复时要求 EntryPrelude Online | 提交 Prepared，发出 `BootPhase.Prepared`，调用 Boot.Setup |
| Setup | Prepared -> Ready | `setup()`；`setup_after_entry_successor()` 恢复 | 精确检查 Prepared；在 `SingleTaskContext` 驱动 `EntrySuccessorPhase.Preset`；恢复时要求 EntrySuccessor Online | 提交 Ready，发出 `BootPhase.Ready`，调用 Boot.Enable |
| Enable | Ready -> Online | `enable()` 和三个子 continuation | 精确检查 Ready；依次驱动 CorePrepare、MmCoreInit、SchedInit 的 Preset，每次恢复检查刚完成子阶段 Online | 最后提交 Online，发出 `BootPhase.Online`，返回 `Kernel.Preset` continuation |

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
  -> Kernel.preset_after_boot()
```

`preset_after_entry_prelude()`、`setup_after_entry_successor()`、
`enable_after_core_prepare()`、`enable_after_mm_core_init()` 和 `enable_after_sched_init()` 都属于
Boot transition；叶子阶段只在 Online 后返回对应 continuation。正常路径不得使用泛化
`handoff()`，也不得让叶子阶段直接启动 sibling。

## 状态与 checkpoint

`BOOT_PHASE_STATE` 持久记录 Base、Prepared、Ready、Online。每个 transition start 检查精确
source state；每次提交后读回 target 并检查对应子阶段完成事实，然后发出同名 checkpoint。
`is_online()` 只在 Boot 和五个直接子阶段都精确 Online 时返回 true。

| 观察点 | owner state | 落点 |
| --- | --- | --- |
| `BootPhase.Started` | Base | `_start` 的 `B`，Rust 不重复 |
| `BootPhase.Prepared` | Prepared | `preset_after_entry_prelude()` |
| `BootPhase.Ready` | Ready | `setup_after_entry_successor()` |
| `BootPhase.Online` | Online | `enable_after_sched_init()` |

子阶段专题为：

- [boot/entry-prelude.md](boot/entry-prelude.md)
- [boot/entry-successor.md](boot/entry-successor.md)
- [boot/core-prepare.md](boot/core-prepare.md)
- [boot/mm-core-init.md](boot/mm-core-init.md)
- [boot/sched-init.md](boot/sched-init.md)
