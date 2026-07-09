# DeviceTree SHOULD coding

本文件承载 `spec/coding/objects/device-tree-should.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/objects/device-tree-should.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`device-tree-should.spec`](device-tree-should.spec)。

### ArceosExDeviceTreeCodingShould

#### Unsafe encapsulation

Raw pointer writes into MemBlock-backed storage should be kept behind
a narrow internal boundary. The public DeviceTree event surface
should expose safe state/query operations.

<!-- formal-predicate-notes:spec/coding/objects/device-tree-should.spec END -->
