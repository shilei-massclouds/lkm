# Virtio core, MMIO, ring and RNG coding

本文件共同承载 `virtio.spec`、`virtio_mmio.spec`、`virtio_ring.spec`、`virtio_rng.spec` 和
`hwrng.spec` 的权威 coding 映射。多个 model 文件共享本专题，但各文件的覆盖归属由
[`README.md`](README.md) 逐项声明。

## Core and platform transport

`VirtioBus` 是 `Context` 下与 `PlatformBus` 并列的全局 virtio core bus，不能成为
`PlatformBus` owned child，也不能把 virtio-specific 参数加入 platform bus API。
`VirtioMmioPlatformDriver` 经 `device_initcall -> platform_driver_register -> PlatformBus OF
match/probe` 进入 `virtio_mmio_probe()`，transport 来源保持为
`PlatformDevice -> Device -> DeviceNodeId -> DeviceTree`，并经 `Ioremap` 建立 MMIO mapping。

合法且非 placeholder 的 header 才能注册 generic `VirtioDevice`；`device_id == 0` 不进入
`VirtioBus`。`VirtioDevice` 保存 core identity/status 并引用 `VirtioMmioTransportDevice`，两者
不能合并。probe 使用临时 `PlatformProbeContext` 窄能力窗口，不保存该窗口、不接收可变全局
`Context`，也不从全局反向获取可变对象。

## Split ring and MMIO queue

`VirtioSplitRing`/`VirtQueue` 是独立可复用对象，支持 single queue、split ring、direct
descriptor、`add_inbuf`、kick/notify 和 `get_buf`。生产 API 不提供 used-entry/completion
injection；测试 fixture 可以建立前置条件，但不得被生产路径或 checkpoint handler 引用。

真实 QEMU 路径使用 device-visible ring backing。legacy version 1 queue 写
`GUEST_PAGE_SIZE/QUEUE_SEL/QUEUE_NUM/QUEUE_ALIGN/QUEUE_PFN`；modern version 2 queue 写
`QUEUE_SEL/QUEUE_NUM/QUEUE_DESC/AVAIL/USED/QUEUE_READY`。分支由 MMIO `VERSION` 决定，
`QueueNotify` 写 queue index；IRQ handler 读取 `INTERRUPT_STATUS`、回写 `INTERRUPT_ACK`，仅在
vring interrupt bit 存在时调用 completion callback。通用 DMA/coherent allocator、cache
maintenance、SWIOTLB/IOMMU、packed ring、indirect descriptor、event idx、多队列和完整
reset/remove 仍为 deferred。

## Virtio RNG and hwrng

`VirtioRngDriver` 只匹配 generic `VirtioDevice` 的 `VIRTIO_ID_RNG`，probe 建立
`VirtioRngDevice`、single input `VirtQueue`、embedded `HwRngDevice` 与 `have_data: Completion`，
并提交 pending entropy request；随后 scan callback 才通过 `HwRngCore::register()` 注册 hwrng。

`HwRngCore` 维护 registered device 与 `current_rng`。读取经
`HwRngCore::read_current -> HwRngDevice::read -> VirtioRngDevice::read_entropy`，普通调用与
smoke 不绕过 core。首片只覆盖 nonblocking consumption 和耗尽后重新提交；blocking wait、
random pool、完整 `/dev/hwrng` file operations、quality/sysfs、freeze/restore 和完整资源回收
保持 deferred。devfs 的 `hwrng` 节点只提供命名空间可发现性，不增加旁路读取入口。
