use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let cache_block_info = &ctx.cache_block_info;

    if cache_block_info.state() != State::Ready {
        printk::write_str("CacheBlockInfo is not ready\n");
        return SmokeResult::Failed;
    }

    printk::write_str("Cache block info:\n");
    write_block_size("CBOM", cache_block_info.cbom_block_size());
    write_block_size("CBOZ", cache_block_info.cboz_block_size());
    printk::write_fmt(format_args!(
        "  Mismatches  : {}\n",
        cache_block_info.mismatch_count()
    ));

    SmokeResult::Passed
}

fn write_block_size(name: &str, block_size: Option<u32>) {
    printk::write_str("  ");
    printk::write_str(name);
    printk::write_str("        : ");
    if let Some(size) = block_size {
        printk::write_fmt(format_args!("{} bytes\n", size));
    } else {
        printk::write_str("unavailable\n");
    }
}
