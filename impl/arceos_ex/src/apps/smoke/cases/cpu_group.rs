use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        cpu::{BOOT_CPU_LOGICAL_ID, CpuRole},
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let cpu_group = &ctx.cpu_group;

    if cpu_group.state() != State::Ready {
        printk::write_str("CpuGroup is not ready\n");
        return SmokeResult::Failed;
    }
    if !cpu_group.possible_cpu_boundary_ready() {
        printk::write_str("CpuGroup possible CPU boundary is invalid\n");
        return SmokeResult::Failed;
    }

    let count = cpu_group.possible_cpu_count();
    if count == 0 {
        printk::write_str("CpuGroup has no possible CPUs\n");
        return SmokeResult::Failed;
    }

    let Some(boot_cpu) = cpu_group.boot_cpu() else {
        printk::write_str("CpuGroup boot CPU is missing\n");
        return SmokeResult::Failed;
    };
    let Some(boot_cpu_ref) = cpu_group.boot_cpu_ref() else {
        printk::write_str("CpuGroup boot CPU ref is missing\n");
        return SmokeResult::Failed;
    };
    if boot_cpu_ref != boot_cpu.cpu_ref()
        || boot_cpu.logical_id() != BOOT_CPU_LOGICAL_ID
        || !boot_cpu_ref.is_boot_cpu()
        || boot_cpu.role() != CpuRole::Boot
        || boot_cpu.state() != State::Online
        || !boot_cpu.is_possible()
        || !boot_cpu.is_present()
        || !boot_cpu.is_online()
        || !cpu_group.possible_contains(boot_cpu_ref)
        || !cpu_group.present_contains(boot_cpu_ref)
        || !cpu_group.online_contains(boot_cpu_ref)
    {
        printk::write_str("CpuGroup boot CPU facts are invalid\n");
        return SmokeResult::Failed;
    }

    let mut logical_id = 0usize;
    while logical_id < count {
        let Some(cpu) = cpu_group.cpu(logical_id) else {
            printk::write_str("CpuGroup logical CPU is missing\n");
            return SmokeResult::Failed;
        };
        let Some(cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
            printk::write_str("CpuGroup possible CPU ref is missing\n");
            return SmokeResult::Failed;
        };

        if cpu.cpu_ref() != cpu_ref
            || cpu_ref.logical_id() != logical_id
            || cpu.logical_id() != logical_id
            || !cpu.is_possible()
            || cpu_group.present_contains(cpu_ref) != cpu.is_present()
            || cpu_group.online_contains(cpu_ref) != cpu.is_online()
        {
            printk::write_str("CpuGroup logical CPU view is invalid\n");
            return SmokeResult::Failed;
        }
        if logical_id == BOOT_CPU_LOGICAL_ID {
            if cpu.role() != CpuRole::Boot || !cpu.is_online() {
                printk::write_str("CpuGroup boot CPU slot is invalid\n");
                return SmokeResult::Failed;
            }
        } else if cpu.role() != CpuRole::Secondary {
            printk::write_str("CpuGroup secondary CPU slot is invalid\n");
            return SmokeResult::Failed;
        }

        let mut previous = 0usize;
        while previous < logical_id {
            let Some(previous_cpu) = cpu_group.cpu(previous) else {
                printk::write_str("CpuGroup previous CPU view is missing\n");
                return SmokeResult::Failed;
            };
            if previous_cpu.hartid() == cpu.hartid() {
                printk::write_str("CpuGroup hartid is duplicated\n");
                return SmokeResult::Failed;
            }
            previous += 1;
        }

        logical_id += 1;
    }

    if cpu_group.cpu(count).is_some()
        || cpu_group.cpu_ref_at(count).is_some()
        || cpu_group.possible_cpu_ref_at(count).is_some()
    {
        printk::write_str("CpuGroup exposes CPU outside possible boundary\n");
        return SmokeResult::Failed;
    }
    if !ctx.secondary_cpus.all_match_cpu_group_views(cpu_group) {
        printk::write_str("CpuGroup secondary CPU views diverge from store\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "CpuGroup topology:\n  Possible CPUs : {}\n  Secondary CPUs: {}\n  Boot hartid   : {}\n",
        count,
        cpu_group.secondary_count(),
        boot_cpu.hartid()
    ));

    logical_id = 0;
    while logical_id < count {
        let Some(cpu) = cpu_group.cpu(logical_id) else {
            return SmokeResult::Failed;
        };
        printk::write_fmt(format_args!(
            "  CPU[{}] hart={} role={} online={}\n",
            logical_id,
            cpu.hartid(),
            role_name(cpu.role()),
            if cpu.is_online() { "yes" } else { "no" }
        ));
        logical_id += 1;
    }

    SmokeResult::Passed
}

fn role_name(role: CpuRole) -> &'static str {
    match role {
        CpuRole::Boot => "boot",
        CpuRole::Secondary => "secondary",
    }
}
