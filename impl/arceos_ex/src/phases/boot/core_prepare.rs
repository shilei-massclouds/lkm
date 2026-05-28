use crate::{
    context::Context,
    objects::{
        earlycon, printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static CORE_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::CorePreparePhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex core prepare event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.device_tree
        .setup(&ctx.raw_dtb, &ctx.vm, &mut ctx.memblock, &ctx.config)?;
    ctx.zones.setup(&ctx.memblock, &ctx.vm)?;
    ctx.resource_tree
        .setup(&ctx.memblock, &ctx.kernel_image, &ctx.lds)?;
    ctx.cpu_group
        .setup_smp(&ctx.device_tree, &ctx.cpu_id_map, &ctx.sbi)?;
    ctx.cpu_id_map.setup(&ctx.cpu_group)?;
    ctx.cache_block_info
        .setup(&ctx.device_tree, &ctx.cpu_group)?;
    ctx.riscv_hwcap
        .setup(&ctx.device_tree, &ctx.cpu_group, &ctx.cache_block_info)?;
    ctx.saved_command_line
        .setup(&ctx.kernel_cmdline, &ctx.memblock)?;
    ctx.static_command_line
        .setup(&ctx.kernel_cmdline, &ctx.saved_command_line, &ctx.memblock)?;
    checkpoint_setup_nr_cpu_ids(&ctx.cpu_id_map, &ctx.cpu_group)?;
    ctx.per_cpu_storage
        .setup(&ctx.memblock, &ctx.vm, &ctx.cpu_group, &ctx.cpu_id_map)?;
    ctx.boot_cpu_hotplug_state
        .setup(&ctx.cpu_group, &ctx.per_cpu_storage)?;
    checkpoint_second_parse_early_param(&ctx.early_param)?;
    ctx.boot_param
        .setup(&ctx.early_param, &ctx.static_command_line)?;
    checkpoint_print_unknown_bootoptions(&ctx.boot_param)?;
    ctx.payload_param.setup(&ctx.boot_param)?;
    ctx.randomness.preset(&ctx.static_command_line)?;
    printk::setup()?;
    ctx.exception_table.setup(&ctx.kernel_image, &ctx.vm)?;
    ctx.exception_stream.setup(&ctx.event_stream)?;
    ctx.exception_stream.page_fault_setup()?;
    ctx.exception_stream.breakpoint_setup()?;
    ctx.exception_stream.unexpected_setup()
}

fn checkpoint_setup_nr_cpu_ids(
    cpu_id_map: &crate::objects::cpu_id_map::CpuIdMap,
    cpu_group: &crate::objects::cpu_group::CpuGroup,
) -> EventResult {
    let boot_entry = cpu_id_map.entry(0);
    if cpu_id_map.state() != State::Ready
        || cpu_group.state() != State::Ready
        || cpu_group.boot_cpu_state() != State::Online
        || cpu_id_map.count() == 0
        || boot_entry.map(|entry| entry.hartid()) != Some(cpu_group.boot_hartid())
        || boot_entry.map(|entry| entry.logical_id()) != Some(0)
        || boot_entry.map(|entry| entry.kind())
            != Some(crate::objects::cpu_id_map::CpuIdMapEntryKind::BootCpu)
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Base,
            State::Ready,
        );
    }
    crate::trace::checkpoint(Checkpoint::SetupNrCpuIdsCheckpoint);
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
    crate::trace::checkpoint(Checkpoint::SecondParseEarlyParamCheckpoint);
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
    crate::trace::checkpoint(Checkpoint::PrintUnknownBootoptionsCheckpoint);
    Ok(())
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
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
        && ctx.resource_tree.state() == State::Ready
        && ctx.cpu_group.state() == State::Ready
        && ctx.cpu_id_map.state() == State::Ready
        && ctx.cache_block_info.state() == State::Ready
        && ctx.riscv_hwcap.state() == State::Ready
        && ctx.saved_command_line.state() == State::Ready
        && ctx.static_command_line.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && ctx.boot_cpu_hotplug_state.state() == State::Ready
        && ctx.boot_param.state() == State::Ready
        && ctx.payload_param.state() == State::Ready
        && ctx.randomness.state() == State::Prepared
        && printk::is_ready()
        && ctx.exception_table.state() == State::Ready
        && ctx.exception_stream.state() == State::Ready
        && ctx.exception_stream.page_fault_state() == State::Ready
        && ctx.exception_stream.syscall_state() == State::Prepared
        && ctx.exception_stream.breakpoint_state() == State::Ready
        && ctx.exception_stream.unexpected_state() == State::Ready
        && earlycon::is_online()
}
