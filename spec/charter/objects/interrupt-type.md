# InterruptType

`InterruptType` 是 `TrapType.interrupt` 的 CPU-local resident 资源。它直接拥有本 CPU 的总门控、分路
门控与待决中断信号、handler binding，以及 save/disable/restore 动作；设备级 source gate 继续由
IRQ chip 资源拥有。

对 BootCPU 执行 Preset 时，必须先关闭它的全部中断分路门控，再清空这些分路门控上的全部待决中断
信号，使随后启动过程不受中断信号干扰。该边界只建立分路门控关闭与待决信号清空事实，不改变或判定
本 CPU 的中断总门控，也不安装 handler、fallback 或正式分派框架。

后续 lifecycle 才依次建立总门控关闭、handler 分派就绪和总入口开放。hardirq 执行期间禁止调度和普通
IRQ 再入；只有 handler 明确重新开放的返回或 softirq 区间才允许嵌套。每次到达正式分派边界时声明
独立 `InterruptFlowType`，其 parent 固定为本资源。

## Mapping

- Model: `spec/model/objects/interrupt_type.spec`
- Coding: `spec/coding/objects/interrupt-type.md`
- Implementation: `impl/arceos_ex/src/objects/interrupt_type.rs`
