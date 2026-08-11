# Defect Tracking

本文记录尚未闭环的实现缺陷和间歇性问题。规划性任务仍放在 `docs/ROADMAP.md`；这里优先记录可复现现象、证据、当前判断和下一步定位方向。

## 待解决

### DF-0016: SSIP 进入 TrapOccurrence 时观察到已发布但失效的 root

- 状态：2026-08-11 按阶段稳定性决策通过。首次真实失败仍作为未解释的概率性样本保留；加入长期诊断后，同一 linux-object、8 CPU、canonical rootfs 和四项 stock LTP supported 名单完成 100/100，阶段接受该样本但不宣称因果实现修复。
- 首次失败：`impl/arceos_ex/tests/stress/out/20260810T152908.926203Z-ltp-supported-linux-object/runs/run-0032/`。QEMU 正常退出但 LTP 完成 marker 缺失；通用诊断记录 SSIP、`TrapOccurrence.BindRoot`、`first_failed=installed_root_stale`、TaskRef/generation/CPU、root refs、`scause` 与 `sepc`，不是 timeout、栈溢出或 guard failure。
- 稳定性证据：`impl/arceos_ex/tests/stress/out/20260810T155305.234196Z-df-0016-ltp-trap-root-stale-linux-object/report.md` 为 100/100；每轮四条精确 PASS 和 `TOTAL=4 PASS=4 FAIL=0 BROK=0 WARN=0 CONF=0` 均完整出现，未再观察 lifecycle failure、panic 或 overflow。
- 当前边界：Impl 的 root cleanup 先于 Task/entry-context 解除发布，历史 SSIP 的 `sepc` 与解除路径中的 TaskRef 解析相符；Linux 6.12 的 exit-to-user/irqentry 路径在 entry teardown 前关闭本地中断。这是后续若复发时的最高优先级定位方向，但现有 100/100 本身不能把该推断升级为已证明根因。
- 长期入口：`df-0016-ltp-trap-root-stale-linux-object` 保留相同 basic identity 和精确 classifier，常规轮数固定为 50。只有同一 class 再次复发时才恢复 100/200/300/500 定位阶梯；差分最多 10 轮。

### DF-0015: wait4 分片诊断与 stock LTP PASS 记录间歇拼接

- 状态：2026-08-10 已用冻结的 10 组 Linux/native 差分精确复现并修正，保留独立 native 长期 stress。修复前 native 的 LTP 语义结果和汇总均通过，但第 2、4、7 组的 `uname02` standalone PASS 记录被插入 `wait4 registry` 分片诊断，横向结果为 7/10；这不是 syscall、harness 或 DF-0007 child-wait 语义失败。
- 首次差分报告：`impl/arceos_ex/tests/stress/out/20260810T045830.218212Z-df0007-ltp-supported-difftest/report.md`。失败行的精确形状为 `wait4 registry ... related_quiesced=--- uname02: PASS (exit 0)`，后续诊断字段另起一段，证明边界是并发 console writer 间的 record 原子性。
- 规格闭合：锁定的 computer Charter reviewed, no change；Model 已要求 runtime diagnostics 与用户 console 共享 TX queue 且 batch 连续；Coding 已要求诊断先格式化一个有界 record，再通过同一 locked batch path 发出。Impl 中该 wait4 诊断原先仍用多次 `putstr`/`putchar` 输出，违反既有约束；现已改为单次 `sbi::write_record`，未改变 LTP 结果或 wait4 行为。
- Linux 6.12 对照：`vprintk_store()` 以 reserve/fill/commit 保存一个 printk record，TTY write 由 `atomic_write_lock`/`tty_write_lock` 串行化；本修正只闭合相同的 record 边界，不引入测试名特判。
- 修正后证据：直接根 `make test` 为 186/186；同一四项名单的 Linux/native 差分报告 `impl/arceos_ex/tests/stress/out/20260810T051033.584619Z-df0007-ltp-supported-difftest/report.md` 为 10/10，四条 PASS 均保持 standalone；长期 stress 报告 `impl/arceos_ex/tests/stress/out/20260810T051928.385442Z-df-0015-ltp-frontier-console-record-interleave-native/report.md` 为 50/50。日常专项轮次固定为 50；100/200/300/500 只用于尚未定位或同 class 再次复发的概率问题，不能作为修复后的机械门禁。

### DF-0014: 跨 CPU checkpoint consumer 并行被全局 reentry guard 间歇误杀

- 状态：2026-08-06 在 DF-0012 的最终 500 轮扩样中复现 1 次，已定位为 checkpoint 观察框架的跨 CPU 并发误判，正按 Linux per-current/per-context recursion 边界修正并加入独立长期 stress。
- 首次记录日期：2026-08-06。
- 失败 artifact：`impl/arceos_ex/tests/stress/out/20260806T040602.535104Z-df-0012-rc-local-direct-setup-native/runs/run-0448/`。整批报告为 499/500；唯一失败轮次 QEMU 正常关机、非 timeout，两次 `Kernel.Online` 已完成，`lkm-rc-local: begin` 已输出，但 `/bin/ls` 的 `lost+found` 和 rc.local end 缺失。
- 首个精确内部边界：AP 调度流已输出 `Scheduler.SwitchTo prev=ApIdleTask next=UserTask`、`Scheduler.Schedule task=ApIdleTask` 和 `Scheduler.SwitchTo.Exit task=None`，紧接着出现 `ccheckpoint reentry`并进入 stress_mem dump。这个形状晚于 DF-0012 的 direct-setup 关机，也不是 DF-0004 timeout。
- 根因证据：该构建只启用 `announce + stress-mem`，announce handler 调用链不发出新 checkpoint，因而同 CPU handler 真递归不能解释该样本。实现却使用全系统唯一 `CHECKPOINT_HANDLER_ACTIVE: AtomicBool`，任意两个 CPU 同时 dispatch 都会让后者 fail-stop。Linux 6.12 `include/linux/trace_recursion.h` 把 recursion 位放在 `current->trace_recursion`，并区分 normal/IRQ/softirq/NMI context；`kernel/trace/ftrace.c` 以该本地 guard 包围 callback list，不用全局布尔位排斥其它 CPU。
- 长期入口：`df-0014-checkpoint-cross-cpu-reentry-native` 原样重复 canonical `rc-local-native`，case-local 深采样 500 轮并进入默认 suite。classifier 必须同时看到 `lkm-rc-local: begin` 和 `checkpoint reentry`；完整成功仍要求 `lost+found` 和 end status 0。

当前修正与验证要求：

- Charter 明确不同 CPU 的 checkpoint consumer 可并行，重入 guard 属于 CPU/执行上下文；锁定的 computer Charter reviewed, no change。Model 将观测 action 绑定到当前 TaskFlow/CPU，Coding 规定 CPU-local acquire/release guard。首片仍对同 CPU normal-context 嵌套 fail-stop，不在未规格化时猜测实现 Linux 全部 context bits。
- 修复后按 `50 -> 100 -> 200 -> 300 -> 500` 执行该 native identity；任意一档出现 reentry、timeout、panic、overflow 或未分类失败即停止扩样并重新定位。本问题是观察框架内部并发语义，不需要把差分扩到高轮数；若做 Linux/arceos_ex 横向观测对比，仍不得超过 10 轮。

### DF-0013: fork 共享 OFD 首次 child 读取后间歇性回收/继续执行超时

- 状态：2026-08-06 已定位并修正，保留长期 stress 回归。首个超时由 child 退出、父进程 wait/reap 与共享进程资源释放的并发交错触发：child exit 已释放共享 files/OFD 引用，但 aggregate 的 `process_resources_present` 所有权事实仍为真，reap 再次尝试释放并拒绝继续。修复后 native 与 linux-object 的独立 500 轮压力均零失败，Linux/arceos_ex 精确行为差分 50/50 一致。
- 首次记录日期：2026-08-05。
- native 失败 artifact：`impl/arceos_ex/tests/basic/out/20260805T211850.171911Z-fork-ofd-offset-625733/`。第一段输出已证明 inherited fd 的读取位置正确；此后没有 `R2=`，`result.json` 记录 timeout 且 QMP snapshot captured，8 个 hart、进程组、私有磁盘和 QMP socket 均完成冻结/清理。
- linux-object 对照 artifact：`impl/arceos_ex/tests/basic/out/20260805T212142.846566Z-fork-ofd-offset-lo-626109/` 在相同第一段 marker 后超时，因此当前证据不支持把问题归于任一 PLIC provider。
- sibling Linux 成功 artifact：`impl/arceos_ex/tests/basic/out/20260805T211838.683774Z-fork-ofd-offset-linux-624344/` 依次打印 `R1=#!/`、`R2=bin`、`R3=/sh` 并完成。arceos_ex 两侧在本轮修改前也曾完整执行同一三段命令，构成概率性成功对照，不能用本次 timeout 否定已定位的 OFD 语义边界。
- 长期入口：`df-0013-fork-ofd-offset-child-return-native` 进入默认 suite；显式 linux-object identity 使用同一 classifier。两者按 50→100→200→300→500 独立扩轮，完整成功必须出现 Linux 相同三段字节及 complete marker；timeout 分类还必须读取结构化 `timed_out=true`，不能把 marker 正常退出时的 host signal-15 文本误判为失败。
- 成对入口：`fork-ofd-offset-difftest` 每轮先运行 sibling Linux，再运行 native arceos_ex，只比较外部 child 实际输出的 `R1=#!/`、`R2=bin`、`R3=/sh` 有序序列，命令回显不进入序列。2026-08-06 的一次性深证据为 50/50，报告 `impl/arceos_ex/tests/stress/out/20260806T030356.771947Z-fork-ofd-offset-difftest/report.md`；此后按横向比较职责默认且最多执行 10 轮，小概率复现由 provider stress 承担。

当前判断与后续要求：

- 通用生命周期诊断把边界收敛到 child exit 已完成 files/OFD release、reap 仍看到资源存在并二次 release。Linux 6.12 `do_exit()` 经 `exit_files()` 在锁内把 `tsk->files` 置空后只执行一次 put，zombie wait 只认领 zombie 并回收 carrier，不再次执行 `exit_files()`；本实现据此在首次成功 release 时同步清除 aggregate 的资源所有权事实，reap 仅回收 carrier。
- Charter 的 aggregate 资源一次释放与 reap carrier 边界已闭合；锁定的 computer Charter reviewed, no change。Model、Coding 与 Impl 均要求成功 release 后所有权事实变为 false，禁止 reap 二次释放。最高阶报告分别为 native `impl/arceos_ex/tests/stress/out/20260806T023529.759838Z-df-0013-fork-ofd-offset-child-return-native/report.md` 和 linux-object `impl/arceos_ex/tests/stress/out/20260806T024951.065166Z-df-0013-fork-ofd-offset-child-return-linux-object/report.md`，均为 500/500。
- DF-0013 与 DF-0007 的结构化 dispatch-preflight 主动终止、DF-0009 的双 writer 完成后 pipe-read timeout、stock LTP parse error 都不是同一终止边界，不得合并 classifier。
- 不得删除外部 child、用 shell 内建替代 fork、提前 complete、延长 timeout、改变 provider/SMP/rootfs 或把第一段正确字节当作整体成功。后续成功样本不能抵消或关闭 DF-0013。

### DF-0012: native rc.local 在 `DmaCachePolicy.Ready` 后间歇性 direct-setup 失败

- 状态：2026-08-10 已用通用首失败谓词、栈保护观测和 RISC-V 硬件写观察点定位并修正。根因是 `CpuGroup::setup_smp()` 先在 BootTask 栈上构造完整的 secondary `Cpu`，其 Scheduler 和 trap emergency stack 使调用链越过 16 KiB 栈底；随后 checkpoint 格式化写入覆盖 stack guard。改为在唯一权威 `Option<Cpu>` slot 中延迟就地构造后，直接根 `make test` 186/186，冻结 identity 压力 500/500。
- 首次记录日期：2026-08-05。
- 首次 artifact：`impl/arceos_ex/tests/basic/out/20260805T205802.609110Z-rc-local-native-617138/`。QEMU 仅运行 0.165 秒并以 0 退出，basic 因 rc.local marker 和两次 `Kernel.Online` 缺失而失败；最后稳定边界为 `checkpoint: DmaCachePolicy.Ready task=BootTask`、`error=C event=S actual=P expected=P target=R`，不是 timeout、panic 或 unsupported syscall。
- 第二次 artifact：`impl/arceos_ex/tests/basic/out/20260805T211030.587299Z-rc-local-native-620758/`。在 OFD 修正后的下一次直接根回归中以相同三条终止 marker 再现，证明它不是单个 artifact 或一次 host 启动噪声。
- 长期入口：`df-0012-rc-local-direct-setup-native` 原样重复 canonical `rc-local-native` 并进入默认 suite；专项深采样固定为 500 轮。classifier 精确要求 `DmaCachePolicy.Ready`、direct-setup 失败和结构化 lifecycle error 同时出现，完整成功仍要求 rc.local begin、`lost+found` 与 end status 0。
- 2026-08-06 的通用诊断将历史样本的首失败唯一收敛为 `boot_task.stack_guard_intact`。为避免诊断本身改变编译器栈布局，在临时重建的诊断前版本上对 stack guard 安装后布置硬件写观察点。首次重放停在 `core::fmt::write` 的 `sd s8, 0x30(sp)`：旧值 `0x57ac6e9d`、新值 `2`，PC `0xffffffff800994c2`，SP `0xffffffff82f75fd0`，比 BootTask stack base `0xffffffff82f76000` 低 48 字节。完整调用链为 `start_kernel -> BootInitFlow::setup::run -> CpuGroup::setup_smp -> Lifecycle::transition -> announce_name_with_context -> sbi::write_record -> core::fmt::write`；继续执行后精确复现原 `DmaCachePolicy.Ready`/direct-setup 失败。

当前判断与后续要求：

- 静态 stack-size 证据与动态写观察点一致：诊断前 `BootInitFlow::setup::run` 栈帧 2144 B、`CpuGroup::setup_smp` 12736 B；再加 trap 预留 256 B 和 `start_kernel` 160 B，仅剩 1088 B 给 checkpoint/格式化嵌套。`Cpu` 大小 8704 B，其中 `TrapType` 4320 B、`Scheduler` 4192 B、trap emergency stack 4096 B。就地构造使 `setup_smp` 栈帧降为 5888 B，不改变对象所有权、发布顺序或 resident resource。
- Linux 6.12 对照为 `kernel/sched/core.c` 的 `DEFINE_PER_CPU_SHARED_ALIGNED(struct rq, runqueues)` 与 RISC-V traps 的 per-CPU `overflow_stack`；`sched_init()` 通过 `cpu_rq(i)` 初始化目标存储，RISC-V `setup_smp()` 仅建立拓扑/可用 CPU map，不在当前启动栈上构造完整 runqueue 和 trap overflow stack。Charter 与 Model reviewed, no change；Coding 要求在权威 slot 就地构造，Impl 使用 `get_or_insert_with`；Compose reviewed, no change；Testing 保留原 classifier 与 identity。
- 修正后完整报告为 `impl/arceos_ex/tests/stress/out/20260810T025621.756778Z-df-0012-rc-local-direct-setup-native/report.md`：500/500 成功、0 失败。该结果是对已由写观察点定位之修正的稳定性验证，不是用成功轮次替代根因定位。
- DF-0012 是约 0.16 秒时的主动早期关机；DF-0004 是 guest 已进入后续启动/rc.local 路径后达到 180 秒的 QEMU timeout。两者共享 basic identity 但终止边界不同，classifier 和 defect 不得合并。
- 长期回归不得放宽 rc.local marker、删除 `Kernel.Online` 计数、延长 timeout、改变 SMP/provider/rootfs 或跳过 `setup_arch_return_ready()` 提高表面通过率。

### DF-0011: linux-object user-smoke preempt 校验后 finalization 间歇超时

- 状态：2026-08-05 在修正 DF-0010 child selection 后的直接根 `make test` 中，正式 `user-smoke-linux-object` 已完成 pipe bytes 收集、两个 child 回收与 A-B-A round-robin 校验，却未打印 preempt case end 或 guest exit，最终达到原 120 秒 timeout；已建立独立默认 stress 项，根因尚未明确。
- 首次记录日期：2026-08-05。
- 首次 artifact：`impl/arceos_ex/tests/basic/out/20260805T202757.172058Z-user-smoke-linux-object-609270/`。日志最后依次为 `user preempt bytes collected order=ABAABBAB`、`user preempt children reaped`、`user timer preemption A-B-A round robin ok`；`result.json` 记录 QEMU 120.015 秒 timeout，QMP snapshot 为 `captured`，私有磁盘、进程组与 QMP socket 均已清理。该轮根回归因此为 185/186，不能作为绿色回归报告。
- 长期入口：`df-0011-user-smoke-preempt-finalize-linux-object` 原样重复 canonical `user-smoke-linux-object` 100 轮并进入默认 suite。classifier 在 timeout 时用完整的 post-reap/A-B-A/host-termination 终态精确分类；完整成功必须继续出现 preempt `end status=0` 与 `user exit status=0`。

当前判断与后续要求：

- DF-0011 晚于 DF-0009：DF-0009 停在两个 writer complete 之后、bytes-collected 之前；DF-0011 已完成 bytes collection、reap 和 round-robin 校验。现有证据不足以把晚期停顿归因于用户态 case return、exit syscall、Task terminal handoff、Scheduler 或 provider，必须比较 timeout QMP PC/TaskRef/generation/CPU 后再改行为。
- 不得通过删减 preempt payload、提前 exit marker、缩短 case、延长 timeout 或把 A-B-A marker 当成整体成功来提高通过率。后续成功样本不能抵消或关闭 DF-0011，也不能替代 DF-0009。

### DF-0010: linux-object LTP frontier 后续 child runqueue publication 失败

- 状态：2026-08-05 在保持 guest 交互会话、完成四项 frontier 后再次执行 `sha256sum /opt/ltp/run-syscalls.sh` 时，内核终止于 `declared child runqueue publish invariant failed`；已建立独立 basic diagnostic identity 和默认 stress 项，根因尚未明确。
- 首次记录日期：2026-08-05。
- 首次 artifact：`impl/arceos_ex/tests/basic/out/20260805T194115.149428Z-shell-lo-595228/`。同一会话先观察到脚本长度 3606、SHA-256 `c79278919640c1881a0bfe433c00a6e5d06eeec8d16f3dd212c86f363c0a53e7` 与 host/staging 完全一致，且 `/bin/sh -n` 成功；随后四个精确 entry 均打印 TPASS，stock harness 报第 203 行 parse error；父 shell 再启动 hash child 时进入新的 runqueue-publication 终止边界。
- 长期入口：`ltp-supported-post-read-lo` 固定首次样本的完整 child 历史：长度检查、带三个不存在参数的失败 `sha256sum` child、由 Ctrl-C 中断的未完成 `tail`、成功 hash、`sh -n`、冻结的第一批 supported selector 和第二次 hash；`df-0010-ltp-frontier-post-read-runqueue-linux-object` 原样重复该 identity 50 轮并进入默认 stress suite。历史 case 名保留首次 frontier artifact 的来源，但 current identity 不得跟随下一批 frontier。classifier 优先识别结构化 `Scheduler/Enqueue.Publish/TaskRunqueue` 诊断，再识别旧终止 marker 与独立 harness parse error。

当前判断与后续要求：

- 单个样本证明该路径存在，但旧终止 marker 没有给出 enqueue 的首失败谓词。已先把 Scheduler enqueue 的现有检查拆成长期通用 `FailureDiagnostic`，clone 终止路径改为打印原 `EventError`；该诊断不改变 admission 或生命周期行为。必须先用冻结 identity 复现并取得首失败，再决定最高受影响规格层和行为修正。
- 加入结构化诊断后的第一次人工精确重放 artifact `impl/arceos_ex/tests/basic/out/20260805T200300.502054Z-shell-lo-601812/` 在 `uname02` 已 TPASS 后停顿，未到达 stock parse error 或第二次 hash；该样本没有产生 enqueue 失败诊断，按独立的调度/pipe timeout 边界保存，不能冒充 DF-0010 的复现，也不能用来删减首次样本的前置历史。
- DF-0010 发生在父 shell 准备第二个 child 的 runqueue publication，不能与更早的 stock harness parse error、四项 cleanup TWARN、unsupported syscall、DF-0007 wait/reap 或 DF-0008 `mark_enqueued` 合并。不得通过删除第二次读取、放宽 marker、替换 harness/selector、切换 provider/SMP/rootfs 或延长 timeout 提高表面通过率；后续成功样本不能抵消或删除压力项。

### DF-0009: linux-object user-smoke preemption pipe-read 间歇超时

- 状态：2026-08-05 在 DF-0008 的 100 轮定向压力中，第 13 轮正式 `user-smoke-linux-object` 于 preempt case 发生 120 秒 timeout；已作为独立默认 stress 项保留，根因尚未明确。
- 首次记录日期：2026-08-05。
- 失败 artifact：`impl/arceos_ex/tests/stress/out/20260805T191932.622566Z-df-0008-user-smoke-fork-enqueue-linux-object/runs/run-0013/`。该轮 `fork_mm` 已完整结束；coordinator 已发布两个 child，A/B 均打印 write-complete，但没有打印 bytes-collected 或 children-reaped。timeout QMP snapshot 状态为 `captured`，捕获前 VM 为 running，8 个 hart 均可查询，进程组和 QMP socket 均完成清理。
- 长期入口：`df-0009-user-smoke-preempt-linux-object` 原样重复 canonical `user-smoke-linux-object` identity，case-local 深采样 100 轮并进入默认 suite；完整成功必须收集 pipe bytes、回收两个 child、完成整个 user-smoke 并正常 guest exit。

当前判断与后续要求：

- 该终止 marker 形状与 native DF-0006 相同，但当前证据不足以证明二者根因相同，也不足以归因于 linux-object PLIC、pipe wakeup、timer preemption、runqueue 或 Task 生命周期；保留 provider 独立入口与 artifact，先比较 QMP PC/TaskRef/generation/CPU 的首差异。
- DF-0009 不得替代最初在 fork publication 处终止的 DF-0008。不得通过改变 provider、SMP、rootfs、timeout、payload 或 marker 提高表面通过率；后续成功样本不能抵消或关闭本缺陷。

### DF-0008: linux-object user-smoke plain-fork child enqueue 间歇失败

- 状态：2026-08-05 已用通用 clone-enqueue 首失败诊断定位并修正；直接根 `make test` 186/186 通过，修复后的同一 linux-object identity 定向压力 50/50 通过。该概率性失败及其精确 classifier 继续保留在默认 stress suite，不因本批未复现而删除。
- 首次记录日期：2026-08-05。
- 失败 artifact：`impl/arceos_ex/tests/basic/out/20260805T190512.009583Z-user-smoke-linux-object-579156/`。失败前已通过 `user fork child private mm ok` 与 `user fork parent COW unique fast path ok`；结构化 clone 诊断为 `clone_kind=plain_fork clone_plain stage=mark_enqueued`，候选 child PID 6、`active_task_record_state=3`、`child_enqueued=0`，随后立即终止。
- 长期入口：`df-0008-user-smoke-fork-enqueue-linux-object` 原样重复 canonical `user-smoke-linux-object` identity，case-local 深采样 100 轮并进入默认 suite；classifier 精确匹配 `mark_enqueued` 和 invariant failure，同一 identity 的完整成功仍要求 COW reuse、整个 user-smoke 和 guest exit 全部完成。

定位、修正与验证：

- 在不改变原判定的前提下，`mark_enqueued` 增加了长期通用的逐谓词诊断。修复前报告 `impl/arceos_ex/tests/stress/out/20260805T191932.622566Z-df-0008-user-smoke-fork-enqueue-linux-object/report.md` 为 100 轮中 87 成功、10 次精确 enqueue 失败、3 次独立 timeout。10 个 enqueue 失败样本均报告同一首失败：对允许集合 `Online || OnCpu` 的两次无锁状态读取先得到 `OnCpu`（6）、再得到 `Online`（3）。两者分别合法，却被旧表达式拼成一次伪失败；该重复证据排除了 PID、active-record、inbox 预留和未知生命周期状态作为这 10 次失败的首边界。
- Charter 与 Model 已允许发布后的 child 在目标 CPU 上并发执行 `Online <-> OnCpu`，均为 reviewed, no change；Coding 增加“兼容 enqueue 判定只读取一次 lifecycle snapshot”的约束。Impl 以同一个状态快照判断两个允许值，保留后续 PID 与 active-record 检查及通用失败诊断。
- 修复后报告 `impl/arceos_ex/tests/stress/out/20260805T193532.483545Z-df-0008-user-smoke-fork-enqueue-linux-object/report.md` 为 50/50 成功、enqueue failure 0、timeout 0。这个结果验证已定位的竞态，不关闭 DF-0009，也不取消 DF-0008 的默认压力入口。
- DF-0008 与 native DF-0006 的“两个 child 已写完但 parent 未收集”不是同一边界，不得合并分类。不得通过改变 provider、SMP、rootfs、timeout、payload 或 marker 提高表面通过率；后续成功样本不能抵消或关闭本缺陷。

### DF-0007: LTP frontier 子进程回收时 dispatch preflight 失败

- 状态：2026-08-10 已用冻结 identity、通用逐谓词诊断和单变量负对照定位并修正。旧 wait4 路由会让当前 SMP 父进程的匹配 registry child 被无关的 PID1 legacy completed-child record 遮蔽，随后错误进入 legacy simulated dispatch；当前实现把 SMP wait/reap 严格绑定到 generation/CPU-checked 当前父进程及其 registry child。
- 首次记录日期：2026-08-05。
- native 失败 artifact：`impl/arceos_ex/tests/basic/out/20260805T183141.572743Z-ltp-frontier-565601/`。最后对象边界为 `prev=TaskRef(35:3)`、`next=TaskRef(32:1)`、`tp_ref=TaskRef(35:3)`、`rq_curr=TaskRef(2:1)`，`first_failed=simulated-next-dispatch-preflight`，随后打印 `child wait handoff dispatch invariant failed`。
- linux-object 对照 artifact：`impl/arceos_ex/tests/basic/out/20260805T183242.824997Z-ltp-frontier-lo-565852/`。它在相同测试阶段、相同 TaskRef/generation 形状和相同 preflight 条件失败，因此当前证据不支持把问题归于任一 PLIC provider。
- 长期入口：`df-0007-ltp-frontier-child-wait-native` 的历史名称保留首次 frontier artifact 来源，但当前原样重复 canonical native `ltp` supported identity 并进入默认 suite；`df-0007-ltp-frontier-child-wait-linux-object` 同理固定 `ltp-lo`，是同一 defect 的非默认第二 provider 入口，不得跟随下一批 frontier。两侧日常 case-local 样本均为 50 轮；100/200/300/500 逐档扩样只用于尚未定位的概率问题，不作为修复后的机械门禁。classifier 固定识别三条现有通用诊断，不改写 LTP 结果或 basic gate。

当前判断与后续要求：

- 历史两侧样本的 `prev=TaskRef(35:3)`、`next=TaskRef(32:1)`、`tp_ref=TaskRef(35:3)`、`rq_curr=TaskRef(2:1)` 已表明 current SMP child 被交给不相关 legacy carrier。2026-08-10 在 `/tmp` 的当前源码副本中只恢复旧 registry/legacy 路由条件，第一次冻结 native 运行即在 `uname01` 两条 TPASS 后恢复相同 TaskRef/generation 形状，新通用诊断把首失败收敛为 `live-sp-in-user-carrier-stack`；未撤回路由修正的工作区同一 identity 先完成 50/50。该负对照证明路由门禁是直接因果修正，而不是以成功轮次猜测根因。
- Linux 6.12 `kernel/exit.c` 的 `do_wait_thread()` 只遍历当前调用者 `tsk->children`，`eligible_child()` 再按 selector/flags 过滤，`wait_task_zombie()` 认领选中的 zombie；它不回退到全局 active carrier。Charter、Model 与 Coding 已分别要求当前 parent-child identity、`Wait4(parent, child)` registry 绑定和无关 completed record 不得遮蔽匹配 child，均 reviewed, no change；Impl 的 SMP registry-only 路由符合该链，Compose reviewed, no change，Testing 保留双 provider identity。
- frontier 日志中的 unsupported syscall 与清理 warning 是另行按“首个真实 syscall 边界”推进的 LTP 能力缺口，不能用它们解释或掩盖已经在 TPASS 后发生的 scheduler failure。
- 后续成功样本不能抵消该失败。不得通过更换名单、顺序、provider、SMP、rootfs、timeout、marker 或 LTP harness 提高表面通过率；若现有 preflight 诊断不足，再先增加长期通用诊断后复现。

### DF-0005: `busybox-init-login-native` 间歇性 post-login timeout

- 状态：2026-08-05 在根 `make test` 的正式 native BusyBox init/login acceptance 中观察到一次 180 秒 timeout；紧接着重跑同一 identity 在 8.12 秒内通过，已作为默认 stress suite 的长期观察项，根因尚未明确。
- 首次记录日期：2026-08-05。
- 调查起点：commit `94f6b09b2856` 加当前未提交 LTP 工作区；固定 basic identity 为 `busybox-init-login-native`，canonical rootfs、native provider、8 vCPU 和三段 scripted stdin 均不变。
- 失败 artifact：`impl/arceos_ex/tests/basic/out/20260805T153931.499906Z-busybox-init-login-native-505717/`。`result.json` 记录三段输入都已发送、QEMU 在 180 秒后 timeout、QMP snapshot 状态为 `captured`，私有磁盘、进程组和 QMP socket 均完成清理。日志已经到达 Alpine greeting、非 root shell 和 `/bin/ls` 的 `lost+found` 输出，随后重复出现 `/dev/ttyS0` open `EINVAL`，但没有到达 wrapper `sync` 和 guest shutdown。
- 成功对照：`impl/arceos_ex/tests/basic/out/20260805T154428.899858Z-busybox-init-login-native-507417/` 使用同一 test identity，在 8.12 秒内满足所有 acceptance marker 并正常退出。
- 长期入口：`df-0005-busybox-init-login-native` 只重复冻结的 basic identity，case-local 深采样为 100 轮并进入默认 suite；顶层 `STRESS_RUNS` 仍可统一覆盖。timeout 固定归类为 failure，并保留完整 basic/QMP artifact。

当前判断与后续要求：

- 一次失败与一次成功足以证明概率性回归风险，但不足以把根因归于 tty、signal、调度、PID 生命周期或其它候选；重复 getty 输出只作为首个可复核现象边界。
- 连续成功不能抵消该失败或关闭缺陷。下一次 timeout 应先比较 QMP hart 寄存器/PC、guest task generation/CPU 与成功路径最后共同事件；若现有诊断仍不能唯一定位，再增加通用长期诊断后复现。
- 不得通过改变 timeout、stdin、rootfs、provider、QEMU 参数或 acceptance marker 提高表面通过率；修复前后都必须保留该默认压力项。

### DF-0004: `rc-local-native` 间歇性 QEMU timeout

- 状态：2026-07-27 在正式 `rc-local-native` basic acceptance 中观察到一次 180 秒 timeout；根因尚未明确。已补充 timeout 前 QMP 冻结诊断，并将专项 case 纳入默认 stress suite；累计四批 710 轮均未复现，问题继续保持待解决。
- 首次记录日期：2026-07-27。
- 调查起点：commit `2249ea0dbc94`；关联范围为 `rc-local-native`、`user-boot`、native PLIC、canonical rootfs、QEMU RISC-V virt、serial8250/PLIC 外部中断、virtio-rng 完成路径及 basic runner 的 180 秒整体 timeout。
- 原始表现：`impl/arceos_ex/tests/basic/out/20260727T080032.751746Z-rc-local-native-45625/result.json` 记录 QEMU 阶段运行 180.009 秒后 timeout，result 为 `execution_status=failed` / `verdict=inconclusive`，进程组和私有磁盘均完成清理。日志最后完整到达 `Serial8250Console.IrqDrivenReady` 和 `Serial8250RxLoopback.Ready`，之后只出现单字节 `c`，没有到达 `VirtioRng.EntropyReady`、用户态 rc.local marker 或正常关机。该边界是可复核事实，不等同于 UART、PLIC、virtio-rng、SBI 或调度器根因。

已完成检查：

- basic runner 已建立每次启动独占的 QMP control socket。只有整体 timeout 到达后，runner 才暂停 VM，并把 `query-status`、`query-cpus-fast`、全 hart 寄存器和通用 IRQ 视图写入 `qemu-timeout-diagnostics.json`，然后终止进程组；捕获失败不得覆盖原始 timeout verdict。正常成功路径不使用 QMP 改变 guest 行为。
- 为排除执行环境混淆，使用固定 1 秒 timeout 的 diagnostic case 做了对照：沙箱内 QEMU 在启动 guest 前稳定报 QMP UNIX socket `Operation not permitted`；同一 case 经获准的非沙箱命令运行时成功捕获两个 hart 的完整寄存器快照，artifact 状态为 `captured`、无错误、捕获耗时 0.002857 秒，QMP socket 和进程组均完成清理。正式 stress 也经非沙箱 `make test-stress` 路径执行，日志未出现 `Operation not permitted`、`Permission denied` 或 seccomp 错误；因此其结果没有混入已知的沙箱 QMP bind 失败。
- `rc-local-native-timeout-focused` stress case 只重复 canonical basic identity，不复制或覆盖 180 秒 timeout、QEMU 参数、probe、rootfs 或 expectation。它现已进入默认 `make stress-test` suite，以统一 `STRESS_RUNS=10` 持续采样；命令 `make test-stress STRESS_CASES=impl/arceos_ex/tests/stress/cases/rc-local-native-timeout-focused.toml STRESS_RUNS=<N>` 仍用于 100 轮等专项深采样。每轮保留完整 basic artifact，timeout 时额外保留 QMP snapshot。
- 首批报告 `20260727T085100.986253Z-rc-local-native-timeout-focused` 完成 100/100，失败和 timeout 均为 0，共 8 个成功序列。
- 追加报告 `20260727T090709.453131Z-rc-local-native-timeout-focused` 完成 500/500，失败和 timeout 均为 0，共 23 个成功序列；复合轮次耗时 min/avg/p95/max 为 2.184/2.320/2.393/2.824 秒，500 轮的私有磁盘、进程组和 QMP socket 清理均为 500/500。两批累计 600/600 未复现，所有成功日志都在 `Serial8250Console.IrqDrivenReady` 前观察到 `VirtioRng.EntropyReady`。
- 默认 suite 调整前的追加报告 `20260727T104525.035467Z-rc-local-native-timeout-focused` 完成 100/100，失败和 timeout 均为 0，共 7 个成功序列，未产生 `qemu-timeout-diagnostics.json`；三批累计 700/700 未复现。
- 纳入默认 suite 后，直接 `make stress-test` 对 DF-0001/DF-0002/DF-0003/DF-0004 各执行 10 轮并全部成功；DF-0004 报告 `20260727T105119.092187Z-rc-local-native-timeout-focused` 为 10/10、4 个成功序列、0 个 timeout/QMP artifact。四批 DF-0004 样本累计 710/710 未复现。

当前判断：

- 当前只有一个未带 QMP snapshot 的原始 timeout 样本；补充诊断后的 710 个样本全部成功，不能计算失败/成功首差异，也不能证明问题已经消失或已找到根因。
- 原始失败缺少 `VirtioRng.EntropyReady`，而成功样本均先完成该 checkpoint，这是边界差异和后续取证锚点，不是足以修改内核/runtime 行为的因果证据。
- 沙箱内 QMP bind 失败是可独立稳定复现的 host 权限问题，发生在 guest 启动前；它不能解释非沙箱正式样本中的 guest 运行 180 秒 timeout，也不得与 DF-0004 归为同一失败类。

后续定位要求：

- 继续使用 checked-in focused case 在非沙箱路径复现，不通过改变 timeout、QEMU 参数、probe、rootfs 或 expectation 提高表面通过率；连续成功只能报告本批未复现。
- 下一次 timeout 首先保留并检查 `qemu-timeout-diagnostics.json`，把各 hart PC/寄存器映射到构建产物符号，并结合 QMP IRQ 视图和成功/失败 checkpoint 首差异定位停机边界；若现有视图仍不足，再先增加长期有用的 checkpoint/diagnostic。
- 在得到同一失败现场的可复核因果链前，不基于 UART、PLIC、virtio-rng、SBI、调度或沙箱候选做猜测性修复。

### DF-0003: 非 PTY `/bin/sh` delayed-input 执行 `/bin/ls` 表现不一致

- 状态：单命令历史不一致继续保留回归；2026-07-16 新增的连续两次 `/bin/ls` 确定性 ENOSYS 已定位并修复，双命令 DF-0003 为 30/30 成功，默认完整 stress 同轮为 10/10。
- 首次记录日期：2026-07-02。
- 关联范围：`impl/arceos_ex` 发行版 rootfs `/bin/sh`、host delayed-input harness、TTY/N_TTY stdin、QEMU stdio、child `clone/wait4/execve` 外部命令链路。
- 表现：此前把默认 shell smoke 从 `echo OK; exit` 升级为外部命令时，非 PTY delayed-input 日志曾稳定到达 `wait4 child handoff` 后截断，未能稳定看到目录输出和 `user exit status=0`；同一代码状态下后续用非 PTY `ls\nexit\n` 路径复现两次均成功输出 rootfs 目录和 `user exit status=0`，其后 50 次 stress 全部成功。2026-07-16 把输入固化为 `/bin/ls\n/bin/ls\nexit\n` 后稳定复现第二次 clone(220) 返回 ENOSYS：诊断为 `UserChild=Ready`、`child_continuation_taken=1`、`parent_wait_resumed=1`、`enqueued=1`、runqueue 仍含内部 child、无 completed record，路由因此落入只接受 Prepared slot 的 `copy_user_process`。这证明第一轮 PID1-originated plain child 已完成 wait/reap，但退出 child 的 slot/runqueue fact 未释放。

当前判断：

- 手工 PTY `/bin/sh` 输入 `ls` 已闭合，说明 plain fork、child continuation、job-control 和 child execve 主线可用。
- 非 PTY delayed-input 路径前后表现不一致，但 50 次 DF-0003 stress 未再复现失败；当前按已进入默认门禁的概率性/时序敏感候选保留回归。
- `make test` 首次升级外部命令门禁时，bash delayed-input harness 会在逐字转发 guest 输出到当前终端的路径上复现截断：日志到达 `wait4 child handoff` 并开始目录输出，但缺失 `lost+found` 和 `user exit status=0`，且 prompt 后可见 `ESC[6n` 交织；同一输入在 DF-0003 Python runner 的非 live-forwarding drain 下 50/50 成功。当前定位为 host harness 输出 drain/终端交织问题：默认 timeout 对齐 DF-0003 为 120s 量级，输出先 drain 到 log 再回放，成功仍以完整 marker 判定。
- 修复把 PID1 plain child 与 observed shell grandchild 分成两个生命周期：前者 status copyout 成功后出队并回到 Prepared，下一 child 使用递增 pid 复用 internal task ref；后者仍由 Ready/enqueued 的 `UserChild` 承载 shell，只清理该轮 grandchild snapshot/wait/exit facts。对象 smoke 已验证同一 observed shell 顺序创建第二 child 时 shell pid/parent/tgid 和 runqueue 可见性保持、child pid 递增。
- 当前回归由历史命名的 `df-0003-distro-sh-ls` case 重复 `TEST=scripted-shell`，显式目标输入为
  `echo "OK"\nls\nls /lib\nls /\nexit\n`；不得通过更换为其它输入命令形态、伪造 ANSI cursor-status
  response 或新增测试专用 kernel API 试修。

后续回归建议：

- 保留 `df-0003-distro-sh-ls` stress identity 作为历史缺陷名；多轮执行 `scripted-shell`，等待 BusyBox
  prompt 后按顺序写入 `echo "OK"\nls\nls /lib\nls /\nexit\n`。
- 成功分类必须观察到完整命令顺序、独立 `OK` 输出、`ld-musl-riscv64.so.1`、两次 `lost+found`
  和 `user exit status=0`；basic acceptance 负责精确计数并禁止 unsupported/panic marker，classifier
  不得仅因命令回显包含 `OK` 就判成功。fork/wait4/execve 边界由 checkpoint/KUnit facts 覆盖。
  timeout、panic、非零退出或缺少上述任一证据均归为失败样本。
- 若 stress 捕获失败样本，先比较成功/失败事件序列并定位最后完整 syscall/output/exit 边界，再决定是否补 `user-syscall-trace`、`user-read-trace` 或新的长期 checkpoint；不得从外部截断症状直接猜修。

### DF-0002: `make run APP=smoke` 间歇性 `InitcallPhase` ready 失败

- 状态：已复现，已由结构化 failure diagnostic 收敛到 UART/PLIC IRQ cycle 与 PLIC source 观测失败；近期 `stress-mem` 样本确认 zero-claim/loop-exit 与全局 claim/complete equality 属过强观察约束，规格和实现已改为以 source-scoped claim/complete delta 为主判据，当前转入保留 stress 回归观察。
- 首次记录日期：2026-06-25。
- 关联范围：`impl/arceos_ex` initcall 边界、UART/TTY initcall 路径、checkpoint/probe 时序。
- 表现：普通 `timeout 90s make run APP=smoke` 曾返回 0，但启动日志输出 `arceos_ex initcall event failed` / `error=C event=S actual=B expected=B target=R`。失败点在 `InitcallPhase` 标记 ready 时，说明 `initcall_phase_ready(...)` 在 `INITCALL_PHASE_STATE` 从 Base 推进到 Ready 前返回 false。

已完成检查：

- 同轮 `make verify`、`make verify REPORT=graph` 和 focused tool tests 均通过，说明规格/图生成本身未暴露确定性失败。
- 使用 `PROBE=uart-irq-chain` 的 smoke run 完整通过 `53/53`，并能输出 UART/TTY 相关诊断计数，例如 `tty_xmit_fifo_probe_ready=1`。
- 后续普通 `make run APP=smoke` 也曾完整通过 `53/53`，因此当前不是稳定可复现失败。
- 对比失败和成功日志时观察到 `.initcall.device` 中 `virtio_mmio` 与 `ns16550a` 的打印顺序不同：失败样本中 `virtio_mmio` 早于 `ns16550a`，成功样本中 `ns16550a` 早于 `virtio_mmio`。这只记录为相关现象，不作为根因结论。
- 2026-06-27 新增普通 smoke 压力测试入口 `impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml`，命令固定为 `make run APP=smoke`，不使用 probe 变体。首轮 30 次运行曾被归类为 30 次 `unknown-failure`，报告在 `/tmp/lkm-stress-out/20260627T014736Z-df-0002-smoke-initcall/report.md`，但代表日志均为 `result: ok. passed=54 failed=0 total=54`；根因是 stress classifier 未在 success/failure 文本匹配前剥离 ANSI 颜色码，属于压力测试工具缺口，不是 DF-0002 复现。
- 修正 stress classifier 的 ANSI 归一化后，重新执行 DF-0002 stress 30 次，完成 30 次、成功 30 次、失败 0 次，全部归入成功序列 `0353704911c8c2bc`；报告在 `/tmp/lkm-stress-out/20260627T015027Z-df-0002-smoke-initcall/report.md`。
- 2026-06-27 在提交 `b543f1b` 后追加 DF-0002 stress 200 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 200 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 200 次、成功 199 次、DF-0002 `InitcallPhase` ready 失败 1 次。失败样本为 `run-0045`，事件序列 `3a657270b6fb3842`，成功序列仍为 `0353704911c8c2bc`；报告在 `/tmp/lkm-stress-out/20260627T020546Z-df-0002-smoke-initcall/report.md`。
- 本轮 `run-0045` 失败日志在 `late_smoke_initcall` 后输出 `arceos_ex initcall event failed` 和 `error=C event=S actual=B expected=B target=R`；按当前事件提取，失败与成功的 common prefix 长度仍为 0，说明当前 stdout 事件只能看到最终症状，尚不能定位 `initcall_phase_ready(...)` 内部第一个 false predicate。失败样本和代表成功样本中已打印的 initcall 顺序同为 `virtio_mmio_platform_driver_init` 早于 `ns16550a_platform_driver_init`，因此该打印顺序不再能作为本轮差异点。
- 2026-06-27 按“先规格、再实现”的流程补充 predicate-level 诊断约束：`spec/model/phases/smp-runtime/initcall/phase.spec` 要求 `initcall_phase_ready(...)` 失败报告第一个失败 predicate，`spec/coding/phases/smp-runtime/initcall.md` 固定结构化输出格式为 `ready_check_failed phase=InitcallPhase check=initcall_phase_ready first_failed=<stable-predicate-name>`。实现侧把原聚合 ready-check 拆为同序诊断函数，并让 stress runner 将 `ready_check_failed` 作为事件点纳入序列 token。下一次 DF-0002 复现时，应优先查看该事件的 `first_failed` 字段，而不是继续只根据 `error=C event=S actual=B expected=B target=R` 推断。
- 2026-06-27 提交 `5e75f00` 后重新执行 DF-0002 stress 200 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 200 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 200 次、成功 200 次、失败 0 次，全部归入成功序列 `0353704911c8c2bc`；报告在 `/tmp/lkm-stress-out/20260627T023942Z-df-0002-smoke-initcall/report.md`。同轮检索未发现 `ready_check_failed`、`arceos_ex initcall event failed`、`error=C event=S` 或 failed smoke result，因此本轮没有捕获可分析的 `first_failed` 样本。
- 2026-06-27 追加普通 DF-0001 `user-boot` stress 500 次作为交叉复测：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 497 次、DF-0002 `InitcallPhase` ready 失败 3 次，失败样本为 `run-0049`、`run-0128`、`run-0294`，失败序列仍为 `3a657270b6fb3842`；报告在 `/tmp/lkm-stress-out/20260627T030109Z-df-0001-user-boot/report.md`。这些失败样本均发生在 `late_smoke_initcall` 后、进入 `user boot start` 前，且没有输出 `ready_check_failed` 行。
- 2026-06-27 追加普通 DF-0002 smoke stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 500 次、失败 0 次，全部归入成功序列 `0353704911c8c2bc`；报告在 `/tmp/lkm-stress-out/20260627T031309Z-df-0002-smoke-initcall/report.md`。同轮检索未发现 `ready_check_failed`、`arceos_ex initcall event failed`、`error=C event=S`、`read user ELF failed` 或 failed smoke result。
- 2026-06-27 提交 `beaab87` 后，把原 ready-check 局部诊断扩展为 `EventError` 携带的结构化 `failure_diagnostic`，覆盖 `InitcallPhase` 的 `setup_objects()` 后半段、`InitcallBoundary.setup()` 和 `checkpoint_ready()`。该信息不是新的 checkpoint；它只在失败路径输出，并保留原有 `error=C event=S actual=B expected=B target=R` 行用于兼容旧分类。
- 2026-06-27 在提交 `beaab87` 后重新执行 DF-0001 `user-boot` stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 498 次、DF-0002 `InitcallPhase` ready 失败 2 次，失败样本为 `run-0302` 和 `run-0431`；报告在 `/tmp/lkm-stress-out/20260627T035519Z-df-0001-user-boot/report.md`。本轮没有复现 `read user ELF failed`，两次失败均携带同一诊断：`phase=InitcallPhase step=setup_objects.uart_interrupt_chain_probe.setup object=UartInterruptChainProbe check=uart_interrupt_chain_probe.setup first_failed=uart_interrupt_chain_probe.setup`。
- 2026-06-27 在提交 `beaab87` 后重新执行 DF-0002 smoke stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 498 次、DF-0002 `InitcallPhase` ready 失败 2 次，失败样本为 `run-0262` 和 `run-0325`；报告在 `/tmp/lkm-stress-out/20260627T040752Z-df-0002-smoke-initcall/report.md`。两次失败同样携带 `setup_objects.uart_interrupt_chain_probe.setup` / `UartInterruptChainProbe` 诊断，说明 DF-0001 与 DF-0002 两个 stress case 都能触发同一个第一层失败点。
- 2026-06-27 按“先规格、再实现”的流程把 `UartInterruptChainProbe.setup` 内部 failure diagnostic 写入 `spec/model/phases/interrupt/irq-time-init/phase.spec` 和 `spec/coding/phases/interrupt/irq-time-init.md`，实现侧按长期稳定名称报告第一失败事实。该扩展仍然是 `EventError` payload，不是新增 checkpoint，也不改变成功路径事件序列。
- 同轮验证：`make verify` 通过，`make -C impl/arceos_ex build APP=smoke` 通过，普通 `make -C impl/arceos_ex run APP=smoke` 通过 `54/54`，`python3 -m unittest test_runner` 通过 8 项。
- 2026-06-27 扩展后重新执行 DF-0001 `user-boot` stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 499 次、DF-0002 `InitcallPhase` ready 失败 1 次，失败样本为 `run-0049`；报告在 `/tmp/lkm-stress-out/20260627T044327Z-df-0001-user-boot/report.md`。本轮没有复现 `read user ELF failed`，失败诊断细化为 `first_failed=uart_interrupt_chain_probe.plic_claim_observed`。
- 2026-06-27 扩展后重新执行 DF-0002 smoke stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 498 次、DF-0002 `InitcallPhase` ready 失败 2 次，失败样本为 `run-0396` 和 `run-0423`；报告在 `/tmp/lkm-stress-out/20260627T045614Z-df-0002-smoke-initcall/report.md`。`run-0396` 细化为 `first_failed=uart_interrupt_chain_probe.irq_cycle_wait_closed`，`run-0423` 进入后续 `setup_objects.serial8250_rx_batch_loopback_probe.setup`，但该对象内部仍只有粗粒度 `first_failed=serial8250_rx_batch_loopback_probe.setup`。
- 2026-06-27 按“通用观察点，而非单个缺陷临时日志”的原则继续扩展规格和实现：`Serial8250RxLoopbackProbe.setup` 与 `Serial8250RxBatchLoopbackProbe.setup` 失败时必须报告统一 `failure_diagnostic` payload，覆盖 RX runtime、PLIC/domain/registry、logical IRQ/source mapping、stimulus、wait、handler、flip buffer、complete/zero-claim 和 source match 等长期事实；同时把 `UartInterruptChainProbe.setup` 的 wait failure 继续细分到 PLIC claim/complete、IRQ-domain dispatch、handler dispatch、THRI 处理和 last claimed/completed source。若失败涉及中断链路，诊断必须保留 PLIC/IRQ-domain/IRQ-registry 事实，不能预先归因到 UART 单侧。
- 本轮验证：`make verify` 通过，`make -C impl/arceos_ex build APP=smoke` 通过，普通 `make -C impl/arceos_ex run APP=smoke` 通过 `54/54`，`python3 -m unittest test_runner` 通过 8 项。
- 2026-06-27 扩展 RX/PLIC 结构化诊断后重新执行 DF-0001 `user-boot` stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 498 次、DF-0002 `InitcallPhase` ready 失败 2 次；报告在 `/tmp/lkm-stress-out/20260627T052721Z-df-0001-user-boot/report.md`。本轮没有复现 `read user ELF failed`；两个失败样本分别细化为 `first_failed=uart_interrupt_chain_probe.plic_claim_observed` 和 `first_failed=plic.last_claimed_source_uart`，均发生在 `UartInterruptChainProbe.setup`。
- 2026-06-27 同轮重新执行 DF-0002 smoke stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 498 次、DF-0002 `InitcallPhase` ready 失败 2 次；报告在 `/tmp/lkm-stress-out/20260627T053947Z-df-0002-smoke-initcall/report.md`。两个失败样本均为 `first_failed=plic.last_claimed_source_uart`，分别出现在 `UartInterruptChainProbe.setup` 和新增覆盖的 `Serial8250RxLoopbackProbe.setup`；本轮没有再出现 RX batch 粗粒度失败。
- 2026-06-27 按“定位靠观察点，不靠猜”的原则补充 PLIC 侧长期通用观察点：规格要求 PLIC provider 暴露按 source 维度的 `claim_count_for_source(source)`、`dispatch_count_for_source(source)` 和 `complete_count_for_source(source)`；native provider 与 Linux-object provider 均通过同一 provider contract 暴露这些计数。现有 UART/RX/TX wait 与 failure diagnostic 已改为优先比较 source-scoped delta，`last_claimed_source` / `last_completed_source` 降级为辅助线索，避免被后续其它合法中断覆盖后造成误判。`PROBE=uart-irq-chain` observer 已能输出 `plic_uart_source_claim_count`、`plic_uart_source_dispatch_count` 和 `plic_uart_source_complete_count`。
- 2026-06-27 新增 PLIC source-scoped counters 后，按集合化差分思路重新执行普通 DF-0001 `user-boot` stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 500 次、失败 0 次，成功集合退化为单一序列 `bd43ae22bfcc7354`；报告在 `/tmp/lkm-stress-out/20260627T094719Z-df-0001-user-boot/report.md`。同轮检索未发现 `failure_diagnostic`、`ready_check_failed`、`arceos_ex initcall event failed`、`error=C event=S`、`read user ELF failed`、panic 或 failed result。
- 2026-06-27 同轮重新执行普通 DF-0002 smoke stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 500 次、失败 0 次，成功集合退化为单一序列 `0353704911c8c2bc`；报告在 `/tmp/lkm-stress-out/20260627T095942Z-df-0002-smoke-initcall/report.md`。同轮检索未发现 `failure_diagnostic`、`ready_check_failed`、`arceos_ex initcall event failed`、`error=C event=S`、`read user ELF failed`、panic 或 failed smoke result。本轮没有失败类集合可与成功集合做首差异点对比。

当前判断：

- 不应把问题直接归因于 `ns16550a` 与 `virtio_mmio` 的顺序。两者同属 `.initcall.device`，`of_platform` 设备枚举已经完成，后续 driver registration 会扫描现有设备；它们绑定的是不同设备，顺序通常只应改变 probe/log 时序，不应单独破坏后续显式 UART/TTY probe。
- 定位已经从“顶层 ready predicate 失败”继续收敛：DF-0001 触发样本显示 `UartInterruptChainProbe` 已触发 UART THRE cycle 但没有观察到 PLIC claim；DF-0002 触发样本显示一类在等待完整 IRQ cycle 闭合时失败，另一类已推进到后续 `Serial8250RxBatchLoopbackProbe.setup`。因此当前问题更像 UART/PLIC/IRQ cycle 观测或后续 RX batch probe 时序敏感问题，而不是 `checkpoint_ready()` 或 `InitcallBoundary.setup()` 聚合谓词不一致。
- 与 DF-0001 类似，该现象可能受 probe、checkpoint handler 或额外日志影响；在完成整个启动流程的并发、锁/guard、IRQ/task context 与内存顺序回顾前，暂不做重型定位。
- 2026-06-27 当前 stress 结论：普通 `APP=smoke` 30 次未复现，但 200 次复现 1 次 DF-0002，因此不能把它标记为已解决，也不能归档为与 DF-0001 同因后已消除。现有证据仍允许二者属于相邻的 initcall/virtio/block 时序敏感问题，但 DF-0002 至少还有未被 DF-0001 修复完全消除的 ready predicate 缺口或观测缺口。
- 2026-06-27 新增 ready-check 诊断后的 200 次 stress 未复现失败，这只能说明本轮没有捕获到 DF-0002 样本；由于上一轮同样规模曾出现 1/200 失败，且本次改动主要是失败路径诊断和聚合谓词同序重构，不构成针对根因的修复证据，暂不应标记已解决。
- 2026-06-27 进一步细分 diagnostic 并复测后，DF-0001 和 DF-0002 两个 500 次 stress case 仍都能触发 DF-0002 类问题。DF-0001 的原始 `/sbin/init` 读取失败没有复现，仍维持已归档判断；DF-0002 则已经有两个更具体方向：`UartInterruptChainProbe` 的 PLIC claim/IRQ cycle closure，以及 `Serial8250RxBatchLoopbackProbe.setup` 内部粗粒度失败。
- 2026-06-27 扩展 RX/PLIC 结构化诊断并复测后，失败进一步集中到 PLIC/source 观察面：DF-0001 交叉样本出现 `uart_interrupt_chain_probe.plic_claim_observed` 和 `plic.last_claimed_source_uart`，DF-0002 smoke 样本两次均为 `plic.last_claimed_source_uart`，其中一次已经落在 `Serial8250RxLoopbackProbe.setup` 的 RX wait 路径。当前不应把问题预设为 UART 单侧问题；更合理的根因方向是 PLIC source claim/complete、last source 记录语义、root external interrupt delivery 与 8250 IRQ request 之间的同步或观测边界。
- 2026-06-27 新增 PLIC source-scoped counters 后，后续样本应优先看 `plic.source_claim_observed`、`plic.source_dispatch_observed`、`plic.source_complete_observed` 和 `plic.source_claim_complete_delta_matched`。如果这些 source-specific facts 成功而旧的全局 last source 被覆盖，则说明之前的 `plic.last_claimed_source_uart` 属于观察模型过粗；如果 source-specific facts 失败，则问题才真正收敛到 UART source 在 PLIC claim/dispatch/complete 链路中的并发或同步边界。
- 2026-06-27 最新两轮 500 次 stress 均未复现 DF-0002，只能说明本轮未捕获失败类集合；由于此前同规模样本多次复现，且本轮主要是观测模型增强后的回归采样，不构成根因修复证据。后续仍应保留 DF-0001 交叉入口和 DF-0002 smoke 入口，直到捕获 source-scoped `failure_diagnostic` 或有明确实现修复后再用更大样本回归。
- 2026-06-27 `stress-mem` 低扰动路径在 DF-0001 交叉入口捕获到一次 `plic.zero_claim_loop_exit_delta_matched`。复核实现后确认 native PLIC 在 `claim()` 中先记录 zero claim，返回上层后才在 claim loop 中记录 loop exit；两个计数之间存在合法并发采样窗口。因此该样本更像观察模型过强，而不是 PLIC claim loop 未退出。规格和实现已改为要求同一轮 claim loop 中 zero claim 与 loop exit 都被观察到，但不再把两个独立 counter 的瞬时不相等作为失败事实。该调整只消除这一类误判，不代表 DF-0002 根因已解决。
- 2026-06-27 去掉 zero-claim/loop-exit 瞬时 equality 后复跑 `stress-mem`：DF-0001 交叉入口 500/500 成功，报告 `/tmp/lkm-stress-out/20260627T141744Z-df-0001-user-boot-stress-mem/report.md`；DF-0002 smoke 500 次中成功 498 次、失败 2 次，报告 `/tmp/lkm-stress-out/20260627T143020Z-df-0002-smoke-initcall-stress-mem/report.md`。两个失败同属序列 `17bfcba2deb0624a`，`first_failed=plic.claim_complete_delta_matched`。由于该诊断发生在 `source_complete_observed` 之后、`source_claim_complete_delta_matched` 之前，当前证据指向全局 claim/complete counter equality 仍过强；一轮 UART source IRQ cycle 应以 source-scoped claim/complete delta 为闭环主判据，全局 counter 只能作为辅助观测，不能因其它 source 或并发采样窗口扰动阻断当前 source cycle。
- 2026-06-27 去掉全局 claim/complete equality 后重新验证：`make verify`、`make -C impl/arceos_ex build APP=smoke`、`make -C impl/arceos_ex run APP=smoke` 和 `git diff --check` 均通过。随后复跑 `stress-mem` 两个 500 次入口：DF-0002 smoke 500/500 成功，报告 `/tmp/lkm-stress-out/20260627T144807Z-df-0002-smoke-initcall-stress-mem/report.md`；DF-0001 交叉入口 500/500 成功，报告 `/tmp/lkm-stress-out/20260627T150119Z-df-0001-user-boot-stress-mem/report.md`。本轮没有失败类集合可差分；这支持“全局 counter equality 属过强观察约束”的修复判断，但仍应保留后续 stress 回归，直到 source-scoped failure 也长期不再出现。
- 2026-06-27/28 继续追加一轮 `stress-mem` 回归采样：DF-0002 smoke 500/500 成功，报告 `/tmp/lkm-stress-out/20260627T154018Z-df-0002-smoke-initcall-stress-mem/report.md`，总耗时 767.253s，平均 1.533s/次；DF-0001 交叉入口 500/500 成功，报告 `/tmp/lkm-stress-out/20260627T161142Z-df-0001-user-boot-stress-mem/report.md`，总耗时 679.55s，平均 1.358s/次。两轮均未发现 `failure_diagnostic`、`ready_check_failed`、`arceos_ex initcall event failed`、`read user ELF failed`、panic 或真实 timeout 记录，只有 manifest 中的 `timeout_seconds=180` 配置项。本轮进一步支持近期失败属于观察机制过强导致的误报，而不是被测功能路径本身的并发缺陷；仍保留 DF-0001/DF-0002 stress 回归入口用于长期确认。
- 2026-06-28 按第一优先级补齐 `Serial8250RxBatchLoopbackProbe.setup` 内部长期诊断：RX 单字符和批量 loopback 的 wait/observation 都优先检查 UART source 维度的 claim/complete delta match，并把 `Serial8250RxBatchLoopbackProbe` 的 initcall boundary / phase ready facts 接入 `serial8250_rx_batch_loopback_probe.source_claim_complete_delta_matched`。该诊断继续复用统一 `failure_diagnostic` payload，不新增 checkpoint 或成功路径输出。本轮验证后追加短采样：DF-0002 smoke `stress-mem` 20/20 成功，报告 `/tmp/lkm-stress-out/20260628T032529Z-df-0002-smoke-initcall-stress-mem/report.md`；DF-0001 交叉入口 `stress-mem` 20/20 成功，报告 `/tmp/lkm-stress-out/20260628T032651Z-df-0001-user-boot-stress-mem/report.md`。
- 2026-06-28 补齐 batch first-failed predicate 后执行正式 `stress-mem` 复测：DF-0002 smoke 500/500 成功，报告 `/tmp/lkm-stress-out/20260628T033737Z-df-0002-smoke-initcall-stress-mem/report.md`，总耗时 799.086s，平均 1.597s/次；DF-0001 交叉入口 500/500 成功，报告 `/tmp/lkm-stress-out/20260628T035123Z-df-0001-user-boot-stress-mem/report.md`，总耗时 724.841s，平均 1.449s/次。两轮均为单一成功序列，failure=0，没有失败类集合可对比；该结果进一步支持近期 DF-0002 失败来自已修正的观察约束和诊断缺口，但仍按长期 intermittent 回归保留入口，暂不作为根因级归档关闭证据。

下一步定位建议：

- 下一轮不应增加缺陷专用 checkpoint；若需要继续提高可见性，也应沿现有 `failure_diagnostic` 增强通用事实。当前第一优先级是分析为什么同一条 UART IRQ/RX wait 链路会出现 `plic_claim_observed` 或 `plic.last_claimed_source_uart` 失败。
- 对 `UartInterruptChainProbe` 和 RX loopback 方向，下一步不是重复拆入口条件，而是结合 Linux-like 8250/PLIC 流程检查 THRI/RX request、LSR/IIR 状态、root external interrupt delivery、PLIC pending/claim/complete、source claim/complete 记录、handler 清 THRI/RX drain 和等待循环边界。
- 复核 `UartInterruptChainProbe` 与 `InitcallPhase` ready 约束之间的关系：若该 probe 是进入 ready 的必要长期条件，应在规格中明确；若它只是 smoke/probe 观测辅助，则需要重新定义它失败时是否应阻断 `InitcallPhase`。
- 后续回顾 UART/TTY、IRQ、task context 和 initcall 并发关系时，把本缺陷作为固定样本回归验证。
- 后续 nightly/stress 至少保留普通 `make run APP=smoke` 的 DF-0002 case，同时继续保留普通 `make run APP=user-boot` 作为交叉触发入口。若后续仍出现 DF-0002，优先查看 `failure_diagnostic` / `ready_check_failed` 中的 `plic.source_claim_observed`、`plic.source_dispatch_observed`、`plic.source_complete_observed`、`plic.source_claim_complete_delta_matched` 和 `serial8250_rx_batch_loopback_probe.source_claim_complete_delta_matched`，再决定是否需要继续扩展通用事实。

## 已归档

### DF-0001: `make run APP=user-boot` 间歇性无法读取 `/sbin/init`

- 状态：已解决，保留 nightly/stress 回归。
- 首次记录日期：2026-06-24。
- 关联范围：`impl/arceos_ex` 用户态启动、VFS/ext2 path read、virtio-blk live read completion。
- 表现：普通 `make run APP=user-boot` 偶发输出 `read user ELF failed` 后关机；同一构建和同一 disk 下也可能成功输出 `user hello` 与 `user exit status=0`。
- 重要现象：`PROBE=user-boot` 或 `PROBE=virtio-blk` 常能改变时序并成功完成 `/sbin/init` 读取与用户态执行，因此 probe 结果不能单独证明普通路径稳定。

已完成检查：

- `make test` 通过，summary 为 `overall total=76 pass=76 fail=0`。
- `make run` 通过，输出 `Hello, world!`。
- 旧 disk 已备份到 `/tmp/lkm-virtio-blk.before.raw`，旧 disk 与重新生成的新 disk 均通过 `e2fsck -n`，状态为 clean。
- 新旧 raw 镜像 SHA256 不同，但差异主要来自 ext2 UUID、时间戳等生成元数据。
- 新旧 `/sbin/init` 内容一致：size 为 `7640`，文件 SHA256 为 `7e0fc592f5c05873d853847391c09961e2acf672b230028ccdfa5c57f8dfdacc`。
- 导出 rootfs 后比较，84 个普通文件 SHA256 一致，334 个 symlink 目标一致。
- 显式使用旧 disk 运行也不是稳定失败；重新生成的新 disk 连续成功多次后仍出现过普通 `make run APP=user-boot` 失败。
- 2026-06-25 在提交 `62ce523 use cpu owned runqueue refs` 后重新执行完整基线验证：`make build APP=smoke` 通过，`make test` summary 为 `overall total=77 pass=77 fail=0`，`make verify` 为 `0 obligation`，普通 `make run` 的 smoke 为 `53/53`。
- 同日连续执行普通 `make run APP=user-boot` 30 次，其中 24 次成功输出 `user exit status=0`，5 次输出 `read user ELF failed`，1 次输出 `arceos_ex initcall event failed` / `error=C event=S actual=B expected=B target=R`。粗略发生率为：`read user ELF failed` 约 16.7%，任意 user-boot 失败约 20%。本轮日志保存在 `/tmp/lkm-userboot-runs/run-01.log` 到 `/tmp/lkm-userboot-runs/run-30.log`，失败样本为 `run-11.log`、`run-12.log`、`run-13.log`、`run-14.log`、`run-17.log` 和 `run-18.log`。
- 同日推进 `SchedInitPhase` trimmed/no-op 边界时，完整验收中的普通 `make run APP=user-boot` 连续两次输出 `read user ELF failed`，第三次在同一构建和同一 disk 下成功输出 `user hello` / `user exit status=0`。同轮 `make verify`、`make verify REPORT=graph`、`make -C impl/arceos_ex build APP=smoke`、`make -C impl/arceos_ex run APP=smoke` 和普通 `make run` 均通过。
- 2026-06-27 使用 `impl/arceos_ex/tests/stress/runner.py` 针对 DF-0001 执行压力测试，先运行 `--runs 10 --timeout 180`，再追加 `--runs 20 --timeout 180`，总计 30 次。第一批结果为成功 6 次、DF-0001 `read user ELF failed` 3 次、DF-0002 initcall ready failure 1 次；第二批结果为成功 13 次、DF-0001 `read user ELF failed` 6 次、rootfs phase unknown failure 1 次。两批合计成功 19 次、DF-0001 失败 9 次、非 DF-0001 失败 2 次。报告分别保存在 `impl/arceos_ex/tests/stress/out/20260626T162257Z-df-0001-user-boot/report.md` 和 `impl/arceos_ex/tests/stress/out/20260626T162352Z-df-0001-user-boot/report.md`。
- 同轮压力测试中，DF-0001 失败全部归到同一个事件序列 `deff92e2396ec2f6`，成功全部归到同一个事件序列 `bd43ae22bfcc7354`。当前事件视角下二者共同前缀长度为 0：失败侧第一个可见事件是 `symptom:ReadUserElfFailed`，成功侧第一个可见事件是 `user_output:UserHello`。代表日志显示成功和 DF-0001 失败在 `late_smoke_initcall` 之前基本同形，差异出现在 `arceos_ex user boot start` 之后。
- 2026-06-27 在提交 `b4e600e` 后重新运行普通 DF-0001 压力测试。第一次执行因 `/mnt/d/gitHome` 与 `/mnt/d/githome` 大小写路径在沙箱内被判为只读，10 次均为构建期 `nonzero-exit`，不是 DF-0001 样本；随后在沙箱外显式输出到 `/tmp/lkm-stress-out`，执行 `--runs 10 --timeout 180` 和追加 `--runs 20 --timeout 180`。有效 30 次样本中，成功 22 次、DF-0001 失败 7 次、rootfs phase unknown failure 1 次；报告在 `/tmp/lkm-stress-out/20260627T004632Z-df-0001-user-boot/report.md` 和 `/tmp/lkm-stress-out/20260627T004752Z-df-0001-user-boot/report.md`。
- 新样本中 DF-0001 仍全部归到 `deff92e2396ec2f6`，成功仍全部归到 `bd43ae22bfcc7354`。Boot HART 0 上既有失败也有成功，非 0 HART 上也有成功，因此 Boot HART 不是充分原因。失败日志同样在 `late_smoke_initcall` 后、`arceos_ex user boot start` 后折叠为 `read user ELF failed`。
- 2026-06-27 按 Linux-like 同步请求生命周期补齐规格后实现修复候选：initcall 首个 ext2 superblock read 改为 submit-and-wait 收束，live read 进入同步读前会主动收束 inherited pending request，IRQ 与 task-poll completion 通过单一 owner 避免同时消费同一 pending token，并为 virtqueue avail publish / MMIO notify / used-ring observe 补入 release/acquire 顺序边界。验证结果：`make verify` 通过，`make -C impl/arceos_ex build APP=smoke` 通过，`make -C impl/arceos_ex run APP=user-boot` 输出 `user hello` / `user exit status=0`。随后执行 DF-0001 stress 10 次和追加 20 次，总计 30/30 成功、0 失败；报告在 `/tmp/lkm-stress-out/20260627T011328Z-df-0001-user-boot/report.md` 和 `/tmp/lkm-stress-out/20260627T011357Z-df-0001-user-boot/report.md`。
- 2026-06-27 提交 `1ba2bf8` 后追加普通 DF-0001 stress 200 次：`impl/arceos_ex/tests/stress/runner.py --runs 200 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 200 次、成功 200 次、失败 0 次，全部归入成功序列 `bd43ae22bfcc7354`；报告在 `/tmp/lkm-stress-out/20260627T012229Z-df-0001-user-boot/report.md`。
- 2026-06-27 提交 `a26af65` 后追加普通 DF-0001 stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 497 次、失败 3 次；报告在 `/tmp/lkm-stress-out/20260627T030109Z-df-0001-user-boot/report.md`。本轮没有复现 `read user ELF failed`，成功样本仍全部归入 `bd43ae22bfcc7354`；3 次失败均归类为 DF-0002 `InitcallPhase` ready failure，序列 `3a657270b6fb3842`，样本为 `run-0049`、`run-0128`、`run-0294`。
- 2026-06-27 提交 `beaab87` 后追加普通 DF-0001 stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 498 次、失败 2 次；报告在 `/tmp/lkm-stress-out/20260627T035519Z-df-0001-user-boot/report.md`。本轮仍没有复现 `read user ELF failed`；2 次失败均归类为 DF-0002 `InitcallPhase` ready failure，并由结构化诊断定位到 `setup_objects.uart_interrupt_chain_probe.setup` / `UartInterruptChainProbe`。
- 2026-06-27 扩展 `UartInterruptChainProbe.setup` 内部 failure diagnostic 后追加普通 DF-0001 stress 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 499 次、失败 1 次；报告在 `/tmp/lkm-stress-out/20260627T044327Z-df-0001-user-boot/report.md`。本轮仍没有复现 `read user ELF failed`；1 次失败归类为 DF-0002 `InitcallPhase` ready failure，并细化为 `first_failed=uart_interrupt_chain_probe.plic_claim_observed`。
- 2026-06-28 在 DF-0002 观察约束修正后追加低扰动 DF-0001 `stress-mem` 交叉回归 500 次：`impl/arceos_ex/tests/stress/runner.py impl/arceos_ex/tests/stress/cases/df-0001-user-boot-stress-mem.toml --runs 500 --timeout 180 --out-dir /tmp/lkm-stress-out`，完成 500 次、成功 500 次、失败 0 次；报告在 `/tmp/lkm-stress-out/20260627T161142Z-df-0001-user-boot-stress-mem/report.md`。本轮成功分类为 `user-boot-success`，序列摘要为 `UserHello` 与 `UserExitStatus:status=0`，没有复现 `read user ELF failed`，也没有 DF-0002 类 initcall ready failure。

当前判断：

- 不应把该问题归因于 rootfs/disk 内容损坏。
- 更可能是普通 `user-boot` 路径中的时序敏感问题，候选方向包括 virtio-blk live read 的 completion polling、VFS/ext2 读路径错误传播不足，或启动后第一次用户态 payload 读取时的设备状态边界。
- 需要重点保留一种并发/同步假设：该问题可能来自中断上下文与任务上下文之间的协作缺口。virtio-blk completion、virtqueue used ring 更新、IRQ handler、VFS/ext2 同步读路径之间都存在跨上下文状态传递；这类问题通常具有随机性，并且容易被 probe 输出、checkpoint handler 或额外日志改变时序后掩盖。
- 当前 Linux 对照补缺口尚未完成全部锁和并发原语检查，因此不能只按轮询参数或 disk 生成问题处理。后续推进到 ext2/VFS、virtio-blk、virtio IRQ、block layer 或相关 guard/lock 规格阶段时，必须回顾本缺陷，看新增的并发控制规格和实现是否解释或消除该现象。
- 2026-06-25 结论：本条暂时只作为测试现象积累，不在当前轮展开深入追踪；待并发机制、锁/guard、IRQ/task context 与内存顺序等回顾检查完善后，再把这些样本作为后续定位参考。
- 2026-06-27 压力测试结论：新 stress 框架已经能够稳定复现和归类 DF-0001，但当前普通日志/checkpoint 只能看到折叠后的外部症状，尚不足以定位第一个内部差异点。`read_user_path_image()` 将 `VfsCore::read_path()` 的任意错误统一折叠为 `read user ELF failed`，而实际调用链还会经过 `VfsCore::open_path/walk_path`、`Ext2FileSystem::lookup_child/read_file_inode`、`bio::sb_bread_by_devt_block`、`BlockDeviceRegistry::read_device` 和 `VirtioBlkLiveProvider::read_live_block`。偶发的 rootfs phase unknown failure 也指向相邻的 rootfs/ext2/block 读链路，说明下一步应优先提高这条长期关键路径的内部可见性，而不是只给 `user-boot` 临时打点。
- 2026-06-27 代码审阅后的当前根因候选收敛到 virtio-blk live read 的 completion 收束。`setup_live_driver()` 在 initcall 中提交 `submit_ext2_superblock_read()` 后没有同步等待完成，只依赖后续 IRQ 及时调用 `handle_irq_completion()` 消费 used ring。之后 rootfs 阶段和 user-boot 阶段都会通过 `VirtioBlkLiveProvider::read_live_block()` 进入同步读；该函数先执行 `wait_for_no_pending_read()`，只忙等 `read_request_pending == false`，不会主动 poll/complete 已遗留的 pending request。因此若 initcall 首个 superblock read 的 IRQ completion 尚未及时收束，后续 rootfs 或 `/sbin/init` 读取会在进入自身请求提交前失败，并被上层折叠为 rootfs phase failure 或 `read user ELF failed`。
- 同一段代码还存在第二个同步风险：任务侧 `wait_for_completion_after()` 会直接调用 `poll_read_completion()` 消费 used ring，而 IRQ 侧 `handle_irq_completion()` 也会调用 `complete_read_from_irq()` 消费同一个 `VirtioBlkDevice` / `VirtQueue` / 静态读缓冲。两条路径之间没有互斥、pending token 原子状态或明确内存序；virtqueue 发布 descriptor/avail idx 后也未显式建立 notify 前的 fence，读取 used idx 前只有 volatile 读。这些都与“probe/trace 改变时序后问题消失”的现象一致。
- 2026-06-27 修复验证结果支持上述根因判断：在消除 fire-and-forget 首读、inherited pending 等待和双 completion consumer 后，原先 30 次中可出现 7 次 DF-0001 的普通 stress 样本变为 30/30 成功；提交 `1ba2bf8` 后追加 200 次普通 stress 继续保持 200/200 成功。因此 DF-0001 按当前证据判定已解决，后续只作为 nightly/stress 回归项保留；若同类现象再次出现，应按回归重新打开。
- 2026-06-27 多轮 500 次复测进一步增强了“原始 `/sbin/init` 读取失败已消除”的证据：这些复测均为 0 次 `read user ELF failed`。同轮出现的 DF-0002 类 initcall ready failure 已由后续结构化诊断定位到 UART/PLIC IRQ cycle 或后续 serial8250 RX probe；该现象归入 DF-0002 继续处理，不改变 DF-0001 当前归档状态。

后续回归建议：

- 按 `spec/charter/main.md` 中“内部可见性与 checkpoint 规格化”的分工，继续把 VFS/ext2/block/virtio 读链路的长期观察点写入 `spec/model` 和 coding 规格，再由实现生成结构化 trace/checkpoint。DF-0001 后续回归观察重点是：`read_user_path_image()` 或其下层 VFS/block provider 的错误分类，以及 KernelInitTask 发起 block I/O、进入等待、观察 completion 继续执行，与 InterruptType 开始/结束处理 completion 之间的同步关系。
- 已完成修复并转入回归：initcall 首读收束、live read inherited pending 主动收束、IRQ/task-poll 单 owner 和 virtqueue release/acquire 顺序边界已经纳入规格和实现。
- 若后续 stress 再出现失败，应优先补充 virtio-blk live read 的失败分支诊断或独立 debug counter，重点观察 request submitted、notify、used idx、pending 状态、completion count、completion source，以及同步等待超时前的 pending sector。
- 在继续补齐 ext2/VFS 与 virtio-blk/IRQ 相关锁、guard、memory ordering、IRQ/task context 约束时，把本缺陷作为固定回归问题重新验证。
- 补充长期压力测试，分别覆盖 virtio-blk/block 设备层和 ext2/VFS 文件系统层：
  - virtio-blk/block 层测试应持续通过 block/bio/buffer-head 公开路径读取固定 sector、跨 sector 范围和重复队列提交场景，观察 completion、pending、used idx、status、读计数和错误返回。
  - ext2/VFS 层测试应持续通过 path read/open/read 路径读取 `/sbin/init`、interpreter、跨 block 文件和多级目录文件，观察 lookup、inode、direct/indirect block read、buffer copy 和错误传播。
  - 这些压力测试既用于提高间歇性问题复现概率，也必须作为修复后的回归测试保留。
- 后续回归仍必须包含普通 `make run APP=user-boot`，不能只用 `PROBE=user-boot` 或 `PROBE=virtio-blk` 代替。
