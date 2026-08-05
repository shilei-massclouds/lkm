# InterruptFlowType

`InterruptFlowType` 是一次中断分派 occurrence，正式 parent 始终是入口 CPU 的 `InterruptType`，而不是
root TrapFlow。root 通过活动 child FlowRef 保存执行关系。Preset 保存分类前状态，Setup 执行 handler，
Enable 提交完成，Disable 结束 handler/hardirq 区间，Cleanup 回收 occurrence。handler/hardirq 区间保持
禁止调度和普通 IRQ 再入，除非进入显式重新开放区间。

timer/SSIP handler 产生的 `need_resched` 不是 handler 内的调度许可。SPP=U 时，Disable 已完成但本 Flow
尚未 Cleanup 的返回 continuation 是允许调度的安全点：它保存仍可解析的 root/leaf identity 与机器
frame；若切出，这些 identity 随 TaskThreadContext 保存并在 A→B→A 时精确恢复，随后才执行 Cleanup。
SPP=S 时该安全点不消费请求，pending 保留到后续用户返回边界。

## Mapping

- Model: `spec/model/objects/interrupt_flow_type.spec`
- Coding: `spec/coding/objects/interrupt-flow-type.md`
- Implementation: `impl/arceos_ex/src/objects/interrupt_flow_type.rs`
