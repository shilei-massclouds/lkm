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
    let root_domain = ctx.scheduler.default_root_domain();
    let cpu_group = &ctx.cpu_group;

    if root_domain.state() != State::Ready {
        printk::write_str("DefaultSchedRootDomain is not ready\n");
        return SmokeResult::Failed;
    }
    if cpu_group.state() != State::Ready || !cpu_group.possible_cpu_boundary_ready() {
        printk::write_str("CpuGroup possible CPU boundary is invalid\n");
        return SmokeResult::Failed;
    }
    if root_domain.covered_cpu_count() != cpu_group.possible_cpu_count()
        || root_domain.possible_cpu_count() != cpu_group.possible_cpu_count()
        || !root_domain.covers_cpu_group_possible(cpu_group)
    {
        printk::write_str("DefaultSchedRootDomain coverage count is invalid\n");
        return SmokeResult::Failed;
    }
    if !root_domain.smp_topology_deferred() {
        printk::write_str("DefaultSchedRootDomain SMP topology deferral missing\n");
        return SmokeResult::Failed;
    }

    let Some(boot_cpu_ref) = cpu_group.boot_cpu_ref() else {
        printk::write_str("CpuGroup boot CPU ref is missing\n");
        return SmokeResult::Failed;
    };
    if !root_domain.covers_cpu_ref(boot_cpu_ref)
        || root_domain.covered_cpu_ref(BOOT_CPU_LOGICAL_ID) != Some(boot_cpu_ref)
    {
        printk::write_str("DefaultSchedRootDomain does not cover boot CPU\n");
        return SmokeResult::Failed;
    }

    let count = cpu_group.possible_cpu_count();
    let mut logical_id = 0usize;
    while logical_id < count {
        let Some(cpu) = cpu_group.cpu(logical_id) else {
            printk::write_str("CpuGroup CPU view is missing\n");
            return SmokeResult::Failed;
        };
        let Some(possible_cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
            printk::write_str("CpuGroup possible CPU ref is missing\n");
            return SmokeResult::Failed;
        };
        let Some(covered_cpu_ref) = root_domain.covered_cpu_ref(logical_id) else {
            printk::write_str("DefaultSchedRootDomain covered CPU ref is missing\n");
            return SmokeResult::Failed;
        };

        if covered_cpu_ref != possible_cpu_ref
            || covered_cpu_ref != cpu.cpu_ref()
            || covered_cpu_ref.logical_id() != logical_id
            || !root_domain.covers_cpu_ref(covered_cpu_ref)
        {
            printk::write_str("DefaultSchedRootDomain covered CPU ref is invalid\n");
            return SmokeResult::Failed;
        }
        if logical_id == BOOT_CPU_LOGICAL_ID {
            if cpu.role() != CpuRole::Boot {
                printk::write_str("DefaultSchedRootDomain boot coverage is invalid\n");
                return SmokeResult::Failed;
            }
        } else if cpu.role() != CpuRole::Secondary {
            printk::write_str("DefaultSchedRootDomain secondary coverage is invalid\n");
            return SmokeResult::Failed;
        }

        logical_id += 1;
    }

    if root_domain.covered_cpu_ref(count).is_some() {
        printk::write_str("DefaultSchedRootDomain exposes CPU outside covered boundary\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "DefaultSchedRootDomain:\n  Covered CPUs : {}\n  Source       : CpuGroup.possible\n  SMP topology : deferred\n",
        root_domain.covered_cpu_count()
    ));
    SmokeResult::Passed
}
