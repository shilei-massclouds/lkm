# PageFaultExceptionFlowType

`PageFaultExceptionFlowType` 是一次 page fault occurrence，parent 固定为入口 CPU 的
`PageFaultExceptionType`。它分别保存 user/kernel 来源、fault address、原始 `sepc`、访问分类、
nested/hardirq/入口前中断状态和 fixup 或恢复结果。user occurrence 将 Task/mm-bound 请求交给
`UserAddressSpace`，记录互斥 VMA 分类；`RetrySameInstruction` 必须保持原 `sepc`。user recovery
默认可睡眠；具有有效 exception-table fixup 的 kernel fault仅在
非嵌套、非 hardirq、入口前可中断的当前 Task context 中可调度。atomic 路径立即设置 fixup `sepc`；
可调度 kernel 路径可以在观察到已发布 mailbox/`need_resched` 时先经 owner Scheduler 切换，恢复后重新
验证同一 root/leaf，再提交 fixup。无 fixup、stale fixup、错误 CPU/epoch 或已 Cleanup leaf 均 terminal。
user 与 kernel 分支的请求、结果和恢复状态互不消费。

## Mapping

- Model: `spec/model/objects/page_fault_exception_flow_type.spec`
- Coding: `spec/coding/objects/page-fault-exception-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/page_fault_exception_flow_type.rs`
