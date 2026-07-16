# Coding Objects

Object-level coding constraints live here when they need a dedicated topic. Coverage is audited at the
`spec/model/objects/*.spec` file level, not by mechanically creating one coding file for every model object
instance. Every model object specification file has exactly one row below.

Classification meanings:

- `dedicated`: the model file has its own coding topic.
- `grouped`: multiple model files share one subsystem topic.
- `phase-owned`: the mapping is inseparable from the phase that constructs/drives the object and is owned by
  that phase coding file.
- `general-only`: the file only defines shared model vocabulary and needs no object-specific rule beyond the
  general [`mapping.md`](../mapping.md) contract.

## Model object file coverage

| Model object specification | Classification | Authoritative coding landing |
| --- | --- | --- |
| [`allocator.spec`](../../model/objects/allocator.spec) | `phase-owned` | [`MmCoreInitPhase`](../phases/boot/mm-core-init.md) |
| [`bio.spec`](../../model/objects/bio.spec) | `dedicated` | [`bio.md`](bio.md) |
| [`block_device.spec`](../../model/objects/block_device.spec) | `dedicated` | [`block-device.md`](block-device.md) |
| [`bus_type.spec`](../../model/objects/bus_type.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`console.spec`](../../model/objects/console.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`devfs.spec`](../../model/objects/devfs.spec) | `grouped` | [`vfs.md`](vfs.md) |
| [`device.spec`](../../model/objects/device.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`binary_format_registry.spec`](../../model/objects/binary_format_registry.spec) | `dedicated` | [`binary-format-registry.md`](binary-format-registry.md) |
| [`elf_object.spec`](../../model/objects/elf_object.spec) | `dedicated` | [`elf-object.md`](elf-object.md) |
| [`exec_sync_boundaries.spec`](../../model/objects/exec_sync_boundaries.spec) | `dedicated` | [`exec-sync-boundaries.md`](exec-sync-boundaries.md) |
| [`exec_transaction.spec`](../../model/objects/exec_transaction.spec) | `dedicated` | [`exec-transaction.md`](exec-transaction.md) |
| [`ext2.spec`](../../model/objects/ext2.spec) | `dedicated` | [`ext2.md`](ext2.md) |
| [`files.spec`](../../model/objects/files.spec) | `grouped` | [`user-boot.md`](user-boot.md) |
| [`hwrng.spec`](../../model/objects/hwrng.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`initcall.spec`](../../model/objects/initcall.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`ioremap.spec`](../../model/objects/ioremap.spec) | `phase-owned` | [`MmCoreInitPhase`](../phases/boot/mm-core-init.md) |
| [`main.spec`](../../model/objects/main.spec) | `grouped` | [`mapping.md`](../mapping.md) |
| [`ns16550a_driver.spec`](../../model/objects/ns16550a_driver.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`object_kinds.spec`](../../model/objects/object_kinds.spec) | `general-only` | [`mapping.md`](../mapping.md) |
| [`user_boot.spec`](../../model/objects/user_boot.spec) | `dedicated` | [`user-boot.md`](user-boot.md) |
| [`user_stack.spec`](../../model/objects/user_stack.spec) | `dedicated` | [`user-stack.md`](user-stack.md) |
| [`vfs.spec`](../../model/objects/vfs.spec) | `dedicated` | [`vfs.md`](vfs.md) |
| [`virtio.spec`](../../model/objects/virtio.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`virtio_blk.spec`](../../model/objects/virtio_blk.spec) | `dedicated` | [`virtio-blk.md`](virtio-blk.md) |
| [`virtio_mmio.spec`](../../model/objects/virtio_mmio.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`virtio_ring.spec`](../../model/objects/virtio_ring.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`virtio_rng.spec`](../../model/objects/virtio_rng.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`vmalloc.spec`](../../model/objects/vmalloc.spec) | `phase-owned` | [`MmCoreInitPhase`](../phases/boot/mm-core-init.md) |

## Additional object mappings

- [`device-tree.md`](device-tree.md): DeviceTree unflatten/storage rules used by CorePreparePhase.
- [`completion.md`](completion.md): `main.spec` 中 reusable Completion primitive 的附加专用规则。
- [`effective-context.md`](effective-context.md): `main.spec` 中 context/guard primitive 的附加专用规则。

Coding `.spec` files are forbidden by the repository `coding-spec-check` gate. When a model object file is
added, removed or changes ownership, update this table in the same specification change.
