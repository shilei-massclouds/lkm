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
    if cpu_id_map.count() != ctx.cpu_group.possible_cpu_count() {
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

    let mut index = 0usize;
    while index < ctx.cpu_group.secondary_count() {
        let Some(cpu) = ctx.cpu_group.secondary_cpu(index) else {
            printk::write_str("secondary CPU entry missing\n");
            return SmokeResult::Failed;
        };
        if !cpu.is_possible() || !cpu.is_present() || cpu.is_online() {
            printk::write_str("secondary CPU state invalid\n");
            return SmokeResult::Failed;
        }

        let logical_id = index + 1;
        let Some(entry) = cpu_id_map.entry(logical_id) else {
            printk::write_str("secondary CPU map entry missing\n");
            return SmokeResult::Failed;
        };
        if entry.logical_id() != logical_id
            || entry.kind() != CpuIdMapEntryKind::SecondaryCpu
            || entry.hartid() != cpu.hartid()
        {
            printk::write_str("secondary CPU map entry invalid\n");
            return SmokeResult::Failed;
        }
        index += 1;
    }

    printk::write_fmt(format_args!(
        "CPU group:\n  Boot CPU     : hart {}, online\n  Secondary    : {} CPUs\nCPU ID map:\n  Entries      : {}\n  Boot CPU     : logical {} -> hart {}\n",
        ctx.cpu_group.boot_hartid(),
        ctx.cpu_group.secondary_count(),
        cpu_id_map.count(),
        boot_entry.logical_id(),
        boot_entry.hartid()
    ));
    SmokeResult::Passed
}
