use core::arch::global_asm;
use core::sync::atomic::AtomicU8;

use crate::{
    context::Context,
    objects::{
        boot_args::BootArgs,
        entry_prelude::Soc,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};

#[unsafe(link_section = ".data.phase")]
static ENTRY_PRELUDE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

const HEAD_TEXT_ALIGN: usize = 2;
const SSTATUS_FPU_VECTOR_MASK: usize = (0b11 << 9) | (0b11 << 13);
#[cfg(checkpoint_sbi_char)]
const SBI_LEGACY_CONSOLE_PUTCHAR: usize = 1;

const TRACE_ADOPT_BEGIN: usize = b'A' as usize;
const TRACE_INTERRUPT_PRESET: usize = b'I' as usize;
const TRACE_KERNEL_IMAGE_PRESET: usize = b'K' as usize;
const TRACE_ROOT_STREAM_PRESET: usize = b'O' as usize;
const TRACE_BSS_ZEROED: usize = b'Z' as usize;
const TRACE_BOOT_CPU_PRESET: usize = b'H' as usize;
const TRACE_CPU_GROUP_PRESET: usize = b'G' as usize;
const TRACE_INIT_TASK_PRESET: usize = b'T' as usize;
const TRACE_INIT_STACK_PRESET: usize = b'S' as usize;

#[repr(C, align(8))]
struct HeadHandoffWord(usize);

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".head.handoff")]
static mut head_boot_hartid: HeadHandoffWord = HeadHandoffWord(0);

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".head.handoff")]
static mut head_init_stack_sp: HeadHandoffWord = HeadHandoffWord(0);

global_asm!(
    r#"
    .section .head.text.entry, "ax"
    .align {head_text_align}
    .globl _start
_start:
    # Preserve OpenSBI boot arguments while the head segment rewrites a0.
    mv s0, a0
    mv s1, a1

    /*
     * EntryPreludePhase.setup() head segment.
     *
     * This code is physically owned by the EntryPreludePhase file because it
     * is the first executable part of that phase.  It performs only the model
     * events that must happen before Rust can run: close the interrupt stream,
     * establish gp, disable kernel FPU/vector use, zero BSS, record the boot
     * CPU group input, install the init task pointer, and create the initial
     * stack.  The Rust segment below continues the same setup() event.
     */
    li a0, {trace_adopt_begin}
    call {head_checkpoint}

    # InterruptStream.Preset: S-mode interrupt pending/enabled state is closed.
    csrw sie, zero
    csrw sip, zero
    li a0, {trace_interrupt_preset}
    call {head_checkpoint}

    # KernelImage.Preset: establish gp before accessing small data.
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    li a0, {trace_kernel_image_preset}
    call {head_checkpoint}

    # RootStream.Preset: disable kernel FPU/vector use in sstatus.
    li t0, {sstatus_fpu_vector_mask}
    csrrc zero, sstatus, t0
    li a0, {trace_root_stream_preset}
    call {head_checkpoint}

    # KernelImage.Setup: clear BSS before Rust observes static storage.
    la t0, _sbss
    la t1, _ebss
1:
    bgeu t0, t1, 2f
    sd zero, 0(t0)
    addi t0, t0, 8
    j 1b
2:
    li a0, {trace_bss_zeroed}
    call {head_checkpoint}

    # CpuGroup.Preset: publish the boot hart id for Rust-side adoption.
    la t0, {head_boot_hartid}
    sd s0, 0(t0)
    li a0, {trace_boot_cpu_preset}
    call {head_checkpoint}
    li a0, {trace_cpu_group_preset}
    call {head_checkpoint}

    # InitTask.Preset: install the init task pointer in tp.
    la tp, {init_task_storage}
    li a0, {trace_init_task_preset}
    call {head_checkpoint}

    # InitStack.Preset: reserve temporary page-table space on the boot stack.
    la sp, init_stack_end
    addi sp, sp, -{pt_size_on_stack}
    la t0, {head_init_stack_sp}
    sd sp, 0(t0)
    li a0, {trace_init_stack_preset}
    call {head_checkpoint}

    # Continue EntryPreludePhase.setup() in Rust with the original boot args.
    mv a0, s0
    mv a1, s1
    tail {rust_entry}

"#,
    head_boot_hartid = sym head_boot_hartid,
    head_checkpoint = sym arceos_ex_head_checkpoint,
    head_init_stack_sp = sym head_init_stack_sp,
    head_text_align = const HEAD_TEXT_ALIGN,
    init_task_storage = sym crate::objects::entry_prelude::init_task_storage,
    pt_size_on_stack = const crate::objects::entry_prelude::PT_SIZE_ON_STACK,
    rust_entry = sym entry_prelude_rust_entry,
    sstatus_fpu_vector_mask = const SSTATUS_FPU_VECTOR_MASK,
    trace_adopt_begin = const TRACE_ADOPT_BEGIN,
    trace_boot_cpu_preset = const TRACE_BOOT_CPU_PRESET,
    trace_bss_zeroed = const TRACE_BSS_ZEROED,
    trace_cpu_group_preset = const TRACE_CPU_GROUP_PRESET,
    trace_init_stack_preset = const TRACE_INIT_STACK_PRESET,
    trace_init_task_preset = const TRACE_INIT_TASK_PRESET,
    trace_interrupt_preset = const TRACE_INTERRUPT_PRESET,
    trace_kernel_image_preset = const TRACE_KERNEL_IMAGE_PRESET,
    trace_root_stream_preset = const TRACE_ROOT_STREAM_PRESET,
);

#[cfg(checkpoint_sbi_char)]
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn arceos_ex_head_checkpoint() {
    core::arch::naked_asm!(
        "li a7, {sbi_legacy_console_putchar}",
        "ecall",
        "ret",
        sbi_legacy_console_putchar = const SBI_LEGACY_CONSOLE_PUTCHAR,
    )
}

#[cfg(not(checkpoint_sbi_char))]
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn arceos_ex_head_checkpoint() {
    core::arch::naked_asm!("ret")
}

#[unsafe(no_mangle)]
extern "C" fn entry_prelude_rust_entry(hartid: usize, dtb_pa: usize) -> ! {
    let boot_args = BootArgs::new(hartid, dtb_pa);
    crate::phases::shutdown_on_error(
        crate::phases::prepare::adopt_head_prefix(&boot_args),
        "arceos_ex prepare event failed\n",
    );
    setup(&boot_args)
}

/// Continues `EntryPreludePhase.setup()` after the `_start` head segment.
///
/// The head segment above has already performed and checkpointed the pre-Rust
/// lifecycle events:
///
/// - `InterruptStream.Preset`
/// - `KernelImage.Preset`
/// - `RootStream.Preset`
/// - `KernelImage.Setup`
/// - `CpuGroup.Preset`
/// - `InitTask.Preset`
/// - `InitStack.Preset`
///
/// This Rust segment adopts those completed events into the resource objects,
/// then continues the remaining `EntryPreludePhase.setup()` drives in model
/// order. It also supplies the phase-owned continuation that resumes this same
/// setup() after `Vm.Setup` switches to the early virtual address space.
fn setup(boot_args: &BootArgs) -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        setup_until_vm_switch(ctx, boot_args),
        "arceos_ex entry prelude event failed\n",
    );
    ctx.vm.setup(
        &ctx.config,
        &ctx.static_objects,
        &ctx.lds,
        &mut ctx.kernel_image,
        after_vm_setup_continuation,
    )
}

fn setup_until_vm_switch(ctx: &mut Context, boot_args: &BootArgs) -> EventResult {
    adopt_head_prefix(ctx, boot_args)?;
    ctx.event_stream.preset(&ctx.config)?;
    ctx.vm.preset(
        &ctx.config,
        &mut ctx.static_objects,
        &ctx.lds,
        &ctx.kernel_image,
        boot_args,
        &mut ctx.raw_dtb,
        &mut ctx.fix_map,
    )
}

fn adopt_head_prefix(ctx: &mut Context, boot_args: &BootArgs) -> EventResult {
    ctx.interrupt_stream.adopt_head_preset()?;
    ctx.kernel_image.adopt_head_preset(&ctx.config, &ctx.lds)?;
    ctx.root_stream.adopt_head_preset()?;
    ctx.kernel_image.adopt_head_setup(&ctx.config, &ctx.lds)?;
    ctx.cpu_group.adopt_head_preset(boot_args)?;
    ctx.init_task.adopt_head_preset(&ctx.config)?;
    ctx.init_stack.adopt_head_preset(&ctx.config, &ctx.lds)
}

/// Continues the same `EntryPreludePhase.setup()` after `Vm.Setup` has switched
/// to the early virtual address space.  Control returns here directly from the
/// continuation selected by this phase; it does not pass through `BootPhase`.
extern "C" fn after_vm_setup_continuation() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        after_vm_setup(ctx),
        "arceos_ex entry prelude event failed\n",
    );
    handoff(ctx)
}

/// Finishes `EntryPreludePhase.setup()` after `Vm.Setup` has switched address
/// spaces and returned through the virtual continuation path.
fn after_vm_setup(ctx: &mut Context) -> EventResult {
    ctx.event_stream.enable(&ctx.vm, &ctx.static_objects)?;
    ctx.init_task.enable(&ctx.config, &ctx.vm)?;
    ctx.init_stack.setup(&ctx.vm)?;
    Soc::preset()?;
    checkpoint_ready(ctx)
}

/// Implements the Phase handoff edge from `EntryPreludePhase` to the next
/// BootPhase child, `EntrySuccessorPhase`.
fn handoff(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        handoff_event(ctx),
        "arceos_ex entry prelude handoff failed\n",
    );
    crate::phases::boot::entry_successor::setup(ctx)
}

fn handoff_event(ctx: &mut Context) -> EventResult {
    cleanup_entry_prelude_phase(ctx)?;
    crate::phases::state::mark(
        &ENTRY_PRELUDE_PHASE_STATE,
        LifecycleEvent::Cleanup,
        State::Ready,
        State::Destroyed,
        Checkpoint::EntryPreludePhaseDestroyed,
    )
}

fn cleanup_entry_prelude_phase(_ctx: &mut Context) -> EventResult {
    Ok(())
}

/// Checks the `EntryPreludePhase.Ready` model boundary before emitting its
/// checkpoint.  This is the coding counterpart of the phase invariant.
fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !entry_prelude_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&ENTRY_PRELUDE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &ENTRY_PRELUDE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::EntryPreludePhaseReady,
    )
}

fn entry_prelude_phase_ready(ctx: &Context) -> bool {
    ctx.root_stream.state() == State::Prepared
        && ctx.interrupt_stream.state() == State::Prepared
        && ctx.event_stream.state() == State::Online
        && ctx.kernel_image.state() == State::Online
        && ctx.raw_dtb.state() == State::Ready
        && ctx.init_task.state() == State::Online
        && ctx.init_stack.state() == State::Ready
        && ctx.vm.state() == State::Ready
        && ctx.vm.entry_prelude_ready()
        && ctx.cpu_group.state() == State::Prepared
        && ctx.cpu_group.boot_cpu_state() == State::Prepared
}
