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
static ENTRY_SUCCESSOR_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::EntrySuccessorPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex entry successor event failed\n",
    );
    handoff(ctx)
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.init_stack.enable()?;

    let boot_hartid = ctx.cpu_group.boot_hartid();
    ctx.early_dtb.preset(
        &ctx.raw_dtb,
        &ctx.fix_map,
        boot_hartid,
        &mut ctx.platform_cpu_info,
        &mut ctx.physical_memory,
    )?;
    ctx.cpu_id_map.preset(boot_hartid)?;
    ctx.interrupt_stream.setup()?;
    ctx.cpu_group
        .boot_cpu_setup(ctx.platform_cpu_info.contains(boot_hartid))?;
    ctx.cpu_group.boot_cpu_enable()?;
    printk::preset()?;
    printk::write_str("arceos_ex object kernel\n");

    ctx.early_dtb.setup(
        &ctx.raw_dtb,
        &mut ctx.memblock,
        &mut ctx.command_line,
        &mut ctx.kernel_cmdline,
        &ctx.physical_memory,
    )?;
    ctx.init_mm.setup(&ctx.lds)?;
    ctx.early_ioremap.setup(&ctx.fix_map)?;
    ctx.sbi.setup()?;
    ctx.params.preset(
        &ctx.command_line,
        &ctx.kernel_cmdline,
        &ctx.sbi,
        &mut ctx.early_param,
    )?;
    ctx.memblock.setup(
        &ctx.early_dtb,
        &ctx.kernel_image,
        &ctx.raw_dtb,
        &ctx.config,
        &ctx.lds,
        &ctx.physical_memory,
    )?;
    ctx.vm.enable(
        &ctx.config,
        &mut ctx.static_objects,
        &ctx.lds,
        &ctx.kernel_image,
        &ctx.memblock,
    )?;
    ctx.memblock.enable(ctx.vm.state())?;
    ctx.early_dtb.cleanup(&ctx.memblock, &ctx.early_param)
}

fn handoff(ctx: &mut Context) -> ! {
    crate::phases::boot::core_prepare::setup(ctx)
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !entry_successor_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &ENTRY_SUCCESSOR_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::EntrySuccessorPhaseReady,
    )
}

fn entry_successor_phase_ready(ctx: &Context) -> bool {
    ctx.init_stack.state() == State::Online
        && ctx.cpu_group.boot_cpu_state() == State::Online
        && ctx.interrupt_stream.state() == State::Ready
        && ctx.vm.state() == State::Online
        && ctx.vm.entry_successor_ready()
        && ctx.cpu_id_map.state() == State::Prepared
        && printk::is_prepared()
        && ctx.early_dtb.state() == State::Destroyed
        && ctx.command_line.state() == State::Prepared
        && ctx.kernel_cmdline.state() == State::Ready
        && ctx.init_mm.state() == State::Ready
        && ctx.early_ioremap.state() == State::Ready
        && ctx.sbi.state() == State::Ready
        && ctx.params.state() == State::Prepared
        && ctx.early_param.state() == State::Ready
        && earlycon::is_online()
        && ctx.memblock.state() == State::Online
}
