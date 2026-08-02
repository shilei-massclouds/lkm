# Signal-driven tools2 独立工具链

## 目标与边界

在 `tools2/` 从头建立 Signal 驱动的 `parse -> model -> derive -> check -> view -> render/animate` Python
纵切，并由独立 `pyveri` driver 串联。它不替换 `tools/`，不进入根 `make test`；当前 v9 工具链已经
能够消费完整主模型并生成 text 与独立离线 HTML，仍不实现或复用老静态 SVG。

权威设计见 [`../../spec/charter/system-signal.md`](../../spec/charter/system-signal.md)，formal semantics
见 [`../../spec/model/SEMANTICS.md`](../../spec/model/SEMANTICS.md#sem-signal-tools2-001-first-signal-derivation-is-an-isolated-compatibility-semantics)，
实现协议见 [`../../spec/coding/tools2.md`](../../spec/coding/tools2.md)。本文只记录里程碑和实施证据，
不覆盖上述规格。

## 主模型集成测试时长优化完成证据

2026-08-02 在同一主机和 warm filesystem cache 下完成 Testing/工具性能优化。最高受影响责任为
Testing；Charter、Model、Coding、Impl 依次审查为“无需修改”，Compose 与正式 Testing 语义也审查为
“无需修改”。生产 `tools2/bin/pyveri`、JSON schema、协议版本、canonical snapshot、Model fingerprint、
生产缓存策略和旧 `tools/` 的 shadow 责任均未改变。

修改前连续三次 focused test 为 111.646 / 112.016 / 111.693 秒，中位数 111.693 秒；
`make -C tools2 test` 的 101 项为 171.990 / 174.399 / 175.432 秒，中位数 174.399 秒。独立阶段诊断显示：
parse 1.095 秒、model 0.723 秒、到 `Kernel.Enable` 发送前的 derive/check/view 为
0.723 / 0.012 / 0.022 秒；模型初态完整闭包为 10.327 / 3.566 / 7.946 秒，canonical snapshot 恢复闭包
为 9.942 / 3.469 / 8.145 秒。完整闭包每次还分别序列化约 450 MB derive JSON 和 450 MB view JSON，
确认热点是三次近似等价闭包及重复大产物读写，而不是边界断言。

测试现在由独立 `MainModelIntegrationTests` 承载。会话级 fixture 只 parse/model 一次，以
`spec/model/main.spec` 和 Model fingerprint
`sha256:d68a339631ec34d17e403b5875a1d8b6a75723305bc77fd0c3fa7c1460b34af4` 标识；AST/Model 文件设为
只读，并在每个 prepared case 前后校验 hash/fingerprint。每个 derive 都使用独立初态、scenario、
输出目录和 Engine 可变状态，并输出阶段计时、cache-hit、Signal/event/inventory/obligation 数量及测试
摘要大小。

保留的完整闭包只有两次：模型初态闭包和真实 `Kernel.Enable` canonical snapshot 恢复闭包。二者在
入口 carrier 规范化后逐项比较后续 Signal/source/target/delivery/handler/outcome，并比较最终
states、facts、references、instances、boundary inventory、obligations 和 verdict。一次真实 CLI 纵切
仍从仓库根执行 spec -> AST -> Model -> Derive -> Check -> View -> Render，生成 pre-send snapshot 并与
已提交 bytes 比较；自动 canonical scenario/source、显式 source override、bypass、duplicate enable、
错误初态和 stale fingerprint 都停在最近边界，后者仍走独立的错误模型 parse/model 路径。

修改后连续三次 focused test 为 26.915 / 26.850 / 26.966 秒，中位数 26.915 秒，较修改前缩短 75.9%；
101 项 Python suite 为 89.571 / 89.124 / 88.690 秒，中位数 89.124 秒，缩短 48.9%。focused 中位数低于
65 秒、suite 中位数低于 130 秒，两个目标均完成。prepared fixture 的三次中位数约为 parse 1.083 秒、
model 0.766 秒、CLI pre-send 纵切 2.624 秒、模型初态 derive 6.355 秒、snapshot 恢复 derive 6.422 秒；
focused cases 的 derive 均低于 0.55 秒。计时只保留为同机验收证据，没有加入固定 CI timeout。

### 第二轮：七项主模型测试统一复用

第一轮结束后，`MainModelIntegrationTests` 中的一项共享 fixture 测试约 26.9 秒，另外六项
`test_main_model_*` 仍留在 `SignalPipelineTests` 并各自执行完整 CLI；本轮开始时复现七项合计
83.540 秒，其中六项重复路径合计 56.036 秒，单项约 2.9–15.2 秒。这些路径反复 parse/model、写入
derive/view/HTML 大产物，并为同一个边界同时承担 wrapper 协议和 Model 语义验证，是第二轮热点。

七项测试现在都直接归入 `MainModelIntegrationTests`，共享一次只读 AST/Model fixture。每个 prepared
case 仍独立创建 scenario、Engine 推导状态和摘要输出目录，并在前后校验主规格 source、AST、Model
文件 hash 与 Model fingerprint；每个 unittest 的 setup/teardown 另校验完整内存 Model hash，防止
case 污染共享内容。唯一真实 CLI 纵切继续从仓库根覆盖 spec -> AST -> Model -> Derive -> Check ->
View -> Render 和 `Kernel.Enable` canonical bytes；另外两个 canonical snapshot 由同一 prepared Model
重建并逐字节比较。setup_arch 的 HTML 断言仍通过 prepared view 的 animation v4 投影生成，既有两项
独立主模型动画测试保持不变。

wrapper 参数协议与模型语义分开验证：canonical scenario/source 自动选择、显式 source override、
缺失/畸形/越界路径由真实 shortcut 参数解析配合隔离的 downstream 调用验证，不再重跑主模型；对应
Signal 结果由 prepared case 验证。stale fingerprint 仍用错误 fixture 独立执行 parse/model，错误模型
初态仍从共享 Model 的真实初态推导，二者都不能误用 fixture scenario。

测试矩阵审计结论如下：

- 必须保留两次完整闭包、三份 canonical snapshot bytes、OpenSBI/BootInit/setup_arch/Scheduler 主要
  边界、严格 prerequisite/guard 拒绝、canonical resolver 安全性和 animation v4 投影；本轮均继续覆盖。
- 多个边界语义、bypass、缺失 guard、duplicate enable 和缺失 prerequisite 可以共享 prepared Model，
  但仍以独立 test/subtest、scenario 和断言定位失败。
- 不必让每份 snapshot 都执行完整 CLI，不必让每个负例重新 parse/model，也不必为同一边界重复生成
  等价完整流水线；这些重复成本已移除，没有删测试或放宽断言。

最终 `MainModelIntegrationTests` 连续三次为 38.449 / 38.429 / 37.879 秒，中位数 38.429 秒，低于
45 秒目标，较本轮 83.540 秒基线缩短 54.0%；`make -C tools2 test` 的 101 项连续三次为
46.098 / 46.697 / 46.917 秒，中位数 46.697 秒，低于 60 秒目标，较第一轮 89.124 秒中位数再缩短
47.6%。未增加固定 CI timeout，也未引入生产缓存、并行完整 derive 或可变 snapshot 复用。

当前三个已提交 canonical snapshot bytes 为：`Kernel.Enable` SHA-256 为
`18ef873bf620122df2a7896053370a9d677f3ebb74b33652356025033db017ba`，`BootInitFlow.Setup` 为
`bd8f82a57de7fec4c9a90bc98a245e90180a65673ea8021ee382580b83a8f257`，`Cpu0Scheduler.Schedule` 为
`f8b901553678c8c02265486a326fb7c0761084027804ab5574ff80353c72e02e`；AST/Model/Derive/Check/View/Snapshot
协议继续全部为 v10。101 项测试和全部严格拒绝覆盖均保留，没有调用旧工具、并行完整 derive、复用
可变 snapshot 或放宽断言来取得性能数字。

## 已完成：Deferred / Trimmed / Obligation 处理闭环

2026-07-30 对照检查确认：`tools2` 能在 AST 和 Model handler body 中保留结构化
`deferred` / `trimmed` member，但尚未形成与 Model 语义一致的 boundary inventory、结构校验、
evidence 证明、obligation、check policy 和展示闭环。最小负例中，旧 `tools` 对两个未证明 evidence
报告 `deferred=1`、`trimmed=1`、`obligation=2` 并使 check 返回 1；`tools2` 仅因为 evidence
表达式具有 fact 形状就将两项记为 `result=true`，最终返回 complete/0。包含非法分类、空
summary、空 evidence、缺少 resolution 和重复 ID 的结构负例也被 `tools2` 以零诊断接受。

主模型源码当前包含 138 个唯一 deferred ID 和 52 个唯一 trimmed ID。旧工具 Model 只记录
135/52，漏掉 `smp_bringup.001`–`.003`；`tools2` Model 树能保留全部 138/52，但 derive 只有
119/64 次 evidence event，按 ID 去重后是 106/52。差异表明全局 inventory、可达 owner 和动态
occurrence 必须分别表示；本项不以复制旧工具计数为完成标准。

本 P0 是 tools2 修正/完善计划，不把 tools2 当作 System，也不建立新的系统 lifecycle 或独立
charter-first 校准段。实施以现有 `SEM-BOUNDARY-001` 及 tools2 Signal 语义为约束，按以下步骤推进：

1. **固化基线 fixture 与验收矩阵**：增加合法已证明 evidence、合法未证明 evidence、非法分类、
   缺字段、空 evidence、非法/重复 ID，以及 State、Transition、Action、`within`、Type process 和
   重复 Signal occurrence 用例；记录 tools2 当前结果和旧工具 shadow comparison。
2. **完善 parse/model inventory 与结构校验**：建立按 boundary ID 唯一索引的结构化 inventory，
   保存 status、category、summary、resolution、evidence、词法 owner/context 和 source span；拒绝缺字段、
   非法分类、非法/重复 ID、legacy 记录和空 evidence。
3. **明确 inventory 与 occurrence**：inventory 中每个 ID 只出现一次；同一 owner 多次执行时分别记录
   occurrence、Signal ID、执行序号和 proof result，不用动态执行次数冒充 inventory 计数。
4. **完善 derive evidence 验证**：在所属 State/Transition/Action/`within` 边界到达时，使用该成功边界
   可见的 Model 结构、参考配置/架构/输入事实、前序已证明事实以及 transition candidate snapshot
   验证 evidence；不得按表达式语法种类直接认定成立，也不得合成缺失事实。
5. **增加结构化 obligation**：无法证明的每个 evidence 产生可追溯到 boundary ID、owner、occurrence、
   expression、proof source/classification 和 source span 的 verification obligation。obligation 不改写
   Signal 因果结果，不伪装为 runtime failure，也不关闭对应 Deferred/Trimmed boundary。
6. **完善 check 与 snapshot 门禁**：`complete` / `reached` 只是必要条件；默认 policy 允许 evidence
   已证明的 Deferred/Trimmed inventory，但任何 unresolved obligation 都使 check 返回 1。check 未通过时
   不得写 canonical snapshot；诊断用 derive/view 产物仍可保留。
7. **完善 view/render/animation 投影**：view 只复制 derive 的 inventory、occurrence、proof result 和
   obligation；compact 输出提供计数摘要，verbose 输出提供逐项来源。render/animation 不重新求值 evidence，
   boundary 观察事件不制造额外 Signal、moment、状态或事实。
8. **更新 tools2 协议和 Coding 映射**：为新增必需字段定义稳定 schema；若现有 v8 消费者不能在保持
   严格协议身份的前提下解释结果，则升级协议版本。同步修正“complete/reached 无条件返回 0”等与
   obligation policy 冲突的 Coding 描述。
9. **建立新旧工具差异审计**：旧工具仅作 shadow evidence，不是真值 oracle。逐项审查旧工具漏掉的
   `smp_bringup.001`–`.003`、tools2 漏掉的 state-level boundaries、Type process composition 和重复
   occurrence；以正式 Model owner/可达语义决定结果，不以任一侧历史计数覆盖源码 inventory。
10. **完成回归和接管判定**：运行 focused parse/model/derive/check/view/animation 正反例、完整主模型
    推导、`make -C tools2 test-all`、`make verify`、`git diff --check` 和仓库根直接 `make test`。只有结构
    负例被拒绝、未证明 evidence 稳定形成 obligation/check=1、已证明 inventory check=0、主模型
    obligation=0，且 snapshot 与展示门禁均符合上述规则，才完成本 P0。

## Deferred / Trimmed / Obligation v9 完成证据

2026-07-30 按 charter-first 完成闭环。Charter 明确 obligation 不改变 Signal 因果结果但阻止 check 与
canonical snapshot；Model 区分全局 inventory 与动态 occurrence，并要求 evidence 在 candidate snapshot
上只读求值；Coding 固定 v9 的 AST/Model/Derive/Check/View/Snapshot 协议及 v3 animation 投影。锁定的
`spec/charter/systems/computer.md` 仅复核、未修改；Compose 复核后无组件边界变化。

- parser/model 现在拒绝非法分类、缺失或空字段、非法/重复 ID、重复属性/evidence block、错误
  resolution 与 legacy boundary；Type process composition 共享 inventory ID，执行实例分别产生 occurrence。
- derive 不再按 evidence AST 形状直接判真；proof 只读消费 candidate snapshot、Model 结构、参考输入和
  前序已证明事实。每条未证明 evidence 形成结构化 obligation，但不增加 Signal、状态、事实或 causal
  moment。check 仅在 verdict 为 `complete` / `reached` 且 unresolved obligation 为零时 allowed，driver
  只依据该结果创建或覆盖 snapshot。
- 完整主模型 inventory 精确为 138 deferred、52 trimmed；完整推导为 185 个 occurrence、0 个 unresolved
  obligation，且 `smp_bringup.001`–`.003`、state-level boundary 与 Type-process boundary 均有正式记录。
  旧 `tools/` 的 135/52 与事件计数仅保留为 shadow comparison，没有覆盖 v9 结果。
- `make -C tools2 test-all` 通过：Python 91/91、Svelte 0 error/0 warning、Vitest 16/16、bundle stale
  check 和 Playwright 9/9。测试覆盖 v1–v8 全拒绝、v9 round-trip、已证明/未证明 evidence、重复
  occurrence、非因果 obligation、check/snapshot 门禁以及 animation v4 同源投影。
- `make verify` 通过；仓库根直接 `make test` 最终为 184/184，两侧 KUnit 各 25/25、app smoke 各
  56/56，LTP close list native/linux-object 均通过。
- 两个 canonical snapshot 已重建为 v9，Model fingerprint 均为
  `sha256:658f02fd9242257810f6c7d43e83235999960148febb30e0357e20aeec7828a7`；`Kernel.Enable` snapshot
  SHA-256 为 `da0e8208407c4d5cbbb6563ca7208ad8df4da7cea6296077928824cf80d53c99`，
  `BootInitFlow.Setup` snapshot SHA-256 为
  `8cd841b9904ad98f92a619e627be657b7d610a050ad2625ca68841b1c6c208e4`。

## 里程碑

1. 首期最小闭环（已完成）：独立包和 producer/version 隔离；必要 DSL 子集；Transition/Action 隐式 Signal；
   drives 同步、emits post-commit FIFO；严格 Signal；层级预算；scenario/snapshot；结构化
   derive/view JSON 和 text renderer；稳定性及端到端测试。
2. 显式 Signal DSL：在单独的 charter/model-first 决策中引入 `signal` 声明和 `on Signal` 语法，
   停止依赖调用表达式的隐式规范化。首期不得提前接受该语法。
3. handler 命名：评审兼容 handler 从同名 Transition/Action 迁移到 `OnPreset` 等显式响应过程的规则，
   包括歧义、重载和迁移诊断。
4. 通用 Signal pending 与 continuation：定义接受后等待任意未来 Signal、保存/恢复 continuation、队列
   所有权、timeout、cancel 和 snapshot 可续跑语义；首期条件失败必须保持 rejected。Scheduler 的
   Schedule-return TaskFlow lane、SMP CpuLane 仲裁、cross-CPU mailbox、迁移和 schedule replay 不属于
   本里程碑，由独立的
   [`deterministic-smp-lanes.md`](deterministic-smp-lanes.md) 统一规划。
5. 交互 HTML（已完成）：独立 animate 阶段共同消费 tools2 v10 `model.json` 和 `view.json`，生成内嵌
   `lkm.spec.signal-animation` v3 因果时刻的自包含 HTML；按 request/feedback/settle/terminal moment
   前进/后退，不重新求值 v9 view 的 boundary/obligation 投影，也不把浏览器变成推导器。首轮 v1 计划见
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
  authority、entry `satp` 和 RawDtb guard 的首失败短路。animation v4 精确为 50 request、46 feedback、
  3 settle、1 terminal，共 100 moments；0016 feedback 位于全部嵌套 child feedback 之后。
- Linux 6.12 RISC-V `head.S`、`setup_vm()`/`relocate_enable_mmu()` 与现有 model/coding/compose/实现、
  checkpoint 顺序复核一致，因此本组没有修改 model、coding、compose 或 `impl/arceos_ex`。focused
  `hello-native` 为 schema v2 passed/completed、exit 0、无 QEMU timeout；入口序列在
  `BootInitFlow.Prepared` 后才发送 `BootInitFlow.Setup`，其直接对象 Signal 均归因到 BootInitFlow。
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
