# RawDtb Coding

`RawDtb` 必须由独立只读描述类型保存固件传入的 DTB 物理地址、固定头部范围、已读取的头部和最终
完整物理范围。它不是 `KernelAddrSpace` 的区域，也不得被 `FixMap` 或 `EarlyVm` 按所有权嵌入。

`preset` 只做不会越过固定头部的检查：地址非零、`dtb_pa + sizeof(DtbHeader)` 不溢出，并依据
OpenSBI 交接契约确认该范围可读。它可以读取并保存头部，但不得在 Preset 中提交 magic 或总长度已经
有效的事实。

`setup` 才验证 magic、头部格式、`total_size >= sizeof(DtbHeader)`、完整范围加法不溢出，以及完整
范围不超过 FDT fixed slot 容量。完整 blob 的可访问性来自固件交接契约，不得伪装为逐字节物理探测。
本对象不解析 `/memory`、`/cpus` 或其它节点。
