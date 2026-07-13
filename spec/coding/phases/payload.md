# PayloadPhase 编码指引

PayloadPhase 是[Kernel 系统编码](../systems/kernel.md)中 `Kernel.Enable` 的最后一个直接 drive
目标。它必须遵循[阶段范式代码映射](../phase-paradigm.md)，由 Kernel continuation 调用
`Payload.Preset`，不能由 Finalize 子阶段直接作为 sibling 推进。

当前 model 已定义 `Payload.Preset -> Prepared` 和 `Payload.Enable -> Online`，但尚未定义
`Prepared -> Ready` 的 `Setup` 以及 Preset/Setup 的同对象 `emits`。因此 coding 层不能生成一条
虚构的完整调用链；该 model 缺口在 Payload 批次按 charter -> model -> coding -> impl 顺序
修正。修正前，现有实现路径只作为审计证据，不覆盖阶段范式。

Payload.Online 提交、Kernel.Online continuation 和 selected payload 不返回交接必须是三个可
定位边界；具体状态、checkpoint 和函数落点在 Payload 审计批次细化。
