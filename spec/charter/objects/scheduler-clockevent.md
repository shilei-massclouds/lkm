# SchedulerClockevent

每个 online CPU 拥有一个 `SchedulerClockevent`，与既有 CPU0 one-shot callback 共同复用本 CPU 的
RISC-V timer deadline。mux 总是向 SBI 编程最早 deadline，并分别保存 callback 与 scheduler deadline；
一个来源到期不得丢失另一个来源。

Scheduler 每次 dispatch 开始一个 10 ms 时间片。scheduler deadline 以“上一 deadline + period”推进；
若 hardirq 观察时已经跨过多个 period，则跳过所有已错过周期并编程第一个未来 deadline，禁止按
`now + period` 漂移。timer hardirq 只确认到期来源、运行既有短小 one-shot callback、重装下一 deadline，
并把 CPU-local `need_resched` 合并置位；它不消费 inbox/runqueue，不直接调用 Scheduler。

返回边界根据 trap 入口保存的 SPP 决定抢占时机。SPP=U 时，具体 leaf 的 handler 与 Disable 已完成、
但 leaf/root 尚未 Cleanup 的安全 continuation 先 acquire-drain 本 CPU 可见 inbox，再消费合并的
`need_resched`；此时可以保存当前用户 Task 的完整 trap overlay 并调度。该 continuation 不属于 hardirq
handler，未 Cleanup 的具体 leaf 只作为可恢复坐标保留。SPP=S 时只保留 pending，最迟在随后返回用户态
的同类边界消费。syscall、可恢复 page fault 和普通 IRQ overlay 都服从同一规则。若本地 runqueue 没有
其它 runnable Task，安全点消费 pending、续订当前时间片并直接返回，不制造 identity context switch。
A→B→A 恢复必须从 A 已保存的具体 trap leaf continuation 返回，重验 identity 后才继续 leaf/root Cleanup。

由用户态进入的阻塞 syscall 在进入 `wfi` 或其它主动睡眠之前，其显式 Scheduler handoff
也是同等的安全 continuation。该 syscall 的具体 leaf 可以保持 Ready 且未释放；handoff 必须
acquire-drain 所有可见的 CPU-local inbox，并消费 SPP=S hardirq 留下的合并
`need_resched`。非 identity 交换必须保存并恢复同一 syscall leaf，返回后重验
Task/Flow/CPU、root generation、context epoch 和 active leaf；内层 hardirq 仍不得直接调度。

本对象不引入内核正文立即抢占、进程迁移、ASID 或远程 TLB shootdown。

## Mapping

- Model: `spec/model/objects/scheduler.spec`, `spec/model/objects/interrupt_type.spec`
- Coding: `spec/coding/objects/scheduler.md`, `spec/coding/phases/interrupt/irq-time-init.md`
- Implementation: `impl/arceos_ex/src/objects/scheduler_clockevent.rs`
