use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        earlycon,
        mm_core::NamedSlubCacheKind,
        printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
        static_branch::StaticKey,
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static MM_CORE_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::checkpoint::checkpoint(Checkpoint::MmCoreInitPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared_with_check(ctx)),
        "arceos_ex mm core init preset failed\n",
    );
    setup(ctx)
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.memory_topology
        .setup(&ctx.zones, &ctx.cpu_group, &ctx.config)?;
    ctx.page_allocator.preset(
        &ctx.memory_topology,
        &ctx.page_metadata_map,
        &ctx.cpu_hotplug_state,
        &ctx.per_cpu_storage,
        &mut ctx.boot_cpu_local_interrupt,
    )?;
    ctx.memory_debug_hardening
        .setup(&mut ctx.static_branch, &ctx.early_param, &ctx.config)?;
    ctx.stack_depot
        .setup(&ctx.config, &ctx.memblock, &ctx.memory_debug_hardening)?;
    ctx.swiotlb
        .setup(&ctx.dma_cache_policy, &ctx.memblock, &ctx.zones)?;
    ctx.page_allocator.setup(
        &mut ctx.memblock,
        &ctx.zones,
        &ctx.page_metadata_map,
        &ctx.config,
        &ctx.memory_debug_hardening,
        &ctx.swiotlb,
    )?;
    crate::checkpoint::dispatch(Checkpoint::PageAllocatorMemBlockHandoffReady, ctx);
    ctx.slub_subsystem
        .preset(&ctx.page_allocator, &ctx.per_cpu_storage)?;
    ctx.slub_subsystem.setup(
        &ctx.page_allocator,
        &ctx.stack_depot,
        &ctx.per_cpu_storage,
        &ctx.cpu_hotplug_state,
    )?;
    crate::checkpoint::dispatch(Checkpoint::SlubSubsystemReady, ctx);
    ctx.kernel_global_allocator.setup(&ctx.slub_subsystem)?;
    ctx.dynamic_container_runtime
        .setup(&ctx.kernel_global_allocator)?;
    ctx.page_table_caches.setup(
        &mut ctx.slub_subsystem,
        &ctx.page_allocator,
        &ctx.page_metadata_map,
        &ctx.config,
        &ctx.vm,
        &ctx.static_objects,
        &ctx.kernel_image,
    )?;
    ctx.vmalloc_allocator.setup(
        &mut ctx.slub_subsystem,
        &ctx.page_table_caches,
        &ctx.per_cpu_storage,
    )?;
    ctx.ioremap.setup(
        &ctx.vm,
        &ctx.vmalloc_allocator,
        &ctx.page_table_caches,
        &ctx.fix_map,
        &ctx.config,
    )?;
    ctx.mm_struct_cache
        .setup(&mut ctx.slub_subsystem, &ctx.cpu_group)?;
    ctx.mm_core_trimmed_paths.setup(
        &ctx.config,
        &ctx.memory_debug_hardening,
        &ctx.stack_depot,
        &ctx.ioremap,
        &ctx.mm_struct_cache,
    )
}

fn adopt_prepared_with_check(ctx: &Context) -> EventResult {
    crate::phases::state::mark(
        &MM_CORE_INIT_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::MmCoreInitPhaseReady,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        adopt_ready(ctx),
        "arceos_ex mm core init setup failed\n",
    );
    enable(ctx)
}

fn adopt_ready(ctx: &Context) -> EventResult {
    if !mm_core_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&MM_CORE_INIT_PHASE_STATE),
            State::Prepared,
            State::Ready,
        );
    }

    crate::phases::state::adopt(
        &MM_CORE_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
    )
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        enable_event(ctx),
        "arceos_ex mm core init enable failed\n",
    );
    crate::phases::boot::sched_init::preset(crate::context::context())
}

fn enable_event(ctx: &mut Context) -> EventResult {
    crate::phases::state::mark(
        &MM_CORE_INIT_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::MmCoreInitPhaseOnline,
    )
}

pub fn is_ready() -> bool {
    let s = crate::phases::state::load(&MM_CORE_INIT_PHASE_STATE);
    s == State::Ready || s == State::Online
}

fn mm_core_init_phase_ready(ctx: &Context) -> bool {
    crate::phases::boot::core_prepare::is_ready()
        && ctx.exception_stream.state() == State::Ready
        && ctx.memblock.state() == State::Offline
        && ctx.memory_topology.state() == State::Ready
        && ctx.memory_topology.node_count() == 1
        && ctx.memory_topology.memory_node().state() == State::Ready
        && ctx.memory_topology.memory_node().node_id() == 0
        && ctx.memory_topology.memory_node().present_pages() != 0
        && ctx.memory_topology.zone_set().state() == State::Ready
        && ctx.page_metadata_map.state() == State::Ready
        && ctx.page_metadata_map.metadata_count() != 0
        && ctx.page_metadata_map.metadata_bytes() != 0
        && ctx.page_metadata_map.metadata_storage_size() >= ctx.page_metadata_map.metadata_bytes()
        && ctx.page_allocator.state() == State::Ready
        && ctx.page_allocator.zonelist_set().state() == State::Ready
        && ctx.page_allocator.page_metadata_map_bound()
        && ctx.page_allocator.zonelist_update_seq_irqsave_guard_ready()
        && ctx.page_allocator.zonelist_update_seq().entered_count() == 1
        && ctx.page_allocator.zonelist_update_seq().exited_count() == 1
        && ctx
            .page_allocator
            .zonelist_update_seq()
            .irqsave_entered_count()
            == 1
        && ctx
            .page_allocator
            .zonelist_update_seq()
            .irqrestore_exited_count()
            == 1
        && ctx.page_allocator.zonelist_printk_deferred_section_ready()
        && ctx
            .page_allocator
            .zonelist_printk_deferred_section()
            .entered_count()
            == 1
        && ctx
            .page_allocator
            .zonelist_printk_deferred_section()
            .exited_count()
            == 1
        && !ctx.page_allocator.zonelist_update_seq().writer_active()
        && !ctx
            .page_allocator
            .zonelist_printk_deferred_section()
            .active()
        && ctx.page_allocator.cpuhp_step_registered()
        && ctx.page_allocator.boot_pageset_checkpoint_ready()
        && ctx
            .page_allocator
            .boot_pagesets_initialized_for_possible_cpus()
        && ctx.page_allocator.boot_pageset_possible_cpu_count()
            == ctx.per_cpu_storage.first_chunk().unit_count()
        && ctx.page_allocator.buddy_free_page_sets_ready()
        && ctx.page_allocator.buddy_total_free_pages() != 0
        && ctx.page_allocator.buddy_free_block_count() != 0
        && ctx.memory_debug_hardening.state() == State::Ready
        && ctx.memory_debug_hardening.early_params_scanned()
        && ctx.memory_debug_hardening.early_param_policy_trimmed()
        && ctx.memory_debug_hardening.default_policy_selected()
        && ctx
            .memory_debug_hardening
            .static_keys_resolved_from_default_policy()
        && !ctx.memory_debug_hardening.init_on_alloc()
        && !ctx.memory_debug_hardening.init_on_free()
        && !ctx.memory_debug_hardening.debug_pagealloc()
        && !ctx.memory_debug_hardening.debug_guardpage()
        && ctx.memory_debug_hardening.check_pages()
        && ctx.static_branch.key_count() >= 5
        && ctx.static_branch.enabled(StaticKey::CheckPages) == Some(true)
        && ctx.static_branch.enabled(StaticKey::InitOnAlloc) == Some(false)
        && ctx.static_branch.enabled(StaticKey::InitOnFree) == Some(false)
        && ctx.static_branch.enabled(StaticKey::DebugPageAlloc) == Some(false)
        && ctx.static_branch.enabled(StaticKey::DebugGuardPage) == Some(false)
        && ctx.stack_depot.state() == State::Ready
        && ctx.stack_depot.config_enabled()
        && ctx.stack_depot.early_init_passed()
        && !ctx.stack_depot.early_init_requested()
        && ctx.stack_depot.early_table_allocation_not_required()
        && !ctx.stack_depot.early_table_allocated()
        && ctx.stack_depot.late_init_deferred()
        && ctx.swiotlb.state() == State::Ready
        && ctx.swiotlb.early_pool_ready()
        && ctx.swiotlb.pool_required()
            == (ctx.dma_cache_policy.noncoherent_supported()
                && ctx.dma_cache_policy.cache_alignment() > 1
                && ctx.page_allocator.totalram_pages() != 0)
        && (!ctx.swiotlb.pool_required()
            || (ctx.swiotlb.static_pool_area_locks_ready()
                && ctx.swiotlb.static_pool_area_count() != 0
                && ctx.swiotlb.static_pool_lock_count() == ctx.swiotlb.static_pool_area_count()))
        && ctx.swiotlb.dynamic_growth_trimmed()
        && ctx.slub_subsystem.state() == State::Ready
        && ctx.slub_subsystem.cache_registry().state() == State::Ready
        && ctx
            .slub_subsystem
            .cache_registry()
            .has_named_cache(NamedSlubCacheKind::PageTableLock)
        && ctx
            .slub_subsystem
            .cache_registry()
            .has_named_cache(NamedSlubCacheKind::VmapArea)
        && ctx
            .slub_subsystem
            .cache_registry()
            .has_named_cache(NamedSlubCacheKind::MmStruct)
        && ctx.slub_subsystem.kmalloc_caches().state() == State::Ready
        && ctx.kernel_global_allocator.state() == State::Ready
        && ctx.kernel_global_allocator.uses_slub_subsystem()
        && ctx.kernel_global_allocator.alloc_api_ready()
        && ctx.kernel_global_allocator.alloc_zeroed_api_ready()
        && ctx.kernel_global_allocator.dealloc_api_ready()
        && ctx.dynamic_container_runtime.state() == State::Ready
        && ctx.dynamic_container_runtime.uses_global_allocator()
        && ctx.dynamic_container_runtime.vec_api_ready()
        && ctx.dynamic_container_runtime.list_api_ready()
        && ctx.dynamic_container_runtime.set_api_ready()
        && ctx.page_table_caches.state() == State::Ready
        && ctx.page_table_caches.vmalloc_pgtable_preallocated()
        && ctx
            .page_table_caches
            .vmalloc_pgtable_dynamic_allocator_ready()
        && ctx
            .page_table_caches
            .vmalloc_pgtable_dynamic_metadata_ready()
        && ctx.page_table_caches.lock_cache().state() == State::Ready
        && ctx.page_table_caches.lock_cache().page_ptl_cache_created()
        && ctx
            .page_table_caches
            .lock_cache()
            .registered_in_slub_registry()
        && ctx.page_table_caches.lock_cache().object_size() != 0
        && ctx.vmalloc_allocator.state() == State::Ready
        && ctx.vmalloc_allocator.area_cache().state() == State::Ready
        && ctx
            .vmalloc_allocator
            .area_cache()
            .registered_in_slub_registry()
        && ctx.vmalloc_allocator.area_cache().object_size() != 0
        && ctx.vmalloc_allocator.address_space().state() == State::Ready
        && ctx.vmalloc_allocator.node_set().state() == State::Ready
        && ctx.vmalloc_allocator.block_queues().state() == State::Ready
        && ctx.vmalloc_allocator.deferred_set().state() == State::Ready
        && ctx.vmalloc_allocator.vmap_area_api_ready()
        && ctx.vmalloc_allocator.page_range_mapping_api_ready()
        && ctx.vmalloc_allocator.vm_struct_metadata_ready()
        && ctx.vmalloc_allocator.vmap_area_metadata_ready()
        && ctx.vmalloc_allocator.mapping_policy_external()
        && ctx.vmalloc_allocator.physical_resource_policy_external()
        && ctx.vmalloc_allocator.runtime_page_table_mapping_ready()
        && ctx.vmalloc_allocator.runtime_mapping_window_ready()
        && ctx.vmalloc_allocator.multi_window_mapping_supported()
        && ctx.vmalloc_allocator.preallocated_mapping_window_bound()
        && ctx
            .vmalloc_allocator
            .dynamic_l0_window_allocation_supported()
        && ctx.vmalloc_allocator.duplicate_area_mapping_rejected()
        && ctx.vmalloc_allocator.dynamic_record_storage_ready()
        && ctx.vmalloc_allocator.mapping_guard_contract_ready()
        && ctx.vmalloc_allocator.unmapping_guard_contract_ready()
        && ctx.vmalloc_allocator.mapping_sync_contract_ready()
        && ctx.vmalloc_allocator.unmapping_flush_contract_ready()
        && ctx.vmalloc_allocator.failure_rollback_contract_ready()
        && ctx.vmalloc_allocator.cross_cpu_vmalloc_flush_deferred()
        && ctx.vmalloc_allocator.node_set().guard_contract_ready()
        && ctx.vmalloc_allocator.deferred_set().guard_contract_ready()
        && ctx
            .vmalloc_allocator
            .deferred_set()
            .rcu_runtime_path_deferred()
        && ctx.ioremap.state() == State::Ready
        && ctx.ioremap.runtime_ready()
        && ctx.ioremap.uses_vmalloc_area_management()
        && ctx.ioremap.uses_vmalloc_mapping_execution()
        && ctx.ioremap.uses_vmap_address_space()
        && ctx.ioremap.distinct_from_vmalloc_allocation()
        && ctx.ioremap.does_not_use_fixmap()
        && ctx.ioremap.physical_resource_policy_ready()
        && ctx.ioremap.vm_ioremap_flags_ready()
        && ctx.ioremap.io_page_protection_ready()
        && ctx.ioremap.mapping_guard_contract_ready()
        && ctx.ioremap.unmapping_guard_contract_ready()
        && ctx.ioremap.mapping_sync_contract_ready()
        && ctx.ioremap.unmapping_flush_contract_ready()
        && ctx.ioremap.failure_rollback_contract_ready()
        && ctx.mm_struct_cache.state() == State::Ready
        && ctx.mm_struct_cache.registered_in_slub_registry()
        && ctx.mm_struct_cache.object_size() != 0
        && ctx
            .slub_subsystem
            .cache_registry()
            .named_cache(NamedSlubCacheKind::MmStruct)
            .map(|cache| {
                cache.object_size() == ctx.mm_struct_cache.object_size()
                    && cache.usercopy_offset() == ctx.mm_struct_cache.usercopy_offset()
                    && cache.usercopy_size() == ctx.mm_struct_cache.usercopy_size()
            })
            == Some(true)
        && ctx
            .slub_subsystem
            .cache_registry()
            .named_cache(NamedSlubCacheKind::PageTableLock)
            .map(|cache| cache.object_size() == ctx.page_table_caches.lock_cache().object_size())
            == Some(true)
        && ctx
            .slub_subsystem
            .cache_registry()
            .named_cache(NamedSlubCacheKind::VmapArea)
            .map(|cache| cache.object_size() == ctx.vmalloc_allocator.area_cache().object_size())
            == Some(true)
        && ctx.mm_struct_cache.saved_auxv_usercopy_ready()
        && ctx.mm_struct_cache.vma_caches_deferred()
        && ctx.mm_core_trimmed_paths.state() == State::Ready
        && ctx.mm_core_trimmed_paths.page_ext_flatmem_trimmed()
        && ctx.mm_core_trimmed_paths.kfence_pool_trimmed()
        && ctx.mm_core_trimmed_paths.kmsan_shadow_trimmed()
        && ctx.mm_core_trimmed_paths.page_ext_flatmem_late_trimmed()
        && ctx.mm_core_trimmed_paths.kmemleak_init_trimmed()
        && ctx.mm_core_trimmed_paths.debug_objects_mem_trimmed()
        && ctx.mm_core_trimmed_paths.page_ext_final_trimmed()
        && ctx.mm_core_trimmed_paths.x86_espfix_not_applicable()
        && ctx.mm_core_trimmed_paths.x86_pti_not_applicable()
        && ctx.mm_core_trimmed_paths.kmsan_runtime_trimmed()
        && ctx.mm_core_trimmed_paths.execmem_init_trimmed_noop()
        && ctx
            .mm_core_trimmed_paths
            .execmem_trimmed_because_config_execmem_disabled()
        && ctx.mm_core_trimmed_paths.position_preserved()
        && printk::is_ready()
        && (earlycon::is_online() || printk::console_handoff_complete())
}
