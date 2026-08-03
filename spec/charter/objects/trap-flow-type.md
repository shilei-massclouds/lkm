# TrapFlowType

`TrapFlowType` 是 `TrapType` 所承载的一次正式陷入响应流，二者的关系如同 `TaskFlow` 与 `Task`：
`TrapType` 是所属 CPU 上稳定的响应资源，每次正式陷入则建立 fresh `TrapFlowType` root occurrence。
`Bind` 把 parent 永久绑定到最初接收陷入的 `TrapType`，保存入口原因、上下文与返回检查点，并建立
非零 generation 的 `TrapFlowRef`。即使可调度异常随 Task 迁移，该 parent 也不改写。

完整 lifecycle 为 Preset（验证入口并保存检查点）→ Setup（分类并同步驱动一个 child Flow）→ Enable
（处理完成并生成一次性 `TrapReturnToken`）→ Disable/Cleanup（先回收 child 再回收自身）。TrapFlow
保存当前活动 child FlowRef，以形成真实执行链。架构返回只能在 Cleanup 后消费 token 一次。

所有到达正式 trap 入口的事件都遵守该 lifecycle；reschedule SSIP 也不得绕过 occurrence。每次 SSIP
建立 fresh、非零 generation 且可校验的 TrapFlow/InterruptFlow 栈，handler 只清本 hart pending、合并
`need_resched`，随后按 child→root 顺序 Cleanup 并消费一次 token。未发生 Task switch 的普通返回不制造
Task.Dispatch 或 TaskFlow.Enter。

occurrence 与陷入现场相邻存放在实际内核栈的 `TrapExecutionRecord`，不使用堆或固定池。

返回时的长期 continuation 必须仍是入口 Task 的固定 `Task.flow`，且 TaskRef、FlowRef、generation、
CPU 与 context epoch 全部匹配。exec 只替换 Flow-owned UserAppRuntime 内的 ApplicationInstance，不会
产生 successor/predecessor Flow。清理完成后先消费一次性 TrapReturnToken，再恢复同一个固定
TaskFlow；stale、错误 Flow 或重复返回终止失败。

若 leaf 内发生真实 Task switch，root `TrapFlowRef` 保存在被换出 Task 的 context；未来匹配的 contextual
TaskFlow.Enter 必须在执行 leaf continuation 前验证 root 仍存活且未 Cleanup、root 的入口 TaskFlow 与
owner CPU、活动 child、具体 leaf generation 和 context epoch 全部匹配。Enter 只证明并记录已恢复到该
leaf；恢复坐标仍由保存的机器 `ra/sp` 决定，不按异常或中断类型选择入口。

## Mapping

- Model: `spec/model/objects/trap_flow_type.spec`
- Coding: `spec/coding/objects/trap-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/trap_flow_type.rs`
