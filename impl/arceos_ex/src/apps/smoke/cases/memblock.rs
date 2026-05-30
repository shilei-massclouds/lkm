use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let page_size = ctx.config.page_size();

    if ctx.memblock.state() != State::Offline {
        printk::write_str("memblock is not offline after mm core init\n");
        return SmokeResult::Failed;
    }
    if ctx.memblock.usable_ranges().count() == 0 {
        printk::write_str("memblock usable metadata missing after offline\n");
        return SmokeResult::Failed;
    }
    if ctx.memblock.reserved_ranges().count() == 0 {
        printk::write_str("memblock reserved metadata missing after offline\n");
        return SmokeResult::Failed;
    }

    if ctx.memblock.alloc_phys(page_size, page_size).is_some() {
        printk::write_str("memblock allocation succeeded while offline\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "state=offline usable={} reserved={}\n",
        ctx.memblock.usable_ranges().count(),
        ctx.memblock.reserved_ranges().count()
    ));
    SmokeResult::Passed
}
