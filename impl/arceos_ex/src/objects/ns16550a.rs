use super::{
    device::DeviceRef,
    device_tree::DeviceTree,
    driver::{DeviceDriverRef, OfMatchEntry, OfMatchTable, PlatformDriver, ProbeResult},
    initcall::{ContextRef, InitcallReturn},
};

const NS16550A_OF_MATCH: [OfMatchEntry; 1] = [OfMatchEntry::new(b"ns16550a")];

pub static NS16550A_PLATFORM_DRIVER: PlatformDriver = PlatformDriver::new(
    "of_serial",
    OfMatchTable::new(&NS16550A_OF_MATCH),
    ns16550a_probe,
);

pub const NS16550A_PLATFORM_DRIVER_REF: DeviceDriverRef =
    DeviceDriverRef::new(&NS16550A_PLATFORM_DRIVER);

pub fn ns16550a_platform_driver_init(ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: ns16550a_platform_driver_init\n");
    let device_tree = &ctx.device_tree;
    ctx.platform_bus
        .platform_driver_register(NS16550A_PLATFORM_DRIVER_REF, device_tree)
}

crate::device_initcall!(ns16550a_platform_driver_init);

pub fn is_ns16550a_platform_driver(driver: DeviceDriverRef) -> bool {
    driver == NS16550A_PLATFORM_DRIVER_REF
}

fn ns16550a_probe(_device_tree: &DeviceTree, _device: DeviceRef) -> ProbeResult {
    ProbeResult::Bound
}
