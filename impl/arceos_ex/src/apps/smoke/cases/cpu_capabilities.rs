use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{cpu_capabilities::IsaFacts, printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let capabilities = &ctx.cpu_capabilities;

    if capabilities.state() != State::Ready {
        printk::write_str("CpuCapabilities is not ready\n");
        return SmokeResult::Failed;
    }
    if capabilities.hart_count() != ctx.cpu_group.possible_cpu_count() {
        printk::write_str("CpuCapabilities hart count mismatch\n");
        return SmokeResult::Failed;
    }

    let common = capabilities.common_isa();
    if !common.i || !common.m || !common.a {
        printk::write_str("CpuCapabilities common base ISA missing\n");
        return SmokeResult::Failed;
    }
    if common.f && !common.d {
        printk::write_str("CpuCapabilities exposes F without D\n");
        return SmokeResult::Failed;
    }
    if common.zicbom && ctx.cache_block_info.cbom_block_size().is_none() {
        printk::write_str("CpuCapabilities exposes Zicbom without CBOM block size\n");
        return SmokeResult::Failed;
    }
    if common.zicboz && ctx.cache_block_info.cboz_block_size().is_none() {
        printk::write_str("CpuCapabilities exposes Zicboz without CBOZ block size\n");
        return SmokeResult::Failed;
    }

    printk::write_str("CPU capabilities:\n");
    printk::write_fmt(format_args!(
        "  Harts       : {}\n",
        capabilities.hart_count()
    ));
    printk::write_str("  Common ISA  : ");
    write_isa(common);
    printk::write_str("\n");
    write_feature("FPU", capabilities.fpu_supported());
    write_feature("Vector", capabilities.vector_supported());
    write_feature("Zicbom", common.zicbom);
    write_feature("Zicboz", common.zicboz);
    write_feature("Fallback", capabilities.fallback_isa_used());

    SmokeResult::Passed
}

fn write_isa(isa: IsaFacts) {
    if isa.i {
        printk::write_str("i");
    }
    if isa.m {
        printk::write_str("m");
    }
    if isa.a {
        printk::write_str("a");
    }
    if isa.f {
        printk::write_str("f");
    }
    if isa.d {
        printk::write_str("d");
    }
    if isa.c {
        printk::write_str("c");
    }
    if isa.v {
        printk::write_str("v");
    }
    if isa.zicbom {
        printk::write_str(" zicbom");
    }
    if isa.zicboz {
        printk::write_str(" zicboz");
    }
}

fn write_feature(name: &str, enabled: bool) {
    printk::write_str("  ");
    printk::write_str(name);
    if name.len() < 8 {
        let mut pad = name.len();
        while pad < 8 {
            printk::write_str(" ");
            pad += 1;
        }
    }
    printk::write_str("    : ");
    if enabled {
        printk::write_str("yes\n");
    } else {
        printk::write_str("no\n");
    }
}
