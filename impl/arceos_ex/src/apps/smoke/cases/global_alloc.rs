use alloc::vec::Vec;

use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

const ITEM_COUNT: usize = 96;

pub fn run() -> SmokeResult {
    let ctx = context();
    if ctx.kernel_global_allocator.state() != State::Ready
        || !ctx.kernel_global_allocator.uses_slub_subsystem()
        || !ctx.kernel_global_allocator.alloc_api_ready()
        || !ctx.kernel_global_allocator.alloc_zeroed_api_ready()
        || !ctx.kernel_global_allocator.dealloc_api_ready()
    {
        printk::write_str("kernel global allocator is not ready\n");
        return SmokeResult::Failed;
    }
    if ctx.dynamic_container_runtime.state() != State::Ready
        || !ctx.dynamic_container_runtime.uses_global_allocator()
        || !ctx.dynamic_container_runtime.vec_api_ready()
    {
        printk::write_str("dynamic container runtime is not ready\n");
        return SmokeResult::Failed;
    }

    let initial_free = ctx.slub_subsystem.kmalloc_caches().free_object_count(1024);
    let mut values = Vec::new();
    let mut growths = 0usize;
    let mut last_capacity = values.capacity();
    let mut index = 0usize;
    while index < ITEM_COUNT {
        values.push(index * 3 + 7);
        if values.capacity() != last_capacity {
            growths += 1;
            last_capacity = values.capacity();
        }
        index += 1;
    }

    if growths == 0 || values.len() != ITEM_COUNT {
        printk::write_str("Vec did not grow as expected\n");
        return SmokeResult::Failed;
    }

    let mut checksum = 0usize;
    for (index, value) in values.iter().enumerate() {
        let expected = index * 3 + 7;
        if *value != expected {
            printk::write_str("Vec value mismatch\n");
            return SmokeResult::Failed;
        }
        checksum = checksum.wrapping_add(*value);
    }
    let final_capacity = values.capacity();
    drop(values);
    let final_free = ctx.slub_subsystem.kmalloc_caches().free_object_count(1024);

    printk::write_fmt(format_args!(
        "global_alloc vec_len={} capacity={} growths={} free1024_before={} free1024_after={} checksum={}\n",
        ITEM_COUNT, final_capacity, growths, initial_free, final_free, checksum
    ));

    SmokeResult::Passed
}
