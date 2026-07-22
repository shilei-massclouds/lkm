# Boot 叶阶段 namespace 编码指引

`phases/boot/` 仅组织 `EntryPreludePhase`、`EntrySuccessorPhase`、`CorePreparePhase`、
`MmCoreInitPhase` 与 `SchedInitPhase` 的实现。不存在 `BootPhase` 对象、状态机、checkpoint 或父
continuation。

- `EntryPreludePhase` 由 `BootInitFlow.Preset` 直接驱动并返回其 Preset continuation。
- 其余四个阶段由 `BootInitFlow.Setup` 按顺序直接驱动，每个阶段 Online 后只返回
  `BootInitFlow.Setup` 的下一 continuation。
- namespace module 可以提供子 module 声明和纯查询聚合，但不得保存 wrapper lifecycle 或代发叶
  checkpoint。
- 所有叶阶段代码仍运行于 BootTask 且必须满足 BootInitFlow 的 dispatch guard。

叶阶段详细动作和 checkpoint 见本目录对应 coding 文件。旧 `BootPhase.*` checkpoint 已删除并整体
重编号，不留 ID 墓碑。
