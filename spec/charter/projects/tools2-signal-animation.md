# tools2 Signal 动画章程

## 目标与责任边界

tools2 必须能够把已经确定的 Signal trace 发布为单文件、可离线打开的交互 HTML，使读者可以按
Signal 顺序观察发送者、接收者、响应类型、真实状态变化和失败原因。该能力属于 tools2 的展示与
发布责任，不是新的 model 语义、derive 输入或浏览器 runtime。

动画采用独立 `animate` 阶段，且只消费 tools2 version 4 `model.json` 与 `view.json`。model 提供
System identity、有效 parent、状态集合和静态结构，view 提供 derive 已经选择的 Signal 顺序、
source/target、handler、outcome、reason 与 before/after snapshot。animate 不重新求值 guard、选择
choice、创建 Signal、重跑 handler 或逆向执行 transition。

## 确定步进

- 每个已经发送的 Signal 是一个完整步进单位。同步 `drives`、异步 `emits` 以及它们产生的嵌套
  Signal 均保持 v4 view 的全局顺序，各占一步。
- 初始帧只显示第一个 Signal source 和维持 parent 结构所需的祖先；target 及其它 System 在第一次
  被 trace 需要时出现。
- 前进播放当前 Signal 的发送、目标响应和结束，并停在该 Signal 的确定 after frame。后退直接恢复
  前一个预生成 frame，不执行任何模型行为。
- 任意前进、后退和重复往返必须恢复相同的可见 System、parent 结构、状态、失败事实与同级顺序。

Transition 响应展示 target 的真实 before/after state；Action 不伪造 lifecycle state 变化。异常
outcome 必须保持原 outcome 和结构化 reason。没有自身 snapshot state、仅为补齐 parent 链而显示的
祖先是结构容器，不得获得伪造状态。外部 `Human` 等不属于 model parent 树的端点仍可作为独立端点
显示。

## 发布协议与播放器

animate 输出内嵌 `lkm.spec.signal-animation` version `1` 的自包含 HTML。animation v1 是由 v4 输入
确定生成的播放协议，至少包含输入身份/fingerprint、稳定步骤、handler/outcome/reason、初始与
after frames、可见节点、结构祖先和预计算 sibling order。JSON 必须安全内嵌，HTML 必须原子写入；
协议或 I/O 失败不得留下半成品。

播放器只负责把确定 frames 布局并呈现。父子包含优先于左右方向；同一 parent 下较晚首次出现的
System 位于较早兄弟之上。普通 Signal 使用 source 到 target 的箭头，self Signal 使用节点下方的
回环。页面提供前后按钮、`ArrowLeft`/`ArrowRight`、步数和当前 Signal 文字说明，并尊重
`prefers-reduced-motion`。首轮不包含自动播放、速度、时间线跳转、浏览器 derive 或视频导出。

## 兼容性决定

tools2 v4 model/view 的现有结构化字段已经提供上述生成语义。本轮不修改
[`../../model/SEMANTICS.md`](../../model/SEMANTICS.md)、v4 schema、derive、text renderer 或正式 model
含义；不得为前端便利从显示名称推断 handler/state，也不得复用老 `tools/` 静态 SVG renderer。
独立 CLI 是 `lkm-animate MODEL VIEW -o HTML`；tools2 driver 与便利入口另提供 `--html-out PATH`，并
保持 text `-o`、scenario、snapshot、work-dir 和 check 的 0/1 结果可组合。animate 协议/I/O 错误
返回 2。
