use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{cpu_id_map::CpuIdMapEntryKind, printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let cpu_id_map = &ctx.cpu_id_map;

    if cpu_id_map.state() != State::Ready {
        printk::write_str("CpuIdMap is not ready\n");
        return SmokeResult::Failed;
    }
    if cpu_id_map.count() != 1 {
        printk::write_str("CpuIdMap has unexpected entry count\n");
        return SmokeResult::Failed;
    }

    let Some(boot_entry) = cpu_id_map.entry(0) else {
        printk::write_str("CpuIdMap logical CPU 0 missing\n");
        return SmokeResult::Failed;
    };
    if boot_entry.logical_id() != 0
        || boot_entry.kind() != CpuIdMapEntryKind::BootCpu
        || boot_entry.hartid() != ctx.cpu_group.boot_hartid()
    {
        printk::write_str("CpuIdMap boot CPU entry invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "CPU ID map:\n  Entries      : {}\n  Boot CPU     : logical {} -> hart {}\n",
        cpu_id_map.count(),
        boot_entry.logical_id(),
        boot_entry.hartid()
    ));
    SmokeResult::Passed
}
