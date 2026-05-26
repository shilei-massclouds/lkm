use crate::{
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventOutcome, EventResult, LifecycleEvent, State},
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
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventOutcome {
    ctx.init_stack.enable().into_result()?;

    let boot_hartid = ctx.cpu_group.boot_hartid();
    ctx.early_dtb
        .preset(
            &ctx.raw_dtb,
            &ctx.fix_map,
            boot_hartid,
            &mut ctx.platform_cpu_info,
            &mut ctx.physical_memory,
        )
        .into_result()?;
    ctx.cpu_id_map.preset(boot_hartid).into_result()?;
    ctx.interrupt_stream.setup().into_result()?;
    ctx.cpu_group
        .boot_cpu_setup(ctx.platform_cpu_info.contains(boot_hartid))
        .into_result()?;
    ctx.cpu_group.boot_cpu_enable().into_result()?;
    printk::preset().into_result()?;
    printk::write_str("arceos_ex object kernel\n");

    ctx.early_dtb
        .setup(
            &ctx.raw_dtb,
            &mut ctx.memblock,
            &mut ctx.kernel_cmdline,
            &ctx.physical_memory,
        )
        .into_result()?;
    ctx.init_mm.setup(&ctx.lds).into_result()?;
    ctx.early_ioremap.setup(&ctx.fix_map).into_result()?;
    ctx.sbi.setup().into_result()?;
    ctx.kernel_param
        .setup(&ctx.kernel_cmdline, &ctx.sbi)
        .into_result()?;
    ctx.memblock
        .setup(
            &ctx.early_dtb,
            &ctx.kernel_image,
            &ctx.raw_dtb,
            &ctx.config,
            &ctx.lds,
            &ctx.physical_memory,
        )
        .into_result()?;
    ctx.vm
        .enable(
            &ctx.config,
            &mut ctx.static_objects,
            &ctx.lds,
            &ctx.kernel_image,
            &ctx.memblock,
        )
        .into_result()?;
    ctx.memblock.enable(ctx.vm.state()).into_result()?;
    ctx.early_dtb
        .cleanup(&ctx.memblock, &ctx.kernel_param)
        .into_result()
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
}

fn checkpoint_ready(ctx: &Context) -> EventOutcome {
    if !entry_successor_phase_ready(ctx) {
        return EventResult::failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE),
            State::Base,
            State::Ready,
        )
        .into_result();
    }

    crate::phases::state::mark(
        &ENTRY_SUCCESSOR_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::EntrySuccessorPhaseReady,
    )
    .into_result()
}

fn entry_successor_phase_ready(ctx: &Context) -> bool {
    ctx.init_stack.state() == State::Online
        && ctx.cpu_group.boot_cpu_state() == State::Online
        && ctx.interrupt_stream.state() == State::Ready
        && ctx.vm.state() == State::Online
        && ctx.vm.entry_successor_ready()
        && ctx.cpu_id_map.state() == State::Ready
        && printk::is_prepared()
        && ctx.early_dtb.state() == State::Destroyed
        && ctx.kernel_cmdline.state() == State::Ready
        && ctx.init_mm.state() == State::Ready
        && ctx.early_ioremap.state() == State::Ready
        && ctx.sbi.state() == State::Ready
        && ctx.kernel_param.state() == State::Ready
        && earlycon::is_online()
        && ctx.memblock.state() == State::Online
}
