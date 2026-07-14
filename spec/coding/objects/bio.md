# Bio and BufferHead coding

本文件是 `spec/model/objects/bio.spec` 的权威 coding 映射，保留从旧
`ArceosExBlockIoCodingMust` 分组迁入的稳定 rule ID 和 MUST 层级。

## Linux-like block I/O adapter

Rule ID: `arceos_ex_must_block_io_model_bio_buffer_head_before_ext2` (MUST).

Before read-only ext2 is introduced, the first filesystem-facing
block I/O surface must be modeled as Bio, submit_bio_wait(),
minimal blk_mq_submit_bio(), and BufferHead/sb_bread(). It must not
introduce BlockReadRequest or BlockIoBuffer as substitute Linux
top-level objects.

`Bio` carries the read operation and target block device. `submit_bio_wait()` is the synchronous
filesystem-facing entry, the minimal `blk_mq_submit_bio()` shape performs provider dispatch, and
`BufferHead`/`sb_bread()` is the filesystem block cache adapter for the current read-only slice.

## Smoke entry

Rule ID: `arceos_ex_must_block_io_smoke_use_sb_bread_path` (MUST).

App smoke coverage for the current ext2-superblock read must use
sb_bread()/BufferHead over submit_bio_wait(). It may still validate
registry facts produced underneath, but it must not bypass the new
block I/O adapter by directly calling registry read APIs.

## BufferHead storage

Rule ID: `arceos_ex_must_buffer_head_data_not_be_large_stack_storage` (MUST).

BufferHead may carry up to 4KiB ext2 blocks, so its data payload
must not be embedded as a large stack-allocated array or returned
through nested stack frames. The BufferHead object should own
heap-backed or equivalent exclusive dynamic storage for block data.
