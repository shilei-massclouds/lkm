# FixMap

`FixMap` 是 `KernelAddrSpace` 拥有的固定虚拟槽位区域。它以预先确定的虚拟位置为启动期对象提供临时
映射能力，但不是 translation controller，也不拥有被映射的物理对象。

`FixMap.Preset` 建立固定槽位布局，并确认 FDT 槽位能够容纳 `RawDtb` 的完整物理范围。实际让该槽位
可通过早期页表访问的映射由 `EarlyVm.Setup` 建立。`FixMap` 在本段只服务启动早期映射；其它槽位和
后续使用者由各自阶段定义。

## Mapping

- Model: `spec/model/objects/fix_map.spec`
- Coding: `spec/coding/objects/fix-map.md`
- Implementation: `impl/arceos_ex/src/objects/fix_map.rs`
