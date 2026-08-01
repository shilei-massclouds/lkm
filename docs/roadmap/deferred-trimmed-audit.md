# Deferred / Trimmed 结构化治理审计归档

本专题归档 2026-07-16 对根模型 legacy `deferred` inventory 的一次性全量治理。审计基线为
工具原先报告的 85 个自由文本 entry；此外还发现 11 个位于 type/action 原始块内、未被旧计数器
看到的 legacy block。它们均已逐子句归入正式事实、结构化 deferred、结构化 trimmed、重复合并
或历史说明删除。本专题不维护活跃 backlog；活跃责任以 model boundary ID 为唯一来源。

## 验收结果

- 根模型现有 141 个 deferred、52 个 trimmed，共 193 个可执行 inventory record；数量变化来自
  复合文本拆分，不与旧 85 做一对一保持。
- 每条记录都有全局唯一 ID、受控分类、evidence、`close_when` / `revisit_when`、AST 推导 owner
  和真实 include 源文件行号。
- `make verify` 结果为 `legacy_boundaries: 0`、`obligation: 0`；默认 policy 允许已证明的
  deferred/trimmed，但不允许结构错误或 evidence obligation。
- 本轮没有改内核运行时行为，也没有退休任何既有结构化 ID；被删除的是从未分配稳定 ID 的
  legacy 历史文本。后续关闭结构化 ID 时仍须记录提交和完成证据，且不得复用 ID。

## 第一批：启动、MM、调度和 IRQ

| 原 legacy owner / 子句范围 | disposition |
| --- | --- |
| Entry prelude VM、swapper VM、SoC early platform | 活跃责任拆为 `entry_vm.001`、`swapper_vm.001`、`soc.001`；入口前导已成立事实保留在 ensures/invariant。 |
| MemBlock hugetlb CMA；EFI、build-id、page-address metadata | 活跃责任拆为 `memblock.001`、`boot_init_setup.001`–`.003`；混合调用位置说明删除。 |
| CorePrepare 的 ACPI、memtest、sparse/vmemmap、crashkernel、KASAN、ACPI topology、CBOP、alternatives、RT signal、user ISA、static-call、LSM、bootconfig、boot-CPU hook、extra init args、early VFS cache | 每个子句独立归入 `core_prepare.001`–`.018`；配置/架构/参考输入 no-op 使用 trimmed，其余使用 deferred。setup_nr_cpu_ids、第二次 early-param parse 和 unknown-option printk 等已实现 checkpoint 不进入 inventory。 |
| Page allocator GFP reclaim、compaction、OOM、失败传播 | 拆为 `page_alloc.001`–`.004`。 |
| SLUB reclaim、NUMA、memcg、redzone、freelist randomization、freelist hardening | 拆为 `slub_alloc.001`–`.006`。 |
| GlobalAlloc large allocation、OOM policy、realloc、特殊 alignment | 拆为 `global_alloc.001`–`.004`。 |
| vmalloc reusable holes、augmented tree、lazy purge、RCU metadata、cross-CPU lazy fault、cache/TLB batching、per-CPU deferred free | 去重后拆为 `vmalloc_runtime.001`–`.007`。 |
| PageExt、KFENCE、KMSAN、kmemleak、debug objects、execmem、x86 espfix/PTI | 逐项证明为构建配置、编译期 no-op 或架构裁剪，归入 `mm_core.001`–`.008`。 |
| tracing、housekeeping、scheduler class、workqueue worker、context tracking 及相关 no-op | `sched_init.001`–`.008`；Tasks RCU setup 已由后续正式状态覆盖，删除历史延期描述。 |
| Scheduler/SwitchTo/SelectRunQueue、RunQueue class queue、boot idle action 内的重复文本 | 已实现 cooperative smoke 和环境分界移出 inventory；未闭合部分合并到 `sched_init.004`、`schedule_handoff.001`–`.003`、`boot_idle.001`–`.005`，不在 action 内复制第二套责任。 |
| irq_desc、PMU/breakpoint/profile、IRQ stack、timer、UART RX、TTY、FIFO concurrency 及配置裁剪 | 逐项归入 `irq_time.001`–`.012`。 |
| console handoff、SLUB FULL、lockdep/selftest/initrd/NUMA/ACPI/finalize hooks | 逐项归入 `irq_open.001`–`.011`。 |
| local_irq_enable block | SIE、PLIC/root gate owner 和后续 CPU/task runtime 已由正式阶段事实或上述稳定 ID 覆盖，作为重复记录删除。 |

## 第二批：进程准备、SMP、runtime、driver 和 initcall

| 原 legacy owner / 子句范围 | disposition |
| --- | --- |
| copy_process 的 sighand、tasklist、pidmap、sched_fork/PI、creds/files/fs/signal/mm 引用同步、失败回滚 | 拆为共享 owner `task_creation.001`–`.006`。 |
| ProcessPrepare 的 network namespace、pagecache/folio/writeback、seq/proc/nsfs/pidfs、bdev/chrdev、SignalCore、RootPidNamespace、vm-stack hotplug、key/LSM/pseudo-fs locks | 逐项归入 `process_prepare.001`–`.014`；其余配置/架构路径逐项归入 trimmed `process_prepare.015`–`.025`。out-of-scope 和“后续创建任务”等时序说明移出 inventory。 |
| kthreadd request/service loop、schedule handoff、finite idle loop | 分别归入 `kthreadd.001`、`schedule_handoff.001`–`.003`、`boot_idle.001`–`.005`；已经发布的 task entry/provider facts 不再混入延期文本。 |
| pre-SMP CAD PID、proc vmstat、lockup detector | 拆为 `pre_smp.001`–`.003`；`smp_init()` 已由下一阶段正式驱动，删除重复延期。 |
| AP CPU-local chain、hotplug memory ordering、callback execution | 拆为 `smp_bringup.001`–`.003`；Finalize 对 AP 的泛化描述合并到这些 owner。 |
| async domains/cookies/pending/waitqueue/workers、padata hotplug/work/free list/instances、late page allocator trims | 逐项归入 `runtime_core.001`–`.012`。 |
| driver core 的 BDI、devtmpfs、OF synchronization、auxiliary/memory/node/cpu/container tails | 拆为 `driver_core.001`–`.010`。 |
| `/proc/irq`、default/effective affinity | 拆为 `irq_proc.001`–`.003`。 |
| 同级 initcall 顺序无关性 | 归入 proof boundary `initcall.001`；nightly permutation 是证据手段，不再是第二条责任。 |
| async full-sync lock/wait/atomic/wake ordering | 拆为 `finalize_async.001`–`.004`。 |

## 第三批：rootfs、exec 与 user-mode

| 原 legacy owner / 子句范围 | disposition |
| --- | --- |
| device-probe wait、MD、root name/device variants、rootwait、initrd、NFS/CIFS、devtmpfs、ext2 root 后 `/dev` | 逐项归入 `rootfs.001`–`.010`；其中 reference-input/build-config 路径为 trimmed。ext4-for-ext2 的阶段性实现事实移出 inventory。 |
| IMA/EVM key loading | 分别归入 trimmed `integrity_keys.001`、`.002`。 |
| exec locks、credential/signal/LSM hooks、namespace/accounting、binfmt module retry | 拆为 `exec_sync.001`–`.004`；module retry 由 `CONFIG_MODULES=n` 证明为 trimmed。 |
| 已退役 wrapper 中实际存在的 transition/action facts | 从错误的 deferred block 移回 ensures/invariant，不保留 active responsibility。 |

### UserCloneDeferredBoundaries 复合记录逐项 disposition

旧 `user_boot.spec` 复合字符串同时包含已实现诊断历史和真实剩余责任，已按下列规则拆分：

| 旧子句 | disposition |
| --- | --- |
| legacy clone ABI/CSIGNAL、plain fork、newsp/TLS 首片 | 已实现；由正式 clone facts 和测试承载，删除历史叙述。 |
| bounded wait4、parent frame/snapshot、writable-page rollback、ECHILD | 已实现首片；由 wait/rollback ensures、coding 和 user-mode 归档承载。 |
| dynamic child Task identity、completed record/archive/release、递增 PID、有限 pidfd-like 可读性 | 正式模型使用 fresh Task；实现中的有界 record storage 只按 generation 承载记录，不定义单 child identity；从 active inventory 删除。 |
| observed pending-child `setpgid` / `TIOCSPGRP`、login nested takeover、plain-fork grandchild continuation | 已完成并有 user-mode 专题证据；从 active inventory 删除。 |
| complete clone3 | `user_clone.001`。 |
| thread group / multi-thread lifecycle | `user_clone.002`。 |
| complete CLONE_VM/vfork completion scheduling | `user_clone.003`。 |
| complete dup_mm/COW address spaces | `user_clone.004`。 |
| complete pidfs/pidfd operations | `user_clone.005`。 |
| ptrace、seccomp、cgroup、audit hooks | 分别为 `user_clone.006`–`.009`，不再共用过宽谓词。 |
| namespaces | `user_clone.010`。 |
| robust futex | `user_clone.011`。 |
| clear_child_tid store/futex wake | `user_clone.012`。 |
| general wait sleep/wakeup | `user_clone.013`。 |
| general task graph/zombie/reap/PID hash/accounting | `user_clone.014`。 |
| full session/controlling-TTY/process-group/PTY/job-control signals | `user_clone.015`；已完成的 bounded pending-child job-control 不再混入。 |
| unobserved/unsupported clone flags and wait options | `user_clone.016`。 |

## 后续关闭规则

关闭任何上述 deferred ID 时，同一变更必须删除 active boundary、补齐正式 facts/实现/测试，并在
对应专题归档原 ID、完成证据和提交。触发 trimmed 的 `revisit_when` 时先重新审计：仍不可达则更新
证据，变为实现责任则转为新的 deferred 语义；任何情况下都不得复用已退休 ID。
