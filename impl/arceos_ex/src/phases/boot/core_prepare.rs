use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        earlycon, printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static CORE_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::checkpoint::checkpoint(Checkpoint::CorePreparePhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex core prepare event failed\n",
    );
    handoff(ctx)
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.device_tree
        .setup(&ctx.raw_dtb, &ctx.vm, &mut ctx.memblock, &ctx.config)?;
    ctx.zones.setup(&ctx.memblock, &ctx.vm)?;
    ctx.page_metadata_map
        .setup(&mut ctx.memblock, &ctx.zones, &ctx.vm, &ctx.config)?;
    ctx.resource_lock.preset_static()?;
    ctx.resource_lock.setup()?;
    ctx.resource_tree.setup(
        &ctx.memblock,
        &ctx.kernel_image,
        &ctx.lds,
        &mut ctx.resource_lock,
    )?;
    ctx.cpu_group
        .setup_smp(&ctx.device_tree, &ctx.sbi, &mut ctx.secondary_cpus)?;
    ctx.cache_block_info
        .setup(&ctx.device_tree, &ctx.cpu_group)?;
    ctx.cpu_capabilities
        .setup(&ctx.device_tree, &ctx.cpu_group, &ctx.cache_block_info)?;
    ctx.dma_cache_policy
        .setup(&ctx.cpu_capabilities, &ctx.cache_block_info)?;
    ctx.per_cpu_storage.preset(&ctx.lds, &ctx.static_objects)?;
    ctx.cpu_hotplug_lock
        .preset_static_with_per_cpu_storage(&ctx.per_cpu_storage)?;
    ctx.cpu_hotplug_lock.setup()?;
    ctx.jump_label_mutex.preset_static()?;
    ctx.jump_label_mutex.setup()?;
    ctx.static_branch.setup(
        &ctx.kernel_image,
        &ctx.vm,
        &mut ctx.cpu_hotplug_lock,
        &mut ctx.jump_label_mutex,
        &ctx.init_task,
    )?;
    ctx.command_line.setup(
        &ctx.kernel_cmdline,
        &mut ctx.saved_command_line,
        &mut ctx.static_command_line,
        &ctx.memblock,
    )?;
    checkpoint_setup_nr_cpu_ids(&ctx.cpu_group)?;
    ctx.per_cpu_storage
        .setup(&mut ctx.memblock, &ctx.vm, &ctx.config, &ctx.cpu_group)?;
    ctx.cpu_hotplug_state
        .setup(&ctx.cpu_group, &ctx.per_cpu_storage)?;
    checkpoint_second_parse_early_param(&ctx.early_param)?;
    ctx.params.setup(
        &ctx.early_param,
        &ctx.static_command_line,
        &mut ctx.boot_param,
        &mut ctx.payload_param,
        checkpoint_print_unknown_bootoptions,
    )?;
    ctx.randomness.preset(&ctx.static_command_line)?;
    printk::setup(
        &ctx.memblock,
        &ctx.per_cpu_storage,
        &ctx.boot_param,
        &mut ctx.boot_cpu_local_interrupt,
    )?;
    crate::checkpoint::dispatch(Checkpoint::PrintkBufferReady, ctx);
    ctx.exception_table.setup(&ctx.kernel_image, &ctx.vm)?;
    ctx.exception_stream.setup(&ctx.event_stream)
}

fn checkpoint_setup_nr_cpu_ids(cpu_group: &crate::objects::cpu_group::CpuGroup) -> EventResult {
    let boot_cpu = cpu_group.boot_cpu();
    if cpu_group.state() != State::Ready
        || cpu_group.boot_cpu_state() != State::Online
        || !cpu_group.possible_cpu_boundary_ready()
        || boot_cpu.map(|cpu| cpu.cpu_ref()) != cpu_group.boot_cpu_ref()
        || !boot_cpu
            .map(|cpu| cpu.cpu_ref().is_boot_cpu() && cpu.logical_id() == 0)
            .unwrap_or(false)
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Base,
            State::Ready,
        );
    }
    crate::checkpoint::checkpoint(Checkpoint::SetupNrCpuIdsCheckpoint);
    Ok(())
}

fn checkpoint_second_parse_early_param(
    early_param: &crate::objects::early_param::EarlyParam,
) -> EventResult {
    if early_param.state() != State::Ready {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Base,
            State::Ready,
        );
    }
    crate::checkpoint::checkpoint(Checkpoint::SecondParseEarlyParamCheckpoint);
    Ok(())
}

fn checkpoint_print_unknown_bootoptions(
    boot_param: &crate::objects::boot_param::BootParam,
) -> EventResult {
    if boot_param.state() != State::Ready {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Base,
            State::Ready,
        );
    }
    crate::checkpoint::checkpoint(Checkpoint::PrintUnknownBootoptionsCheckpoint);
    Ok(())
}

fn handoff(ctx: &mut Context) -> ! {
    crate::phases::boot::mm_core_init::setup(ctx)
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !core_prepare_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&CORE_PREPARE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &CORE_PREPARE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::CorePreparePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&CORE_PREPARE_PHASE_STATE) == State::Ready
}

fn core_prepare_phase_ready(ctx: &Context) -> bool {
    ctx.device_tree.state() == State::Ready
        && ctx.zones.state() == State::Ready
        && ctx.page_metadata_map.state() == State::Ready
        && ctx.page_metadata_map.metadata_count() != 0
        && ctx.page_metadata_map.metadata_bytes() != 0
        && ctx.page_metadata_map.metadata_storage_size() >= ctx.page_metadata_map.metadata_bytes()
        && ctx.resource_lock.state() == State::Ready
        && ctx.resource_lock.ready()
        && ctx.resource_lock.boot_init_task_write_guard_completed()
        && ctx.resource_tree.state() == State::Ready
        && ctx.resource_tree.write_lock_guard_used()
        && ctx
            .resource_tree
            .resource_lock_write_guard_used_by(&ctx.resource_lock)
        && ctx.cpu_group.state() == State::Ready
        && !ctx.cpu_group.smp_concurrency_open()
        && ctx.cache_block_info.state() == State::Ready
        && ctx.cpu_capabilities.state() == State::Ready
        && ctx.dma_cache_policy.state() == State::Ready
        && ctx.cpu_hotplug_lock.state() == State::Ready
        && ctx.cpu_hotplug_lock.ready()
        && ctx.cpu_hotplug_lock.boot_init_task_read_guard_completed()
        && ctx.jump_label_mutex.state() == State::Ready
        && ctx.jump_label_mutex.ready()
        && ctx.jump_label_mutex.boot_init_task_guard_completed()
        && ctx.static_branch.state() == State::Ready
        && ctx.static_branch.cpu_hotplug_read_guard_used()
        && ctx
            .static_branch
            .cpu_hotplug_read_guard_used_by(&ctx.cpu_hotplug_lock)
        && ctx
            .static_branch
            .jump_label_mutex_guard_used(&ctx.jump_label_mutex)
        && ctx.static_branch.text_patch_sync_deferred()
        && ctx.command_line.state() == State::Ready
        && ctx.saved_command_line.state() == State::Ready
        && ctx.static_command_line.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && ctx.per_cpu_storage.static_image().state() == State::Ready
        && ctx.per_cpu_storage.first_chunk().state() == State::Ready
        && ctx.per_cpu_storage.offset_table().state() == State::Ready
        && ctx.cpu_hotplug_state.state() == State::Ready
        && ctx.cpu_hotplug_state.sync() == crate::objects::cpu_hotplug::CpuHotplugSyncState::Online
        && ctx.params.state() == State::Ready
        && ctx.boot_param.state() == State::Ready
        && ctx.payload_param.state() == State::Ready
        && ctx.randomness.state() == State::Prepared
        && ctx.randomness.early_mix_without_input_pool_lock()
        && ctx.randomness.early_conditional_reseed_deferred()
        && ctx.randomness.base_crng_lock_deferred()
        && printk::is_ready()
        && printk::setup_local_irq_save_restore_used()
        && printk::setup_local_irq_guard_used_by(&ctx.boot_cpu_local_interrupt)
        && ctx.exception_table.state() == State::Ready
        && ctx.exception_stream.state() == State::Ready
        && ctx.exception_stream.page_fault_state() == State::Ready
        && ctx.exception_stream.syscall_state() == State::Prepared
        && ctx.exception_stream.breakpoint_state() == State::Ready
        && ctx.exception_stream.unexpected_state() == State::Ready
        && ctx.interrupt_stream.early_boot_irqs_disabled()
        && (earlycon::is_online() || printk::console_handoff_complete())
}
