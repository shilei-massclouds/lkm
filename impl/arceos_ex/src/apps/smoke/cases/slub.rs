use crate::{
    apps::smoke::SmokeResult,
    context::{Context, context},
    objects::{
        mm_core::{GfpFlags, KmallocAllocRef, NamedSlubCacheKind, SlubState},
        printk,
        state::State,
    },
};

const KMALLOC_WORD_8: usize = 0x4b4d_414c_4c4f_4338;
const KMALLOC_WORD_64: usize = 0x4b4d_414c_4c36_3421;
const KMALLOC_WORD_1024: usize = 0x4b4d_414c_4c31_4b21;
const KMALLOC_INDEPENDENT_A: usize = 0x5151_5151_aaaa_0001;
const KMALLOC_INDEPENDENT_B: usize = 0x5151_5151_bbbb_0002;

pub fn run() -> SmokeResult {
    let ctx = context();

    if check_slub_facts(ctx).is_none() {
        return SmokeResult::Failed;
    }
    let Some(diag) = run_kmalloc_api_smoke(ctx) else {
        return SmokeResult::Failed;
    };

    let slub = &ctx.slub_subsystem;
    let registry = slub.cache_registry();
    let kmalloc = slub.kmalloc_caches();
    printk::write_fmt(format_args!(
        "registry_caches={} registry_kmalloc_caches={} kmalloc_refs={} slabs={} backing_pages={} reuse={:#x} zeroed={} cache64={}\n",
        registry.cache_count(),
        registry.kmalloc_cache_count(),
        kmalloc.count(),
        kmalloc.slab_count(),
        kmalloc.backing_page_count(),
        diag.reused_addr,
        diag.zeroed_bytes,
        diag.cache64_size
    ));
    SmokeResult::Passed
}

struct SlubSmokeDiag {
    reused_addr: usize,
    zeroed_bytes: usize,
    cache64_size: usize,
}

fn check_slub_facts(ctx: &Context) -> Option<()> {
    let slub = &ctx.slub_subsystem;
    let registry = slub.cache_registry();
    let kmalloc = slub.kmalloc_caches();
    if slub.state() != State::Ready || slub.slab_state() != SlubState::Up {
        printk::write_str("slub subsystem is not up\n");
        return None;
    }
    if !slub.boot_kmem_cache_node_ready()
        || !slub.bootstrap_completed()
        || !slub.cpu_cache_ready()
        || !slub.cpuhp_step_registered()
    {
        printk::write_str("slub bootstrap facts missing\n");
        return None;
    }
    if registry.state() != State::Ready
        || !registry.boot_caches_registered()
        || !registry.global_list_ready()
        || registry.cache_count() < 2
        || registry.named_cache_count() < 6
    {
        printk::write_str("slub cache registry invalid\n");
        return None;
    }
    if !registry.has_named_cache(NamedSlubCacheKind::PageTableLock)
        || !registry.has_named_cache(NamedSlubCacheKind::VmapArea)
        || !registry.has_named_cache(NamedSlubCacheKind::MmStruct)
        || !registry.has_named_cache(NamedSlubCacheKind::RadixTreeNode)
        || !registry.has_named_cache(NamedSlubCacheKind::MapleNode)
        || !registry.has_named_cache(NamedSlubCacheKind::PoolWorkqueue)
    {
        printk::write_str("named slub cache registry entries missing\n");
        return None;
    }
    if kmalloc.state() != State::Ready
        || !kmalloc.size_index_ready()
        || !kmalloc.default_cache_ready()
        || !kmalloc.random_caches_trimmed()
        || !kmalloc.memcg_caches_trimmed()
        || !kmalloc.has_size(8)
        || !kmalloc.has_size(1024)
        || !kmalloc.has_size(8192)
    {
        printk::write_str("kmalloc cache facts invalid\n");
        return None;
    }
    let minimum_registered_caches =
        2 + registry.kmalloc_cache_count() + registry.named_cache_count();
    if registry.kmalloc_cache_count() != kmalloc.count()
        || registry.cache_count() < minimum_registered_caches
    {
        printk::write_str("kmalloc cache registry/reference facts invalid\n");
        return None;
    }
    Some(())
}

fn run_kmalloc_api_smoke(ctx: &mut Context) -> Option<SlubSmokeDiag> {
    let alloc8 = kmalloc_write_read(ctx, 8, KMALLOC_WORD_8)?;
    let alloc64 = kmalloc_write_read(ctx, 64, KMALLOC_WORD_64)?;
    let alloc1024 = kmalloc_write_read(ctx, 1024, KMALLOC_WORD_1024)?;

    if !ctx.slub_subsystem.kfree(alloc8)
        || !ctx.slub_subsystem.kfree(alloc64)
        || !ctx.slub_subsystem.kfree(alloc1024)
    {
        printk::write_str("kfree basic allocations failed\n");
        return None;
    }

    let zeroed = ctx.slub_subsystem.kzalloc(
        64,
        GfpFlags::kernel(),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    )?;
    if !allocation_is_zeroed(zeroed, 64) {
        printk::write_str("kzalloc did not return zeroed storage\n");
        return None;
    }
    if !ctx.slub_subsystem.kfree(zeroed) {
        printk::write_str("kfree kzalloc allocation failed\n");
        return None;
    }

    let first = ctx.slub_subsystem.kmalloc(
        64,
        GfpFlags::kernel(),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    )?;
    let second = ctx.slub_subsystem.kmalloc(
        64,
        GfpFlags::kernel(),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    )?;
    if first.addr() == second.addr() {
        printk::write_str("kmalloc returned duplicate live allocation\n");
        return None;
    }
    if !write_read_word(first.addr(), KMALLOC_INDEPENDENT_A)
        || !write_read_word(second.addr(), KMALLOC_INDEPENDENT_B)
        || read_word(first.addr()) != Some(KMALLOC_INDEPENDENT_A)
        || read_word(second.addr()) != Some(KMALLOC_INDEPENDENT_B)
    {
        printk::write_str("same-size kmalloc allocations overlap\n");
        return None;
    }
    let reuse_addr = second.addr();
    if !ctx.slub_subsystem.kfree(second) {
        printk::write_str("kfree second allocation failed\n");
        return None;
    }
    let reused = ctx.slub_subsystem.kmalloc(
        64,
        GfpFlags::kernel(),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    )?;
    if reused.addr() != reuse_addr {
        printk::write_str("kmalloc freelist did not reuse recently freed object\n");
        return None;
    }
    if !ctx.slub_subsystem.kfree(reused) || !ctx.slub_subsystem.kfree(first) {
        printk::write_str("kfree reused allocations failed\n");
        return None;
    }

    Some(SlubSmokeDiag {
        reused_addr: reuse_addr,
        zeroed_bytes: 64,
        cache64_size: ctx.slub_subsystem.kmalloc_caches().kmalloc_size(64)?,
    })
}

fn kmalloc_write_read(ctx: &mut Context, size: usize, value: usize) -> Option<KmallocAllocRef> {
    let alloc_ref = ctx.slub_subsystem.kmalloc(
        size,
        GfpFlags::kernel(),
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    )?;
    if alloc_ref.requested_size() != size
        || alloc_ref.cache_size() < size
        || !write_read_word(alloc_ref.addr(), value)
    {
        printk::write_str("kmalloc write/read failed\n");
        return None;
    }
    Some(alloc_ref)
}

fn allocation_is_zeroed(alloc_ref: KmallocAllocRef, size: usize) -> bool {
    let mut offset = 0usize;
    while offset < size {
        let byte = unsafe { core::ptr::read_volatile((alloc_ref.addr() + offset) as *const u8) };
        if byte != 0 {
            return false;
        }
        offset += 1;
    }
    true
}

fn write_read_word(address: usize, value: usize) -> bool {
    if !address.is_multiple_of(core::mem::align_of::<usize>()) {
        return false;
    }
    let word = address as *mut usize;
    unsafe {
        core::ptr::write_volatile(word, value);
        core::ptr::read_volatile(word) == value
    }
}

fn read_word(address: usize) -> Option<usize> {
    if !address.is_multiple_of(core::mem::align_of::<usize>()) {
        return None;
    }
    Some(unsafe { core::ptr::read_volatile(address as *const usize) })
}
