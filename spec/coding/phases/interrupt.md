# Interrupt 叶阶段 namespace 编码指引

`phases/interrupt/` 只组织 `IrqTimeInitPhase`、`LocalIrqEnablePhase`、
`IrqOpenPreparePhase` 与 `ProcessPreparePhase`。不存在 `InterruptPhase` wrapper lifecycle。

四个阶段由 `BootInitFlow.Setup` 在 `SchedInitPhase` 之后依次直接驱动；每个 Online continuation 都
回到 BootInitFlow，不得返回 interrupt namespace wrapper。阶段代码在 BootTask 上执行并继承
BootInitFlow 的 OnCpu 执行权检查。

namespace module 不保存状态、不发 checkpoint。旧 `InterruptPhase.*` checkpoint 已删除并整体
重编号，不留兼容 marker 或 ID 墓碑。
