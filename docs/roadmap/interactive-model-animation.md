# tools2 Signal 交互动画完成归档

## 完成状态

本专题已于 2026-07-24 闭环，没有剩余实现责任；后续新增自动播放、时间线、视频导出或浏览器 derive
均需另行建立任务和规格。实现保持在独立 `tools2/animate/` 目录，老 `tools/` 静态 SVG、tools2 v4
model/view schema、derive 和 text renderer 均未改变。

## 目标与边界

为 tools2 的确定 Signal trace 增加自包含 HTML 动画。动画用于逐个观察 Signal 的发送、目标响应和
确定状态边界，不改变 `.spec`、derive 语义或现有 tools2 v4 中间协议，也不替换老 `tools/` 的静态
SVG 能力。

本专题是交互动画范围、协议、交互和验收的唯一详细计划；主 Roadmap 与 tools2 专题只保留状态摘要
和本页链接。

## 已确认的核心语义

- 每个已发送 Signal 都是一个完整步进单位。同步 `drives`、异步 `emits` 和其它嵌套 Signal 仍按
  tools2 v4 trace 的确定顺序各占一步，不合并为一次顶层响应，也不提供另一种展开/折叠步进模式。
- 初始确定帧只显示首个 Signal 的 source，以及为保持层级结构而必需的祖先。target 和后续对象在
  trace 首次需要时出现；不得用完整 model 的所有对象填满初始画面。
- `ArrowRight` 或页面中的“下一步”播放当前 Signal 的“箭头出现 → target 响应 → 箭头消失”，然后
  停在该 Signal 的确定 after 帧。
- `ArrowLeft` 或页面中的“上一步”恢复前一个确定帧。后退依据已生成的确定帧/快照，不逆向执行
  transition，也不重新运行 Signal handler。
- 浏览器只播放 derive 已经选定的 trace；不得重新 derive、求值 guard、选择 choice/分支、创建新
  Signal 或把播放器变成 runtime 模拟器。
- 任意次数的前进、后退和重复往返必须恢复相同的节点、层级、状态、错误和布局顺序。

## 数据流与协议

动画是 tools2 的独立 `animate` 阶段：

```text
.spec
  -> parse -> model
tools2 v4 model.json --------------------+
                                         +-> animate -> 自包含 trace.html
tools2 v4 view.json（来自 derive -> view）--+
                                                内嵌 lkm.spec.signal-animation v1
```

`model.json` 提供对象、真实 lifecycle state 集合、父子结构和其它静态关系；`view.json` 提供确定 Signal
顺序、source/target、handler、outcome、reason 及 before/after snapshot。`animate` 必须从结构化字段
读取这些语义，不得从显示名称、颜色或排版反推它们。

`animate` 输出单个可离线打开的 HTML，其中内嵌独立的 `lkm.spec.signal-animation` version `1` 数据、
样式与播放器代码。animation v1 至少固化以下内容：

- 输入协议身份和可追溯 source；
- 按 v4 view 顺序排列、每个 Signal 一个单位的步骤；
- 每步的 source、target、Signal、handler kind、outcome、reason 和因果/顺序标识；
- Transition 的真实 before/after state，Action 的无状态变化响应，以及异常 Signal 的确定失败边界；
- 初始帧和各步 after 帧所需的可见节点、结构祖先、层级和稳定布局顺序。

animation v1 是 HTML 播放数据，不是人工维护的动画 DSL，也不是新的 derive 输入。首轮不得扩展或升级
既有 `lkm.spec.view` v4，不复用老 `tools/` 的静态 SVG renderer，不给 tools2 `lkm-render` 增加 HTML
format，也不增加 `animated-svg`。

## 状态与响应视觉

节点始终显示快照中的真实状态名称，不把未知或其它状态伪装成常用 lifecycle state：

- `Prepared`：灰色虚线边框；
- `Ready`：灰色实线边框；
- `Online`：黑色实线边框；
- 其它有状态节点：显示真实 state 名称并使用中性样式；
- 仅为补齐层级而出现、没有自身状态的结构祖先：灰色虚线容器，且不伪造 state 标签。

Transition 响应显示 target 的真实 before state，响应发生后切换到真实 after state。Action 不制造
state 变化，只高亮 target 并短暂振动。不得根据 Signal 名称猜测 Transition、Action 或状态。

outcome 异常的 Signal 使用红色虚线箭头；target 产生错误脉冲，同时显示结构化 `reason`。失败视觉
结束后仍停在该 Signal 的确定 after 帧，不推演未出现在 v4 view 中的后续行为。颜色之外同时使用
线型和文字，保证结果无需仅靠颜色辨认。

## 箭头与层级布局

- 普通 Signal 从 source 右侧出发，指向 target 左侧。
- source 与 target 相同时使用下半圆弧：从节点右侧发出，沿节点下方绕行，在节点左侧结束。
- 子系统必须位于父系统框内。某个已显示节点的祖先尚未作为有状态节点出现时，补成无状态的灰色
  虚线结构容器。
- 父子包含约束优先于“target 放在 source 右侧”的方向偏好；不得为了让箭头朝右而把子系统移出
  父框或破坏祖先链。
- 同一父级下按首次出现顺序动态排列：后出现的子系统放在上方，已出现较早的兄弟向下移动。
- 节点新增或兄弟重排时平滑过渡；当前 Signal 的活动节点自动滚入视区。
- 页面尊重 `prefers-reduced-motion`。启用 reduced motion 时取消位移、振动和脉冲等非必要运动，但
  保留确定帧、状态、箭头线型、reason 和前进/后退能力。

## CLI 与输出共存

仓库便利入口增加：

```bash
tools2/bin/pyveri --html-out PATH
```

底层 tools2 `pyveri` driver 使用同名 `--html-out PATH`，shortcut 只负责透传。driver 在正常生成 v4
`model.json` 和 `view.json` 后调用独立 `animate` 阶段。

`-o/--output` 继续只控制 text 输出，不因 `--html-out` 改变含义；HTML 可以与 stdout/text `-o`、
`--scenario`、`--snapshot-out` 和保留中间文件的 `--work-dir` 同时使用。生成 HTML 不替代 check，driver
最终仍返回原 derive/check 结果对应的 `0`、`1` 或阶段错误 `2`。不得新增 `render --format html`，也
不得让 `lkm-render` 的 `--format text` 接口承担动画输出。

## 首轮控制面

页面提供“上一步”“下一步”、当前步数/总步数，以及当前 Signal、source、target、响应类型、状态变化
或失败 reason 的文字说明。键盘仅需 `ArrowLeft` 和 `ArrowRight` 对应同一操作。

首轮不提供自动播放/暂停、`Space`、`Home`、`End`、播放速度或时间轴跳转控件。它们不得成为 animation
v1 或首轮播放器的兼容性负担；未来如需增加，应另行确认交互和协议边界。

## 实施层次

本能力扩展展示接口和输出协议，实施时遵循 charter-first 权威顺序：

1. 在 charter 确认 Signal 级步进、确定 trace 播放、非模拟器边界、自包含 HTML 和独立 animate
   阶段的职责。
2. 复核 model 语义；只有上述展示边界确实影响 formal semantics 时才修改，不能为了前端便利改变
   已确定的 tools2 v4 Signal 含义。
3. 更新 coding 规格，定义 `model.json + view.json -> animate -> HTML`、animation v1、CLI、退出码、
   布局和 reduced-motion 契约；明确 v4 view schema 保持不变。
4. 复核 compose 层；仅在输出组合、打包或发布责任适用时修改。
5. 新增独立 animate 实现和自包含 HTML 模板，严格验证两个 v4 输入并生成 animation v1 播放数据。
6. 给底层 driver 和 `tools2/bin/pyveri` 增加同名 `--html-out`，保持 text、scenario、snapshot、work-dir
   与退出码组合行为。
7. 增加协议、确定帧、视觉语义、层级布局、键盘/按钮、离线 HTML、reduced-motion 和端到端测试；
   保持老 tools 静态 SVG 与 tools2 text renderer 的现有责任。

## 验证要求

至少覆盖：

- animate 同时验证并消费 tools2 v4 model/view；错误 producer、schema 或 version 明确失败。
- v4 view 中每个 Signal（包括嵌套 `drives`/`emits`）严格对应一个前进步骤，顺序和稳定 ID 不变。
- 初始帧只有首个 source 和必要祖先；逐步出现的 target、祖先与兄弟顺序可重复恢复。
- 右向前进完整播放箭头出现、响应和箭头消失；左向后退直接恢复前一确定帧且不逆向执行语义。
- Transition 使用真实 before/after state；Action 只高亮/振动；Prepared、Ready、Online、其它状态和
  无状态结构容器按已确认样式显示。
- failed/rejected/truncated/stopped 等非正常 outcome 不被伪装为完成；红色虚线、错误脉冲和 reason
  与结构化输入一致。
- 普通箭头、自 Signal 下半圆弧、父子包含优先级和同父后出现者在上的排序均有确定测试。
- 布局更新、自动滚入视区和 `prefers-reduced-motion` 行为可验证；reduced motion 不丢失语义信息。
- `--html-out` 可与 scenario、snapshot、text `-o` 和 work-dir 共存，driver 保留原 check 退出码。
- 生成物包含 animation v1、CSS 和播放器代码，可在无网络依赖下打开；浏览器代码不含 derive、分支
  选择或 transition 逆执行路径。
- tools2 v4 view schema、text renderer 与老 tools 静态 SVG 不因本能力扩展格式或改变默认行为。
- focused validation 通过，`git diff --check` 通过，最后从仓库根目录直接运行 `make test`。

## 实施与验收证据

- 规格、骨架、确定帧、步进、层级、动效、CLI 与浏览器加固分别由提交 `b0ba0093`、`9f66f7f2`、
  `092e74ef`、`568840fd`、`24747ef6`、`7561ff38`、`35d85d4f`、`75ce505f` 闭合。
- Python animate 严格验证 v4 schema/version/producer、source/fingerprint、端点、handler、outcome、
  snapshot 和 parent；合法无状态 system/process 使用真实 `null`，结构祖先另以 `structural` 标识。
- Svelte 5 + TypeScript bundle 固定依赖并纳入仓库；最终 CSS/JS 约 6.05/50.51 KiB，rebuild 后 stale
  check 通过。安全 JSON 内嵌和 HTML 原子写入均有测试。
- 固定 pipeline demo 为 4 步、约 61 KiB；正式 `-u Kernel.Startup` demo 为 reached/12 步、约 96 KiB；
  完整主模型为 failed/277 步、约 3.4 MiB，HTML 成功且保留 check 退出码 1。
- `make -C tools2 test`：54/54；Vitest：8/8；Playwright Chromium：5/5。端到端覆盖按钮、键盘、
  前后确定性、父子包含、兄弟重排、普通跨层箭头、自环、异常 reason、自动滚动、reduced-motion、
  离线无网络加载、稳定截图和 277 步完整往返。
- `git diff --check` 通过；仓库根直接 `make test` 最终汇总 182/182。人工检查的首帧截图只显示居中的
  `Human` 外部端点，结构和配色与初始最小可见集契约一致。

## 非目标

- 浏览器内 derive、guard 求值、分支选择、Signal 创建或 runtime 模拟。
- 把多个嵌套 Signal 合并成一次顶层响应，或维护展开/折叠两套步进语义。
- 人工动画 DSL、人工时间线或浏览器侧的第二份模型规格。
- 自动播放、首轮快捷跳转、播放速度和时间轴控制。
- 复用老 tools 静态 SVG renderer，或新增 HTML/`animated-svg` render format。
- MP4 或其它视频导出。
