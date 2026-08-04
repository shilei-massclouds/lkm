# 用户缺页、独立 mm、COW 与 SIGSEGV 执行边界

> 当前优先级、状态和下一步顺序只由 [`../ROADMAP.md`](../ROADMAP.md) 维护。本文保存已完成基线、
> 后续语义边界、执行顺序和验收清单，不维护第二套活跃优先级表。

## 已完成基线：第一与第二阶段

第一阶段已把普通用户缺页统一到 Task/mm-aware VMA fault core：请求显式携带 TaskRef、SATP、fault
address、`sepc` 和 access；成功分配/映射后执行目标 TLB flush 并返回同指令重试，kernel extable
分支保持隔离。

第二阶段在 COW 前建立语义完整的 eager 过渡基线：每个普通 fork child 拥有独立
`UserAddressSpace`、根页表、SATP、用户栈与已驻留私有 backing page；未驻留 VMA 继续保留 hole。
child 只在 mm、PTE 和 fd 准备全部成功后发布，失败会释放全部保留槽位和页而不改变 parent。调度、
trap 和 usercopy 按 TaskRef/owner/SATP 校验当前 carrier；exec 原子替换本 Task mm，exit 释放 mm，reap
只释放 Task record。普通 fork/wait/exit 不再保存或恢复 parent writable-page、stack-byte 或整个
address-space snapshot；vfork 的既有共享-VM 兼容片不由本阶段泛化。

第二阶段的 eager-copy fixture 要求 child 修改 ELF data/BSS、当前 stack、brk 和已触页匿名
`MAP_PRIVATE`，parent 在 wait 后验证退出状态、全部原字节和 brk 不变，并能继续 munmap；该过渡实现已由
第三阶段真实 COW 替换，不再是当前 fork 路径。

### 第二阶段完整回归证据（2026-08-04）

以提交 `58f6eb03` 为被测基线完成一轮独立验收；本轮只记录验证证据，不改变 Charter、Model、Coding、
Compose 或 Impl 语义：

- 仓库根直接执行 `make test`，结果为 188/188；双 provider 的 KUnit 各 25/25、kernel smoke 各
  58/58，其余 user/rootfs/LTP acceptance 全部通过。
- `make -C tools2 test-all` 通过：106 个 Python 测试、16 个 frontend 测试、bundle check 与 9 个
  Playwright E2E 均成功。
- 默认 `make stress-test` 对 DF-0001 user boot、DF-0002 smoke initcall、DF-0003 scripted shell 和
  DF-0004 canonical rc.local 各执行 10 轮，合计 40/40、failure=0、无 timeout。报告分别保存在
  `impl/arceos_ex/tests/stress/out/20260803T160957.418065Z-df-0001-user-boot/`、
  `impl/arceos_ex/tests/stress/out/20260803T161007.373015Z-df-0002-smoke-initcall/`、
  `impl/arceos_ex/tests/stress/out/20260803T161033.063669Z-df-0003-distro-sh-ls/` 和
  `impl/arceos_ex/tests/stress/out/20260803T161047.998168Z-rc-local-native-timeout-focused/`。
- 默认 `make difftest` 的 preflight 确认 474 个 checkpoint mapping 当前有效、103 个 exact Linux marker
  无 missing/stale/mismatch；`rc-local-difftest` 为 1/1，class=`paired-checkpoint-diff-ok`，
  `first_divergence=None`。报告保存在
  `impl/arceos_ex/tests/stress/out/20260803T161138.191593Z-rc-local-difftest/`。
- 全部命令结束后无残留 QEMU 进程；测试未产生 tracked 修改。该轮证据只验收当时的 eager 过渡基线，
  不把 eager dup_mm 视为真实 COW；真实 COW 的完成证据见下一节。

## 已完成基线：第三阶段真实 COW

第三阶段在独立 per-Task mm 基线上完成真实 COW，并关闭 Model deferred `user_clone.004`：

- `UserFrameRef` 与 `PageMetadataMap` 提供受检查的 user-frame 获取、共享引用和释放；普通 page-table
  page 仍保持唯一 `PageRef`。RISC-V RSW bit 8 表示 COW，bit 9 保留。
- fork prepare 先建立独立 child 根页表/SATP并获取 frame 引用，commit 再把双方原本可写的私有 leaf
  降低为 RO+COW，最后发布 child。只读 ELF 页共享为 RO、不得获得 COW 写权限；失败 fork 不改变
  parent leaf、frame 引用、可见字节、Task 槽或 PID。
- store-page-fault 校验 Task/mm、VMA 原写权限、PTE COW 和 backing frame；共享引用路径复制整页并
  原子替换 leaf，唯一引用路径直接清 COW/恢复 W。两条成功路径均执行目标 `sfence.vma` 并重试原
  `sepc`。
- COW 分配失败保持原 leaf、refcount、free-page 计数和双方数据不变，并进入明确 Task OOM terminal；
  wait 观察 signal 9，既有退出路径产生 SIGCHLD，不把资源失败伪装为 SIGSEGV。
- ELF data/BSS、用户栈、已驻留 brk 与匿名 `MAP_PRIVATE` 均进入 COW；exec、exit、失败 fork、嵌套
  fork 和多轮 fork/write/wait 通过同一受检查引用生命周期回收。`user_clone.002` 与
  `user_clone.003` 继续 deferred，不扩展 thread group、`CLONE_VM` 或完整 vfork。

### 第三阶段完整回归证据（2026-08-04）

- 仓库根直接执行最终 `make test`，结果 188/188；双 provider 的 KUnit 各 25/25、kernel smoke 各
  58/58，user/rootfs/LTP acceptance 全部通过。真实 user fixture 覆盖 child 首次 COW、parent
  refcount=1 快路径和单次启动内 8 轮 fork/COW/reap frame 复用。
- `make -C tools2 test-all` 通过：106 个 Python 测试、16 个 frontend 测试、bundle check 与 9 个
  Playwright E2E 均成功；canonical snapshot fingerprint 与关闭 `user_clone.004` 后的 192 项 boundary
  inventory 已同步。
- 默认 `make stress-test` 四组各执行 10 轮，合计 40/40、failure=0。报告位于
  `impl/arceos_ex/tests/stress/out/20260804T004915.608953Z-df-0001-user-boot/`、
  `impl/arceos_ex/tests/stress/out/20260804T004925.646238Z-df-0002-smoke-initcall/`、
  `impl/arceos_ex/tests/stress/out/20260804T004951.499774Z-df-0003-distro-sh-ls/` 与
  `impl/arceos_ex/tests/stress/out/20260804T005005.345550Z-rc-local-native-timeout-focused/`。
- `make difftest` 确认 474 个 checkpoint mapping 当前有效、103 个 exact Linux marker 为
  `0 missing / 0 stale / 0 mismatch`；`rc-local-difftest` 为 1/1、failure=0，报告位于
  `impl/arceos_ex/tests/stress/out/20260804T005059.583426Z-rc-local-difftest/`。
- 测试结束后无残留 QEMU 进程，未产生额外 tracked 修改。锁定的
  `spec/charter/systems/computer.md` 仅审查、未解锁或修改。

## 已完成基线：第四阶段同步致命 SIGSEGV 与最终收口

第四阶段把统一 fault core 的非法用户访问结果接入既有 task exit/wait/SIGCHLD 生命周期：

- 只有真实 U-mode instruction/load/store page fault 会降低为同步致命 SIGSEGV；无 VMA 为
  `SEGV_MAPERR`，权限、NX、只读写入和非 COW 写保护为 `SEGV_ACCERR`。`TaskSegvInfo` 不可变地保存
  signal 11、code 与精确 fault address，成功 COW 仍只重试原 `sepc`。
- signal terminal 只允许 OnCpu/Live 且尚无 terminal reason 的 Task 记录一次；重复或与 OOM 冲突的
  terminal 确定拒绝。wait word 的低 signal bits 为 11，terminal handoff 唤醒 parent 并产生 SIGCHLD；
  exit 释放当前 mm 一次，reap 只释放 Task record。
- 错误 Task/mm/SATP 仍是内核 invariant failure，syscall usercopy 的非法范围仍返回 `EFAULT`；两者都
  不伪装为用户 SIGSEGV。kernel extable、nested/atomic kernel fault 分支保持独立。
- 第一片不构造用户 signal frame，不进入已登记 handler，也不实现 `rt_sigreturn`、core dump 或 OOM
  killer；COW 资源失败继续使用独立 signal-9 OOM terminal。

### 第四阶段完整回归证据（2026-08-04）

- 对象 smoke 验证 MAPERR/ACCERR、精确 address、重复 terminal 拒绝和 wait word 11。native 与
  Linux-object 的同一真实 user fixture 均以普通 RISC-V 指令触发 unmapped load、read-only store 和
  NX execute；三次 parent wait 都观察 signal 11，诊断中的 MAPERR `si_addr` 精确为 `0x1000`，随后
  parent 继续完成 mm/COW 数据读写与回收。
- 仓库根直接执行 `make test`，结果 188/188；格式、Clippy、规格、双 provider KUnit/kernel smoke、
  user/rootfs 与 LTP acceptance 全部通过。
- `make -C tools2 test-all` 通过：106 个 Python 测试、16 个 frontend 测试、bundle check 与 9 个
  Playwright E2E 均成功；canonical snapshots 已同步到 model fingerprint `39c7df6e...`，boundary
  inventory 保持 192 项。
- 默认 `make stress-test` 四组各执行 10 轮，合计 40/40、failure=0。报告位于
  `impl/arceos_ex/tests/stress/out/20260804T012718.114832Z-df-0001-user-boot/`、
  `impl/arceos_ex/tests/stress/out/20260804T012728.600063Z-df-0002-smoke-initcall/`、
  `impl/arceos_ex/tests/stress/out/20260804T012753.600475Z-df-0003-distro-sh-ls/` 与
  `impl/arceos_ex/tests/stress/out/20260804T012807.379501Z-rc-local-native-timeout-focused/`。
- `make difftest` 确认 474 个 checkpoint mapping 当前有效、103 个 exact Linux marker 为
  `0 missing / 0 stale / 0 mismatch`；`rc-local-difftest` 为 1/1、failure=0，报告位于
  `impl/arceos_ex/tests/stress/out/20260804T012902.853785Z-rc-local-difftest/`。
- 锁定的 `spec/charter/systems/computer.md` 仅审查、未解锁或修改；Compose 经审查无适用装配变更。

## 明确不扩展的边界

本专题未实现 thread group、`CLONE_VM`、完整 vfork mm sharing、用户 handler delivery、
`rt_sigreturn`、page cache、文件后备缺页、`MAP_SHARED`、swap、页面迁移、OOM killer、ASLR 扩展或
SMP 页表并发。上述责任不能被四阶段闭环的实现便利隐式吸收。
