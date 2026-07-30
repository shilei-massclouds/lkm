# FixMap Coding

`FixMap` 必须由独立区域类型保存固定槽位布局；槽位地址和容量来自统一架构配置，不能在 RawDtb、
EarlyVm 与页表 helper 中重复硬编码。本轮 FDT slot 的配置容量为 2 MiB。

`preset` 验证 FDT slot 存在、页对齐且足以覆盖 `RawDtb` 完整范围，并记录 RawDtb 被安排到该槽位。
这一记录只是布局事实，不得表示页表映射已经可访问。真正建立 PTE 并发布可访问事实只能发生在
`EarlyVm.Setup`。
