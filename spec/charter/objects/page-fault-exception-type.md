# PageFaultExceptionType

`PageFaultExceptionType` 是每 CPU `ExceptionType.page_fault` 资源。它拥有 page-fault handler binding、
fixup 能力与上下文分类。来自用户态的 instruction/load/store fault 必须形成绑定当前 Task/mm、
fault address、原始 `sepc` 和 access 的统一请求，并交给 `UserAddressSpace` 分类；成功只能原指令
重试。用户恢复默认可睡眠。kernel origin 与 atomic 是两个独立维度：带有效
exception-table fixup 的 kernel fault，只有在非嵌套、非 hardirq、入口前可中断且当前 Task context 中
才可以调度；nested trap、hardirq 或入口前中断关闭的 kernel fault只能立即提交 fixup，不得调度。
无 fixup、stale fixup 或非法上下文均 fatal。用户 VMA/COW 状态不得被 kernel fixup 路径消费，
kernel extable 状态也不得参与用户 fault 分类。每次处理使用 fresh `PageFaultExceptionFlowType`。

`Preset` 只建立 page fault 的初始兜底，使尚未具备正式 handler 或恢复能力的 fault 具有确定处理
去向；这些更完整的能力由后续 lifecycle 建立，尚未闭合的部分可以保留为 deferred obligation。

## Mapping

- Model: `spec/model/objects/page_fault_exception_type.spec`
- Coding: `spec/coding/objects/page-fault-exception-type.md`
- Implementation: `impl/arceos_ex/src/objects/page_fault_exception_type.rs`
