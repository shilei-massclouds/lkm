# DeviceTree SHOULD coding

本文件是 DeviceTree SHOULD 实现映射的权威 coding 规格，保留稳定 rule ID、原
`ArceosExDeviceTreeCodingShould` type 分组和 SHOULD 层级；这些 ID 用于评审和追踪，不是 pyveri predicate。

## Rule catalog

### ArceosExDeviceTreeCodingShould

#### Unsafe encapsulation

Rule ID: `arceos_ex_should_encapsulate_device_tree_unflatten_unsafe` (SHOULD).

Raw pointer writes into MemBlock-backed storage should be kept behind
a narrow internal boundary. The public DeviceTree event surface
should expose safe state/query operations.
