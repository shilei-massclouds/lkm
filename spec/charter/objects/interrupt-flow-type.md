# InterruptFlowType

`InterruptFlowType` 是一次中断分派 occurrence，正式 parent 始终是入口 CPU 的 `InterruptType`，而不是
root TrapFlow。root 通过活动 child FlowRef 保存执行关系。Preset 保存分类前状态，Setup 执行 handler，
Enable 提交完成，Disable/Cleanup 同步回收。hardirq 全程保持禁止调度和普通 IRQ 再入，除非进入显式
重新开放区间。

## Mapping

- Model: `spec/model/objects/interrupt_flow_type.spec`
- Coding: `spec/coding/objects/interrupt-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/interrupt_flow_type.rs`
