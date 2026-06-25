use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        mm_core::{NamedSlubCacheKind, SlubState},
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let mm_cache = &ctx.mm_struct_cache;
    let slub = &ctx.slub_subsystem;
    let registry = slub.cache_registry();

    if slub.state() != State::Ready
        || slub.slab_state() != SlubState::Up
        || registry.state() != State::Ready
    {
        printk::write_str("slub registry is not ready for mm_struct cache\n");
        return SmokeResult::Failed;
    }
    if mm_cache.state() != State::Ready
        || !mm_cache.registered_in_slub_registry()
        || mm_cache.object_size() == 0
        || !mm_cache.saved_auxv_usercopy_ready()
        || !mm_cache.vma_caches_deferred()
    {
        printk::write_str("mm_struct cache facts invalid\n");
        return SmokeResult::Failed;
    }
    let Some(named_cache) = registry.named_cache(NamedSlubCacheKind::MmStruct) else {
        printk::write_str("mm_struct named slub cache missing\n");
        return SmokeResult::Failed;
    };
    if !registry.has_named_cache(NamedSlubCacheKind::PageTableLock)
        || !registry.has_named_cache(NamedSlubCacheKind::VmapArea)
        || !registry.has_named_cache(NamedSlubCacheKind::RadixTreeNode)
        || !registry.has_named_cache(NamedSlubCacheKind::MapleNode)
        || !registry.has_named_cache(NamedSlubCacheKind::PoolWorkqueue)
    {
        printk::write_str("companion named slub caches missing\n");
        return SmokeResult::Failed;
    }
    if named_cache.kind() != NamedSlubCacheKind::MmStruct
        || named_cache.object_size() != mm_cache.object_size()
        || named_cache.usercopy_offset() != mm_cache.usercopy_offset()
        || named_cache.usercopy_size() != mm_cache.usercopy_size()
        || named_cache.usercopy_size() == 0
    {
        printk::write_str("mm_struct named slub cache binding invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "mm_struct_cache object_size={} usercopy={}..{} named_caches={} vma_deferred={}\n",
        mm_cache.object_size(),
        mm_cache.usercopy_offset(),
        mm_cache.usercopy_offset() + mm_cache.usercopy_size(),
        registry.named_cache_count(),
        mm_cache.vma_caches_deferred()
    ));
    SmokeResult::Passed
}
