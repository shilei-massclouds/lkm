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
| [`breakpoint_exception_flow_type.spec`](../../model/objects/breakpoint_exception_flow_type.spec) | `dedicated` | [`breakpoint-exception-flow-type.md`](breakpoint-exception-flow-type.md) |
| [`breakpoint_exception_type.spec`](../../model/objects/breakpoint_exception_type.spec) | `dedicated` | [`breakpoint-exception-type.md`](breakpoint-exception-type.md) |
| [`bus_type.spec`](../../model/objects/bus_type.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`console.spec`](../../model/objects/console.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`cpu.spec`](../../model/objects/cpu.spec) | `dedicated` | [`cpu.md`](cpu.md) |
| [`cpu_group.spec`](../../model/objects/cpu_group.spec) | `dedicated` | [`cpu-group.md`](cpu-group.md) |
| [`devfs.spec`](../../model/objects/devfs.spec) | `grouped` | [`vfs.md`](vfs.md) |
| [`device.spec`](../../model/objects/device.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`binary_format_registry.spec`](../../model/objects/binary_format_registry.spec) | `dedicated` | [`binary-format-registry.md`](binary-format-registry.md) |
| [`elf_object.spec`](../../model/objects/elf_object.spec) | `dedicated` | [`elf-object.md`](elf-object.md) |
| [`early_vm.spec`](../../model/objects/early_vm.spec) | `dedicated` | [`early-vm.md`](early-vm.md) |
| [`exception_flow_type.spec`](../../model/objects/exception_flow_type.spec) | `dedicated` | [`exception-flow-type.md`](exception-flow-type.md) |
| [`exception_type.spec`](../../model/objects/exception_type.spec) | `dedicated` | [`exception-type.md`](exception-type.md) |
| [`exec_sync_boundaries.spec`](../../model/objects/exec_sync_boundaries.spec) | `dedicated` | [`exec-sync-boundaries.md`](exec-sync-boundaries.md) |
| [`exec_transaction.spec`](../../model/objects/exec_transaction.spec) | `dedicated` | [`exec-transaction.md`](exec-transaction.md) |
| [`ext2.spec`](../../model/objects/ext2.spec) | `dedicated` | [`ext2.md`](ext2.md) |
| [`files.spec`](../../model/objects/files.spec) | `grouped` | [`user-boot.md`](user-boot.md) |
| [`fix_map.spec`](../../model/objects/fix_map.spec) | `dedicated` | [`fix-map.md`](fix-map.md) |
| [`hwrng.spec`](../../model/objects/hwrng.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`initcall.spec`](../../model/objects/initcall.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`interrupt_flow_type.spec`](../../model/objects/interrupt_flow_type.spec) | `dedicated` | [`interrupt-flow-type.md`](interrupt-flow-type.md) |
| [`interrupt_type.spec`](../../model/objects/interrupt_type.spec) | `dedicated` | [`interrupt-type.md`](interrupt-type.md) |
| [`ioremap.spec`](../../model/objects/ioremap.spec) | `phase-owned` | [`MmCoreInitPhase`](../phases/boot/mm-core-init.md) |
| [`kernel_image.spec`](../../model/objects/kernel_image.spec) | `dedicated` | [`kernel-image.md`](kernel-image.md) |
| [`kernel_addr_space.spec`](../../model/objects/kernel_addr_space.spec) | `dedicated` | [`kernel-address-space.md`](kernel-address-space.md) |
| [`linear_map.spec`](../../model/objects/linear_map.spec) | `dedicated` | [`linear-map.md`](linear-map.md) |
| [`main.spec`](../../model/objects/main.spec) | `grouped` | [`mapping.md`](../mapping.md) |
| [`ns16550a_driver.spec`](../../model/objects/ns16550a_driver.spec) | `phase-owned` | [`InitcallPhase`](../phases/smp-runtime/initcall.md) |
| [`object_kinds.spec`](../../model/objects/object_kinds.spec) | `general-only` | [`mapping.md`](../mapping.md) |
| [`page_fault_exception_flow_type.spec`](../../model/objects/page_fault_exception_flow_type.spec) | `dedicated` | [`page-fault-exception-flow-type.md`](page-fault-exception-flow-type.md) |
| [`page_fault_exception_type.spec`](../../model/objects/page_fault_exception_type.spec) | `dedicated` | [`page-fault-exception-type.md`](page-fault-exception-type.md) |
| [`physical_direct.spec`](../../model/objects/physical_direct.spec) | `dedicated` | [`physical-direct.md`](physical-direct.md) |
| [`raw_dtb.spec`](../../model/objects/raw_dtb.spec) | `dedicated` | [`raw-dtb.md`](raw-dtb.md) |
| [`scheduler.spec`](../../model/objects/scheduler.spec) | `dedicated` | [`scheduler.md`](scheduler.md) |
| [`swapper_vm.spec`](../../model/objects/swapper_vm.spec) | `dedicated` | [`swapper-vm.md`](swapper-vm.md) |
| [`syscall_exception_flow_type.spec`](../../model/objects/syscall_exception_flow_type.spec) | `dedicated` | [`syscall-exception-flow-type.md`](syscall-exception-flow-type.md) |
| [`syscall_exception_type.spec`](../../model/objects/syscall_exception_type.spec) | `dedicated` | [`syscall-exception-type.md`](syscall-exception-type.md) |
| [`task.spec`](../../model/objects/task.spec) | `dedicated` | [`task.md`](task.md) |
| [`task_flow.spec`](../../model/objects/task_flow.spec) | `dedicated` | [`task-flow.md`](task-flow.md) |
| [`trap_flow_type.spec`](../../model/objects/trap_flow_type.spec) | `dedicated` | [`trap-flow-type.md`](trap-flow-type.md) |
| [`trap_type.spec`](../../model/objects/trap_type.spec) | `dedicated` | [`trap-type.md`](trap-type.md) |
| [`trampoline_vm.spec`](../../model/objects/trampoline_vm.spec) | `dedicated` | [`trampoline-vm.md`](trampoline-vm.md) |
| [`unexpected_exception_flow_type.spec`](../../model/objects/unexpected_exception_flow_type.spec) | `dedicated` | [`unexpected-exception-flow-type.md`](unexpected-exception-flow-type.md) |
| [`unexpected_exception_type.spec`](../../model/objects/unexpected_exception_type.spec) | `dedicated` | [`unexpected-exception-type.md`](unexpected-exception-type.md) |
| [`user_boot.spec`](../../model/objects/user_boot.spec) | `dedicated` | [`user-boot.md`](user-boot.md) |
| [`user_space_reserve.spec`](../../model/objects/user_space_reserve.spec) | `dedicated` | [`user-space-reserve.md`](user-space-reserve.md) |
| [`user_stack.spec`](../../model/objects/user_stack.spec) | `dedicated` | [`user-stack.md`](user-stack.md) |
| [`vfs.spec`](../../model/objects/vfs.spec) | `dedicated` | [`vfs.md`](vfs.md) |
| [`virtio.spec`](../../model/objects/virtio.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`virtio_blk.spec`](../../model/objects/virtio_blk.spec) | `dedicated` | [`virtio-blk.md`](virtio-blk.md) |
| [`virtio_mmio.spec`](../../model/objects/virtio_mmio.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`virtio_ring.spec`](../../model/objects/virtio_ring.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`virtio_rng.spec`](../../model/objects/virtio_rng.spec) | `grouped` | [`virtio.md`](virtio.md) |
| [`vmalloc.spec`](../../model/objects/vmalloc.spec) | `phase-owned` | [`MmCoreInitPhase`](../phases/boot/mm-core-init.md) |
| [`vm.spec`](../../model/objects/vm.spec) | `dedicated` | [`vm.md`](vm.md) |

## Additional object mappings

- [`dynamic-instance-declaration.md`](dynamic-instance-declaration.md): shared parser/model/check/derive/view/render lowering for runtime Type instances.
- [`device-tree.md`](device-tree.md): DeviceTree unflatten/storage rules used by CorePreparePhase.
- [`completion.md`](completion.md): `main.spec` 中 reusable Completion primitive 的附加专用规则。
- [`effective-context.md`](effective-context.md): `main.spec` 中 context/guard primitive 的附加专用规则。

Coding `.spec` files are forbidden by the repository `coding-spec-check` gate. When a model object file is
added, removed or changes ownership, update this table in the same specification change.
