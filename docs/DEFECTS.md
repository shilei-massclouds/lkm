# Defect Tracking

本文记录尚未闭环的实现缺陷和间歇性问题。规划性任务仍放在 `docs/ROADMAP.md`；这里优先记录可复现现象、证据、当前判断和下一步定位方向。

## 待解决

### DF-0002: `make run APP=smoke` 间歇性 `InitcallPhase` ready 失败

- 状态：已复现，待 predicate-level 诊断。
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

当前判断：

- 不应把问题直接归因于 `ns16550a` 与 `virtio_mmio` 的顺序。两者同属 `.initcall.device`，`of_platform` 设备枚举已经完成，后续 driver registration 会扫描现有设备；它们绑定的是不同设备，顺序通常只应改变 probe/log 时序，不应单独破坏后续显式 UART/TTY probe。
- 更可疑的结构性缺口是：`InitcallBoundary.setup()` 接受的条件可能比 `initcall_phase_ready(...)` 更宽，导致 boundary/setup 流程已经推进，但顶层 ready 谓词仍有某个额外条件为 false。当前还未隔离出具体 false predicate。
- 与 DF-0001 类似，该现象可能受 probe、checkpoint handler 或额外日志影响；在完成整个启动流程的并发、锁/guard、IRQ/task context 与内存顺序回顾前，暂不做重型定位。
- 2026-06-27 当前 stress 结论：普通 `APP=smoke` 30 次未复现，但 200 次复现 1 次 DF-0002，因此不能把它标记为已解决，也不能归档为与 DF-0001 同因后已消除。现有证据仍允许二者属于相邻的 initcall/virtio/block 时序敏感问题，但 DF-0002 至少还有未被 DF-0001 修复完全消除的 ready predicate 缺口或观测缺口。

下一步定位建议：

- 增加仅在 debug/probe 下启用的 predicate-level 诊断，或让 `EventError` 携带失败谓词名，避免只看到 `actual=B expected=B target=R`。
- 复核 `InitcallBoundary` 与 `initcall_phase_ready(...)` 的条件是否应保持完全一致；若有意不同，需要在规格和实现中显式说明边界。
- 后续回顾 UART/TTY、IRQ、task context 和 initcall 并发关系时，把本缺陷作为固定样本回归验证。
- 后续 nightly/stress 至少保留普通 `make run APP=smoke` 的 DF-0002 case。下一步第一优先级不是改 initcall 逻辑，而是先记录 `initcall_phase_ready(...)` 内部具体 false predicate，使 failure-vs-success 能对齐到第一个内部差异点；该诊断应作为长期 checkpoint/debug fact 设计，避免一次性临时日志。

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

后续回归建议：

- 按 `spec/charter/main.md` 中“内部可见性与 checkpoint 规格化”的分工，继续把 VFS/ext2/block/virtio 读链路的长期观察点写入 `spec/model` 和 coding 规格，再由实现生成结构化 trace/checkpoint。DF-0001 后续回归观察重点是：`read_user_path_image()` 或其下层 VFS/block provider 的错误分类，以及 KernelInitTask 发起 block I/O、进入等待、观察 completion 继续执行，与 InterruptStream 开始/结束处理 completion 之间的同步关系。
- 已完成修复并转入回归：initcall 首读收束、live read inherited pending 主动收束、IRQ/task-poll 单 owner 和 virtqueue release/acquire 顺序边界已经纳入规格和实现。
- 若后续 stress 再出现失败，应优先补充 virtio-blk live read 的失败分支诊断或独立 debug counter，重点观察 request submitted、notify、used idx、pending 状态、completion count、completion source，以及同步等待超时前的 pending sector。
- 在继续补齐 ext2/VFS 与 virtio-blk/IRQ 相关锁、guard、memory ordering、IRQ/task context 约束时，把本缺陷作为固定回归问题重新验证。
- 补充长期压力测试，分别覆盖 virtio-blk/block 设备层和 ext2/VFS 文件系统层：
  - virtio-blk/block 层测试应持续通过 block/bio/buffer-head 公开路径读取固定 sector、跨 sector 范围和重复队列提交场景，观察 completion、pending、used idx、status、读计数和错误返回。
  - ext2/VFS 层测试应持续通过 path read/open/read 路径读取 `/sbin/init`、interpreter、跨 block 文件和多级目录文件，观察 lookup、inode、direct/indirect block read、buffer copy 和错误传播。
  - 这些压力测试既用于提高间歇性问题复现概率，也必须作为修复后的回归测试保留。
- 后续回归仍必须包含普通 `make run APP=user-boot`，不能只用 `PROBE=user-boot` 或 `PROBE=virtio-blk` 代替。
