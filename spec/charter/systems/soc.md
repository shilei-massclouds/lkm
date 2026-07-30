# Soc 系统

`Soc` 代表底层物理平台的抽象。`Preset` 在内核入口准备期执行平台的早期初始化，并使
`Soc` 从 Base 迁移到 Prepared。该早期初始化的具体平台语义尚未定义，当前处于 Deferred；
Deferred 表示规格闭合状态，不是 `Soc` 的生命周期状态。
