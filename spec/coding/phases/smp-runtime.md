# KernelInitFlow 叶阶段 namespace 编码指引

`phases/smp_runtime/` 只是 KernelInitFlow execution continuation 子阶段的 namespace，不存在
`SmpRuntimePhase` wrapper lifecycle 或 checkpoint。

KernelInitFlow 的直接编排为：

- Preset：`PreSmpInitPhase`，随后 `SmpBringupPhase`，两者 Online 后 Flow Prepared；
- Setup：`RuntimeCorePhase`、`InitcallPhase`、`RootfsPhase`、`FinalizePhase`、
  `PayloadPreparePhase`，最后一个 Online 后 Flow Ready；
- Enable：只驱动 `PayloadHandoffPreparePhase`，其 Online 后 Flow Online。

这一整段是 Kernel.Enable 在 Kernel Ready 源状态内的下层执行过程。所有入口和 continuation 都必须
验证 Kernel Ready 且 Enable 已接受，不得要求 Kernel 已 Online。Flow Enable 提交 Online 后只返回
Kernel owner；它不自行发送 payload handoff。Kernel 在验证应用环境准备闭包后提交 Online，随后才
作为唯一 sender 发出 `KernelInitFlow.Action::CommitPayloadHandoff`。

每个叶阶段的 Online continuation 只能返回 KernelInitFlow 当前 transition。所有入口和 continuation
都必须即时验证 KernelInitTask OnCpu、`CurrentTaskSlot` 指向 KernelInitTask，并验证当前 SP 位于
PID 1 的 vmalloc stack。首次叶阶段不得由 scheduler 的 BootTask 调用栈同步预执行。

SmpBringup 的 BP 协调属于 KernelInitFlow；AP Entry/Callin/OnlineIdle 不属于 PID 1 Flow 的执行
所有权，继续等待逐 AP TaskFlow 正式化。旧 `SmpRuntimePhase.*` checkpoint 全部删除并重编号。
