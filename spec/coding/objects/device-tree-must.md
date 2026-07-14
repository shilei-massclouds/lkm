# DeviceTree MUST coding

本文件是 DeviceTree MUST 实现映射的权威 coding 规格，保留稳定 rule ID、原
`ArceosExDeviceTreeCodingMust` type 分组和 MUST 层级；这些 ID 用于评审和追踪，不是 pyveri predicate。

## Rule catalog

### ArceosExDeviceTreeCodingMust

#### MemBlock allocation

Rule ID: `arceos_ex_must_device_tree_setup_allocates_from_memblock` (MUST).

DeviceTree.setup() must allocate the expanded tree storage through
MemBlock early allocation. Its dependency on MemBlock.Online is an
allocation capability, not merely a read-only state check.

#### Two-pass unflatten

Rule ID: `arceos_ex_must_device_tree_unflatten_uses_two_passes` (MUST).

DeviceTree.setup() must first traverse RawDtb to validate and compute
the required expanded storage, then allocate storage, then traverse
RawDtb again to populate DeviceNode and property relations.

#### Established mapping

Rule ID: `arceos_ex_must_device_tree_storage_uses_established_linear_mapping` (MUST).

MemBlock returns physical storage. DeviceTree.setup() may write the
expanded tree only after resolving that storage through the already
established kernel linear mapping provided by SwapperVm/Config.

#### No heap or fixed static substitute

Rule ID: `arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage` (MUST).

DeviceTree.setup() must not use the ordinary heap, Vec/Box, or a
fixed static array as the final expanded tree storage. A bounded
temporary stack/global workspace is allowed only for traversal state,
not as the DeviceTree storage itself.

#### Checkpoint after validation

Rule ID: `arceos_ex_must_device_tree_checkpoint_after_validation` (MUST).

The DeviceTree Ready checkpoint may be emitted only after the second
pass has populated the tree and the model-level root, parent/child,
path lookup and property query facts have been checked.
