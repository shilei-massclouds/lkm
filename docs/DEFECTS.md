# Defect Tracking

本文记录尚未闭环的实现缺陷和间歇性问题。规划性任务仍放在 `docs/ROADMAP.md`；这里优先记录可复现现象、证据、当前判断和下一步定位方向。

## Open

### DF-0001: `make run APP=user-boot` 间歇性无法读取 `/sbin/init`

- 状态：待查。
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

当前判断：

- 不应把该问题归因于 rootfs/disk 内容损坏。
- 更可能是普通 `user-boot` 路径中的时序敏感问题，候选方向包括 virtio-blk live read 的 completion polling、VFS/ext2 读路径错误传播不足，或启动后第一次用户态 payload 读取时的设备状态边界。
- 需要重点保留一种并发/同步假设：该问题可能来自中断上下文与任务上下文之间的协作缺口。virtio-blk completion、virtqueue used ring 更新、IRQ handler、VFS/ext2 同步读路径之间都存在跨上下文状态传递；这类问题通常具有随机性，并且容易被 probe 输出、checkpoint handler 或额外日志改变时序后掩盖。
- 当前 Linux 对照补缺口尚未完成全部锁和并发原语检查，因此不能只按轮询参数或 disk 生成问题处理。后续推进到 ext2/VFS、virtio-blk、virtio IRQ、block layer 或相关 guard/lock 规格阶段时，必须回顾本缺陷，看新增的并发控制规格和实现是否解释或消除该现象。
- 2026-06-25 结论：本条暂时只作为测试现象积累，不在当前轮展开深入追踪；待并发机制、锁/guard、IRQ/task context 与内存顺序等回顾检查完善后，再把这些样本作为后续定位参考。

下一步定位建议：

- 在不依赖 probe 时序的前提下，让 `read_user_path_image()` 或其下层 VFS/block provider 暴露具体错误类别，区分 lookup、ext2 block read、provider unavailable、empty read 和 completion timeout。
- 为 virtio-blk live read 增加失败分支诊断或独立 debug counter，重点观察 request submitted、notify、used idx、pending 状态和 completion count。
- 复核 `wait_for_completion_after()` / `wait_for_no_pending_read()` 的轮询边界，确认是否存在过早超时或 stale pending 状态。
- 在补齐 ext2/VFS 与 virtio-blk/IRQ 相关锁、guard、memory ordering、IRQ/task context 约束时，把本缺陷作为固定回归问题重新验证。
- 补充长期压力测试，分别覆盖 virtio-blk/block 设备层和 ext2/VFS 文件系统层：
  - virtio-blk/block 层测试应持续通过 block/bio/buffer-head 公开路径读取固定 sector、跨 sector 范围和重复队列提交场景，观察 completion、pending、used idx、status、读计数和错误返回。
  - ext2/VFS 层测试应持续通过 path read/open/read 路径读取 `/sbin/init`、interpreter、跨 block 文件和多级目录文件，观察 lookup、inode、direct/indirect block read、buffer copy 和错误传播。
  - 这些压力测试既用于提高间歇性问题复现概率，也必须作为修复后的回归测试保留。
- 后续修复完成前，完整验证必须包含普通 `make run APP=user-boot`，不能只用 `PROBE=user-boot` 或 `PROBE=virtio-blk` 代替。

### DF-0002: `make run APP=smoke` 间歇性 `InitcallPhase` ready 失败

- 状态：待查，暂缓深入定位。
- 首次记录日期：2026-06-25。
- 关联范围：`impl/arceos_ex` initcall 边界、UART/TTY initcall 路径、checkpoint/probe 时序。
- 表现：普通 `timeout 90s make run APP=smoke` 曾返回 0，但启动日志输出 `arceos_ex initcall event failed` / `error=C event=S actual=B expected=B target=R`。失败点在 `InitcallPhase` 标记 ready 时，说明 `initcall_phase_ready(...)` 在 `INITCALL_PHASE_STATE` 从 Base 推进到 Ready 前返回 false。

已完成检查：

- 同轮 `make verify`、`make verify REPORT=graph` 和 focused tool tests 均通过，说明规格/图生成本身未暴露确定性失败。
- 使用 `PROBE=uart-irq-chain` 的 smoke run 完整通过 `53/53`，并能输出 UART/TTY 相关诊断计数，例如 `tty_xmit_fifo_probe_ready=1`。
- 后续普通 `make run APP=smoke` 也曾完整通过 `53/53`，因此当前不是稳定可复现失败。
- 对比失败和成功日志时观察到 `.initcall.device` 中 `virtio_mmio` 与 `ns16550a` 的打印顺序不同：失败样本中 `virtio_mmio` 早于 `ns16550a`，成功样本中 `ns16550a` 早于 `virtio_mmio`。这只记录为相关现象，不作为根因结论。

当前判断：

- 不应把问题直接归因于 `ns16550a` 与 `virtio_mmio` 的顺序。两者同属 `.initcall.device`，`of_platform` 设备枚举已经完成，后续 driver registration 会扫描现有设备；它们绑定的是不同设备，顺序通常只应改变 probe/log 时序，不应单独破坏后续显式 UART/TTY probe。
- 更可疑的结构性缺口是：`InitcallBoundary.setup()` 接受的条件可能比 `initcall_phase_ready(...)` 更宽，导致 boundary/setup 流程已经推进，但顶层 ready 谓词仍有某个额外条件为 false。当前还未隔离出具体 false predicate。
- 与 DF-0001 类似，该现象可能受 probe、checkpoint handler 或额外日志影响；在完成整个启动流程的并发、锁/guard、IRQ/task context 与内存顺序回顾前，暂不做重型定位。

下一步定位建议：

- 增加仅在 debug/probe 下启用的 predicate-level 诊断，或让 `EventError` 携带失败谓词名，避免只看到 `actual=B expected=B target=R`。
- 复核 `InitcallBoundary` 与 `initcall_phase_ready(...)` 的条件是否应保持完全一致；若有意不同，需要在规格和实现中显式说明边界。
- 后续回顾 UART/TTY、IRQ、task context 和 initcall 并发关系时，把本缺陷作为固定样本回归验证。
