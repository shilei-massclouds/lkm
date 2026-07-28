# Signal-driven tools2 独立工具链

## 目标与边界

在 `tools2/` 从头建立 Signal 驱动的 `parse -> model -> derive -> check -> view -> render/animate` Python
纵切，并由独立 `pyveri` driver 串联。它不替换 `tools/`，不进入根 `make test`；当前 v5 工具链已经
能够消费完整主模型并生成 text 与独立离线 HTML，仍不实现或复用老静态 SVG。

权威设计见 [`../../spec/charter/system-signal.md`](../../spec/charter/system-signal.md)，formal semantics
见 [`../../spec/model/SEMANTICS.md`](../../spec/model/SEMANTICS.md#sem-signal-tools2-001-first-signal-derivation-is-an-isolated-compatibility-semantics)，
实现协议见 [`../../spec/coding/tools2.md`](../../spec/coding/tools2.md)。本文只记录里程碑和实施证据，
不覆盖上述规格。

## 里程碑

1. 首期最小闭环（已完成）：独立包和 producer/version 隔离；必要 DSL 子集；Transition/Action 隐式 Signal；
   drives 同步、emits post-commit FIFO；严格 Signal；层级预算；scenario/snapshot；结构化
   derive/view JSON 和 text renderer；稳定性及端到端测试。
2. 显式 Signal DSL：在单独的 charter/model-first 决策中引入 `signal` 声明和 `on Signal` 语法，
   停止依赖调用表达式的隐式规范化。首期不得提前接受该语法。
3. handler 命名：评审兼容 handler 从同名 Transition/Action 迁移到 `OnPreset` 等显式响应过程的规则，
   包括歧义、重载和迁移诊断。
4. pending 与 continuation：定义接受后等待未来 Signal、保存/恢复 continuation、队列所有权、超时、
   取消和 snapshot 可续跑语义；首期条件失败必须保持 rejected。
5. 交互 HTML（已完成）：独立 animate 阶段共同消费 tools2 v5 `model.json` 和 `view.json`，生成内嵌
   `lkm.spec.signal-animation` v3 因果时刻的自包含 HTML；按 request/feedback/settle/terminal moment
   前进/后退，不扩展 v5 view schema，也不把浏览器变成推导器。首轮 v1 计划见
   [`interactive-model-animation.md`](interactive-model-animation.md)，老 tools 静态 SVG 保持原责任。
6. 老工具迁移/退役：只有用户另行明确决定后才能规划。不得以 tools2 覆盖率或版本号自动触发。

## 首期验收证据

2026-07-22 首期实现完成，证据如下：

- `make -C tools2 test-focused`：1 项同步/异步纵切通过。
- `make -C tools2 test`：22 项通过，覆盖协议互拒、include/span、隐式 handler、payload/reference 类型、
  严格 Signal、invariant、层级坐标和预算、FIFO、快照续跑、稳定 JSON、view 投影、文本失败链和
  `pyveri2` 短参数入口。
- `tools/pyveri/bin/pyveri spec/model/main.spec -T /tmp/lkm-tools2-legacy.trace.svg
  --trace-annotations state,transition --strict`：老静态 SVG 生成成功；主模型 `obligation=0`、
  `blocked=0`、`contradiction=0`。
- `git diff --check`：通过。
- 仓库根直接 `make test`：最终汇总 `182/182`；native/linux-object KUnit 各 `25/25`，app smoke
  各 `55/55`，LTP close list acceptance 两侧通过。

首期已闭环；主 Roadmap 保持“进行中”只表示里程碑 2–6 仍需未来独立决策和实施，不表示首期缺少
验收责任。

2026-07-23 协议升级到 v4：删除 Signal 的 lossy 字段和 discarded outcome；所有已发送 Signal
都必须被接受并处理，异步 emits 失败同样传播为根执行 failed。

## 交互 HTML 验收证据

2026-07-24 里程碑 5 已闭环。实现位于独立 `tools2/animate/`，生成
`lkm.spec.signal-animation` v1 自包含 HTML；driver/shortcut 的 `--html-out` 与 text、scenario、snapshot
和 work-dir 共存并保留 check 0/1。Python 54 项、Vitest 8 项和 Playwright 5 项通过；浏览器测试覆盖
无网络 `file://` 加载、按钮/键盘、确定往返、父子/兄弟布局、普通/自环/异常箭头、自动滚动、
reduced-motion、截图，以及完整主模型 277 个 Signal 的前后往返。完整证据保存在
[`interactive-model-animation.md`](interactive-model-animation.md)。

2026-07-27 动画协议升级到 v3：生成器按 v5 event sequence 区分 request、同步 feedback、异步 settle
和异常 terminal，父 feedback 位于同步子响应之后；`-u Kernel.Enable` 固定验证 13 个 Signal、26 个
moment（13 request、10 feedback、3 settle），且不创建边界 Signal。Python、Svelte/Vitest、确定 bundle
与 Playwright 离线/布局/主模型往返测试闭合。

## 启动语义校准证据

2026-07-27 第 1 组“上游构造与交接”完成。校准基线为 `7b0e7ce061fd367084c4a8c6c17f23029fada5fa`，
开始时工作树干净，编辑前仓库根直接 `make test` 为 182/182。主模型 fingerprint 是
`sha256:0c95ef3df8785912443c07f9a30797f31d4ce781878b27b49affc5e49a490faa`。

- 从仓库根执行 `tools2/bin/pyveri -u Kernel.Enable --snapshot-out
  /tmp/lkm-tools2-group1-kernel.snapshot.json`，输出与提交的
  `tools2/scenarios/Kernel.Enable.snapshot.json` 逐字节一致；两者 SHA-256 都是
  `75e3cb82141d5d5d3d8e9cc7082c6118a0a2ac585d2ccec84505154a85a060d6`。推导精确包含 13 个
  Signal：10 个同步 drives 和 3 个异步 emits；后者依次入队/出队，Kernel.Enable 尚未创建。
- 从模型初态执行 `tools2/bin/pyveri -u BootInitFlow.Preset`，确认实际 sender 是 OpenSBI：
  `sig-0014 OpenSBI -> Kernel.Enable` 被 FIFO 交付、所有入口 guard 通过并开始 handler，
  `sig-0015 Kernel.AcceptEnable` 完成并建立 `kernel_enable_accepted(Kernel)`。截断点尚未创建
  BootInitFlow Signal，Kernel 保持 Ready、BootInitFlow 保持 Base；0014 的 stopped 只表示未提交
  ancestor，不是拒绝。
- Linux 6.12 `Documentation/arch/riscv/boot.rst` 的 a0/a1、`satp=0`、RV64 PMD/2 MiB 物理对齐和
  ordered-boot 要求，与 model snapshot、`_start` live SATP guard 和 Rust entry adoption 一致。根回归
  announce 仍观察到 `R`（Kernel.Started）先于 `I`（InterruptType.Prepared），结构化 basic result
  为 schema v2、passed、`qemu.timed_out=false`。
- charter 已固化 13-Signal 因果账本和第二截断边界。逐层复核 model、coding、compose 和
  `impl/arceos_ex` 后未发现差异，因此不制造 model 或实现改动；JSON/snapshot 保持 v5，animation
  保持 v3，也未引入显式 Signal DSL、continuation、旧工具迁移或新协议。
- 测试固定每个 Signal 的 identity、delivery、cause、handler、target before/after state、FIFO，
  同源 compact/verbose text 和 animation fingerprint，并新增真实 OpenSBI handoff 与缺失 SATP/
  AcceptEnable 负例。`make -C tools2 test` 为 71/71；Svelte check 为 0 error/0 warning，Vitest 16/16，
  bundle stale check 通过，Playwright 8/8；代码修改后的根 `make test` 再次为 182/182。

实际 QEMU 压力与差分验收随后补齐：修改后的默认 `make stress-test` 已包含 DF-0004，四组分别
10/10、合计 40/40；另对 DF-0004 执行 100 轮深采样，结果 100/100、0 timeout、0 QMP timeout
artifact。默认 `rc-local-difftest` 与长期 `linux-exact-baseline-difftest` 各 1/1，通过两侧完整执行和
checkpoint diff，`first_divergence=None`。DF-0004 继续作为独立长期观察项；成功样本只表示本批未
复现。若将来复现，保留 `qemu-timeout-diagnostics.json` 并按该 defect 定位，不以重试掩盖，也不
归因于本组 Signal 语义。下一校准批次是第 2 组 Kernel 与 BootInit 入口；在单独闭合前不提前修改
BootInit 内部语义。

2026-07-27 第 2 组“Kernel 与 BootInit 入口”完成。校准从提交
`e60caef5c16d44a8287bda83fff20028d33e3c83` 和干净工作树开始；编辑前仓库根直接 `make test`
为 182/182，主模型 fingerprint 保持
`sha256:0c95ef3df8785912443c07f9a30797f31d4ce781878b27b49affc5e49a490faa`。

- 从仓库根与 `/tmp` 分别重建 `Kernel.Enable`、`BootInitFlow.Setup` 发送前 snapshot，四次输出均与
  对应提交 golden 逐字节一致。Kernel golden SHA-256 保持
  `75e3cb82141d5d5d3d8e9cc7082c6118a0a2ac585d2ccec84505154a85a060d6`；新增
  `tools2/scenarios/BootInitFlow.Setup.snapshot.json` 的 SHA-256 为
  `9a321a12075d3d078fa4356ee0a250de6bea74f7389a6f5880dc6701413ff250`，并成为第 3 组显式 trigger
  的默认入口。
- `-u BootInitFlow.Setup` 从真实 Human 编排得到精确 50 个 Signal：`sig-0014` 是已进入 handler、
  等待子调用后在发送前边界停止的 OpenSBI emits ancestor；`sig-0015..0050` 是 36 个 completed
  drives。`sig-0016 BootInitFlow.Preset` 只有一对 `SingleTaskContext` enter/exit，没有 Lock 或运行期
  fresh instance。最后状态为 Kernel Ready、BootInitFlow Prepared；边界 provenance 是
  `Kernel -> BootInitFlow.Setup`、drives/cause 0014，且未创建 Setup Signal。
- 测试逐项固定 0014–0050 的 identity、delivery、cause、handler、目标状态、顺序、关键入口事实、
  compact/verbose text、canonical bytes/default scenario，以及缺失 acceptance、BootTask execution
  authority、entry `satp` 和 RawDtb guard 的首失败短路。animation v3 精确为 50 request、46 feedback、
  3 settle、1 terminal，共 100 moments；0016 feedback 位于全部嵌套 child feedback 之后。
- Linux 6.12 RISC-V `head.S`、`setup_vm()`/`relocate_enable_mmu()` 与现有 model/coding/compose/实现、
  checkpoint 顺序复核一致，因此本组没有修改 model、coding、compose 或 `impl/arceos_ex`。focused
  `hello-native` 为 schema v2 passed/completed、exit 0、无 QEMU timeout；入口序列在
  `BootInitFlow.Prepared` 后才出现 `EntrySuccessorPhase.Started`，没有提前的 Setup 归因。
- 精确 `make -C tools2 test-all` 通过：Python 74/74、Svelte 0 error/0 warning、Vitest 16/16、bundle
  stale check 和 Playwright 8/8。默认 `make stress-test` 四组各 10/10、合计 40/40；默认
  `rc-local-difftest` 与长期 `linux-exact-baseline-difftest` 各 1/1，均为
  `paired-checkpoint-diff-ok`、`first_divergence=None`。

tools2 JSON/view schema 继续保持 v5，animation 继续保持 v3；本组未引入显式 Signal DSL、continuation、
旧工具迁移或根门禁接管。下一校准批次是第 3 组 `BootInitFlow.Setup`；在该批次单独 charter-first 闭合前，
不得把 Setup 的任何叶阶段归入本组。

2026-07-28 补齐分组动画的 canonical sender 恢复：shortcut 在显式 `-t` 自动加载已提交
snapshot 时，从 matching boundary provenance 取得真实 source。因此第 2 组可直接用
`-t Kernel.Enable -u BootInitFlow.Setup` 得到 `OpenSBI -> Kernel.Enable` 根 Signal，无需重复
`--source OpenSBI`；显式 `--source` 仍覆盖 provenance，损坏或不匹配的 canonical snapshot 在
derive 前拒绝。该校正不改变 model fingerprint、Signal 因果账本或各组边界。
