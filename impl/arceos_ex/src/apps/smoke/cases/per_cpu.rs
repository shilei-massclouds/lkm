use crate::{
    apps::smoke::SmokeResult,
    context::context,
    define_per_cpu,
    objects::{per_cpu_storage::PerCpuSymbol, printk, state::State},
};

define_per_cpu!(static SMOKE_PER_CPU_COUNTER: usize = 0);

pub fn run() -> SmokeResult {
    let ctx = context();
    let per_cpu = &ctx.per_cpu_storage;
    let initial_state = per_cpu.state();

    if initial_state != State::Ready {
        printk::write_str("per-cpu storage is not ready\n");
        return SmokeResult::Failed;
    }

    let symbol = PerCpuSymbol::new(&SMOKE_PER_CPU_COUNTER);
    let count = ctx.cpu_group.possible_cpu_count();
    if count == 0 || count != ctx.cpu_group.possible_cpu_count() {
        printk::write_str("per-cpu logical CPU count invalid\n");
        return SmokeResult::Failed;
    }

    let template_value = unsafe { core::ptr::read_volatile(&SMOKE_PER_CPU_COUNTER) };
    if template_value != 0 {
        printk::write_str("per-cpu template has unexpected value\n");
        return SmokeResult::Failed;
    }

    let mut logical_id = 0usize;
    while logical_id < count {
        let value = 0x5043_0000usize + logical_id;
        if per_cpu.write_per_cpu(symbol, logical_id, value).is_none() {
            printk::write_str("per-cpu instance write failed\n");
            return SmokeResult::Failed;
        }
        logical_id += 1;
    }

    logical_id = 0;
    while logical_id < count {
        let Some(value) = per_cpu.read_per_cpu(symbol, logical_id) else {
            return SmokeResult::Failed;
        };
        if value != 0x5043_0000usize + logical_id {
            printk::write_str("per-cpu instance readback mismatch\n");
            return SmokeResult::Failed;
        }
        logical_id += 1;
    }

    if unsafe { core::ptr::read_volatile(&SMOKE_PER_CPU_COUNTER) } != 0 {
        printk::write_str("per-cpu template was modified\n");
        return SmokeResult::Failed;
    }
    if per_cpu.state() != initial_state {
        printk::write_str("per-cpu action changed lifecycle state\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "Per-CPU static access:\n  Instances    : {}\n  Template     : unchanged\n",
        count
    ));
    SmokeResult::Passed
}
