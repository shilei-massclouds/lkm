# ExecSyncBoundaries

`ExecSyncBoundaries` 是 boot/runtime exec 共用的同步与提交约束对象。它从旧的
`PayloadExecSyncBoundaries` 泛化，生命周期不再属于 `user_boot`；`PayloadPreparePhase` 只依赖它已经
`Ready`。

首轮实现一个 active transaction slot、bounded CLOEXEC 预检/提交、current/staging mm handoff、SATP/
`sfence.vma` owner handoff、point-of-no-return 以及 retired-mm release。Linux 6.12 的 `binfmt_lock`、
`cred_guard_mutex`、`exec_update_lock`、`mmap_lock`/task_lock/siglock/tasklist_lock、fs lock+RCU、
membarrier、sched_mm_cid、io_uring/timer/namespace/accounting hooks 等完整协议继续作为显式 deferred
边界保留。

Boot owner 在 `KernelInitFlow.CommitPayloadHandoff` 后统一执行最终 `sret`；runtime owner 在 commit 内写 SATP、执行
`sfence.vma` 并重写 syscall return frame。两条路径都必须在 CPU 不再引用旧 mm 后才释放 retired backing。
