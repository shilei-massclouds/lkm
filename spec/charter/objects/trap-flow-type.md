# TrapFlowType

`TrapFlowType` 是 `TrapType` 所承载的一次正式陷入响应流，二者的关系如同 `TaskFlow` 与 `Task`：
`TrapType` 是所属 CPU 上稳定的响应资源，每次正式陷入则建立 fresh `TrapFlowType` root occurrence。
`Bind` 把 parent 永久绑定到最初接收陷入的 `TrapType`，保存入口原因、上下文与返回检查点，并建立
非零 generation 的 `TrapFlowRef`。即使可调度异常随 Task 迁移，该 parent 也不改写。

完整 lifecycle 为 Preset（验证入口并保存检查点）→ Setup（分类并同步驱动一个 child Flow）→ Enable
（处理完成并生成一次性 `TrapReturnToken`）→ Disable/Cleanup（先回收 child 再回收自身）。TrapFlow
保存当前活动 child FlowRef，以形成真实执行链。架构返回只能在 Cleanup 后消费 token 一次。

occurrence 与陷入现场相邻存放在实际内核栈的 `TrapExecutionRecord`，不使用堆或固定池。

返回时的长期 continuation 必须仍是入口 Task 的固定 `Task.flow`，且 TaskRef、FlowRef、generation、
CPU 与 context epoch 全部匹配。exec 只替换 Flow-owned UserAppRuntime 内的 ApplicationInstance，不会
产生 successor/predecessor Flow。清理完成后先消费一次性 TrapReturnToken，再恢复同一个固定
TaskFlow；stale、错误 Flow 或重复返回终止失败。

## Mapping

- Model: `spec/model/objects/trap_flow_type.spec`
- Coding: `spec/coding/objects/trap-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/trap_flow_type.rs`
