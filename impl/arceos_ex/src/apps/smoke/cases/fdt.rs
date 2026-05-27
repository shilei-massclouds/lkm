use crate::{apps::smoke::SmokeResult, context::context, objects::printk};

pub fn run() -> SmokeResult {
    let ctx = context();
    let boot_hartid = ctx.cpu_group.boot_hartid();
    let memory_ranges = ctx.early_dtb.memory_range_count();

    if !ctx.early_dtb.has_boot_hart(boot_hartid) {
        printk::write_str("fdt boot hart missing\n");
        return SmokeResult::Failed;
    }
    if memory_ranges == 0 {
        printk::write_str("fdt memory ranges missing\n");
        return SmokeResult::Failed;
    }
    if !ctx.early_dtb.has_earlycon_sbi_cmdline() {
        printk::write_str("fdt earlycon=sbi cmdline missing\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "boot_hart={} memory_ranges={}\n",
        boot_hartid, memory_ranges
    ));
    SmokeResult::Passed
}
