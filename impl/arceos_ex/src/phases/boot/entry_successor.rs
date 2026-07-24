use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static ENTRY_SUCCESSOR_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
#[unsafe(link_section = ".data.phase")]
static ENTRY_SUCCESSOR_DEFERRED_FACTS: AtomicU8 = AtomicU8::new(0);

const DEFERRED_VMLINUX_BUILD_ID: u8 = 1 << 0;
const DEFERRED_PAGE_ADDRESS_INIT: u8 = 1 << 1;
const DEFERRED_START_KERNEL_POSITION: u8 = 1 << 2;

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(ctx),
        "arceos_ex entry successor preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::EntrySuccessorPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared_with_check()),
        "arceos_ex entry successor preset failed\n",
    );
    setup(ctx)
}

fn preset_start(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE);
    if state != State::Base || !preset_dependencies_ready(ctx) {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_dependencies_ready(ctx: &Context) -> bool {
    crate::flows::boot_init_flow::is_prepared()
        && ctx.vm.state() == State::Ready
        && ctx.vm.entry_prelude_ready()
        && ctx.boot_task.state() == State::OnCpu
        && ctx.init_stack.state() == State::Ready
        && ctx.interrupt_stream.state() == State::Prepared
        && ctx.raw_dtb.state() == State::Ready
        && ctx.fix_map.state() == State::Ready
        && ctx.kernel_image.state() == State::Online
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    record_start_kernel_deferred_facts();
    ctx.init_stack.enable()?;

    let boot_hartid = ctx.boot_current_cpu.hartid();
    ctx.early_dtb.preset(
        &ctx.raw_dtb,
        &ctx.fix_map,
        boot_hartid,
        &mut ctx.platform_cpu_info,
        &mut ctx.physical_memory,
    )?;
    ctx.interrupt_stream
        .setup(&mut ctx.boot_cpu_local_interrupt)?;
    ctx.boot_current_cpu
        .setup_boot_cpu(ctx.platform_cpu_info.contains(boot_hartid))?;
    ctx.cpu_group.register_boot_cpu(&ctx.boot_current_cpu)?;
    ctx.boot_current_cpu.enable_boot_cpu()?;
    ctx.cpu_group.register_boot_cpu(&ctx.boot_current_cpu)?;
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
    crate::checkpoint::dispatch(Checkpoint::MemBlockOnline, ctx);
    ctx.early_dtb.cleanup(&ctx.memblock, &ctx.early_param)
}

fn adopt_prepared_with_check() -> EventResult {
    let state = crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE);
    if state != State::Base || !start_kernel_deferred_facts_ready() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    crate::phases::state::mark_checked(
        &ENTRY_SUCCESSOR_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::EntrySuccessorPhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(adopt_ready(ctx), "arceos_ex entry successor setup failed\n");
    enable(ctx)
}

fn adopt_ready(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE);
    if state != State::Prepared || !entry_successor_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Setup, state, State::Prepared, State::Ready);
    }

    crate::phases::state::mark_checked(
        &ENTRY_SUCCESSOR_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::EntrySuccessorPhaseReady,
    )
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        enable_event(ctx),
        "arceos_ex entry successor enable failed\n",
    );
    crate::flows::boot_init_flow::setup_after_entry_successor()
}

fn enable_event(ctx: &mut Context) -> EventResult {
    let state = crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE);
    if state != State::Ready || !entry_successor_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }

    crate::phases::state::mark_checked(
        &ENTRY_SUCCESSOR_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::EntrySuccessorPhaseOnline,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&ENTRY_SUCCESSOR_PHASE_STATE) == State::Online
}

fn entry_successor_phase_ready(ctx: &Context) -> bool {
    crate::flows::boot_init_flow::is_prepared()
        && ctx.init_stack.state() == State::Online
        && ctx.cpu_group.boot_cpu_state() == State::Online
        && ctx.boot_current_cpu.state() == State::Online
        && ctx.boot_cpu_local_interrupt.state() == State::Ready
        && ctx.boot_cpu_local_interrupt.disabled()
        && ctx.interrupt_stream.state() == State::Ready
        && ctx.interrupt_stream.early_boot_irqs_disabled()
        && ctx.vm.state() == State::Online
        && ctx.vm.entry_successor_ready()
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
        && ctx.memblock.entry_successor_setup_facts_ready()
        && start_kernel_deferred_facts_ready()
}

fn record_start_kernel_deferred_facts() {
    ENTRY_SUCCESSOR_DEFERRED_FACTS.store(
        DEFERRED_VMLINUX_BUILD_ID | DEFERRED_PAGE_ADDRESS_INIT | DEFERRED_START_KERNEL_POSITION,
        core::sync::atomic::Ordering::Relaxed,
    );
}

fn start_kernel_deferred_facts_ready() -> bool {
    ENTRY_SUCCESSOR_DEFERRED_FACTS.load(core::sync::atomic::Ordering::Relaxed)
        == (DEFERRED_VMLINUX_BUILD_ID | DEFERRED_PAGE_ADDRESS_INIT | DEFERRED_START_KERNEL_POSITION)
}
