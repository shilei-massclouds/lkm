# RawDtb

`RawDtb` 是固件在内核入口交付的只读物理 device-tree blob。它不是 `KernelAddrSpace` 的虚拟地址区域，
也不被 `FixMap` 或 `EarlyVm` 拥有；这些系统只为它提供启动早期的访问方式。

`RawDtb.Preset` 确认入口物理地址和固定头部范围可以安全读取。`RawDtb.Setup` 检查头部标识、完整大小
及范围计算，形成可供 `FixMap` 容量判断和 `EarlyVm` 映射使用的完整物理范围。完整 blob 的存在和可
访问性来自固件交接契约，不解释为逐字节探测。

本段不解析 `/memory`、`/cpus` 或其它节点；这些内容属于后续 `EarlyDtb` 与正式 `DeviceTree` 系统。

## Mapping

- Model: `spec/model/objects/raw_dtb.spec`
- Coding: `spec/coding/objects/raw-dtb.md`
- Implementation: `impl/arceos_ex/src/objects/raw_dtb.rs`
