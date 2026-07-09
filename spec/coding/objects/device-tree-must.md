# DeviceTree MUST coding

本文件承载 `spec/coding/objects/device-tree-must.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/objects/device-tree-must.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`device-tree-must.spec`](device-tree-must.spec)。

### ArceosExDeviceTreeCodingMust

#### MemBlock allocation

DeviceTree.setup() must allocate the expanded tree storage through
MemBlock early allocation. Its dependency on MemBlock.Online is an
allocation capability, not merely a read-only state check.

#### Two-pass unflatten

DeviceTree.setup() must first traverse RawDtb to validate and compute
the required expanded storage, then allocate storage, then traverse
RawDtb again to populate DeviceNode and property relations.

#### Established mapping

MemBlock returns physical storage. DeviceTree.setup() may write the
expanded tree only after resolving that storage through the already
established kernel linear mapping provided by SwapperVm/Config.

#### No heap or fixed static substitute

DeviceTree.setup() must not use the ordinary heap, Vec/Box, or a
fixed static array as the final expanded tree storage. A bounded
temporary stack/global workspace is allowed only for traversal state,
not as the DeviceTree storage itself.

#### Checkpoint after validation

The DeviceTree Ready checkpoint may be emitted only after the second
pass has populated the tree and the model-level root, parent/child,
path lookup and property query facts have been checked.

<!-- formal-predicate-notes:spec/coding/objects/device-tree-must.spec END -->
