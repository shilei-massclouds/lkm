# tools2 Signal 工具链 Coding 规格

本文把 [`../model/SEMANTICS.md`](../model/SEMANTICS.md#sem-signal-tools2-001-full-model-signal-derivation-is-an-isolated-compatibility-semantics)
映射到完整主模型 Python 工具实现。它只约束 `tools2/`，不得改变或导入 `tools/`。

## 包与依赖边界

源码固定分为 `common`、`parse`、`model`、`derive`、`check`、`view`、`render` 和 `pyveri` 八个
并列包，各包保留自己的 `pyproject.toml`。阶段入口沿用 `lkm-parse`、`lkm-model`、`lkm-derive`、
`lkm-check`、`lkm-view`、`lkm-render` 和 `pyveri`；源码运行使用只包含 `tools2/*/src` 的独立
`PYTHONPATH`。任何 tools2 Python 文件不得 import `tools/`、老 `common` 或老 `pyveri`。

`common` 只承载 schema 常量、JSON I/O、source span/diagnostic、稳定 canonical JSON/fingerprint 和
跨阶段值校验。parse 不依赖后续阶段；model 只依赖 common 和 ast.json；derive 只依赖 common 和
model.json；check/view 只消费 derive.json；render 只消费 view.json。driver 通过各阶段公开 Python
入口按上述顺序调度，不越过中间协议直接拼装结果。

目录中的 `build/` 是可清理的保留中间产物目录，`out/` 是用户显式选择的长期输出目录；测试不得
依赖二者的预存内容。

## 完整主模型输入与诊断

parser/model 必须让 `spec/model/main.spec` 通过零 error、零 unsupported，并支持 include、嵌套泛型、
enum、type/object/system 与传递继承、属性、owned、association/reference、predicate/function、context/
lock、state、Type lifecycle/process、对象 lifecycle override、命名形参/result、`within`、结构化
`deferred`/`trimmed` evidence、`depends_on`、`ensures`、`updates`、`drives`、`emits` 和 `lossy`。
model 必须把 `drives` 的顶层 `A || B` 规范化为有序 choice 节点；derive 在发送前检查候选可接受性，未选择候选不得进入 Signal 序列或消耗预算。
条件和 invariant 的顶层 `P || Q` 必须规范化为布尔 any-of 节点并按当前快照短路求值，不得保留为无法解释的 assertion 字符串。
state/fact/reference 求值至少包括：

- `Target.state == State::Name` 与 state assignment；
- `predicate(arg, ...)` 的布尔事实和 predicate body 参数替换；
- `Target.ref == Other` 与 reference assignment；
- enum literal、字符串、整数、布尔和系统引用 payload。

保留现有折叠调用形式，不接受显式 `signal` 或 `on Signal`。Type process、对象 override、静态
association/reference 和 effective parent 必须规范化为稳定 model 字段。未知 top-level/block member 或
未知表达式必须产生带 path、line、column 的 `unsupported` diagnostic；语法/引用/类型、unknown/self/
cyclic parent 矛盾使用 `error`。有 error/unsupported 时后续阶段仍可写出诊断 JSON，但不得执行推导。

## 中间协议

所有 JSON 顶层必须包含 `schema`、`version`、`producer: "tools2"` 和 `source`。schema 名沿用
`lkm.spec.ast/model/derive/check/view/snapshot`，tools2 各自使用 version `3`。消费者必须在读取后立即
验证三元组，不接受缺失 producer、老 producer、version 1/2 或其它 version。

AST 保留 include 展开后的声明、source order 和每个声明/语句 span。model 建立 enum、system、parent、
state、handler、参数、条件、effect 和 call 索引，并输出 source 内容指纹。derive/check/view 字段遵循
formal semantics；快照统一包含 `states`、有序 `facts` 和 `references`，JSON 输出使用排序 key 和稳定
列表顺序。

## CLI 与退出码

阶段工具都接受显式 `-o/--output`。parse 接收 `.spec`；model 接收 ast.json；derive 接收 model.json
以及 `--signal Target.Name`、`-u/--until Target.Name`、`--source`、`--scenario`、`--max-depth`、`--max-breadth`；check 和 view
接收 derive.json；render 接收 view.json，首期只实现 `--format text`。

driver 接收 `.spec` 和同一组 derive 参数，另提供 `--snapshot-out`、`--work-dir` 与文本 `-o`。默认
source 是 `Human`，预算默认 `3/3`；`all` 解析为无限，整数必须非负。未给 work-dir 时使用
临时目录，给出时保留 `ast.json`、`model.json`、`derive.json`、`check.json` 和 `view.json`。只有 check
verdict 为 complete 或 reached 才原子写 snapshot-out；reached snapshot 写入 boundary provenance；
failed/bounded/until_signal_not_reached 或阶段错误时目标文件不得出现。

仓库根可直接执行的唯一便利入口是 `tools2/bin/pyveri`；旧 `tools2/pyveri2` 必须删除。入口不得复制
阶段逻辑，只负责从自身路径解析仓库根、建立 tools2 独立
`PYTHONPATH` 并把短参数翻译给 driver：可选的 `-t/--trigger` 对应 `--signal`，`-u/--until` 对应
`--until`，`-s/--scenario` 对应 `--scenario`，`-f/--spec` 选择输入规格。`-t` 的默认值经规范化后是
`ComputerProject.Preset`，帮助 usage 必须显示 `[-t SIGNAL] [-u SIGNAL]`；`-f` 默认指向
`spec/model/main.spec`，快捷 source 默认 `Human`、预算默认 `all/all`；显式参数覆盖它们。预算、source、
work-dir、snapshot-out 和文本输出参数保持透传，底层 driver 的通用默认仍为 `3/3`。入口从脚本自身
位置解析仓库和包路径，因此从仓库根或其它当前目录调用的行为一致。

shortcut、driver 和 derive API/CLI 都调用 common 中同一 Signal request 规范化函数；任何 target 的
末段 `Startup` 都规范化为 `Preset`，其它名称不变。root/until request 和所有结构化产物只保留
canonical 名称，使 `Startup` 与 `Preset` 输入得到相同 Signal ID、事件序列和 canonical JSON。

derive 的发送实现必须通过单一 pre-send 函数创建 envelope。该函数接收已解析 source/target/name、
delivery、cause、coordinate 和 call span，先比较 canonical until request；匹配时记录 boundary 并在
任何 ID/event/enqueue/handler 操作前终止发送。内部使用专用控制流向上展开：尚未提交的 active response
写 `stopped` 和稳定 after snapshot，已完成 response 不改写；FIFO 中已有未处理 envelope 统一写
`stopped`，但不创建目标或余下 emits。该控制流不得被普通 DerivationProblem 捕获为 failed。

parse/model/derive 在成功写出合法诊断 JSON 时返回 0，I/O 或协议损坏返回 2。check 对 complete/reached 返回
0，对 failed/bounded/until_signal_not_reached 返回 1，协议损坏返回 2。view/render 不改变 check verdict；driver 最终采用
check 的退出码。所有用户错误写到 stderr，不输出 Python traceback。

## 展示边界

view 逐字段复制/整理 derive 的 root/until request、reached boundary、verdict、events、signals、snapshots、frontier 和 failure chain；
不得重新求值条件或通过名称推断 handler/outcome。`render_text(view)` 保持单参数调用接口，并在内部仅以
`os.environ.get("VERBOSE") == "1"` 选择 detailed renderer；render CLI、driver、shortcut 及 `-o` 输出
不得各自实现另一套选择。该环境变量只控制最终文本，不能传入或修改 parse/model/derive/check/view。

默认 compact renderer 首行输出 `verdict: VALUE`，随后按 `view["signals"]` 的原顺序输出：已解析
Transition 为 `SOURCE -- SIGNAL --> TARGET[BEFORE:AFTER]`，已解析 Action 为
`SOURCE -- SIGNAL --> TARGET`；未解析 handler 不猜类型，也不显示状态方括号。状态必须读取 target 在
Signal before/after snapshot 中的实际值。Signal 文本名 `Preset` 显示为 `Startup`。每行按
`coordinate.depth - min(0, min(coordinate.depth))` 使用两空格缩进，空 trace 不计算最小值；任何非
`completed` outcome 在同行追加 ` !! OUTCOME: REASON`。reached 另输出
`boundary: SOURCE -- SIGNAL --> TARGET (before send)`，failed 另保留一行 failure chain/reason。

verbose renderer 保持本轮修改前的详细格式和 canonical `Preset` 名称，以 cause depth 缩进 Signal，
明确标记 `drives wait`、`emits enqueue/dequeue`、payload、predicate proof source、effective context、
到达时快照来源、before/after state/fact/reference delta、reached boundary、stopped propagation、
discard/reject、truncated coordinate、source span 和完整因果链。当前 tools2 version 3 view 已包含两种
renderer 所需字段，因此不得为文本模式升级协议或改写 view。
当前 tools2 不实现 DOT、SVG 或 HTML。

交互 HTML 是后续独立 JavaScript/TypeScript frontend 里程碑；它消费稳定 view schema，不在 Python
renderer 中嵌入浏览器模拟器。老静态 trace/SVG 任务继续保留，退役老工具必须由用户另行决定。
