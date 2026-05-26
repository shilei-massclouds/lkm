use crate::{
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static ENTRY_SUCCESSOR_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::EntrySuccessorPhaseStarted);
    require(setup_objects(ctx));
    require(checkpoint_ready(ctx));
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    let result = ctx.init_stack.enable();
    if !result.is_success() {
        return result;
    }

    let boot_hartid = ctx.cpu_group.boot_hartid();
    let result = ctx.early_dtb.preset(
        &ctx.raw_dtb,
        &ctx.fix_map,
        boot_hartid,
        &mut ctx.platform_cpu_info,
        &mut ctx.physical_memory,
    );
    if !result.is_success() {
        return result;
    }

    let result = ctx.cpu_id_map.preset(boot_hartid);
    if !result.is_success() {
        return result;
    }

    let result = ctx.interrupt_stream.setup();
    if !result.is_success() {
        return result;
    }

    let result = ctx
        .cpu_group
        .boot_cpu_setup(ctx.platform_cpu_info.contains(boot_hartid));
    if !result.is_success() {
        return result;
    }

    let result = ctx.cpu_group.boot_cpu_enable();
    if !result.is_success() {
        return result;
    }

    let result = printk::preset();
    if !result.is_success() {
        return result;
    }
    printk::write_str("arceos_ex object kernel\n");

    let result = ctx.early_dtb.setup(
        &ctx.raw_dtb,
        &mut ctx.memblock,
        &mut ctx.kernel_cmdline,
        &ctx.physical_memory,
    );
    if !result.is_success() {
        return result;
    }

    let result = ctx.init_mm.setup(&ctx.lds);
    if !result.is_success() {
        return result;
    }

    let result = ctx.early_ioremap.setup(&ctx.fix_map);
    if !result.is_success() {
        return result;
    }

    let result = ctx.sbi.setup();
    if !result.is_success() {
        return result;
    }

    let result = ctx.kernel_param.setup(&ctx.kernel_cmdline, &ctx.sbi);
    if !result.is_success() {
        return result;
    }

    let result = ctx.memblock.setup(
        &ctx.early_dtb,
        &ctx.kernel_image,
        &ctx.raw_dtb,
        &ctx.config,
        &ctx.lds,
        &ctx.physical_memory,
    );
    if !result.is_success() {
        return result;
    }

    let result = ctx.vm.enable(
        &ctx.config,
        &mut ctx.static_objects,
        &ctx.lds,
        &ctx.kernel_image,
        &ctx.memblock,
    );
    if !result.is_success() {
        return result;
    }

    let result = ctx.memblock.enable(ctx.vm.state());
    if !result.is_success() {
        return result;
    }

    let result = ctx.early_dtb.cleanup(&ctx.memblock, &ctx.kernel_param);
    if !result.is_success() {
        return result;
    }

    EventResult::Success
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !entry_successor_phase_ready(ctx) {
        return EventResult::failed_condition(
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

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex entry successor event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}
