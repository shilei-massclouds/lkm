# InterruptType

`InterruptType` 是 `TrapType.interrupt` 的 CPU-local resident 资源。它直接拥有本 CPU 的总门控、分路
门控与待决中断信号、handler binding，以及 save/disable/restore 动作；设备级 source gate 继续由
IRQ chip 资源拥有。

对 BootCPU 执行 Preset 时，必须先关闭它的全部中断分路门控，再完成一次待决中断清除写。该边界只
建立分路门控关闭、待决清除写已经完成及二者的先后顺序事实；硬件驱动的待决位可以在写完成后再次
置位，因此该边界不保证随后读取的待决位保持为零。Preset 不改变或判定本 CPU 的中断总门控，也不
安装 handler、fallback 或正式分派框架。

后续 lifecycle 才依次建立总门控关闭、handler 分派就绪和总入口开放。hardirq 执行期间禁止调度和普通
IRQ 再入；只有 handler 明确重新开放的返回或 softirq 区间才允许嵌套。每次到达正式分派边界时声明
独立 `InterruptFlowType`，其 parent 固定为本资源。

SMP runtime 为每个 online CPU 开放 supervisor software interrupt 分路。reschedule IPI sender 必须先
release 发布目标 mailbox，再经 SBI 向目标 hart 请求 IPI。SSIP handler 清除本 hart 的硬件 pending，
把 CPU-local `need_resched` 合并置位，并正常返回；handler 内禁止消费 runqueue、调用 Scheduler 或执行
context switch。中断返回后的 idle/Task 安全点负责 acquire 观察 mailbox 与 `need_resched` 并调度。

## Mapping

- Model: `spec/model/objects/interrupt_type.spec`
- Coding: `spec/coding/objects/interrupt-type.md`
- Implementation: `impl/arceos_ex/src/objects/interrupt_type.rs`
