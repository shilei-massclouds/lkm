# tools2 Signal 工具链 Coding 规格

本文把 [`../model/SEMANTICS.md`](../model/SEMANTICS.md#sem-signal-tools2-001-first-signal-derivation-is-an-isolated-compatibility-semantics)
映射到首期 Python 工具实现。它只约束 `tools2/`，不得改变或导入 `tools/`。

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

## 输入子集与诊断

首期 parser 支持 include、enum 受控值、object/system、parent、initial state、state、
`on Transition::Name`、`on Action::Name`、命名形参、系统引用、`depends_on`、`ensures`、`updates`、
`drives`、`emits` 和 `lossy`。state/fact 前提和结果只实现：

- `Target.state == State::Name` 与 state assignment；
- `predicate(arg, ...)` 的布尔事实；
- `Target.ref == Other` 与 reference assignment；
- enum literal、字符串、整数、布尔和系统引用 payload。

保留现有折叠调用形式，不在首期接受显式 `signal` 或 `on Signal`。未知 top-level/block member、复杂
表达式、泛型、declare/within/deferred/trimmed 或其它未列语法必须产生带 path、line、column 的
`unsupported` diagnostic；语法/引用/类型矛盾使用 `error`。有 error/unsupported 时后续阶段仍可写出
诊断 JSON，但不得执行推导。

## 中间协议

所有 JSON 顶层必须包含 `schema`、`version`、`producer: "tools2"` 和 `source`。schema 名沿用
`lkm.spec.ast/model/derive/check/view`，tools2 首期各自使用 version `1`。消费者必须在读取后立即验证
三元组，不接受缺失 producer、老 producer 或其它 version。

AST 保留 include 展开后的声明、source order 和每个声明/语句 span。model 建立 enum、system、parent、
state、handler、参数、条件、effect 和 call 索引，并输出 source 内容指纹。derive/check/view 字段遵循
formal semantics；快照统一包含 `states`、有序 `facts` 和 `references`，JSON 输出使用排序 key 和稳定
列表顺序。

## CLI 与退出码

阶段工具都接受显式 `-o/--output`。parse 接收 `.spec`；model 接收 ast.json；derive 接收 model.json
以及 `--signal Target.Name`、`--source`、`--scenario`、`--max-depth`、`--max-breadth`；check 和 view
接收 derive.json；render 接收 view.json，首期只实现 `--format text`。

driver 接收 `.spec` 和同一组 derive 参数，另提供 `--snapshot-out`、`--work-dir` 与文本 `-o`。默认
source 是虚拟 `Environment`，预算默认 `3/3`；`all` 解析为无限，整数必须非负。未给 work-dir 时使用
临时目录，给出时保留 `ast.json`、`model.json`、`derive.json`、`check.json` 和 `view.json`。只有 check
verdict 为 complete 才原子写 snapshot-out；failed/bounded 或阶段错误时目标文件不得出现。

仓库根可直接执行的便利入口是 `tools2/pyveri2`。它不得复制阶段逻辑，只负责建立 tools2 独立
`PYTHONPATH` 并把短参数翻译给 driver：`-t/--trigger` 对应 `--signal`，`-s/--scenario` 对应
`--scenario`，`-f/--spec` 选择输入规格。`-f` 默认指向 tools2 的最小 pipeline fixture，因此首个实验
可以只写 `tools2/pyveri2 -t Root.Start`；生产或其它实验必须显式选择自己的 spec。预算、source、
work-dir、snapshot-out 和文本输出参数保持透传，不建立第二套默认推导语义。便利入口必须从脚本自身
位置解析仓库路径，因此从仓库根或其它当前目录调用的行为一致。

parse/model/derive 在成功写出合法诊断 JSON 时返回 0，I/O 或协议损坏返回 2。check 对 complete 返回
0，对 failed/bounded 返回 1，协议损坏返回 2。view/render 不改变 check verdict；driver 最终采用
check 的退出码。所有用户错误写到 stderr，不输出 Python traceback。

## 展示边界

view 逐字段复制/整理 derive 的 request、verdict、events、signals、snapshots、frontier 和 failure chain；
不得重新求值条件或通过名称推断 handler/outcome。text renderer 以 cause depth 缩进 Signal，明确标记
`drives wait`、`emits enqueue/dequeue`、payload、before/after state/fact/reference delta、discard/reject、
truncated coordinate 和 root-to-failure chain。首期不实现 DOT、SVG 或 HTML。

交互 HTML 是后续独立 JavaScript/TypeScript frontend 里程碑；它消费稳定 view schema，不在 Python
renderer 中嵌入浏览器模拟器。老静态 trace/SVG 任务继续保留，退役老工具必须由用户另行决定。
