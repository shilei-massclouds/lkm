use alloc::vec::Vec;
use core::mem::size_of;

use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        initcall::OfPlatformCandidate, mm_core::GLOBAL_ALLOC_MAX_SIZE, printk, state::State,
    },
};

const CANDIDATE_SAFE_CAPACITY: usize = 16;
const CANDIDATE_FAILING_CAPACITY: usize = 32;

pub fn run() -> SmokeResult {
    let ctx = context();
    if ctx.kernel_global_allocator.state() != State::Ready
        || ctx.dynamic_container_runtime.state() != State::Ready
    {
        printk::write_str("allocator pressure prerequisites missing\n");
        return SmokeResult::Failed;
    }

    if !check_vec_usize_boundary()
        || !check_of_platform_candidate_growth_boundary(
            ctx.platform_bus.of_platform_candidate_count(),
        )
    {
        return SmokeResult::Failed;
    }

    SmokeResult::Passed
}

fn check_vec_usize_boundary() -> bool {
    let item_size = size_of::<usize>();
    let boundary_capacity = GLOBAL_ALLOC_MAX_SIZE / item_size;
    let overflow_capacity = boundary_capacity + 1;

    let mut values: Vec<usize> = Vec::new();
    if values.try_reserve_exact(boundary_capacity).is_err() {
        printk::write_str("Vec<usize> failed at global allocation boundary\n");
        return false;
    }
    if values.try_reserve_exact(overflow_capacity).is_ok() {
        printk::write_str("Vec<usize> unexpectedly crossed global allocation boundary\n");
        return false;
    }

    printk::write_fmt(format_args!(
        "allocator_pressure usize_size={} cap_ok={} cap_fail={} limit={}\n",
        item_size, boundary_capacity, overflow_capacity, GLOBAL_ALLOC_MAX_SIZE
    ));
    true
}

fn check_of_platform_candidate_growth_boundary(candidate_count: usize) -> bool {
    let item_size = size_of::<OfPlatformCandidate<'static>>();
    let Some(safe_bytes) = item_size.checked_mul(CANDIDATE_SAFE_CAPACITY) else {
        printk::write_str("candidate safe layout overflow\n");
        return false;
    };
    let Some(failing_bytes) = item_size.checked_mul(CANDIDATE_FAILING_CAPACITY) else {
        printk::write_str("candidate failing layout overflow\n");
        return false;
    };
    if safe_bytes > GLOBAL_ALLOC_MAX_SIZE || failing_bytes <= GLOBAL_ALLOC_MAX_SIZE {
        printk::write_str("candidate layout no longer matches allocator pressure scenario\n");
        return false;
    }

    let mut candidates: Vec<OfPlatformCandidate<'static>> = Vec::new();
    if candidates
        .try_reserve_exact(CANDIDATE_SAFE_CAPACITY)
        .is_err()
    {
        printk::write_str("candidate Vec failed before global allocation boundary\n");
        return false;
    }
    if candidates
        .try_reserve_exact(CANDIDATE_FAILING_CAPACITY)
        .is_ok()
    {
        printk::write_str("candidate Vec unexpectedly crossed global allocation boundary\n");
        return false;
    }
    if candidate_count > CANDIDATE_SAFE_CAPACITY && candidate_count <= CANDIDATE_FAILING_CAPACITY {
        printk::write_fmt(format_args!(
            "allocator_pressure of_platform_candidates={} item_size={} cap_ok={} bytes_ok={} cap_fail={} bytes_fail={}\n",
            candidate_count,
            item_size,
            CANDIDATE_SAFE_CAPACITY,
            safe_bytes,
            CANDIDATE_FAILING_CAPACITY,
            failing_bytes
        ));
    } else {
        printk::write_fmt(format_args!(
            "allocator_pressure candidate_count={} outside reproduced growth window item_size={}\n",
            candidate_count, item_size
        ));
    }

    true
}
