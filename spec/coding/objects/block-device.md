# Block device coding

本文件是 `spec/model/objects/block_device.spec` 的权威 coding 映射，覆盖
`BlockDeviceRegistry`、`BlockDevice`、`dev_t`/default-device lookup 和公开读适配边界。

`BlockDevice.Setup` 建立 name、capacity、provider/read callback 等 device shell；
`BlockDevice.Enable` 才经 `BlockDeviceRegistry::register()` 的 add-disk 形状发布 major/minor、
default device 和 registry lookup 事实。Bio、BufferHead、Ext2 和 rootfs 只能依赖 Online 的
`BlockDevice`。

## Registry read role

Rule ID: `arceos_ex_must_block_io_registry_read_remain_lower_level_adapter` (MUST).

BlockDeviceRegistry::read_default()/read_by_devt(), or equivalent
direct registry reads, are a lower-level synchronous adapter below
submit_bio_wait(). They may continue to perform default/dev_t lookup
and provider dispatch, but higher filesystem-facing paths should
enter through Bio/BufferHead rather than treating registry reads as
the public block layer.
