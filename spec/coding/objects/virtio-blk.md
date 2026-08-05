# Virtio block coding

本文件是 `spec/model/objects/virtio_blk.spec` 的权威 coding 映射，覆盖同步请求、completion
owner 和 virtqueue ordering。保留旧 `ArceosExBlockIoCodingMust` 中相关稳定 rule ID 和 MUST
层级。

`VirtioBlkDevice.Setup` 建立 matched/probed device、single `VirtQueue` 和 embedded
`BlockDevice` shell；`Enable` 在 transport/config/feature/queue/DRIVER_OK 完成后提交 capacity、
queue-ready 和 driver-ok 事实。BlockDevice 的发布边界见 [`block-device.md`](block-device.md)。

## Synchronous request lifecycle

Rule ID: `arceos_ex_must_virtio_blk_sync_reads_submit_wait_complete_before_return` (MUST).

The current read-only block path is synchronous from Bio/
BufferHead's perspective. Every live virtio-blk read must submit one
request, wait for that exact pending token to complete through IRQ
or bounded task-side polling, verify the status byte, release the
descriptor chain, and return to the caller only after the request is
no longer pending. A later read must not merely spin on an inherited
pending flag; it must first converge that inherited request or fail
with a structured block I/O error.

The single static request buffer, queue mutation and completion consumer are
protected by one real IRQ-safe acquire/release lock. Task-context synchronous
readers take that lock with the blocking acquire path, so contention with a
different live reader delays submission instead of being translated to
`DeviceNotReady`. The IRQ completion path uses only `try_lock`: when the task
reader owns the lock, the IRQ path leaves used-ring consumption to that
reader's bounded polling loop. A boolean compare/exchange owner whose failed
acquisition is returned through the block/VFS error path does not satisfy this
rule.

## Initcall superblock probe convergence

Rule ID: `arceos_ex_must_virtio_blk_initcall_superblock_probe_converge_before_ready` (MUST).

The initcall-time virtio-blk ext2 superblock probe is the first
production read request. It must complete before VirtioBlkReady and
before RootfsPhase/VFS/ext2 consumers can issue their own reads.
This follows Linux's request lifecycle shape where a request handed
to the queue is eventually ended before synchronous callers proceed,
rather than leaving a fire-and-forget used-ring entry for a later
phase to inherit.

## Single completion consumer

Rule ID: `arceos_ex_must_virtio_blk_completion_consumer_be_single_owner` (MUST).

IRQ completion and task-side polling are both valid observation
sources, but the same pending token may be consumed only once. The
implementation must guard the virtqueue/device/static read-buffer
completion path so an IRQ handler and the waiting task cannot race
through get_buf/status validation/descriptors release for the same
request.

## Virtqueue memory ordering

Rule IDs (MUST):

- `arceos_ex_must_virtqueue_publish_avail_before_notify_with_release_order`
- `arceos_ex_must_virtqueue_observe_used_with_acquire_order`

Publishing a descriptor chain must order descriptor and avail-ring
stores before avail idx and MMIO notify. Observing a used-ring idx
from the device must acquire-order subsequent reads of the used
element, status byte and data buffer. The current static coherent
backing keeps cache maintenance deferred, but it must not omit these
ordering boundaries.
