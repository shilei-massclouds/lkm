use crate::{
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventResult, LifecycleEvent, State, failed_condition},
        static_branch::StaticKey,
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static MM_CORE_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::MmCoreInitPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex mm core init event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.memory_topology
        .setup(&ctx.zones, &ctx.cpu_group, &ctx.config)?;
    ctx.page_allocator.preset(
        &ctx.memory_topology,
        &ctx.cpu_hotplug_state,
        &ctx.per_cpu_storage,
    )?;
    ctx.memory_debug_hardening
        .setup(&mut ctx.static_branch, &ctx.early_param, &ctx.config)?;
    ctx.stack_depot
        .setup(&ctx.memblock, &ctx.memory_debug_hardening)?;
    ctx.swiotlb
        .setup(&ctx.dma_cache_policy, &ctx.memblock, &ctx.zones)?;
    ctx.page_allocator.setup(
        &mut ctx.memblock,
        &ctx.zones,
        &ctx.config,
        &ctx.memory_debug_hardening,
        &ctx.swiotlb,
    )?;
    ctx.slub_allocator
        .preset(&ctx.page_allocator, &ctx.per_cpu_storage)?;
    ctx.slub_allocator.setup(
        &ctx.page_allocator,
        &ctx.stack_depot,
        &ctx.per_cpu_storage,
        &ctx.cpu_hotplug_state,
    )?;
    ctx.page_table_caches.setup(&ctx.slub_allocator, &ctx.vm)?;
    ctx.vmalloc_allocator.setup(
        &ctx.slub_allocator,
        &ctx.page_table_caches,
        &ctx.per_cpu_storage,
    )?;
    ctx.mm_struct_cache
        .setup(&ctx.slub_allocator, &ctx.cpu_group)
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !mm_core_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&MM_CORE_INIT_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &MM_CORE_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::MmCoreInitPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&MM_CORE_INIT_PHASE_STATE) == State::Ready
}

fn mm_core_init_phase_ready(ctx: &Context) -> bool {
    crate::phases::boot::core_prepare::is_ready()
        && ctx.exception_stream.state() == State::Ready
        && ctx.memblock.state() == State::Offline
        && ctx.memory_topology.state() == State::Ready
        && ctx.memory_topology.node_count() == 1
        && ctx.memory_topology.boot_memory_node().state() == State::Ready
        && ctx.memory_topology.boot_memory_node().node_id() == 0
        && ctx.memory_topology.boot_memory_node().present_pages() != 0
        && ctx.memory_topology.boot_zone_set().state() == State::Ready
        && ctx.page_allocator.state() == State::Ready
        && ctx.page_allocator.boot_zonelist_set().state() == State::Ready
        && ctx.memory_debug_hardening.state() == State::Ready
        && !ctx.memory_debug_hardening.init_on_alloc()
        && !ctx.memory_debug_hardening.init_on_free()
        && !ctx.memory_debug_hardening.debug_pagealloc()
        && !ctx.memory_debug_hardening.debug_guardpage()
        && ctx.memory_debug_hardening.check_pages()
        && ctx.static_branch.key_count() >= 5
        && ctx.static_branch.enabled(StaticKey::CheckPages) == Some(true)
        && ctx.static_branch.enabled(StaticKey::InitOnAlloc) == Some(false)
        && ctx.stack_depot.state() == State::Ready
        && ctx.stack_depot.early_storage_ready()
        && ctx.swiotlb.state() == State::Ready
        && ctx.swiotlb.early_pool_ready()
        && ctx.swiotlb.pool_required()
            == (ctx.dma_cache_policy.noncoherent_supported()
                && ctx.dma_cache_policy.cache_alignment() > 1
                && ctx.page_allocator.totalram_pages() != 0)
        && ctx.slub_allocator.state() == State::Ready
        && ctx.slub_allocator.cache_registry().state() == State::Ready
        && ctx.slub_allocator.kmalloc_caches().state() == State::Ready
        && ctx.page_table_caches.state() == State::Ready
        && ctx.page_table_caches.vmalloc_pgtable_preallocated()
        && ctx.page_table_caches.lock_cache().state() == State::Ready
        && ctx.page_table_caches.lock_cache().page_ptl_cache_created()
        && ctx.vmalloc_allocator.state() == State::Ready
        && ctx.vmalloc_allocator.area_cache().state() == State::Ready
        && ctx.vmalloc_allocator.address_space().state() == State::Ready
        && ctx.vmalloc_allocator.node_set().state() == State::Ready
        && ctx.vmalloc_allocator.block_queues().state() == State::Ready
        && ctx.vmalloc_allocator.deferred_set().state() == State::Ready
        && ctx.mm_struct_cache.state() == State::Ready
        && ctx.mm_struct_cache.object_size() != 0
        && ctx.mm_struct_cache.saved_auxv_usercopy_ready()
        && ctx.mm_struct_cache.vma_caches_deferred()
        && printk::is_ready()
        && earlycon::is_online()
}
