use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{cpu::CpuRole, cpu_id_map::CpuIdMapEntryKind, printk, state::State},
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
    let Some(boot_cpu) = ctx.cpu_group.cpu(0) else {
        printk::write_str("CpuGroup logical CPU 0 missing\n");
        return SmokeResult::Failed;
    };
    if boot_entry.logical_id() != 0
        || boot_entry.kind() != CpuIdMapEntryKind::BootCpu
        || boot_entry.hartid() != boot_cpu.hartid()
        || boot_cpu.role() != CpuRole::Boot
        || !boot_cpu.cpu_ref().is_boot_cpu()
        || !boot_cpu.is_possible()
        || !boot_cpu.is_present()
        || !boot_cpu.is_active()
        || !boot_cpu.is_online()
        || !ctx.cpu_group.possible_contains(boot_cpu.cpu_ref())
        || !ctx.cpu_group.present_contains(boot_cpu.cpu_ref())
        || !ctx.cpu_group.online_contains(boot_cpu.cpu_ref())
    {
        printk::write_str("CpuIdMap boot CPU entry invalid\n");
        return SmokeResult::Failed;
    }

    let mut logical_id = 1usize;
    while logical_id < ctx.cpu_group.possible_cpu_count() {
        let Some(cpu) = ctx.cpu_group.cpu(logical_id) else {
            printk::write_str("CpuGroup secondary CPU view missing\n");
            return SmokeResult::Failed;
        };
        if cpu.logical_id() != logical_id
            || cpu.role() != CpuRole::Secondary
            || cpu.cpu_ref().is_boot_cpu()
            || !cpu.is_possible()
            || !cpu.is_present()
            || cpu.is_active() != cpu.is_online()
            || !ctx.cpu_group.possible_contains(cpu.cpu_ref())
            || !ctx.cpu_group.present_contains(cpu.cpu_ref())
            || ctx.cpu_group.online_contains(cpu.cpu_ref()) != cpu.is_online()
        {
            printk::write_str("CpuGroup secondary CPU view invalid\n");
            return SmokeResult::Failed;
        }

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
        logical_id += 1;
    }

    printk::write_fmt(format_args!(
        "CPU group:\n  Boot CPU     : hart {}, online\n  Secondary    : {} CPUs\nCPU ID map:\n  Entries      : {}\n  Boot CPU     : logical {} -> hart {}\n",
        boot_cpu.hartid(),
        ctx.cpu_group.secondary_count(),
        cpu_id_map.count(),
        boot_entry.logical_id(),
        boot_entry.hartid()
    ));

    let mut logical_id = 1usize;
    while logical_id < ctx.cpu_group.possible_cpu_count() {
        let Some(entry) = cpu_id_map.entry(logical_id) else {
            return SmokeResult::Failed;
        };
        printk::write_fmt(format_args!(
            "  Secondary    : logical {} -> hart {}\n",
            entry.logical_id(),
            entry.hartid()
        ));
        logical_id += 1;
    }

    SmokeResult::Passed
}
