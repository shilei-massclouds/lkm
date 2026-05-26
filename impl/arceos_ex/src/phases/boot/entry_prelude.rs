use core::arch::global_asm;
use core::sync::atomic::AtomicU8;

use crate::{
    objects::{
        boot_args::BootArgs,
        entry_prelude::EntryPreludeObjects,
        state::{EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};

#[unsafe(link_section = ".data.phase")]
static ENTRY_PRELUDE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
static mut ENTRY_PRELUDE: EntryPreludeObjects = EntryPreludeObjects::new();

global_asm!(
    r#"
    .section .head.text.entry, "ax"
    .globl _start
_start:
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
    li a0, '['
    call arceos_ex_head_checkpoint
    li a0, 123
    call arceos_ex_head_checkpoint
    li a0, 125
    call arceos_ex_head_checkpoint
    li a0, '('
    call arceos_ex_head_checkpoint
    li a0, 'A'
    call arceos_ex_head_checkpoint

    csrw sie, zero
    csrw sip, zero
    li a0, 'I'
    call arceos_ex_head_checkpoint

    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    li a0, 'K'
    call arceos_ex_head_checkpoint

    li t0, (0b11 << 9) | (0b11 << 13)
    csrrc zero, sstatus, t0
    li a0, 'O'
    call arceos_ex_head_checkpoint

    la t0, _sbss
    la t1, _ebss
1:
    bgeu t0, t1, 2f
    sd zero, 0(t0)
    addi t0, t0, 8
    j 1b
2:
    li a0, 'Z'
    call arceos_ex_head_checkpoint

    la t0, head_boot_hartid
    sd s0, 0(t0)
    li a0, 'H'
    call arceos_ex_head_checkpoint
    li a0, 'G'
    call arceos_ex_head_checkpoint

    la tp, init_task_storage
    li a0, 'T'
    call arceos_ex_head_checkpoint

    la sp, init_stack_end
    addi sp, sp, -256
    la t0, head_init_stack_sp
    sd sp, 0(t0)
    li a0, 'S'
    call arceos_ex_head_checkpoint

    mv a0, s0
    mv a1, s1
    tail entry_prelude_rust_entry

    .section .boot.stack, "aw", @nobits
    .align 12
    .space 4096 * 4

    .section .head.handoff, "aw", @nobits
    .align 3
    .globl head_boot_hartid
head_boot_hartid:
    .space 8
    .globl head_init_stack_sp
head_init_stack_sp:
    .space 8
"#
);

#[cfg(checkpoint_sbi_char)]
global_asm!(
    r#"
    .section .head.text.checkpoint, "ax"
    .align 2
    .globl arceos_ex_head_checkpoint
arceos_ex_head_checkpoint:
    li a7, 1
    ecall
    ret
"#
);

#[cfg(not(checkpoint_sbi_char))]
global_asm!(
    r#"
    .section .head.text.checkpoint, "ax"
    .align 2
    .globl arceos_ex_head_checkpoint
arceos_ex_head_checkpoint:
    ret
"#
);

#[unsafe(no_mangle)]
extern "C" fn entry_prelude_rust_entry(hartid: usize, dtb_pa: usize) -> ! {
    let boot_args = BootArgs::new(hartid, dtb_pa);
    require(crate::phases::prepare::adopt_head_prefix(&boot_args));
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
    let objects = objects();
    require(objects.adopt_head_prefix(boot_args));
    require(objects.event_stream_preset());
    require(objects.vm_preset(boot_args));
    objects.vm_setup(after_vm_setup_continuation)
}

/// Continues the same `EntryPreludePhase.setup()` after `Vm.Setup` has switched
/// to the early virtual address space.  Control returns here directly from the
/// continuation selected by this phase; it does not pass through `BootPhase`.
extern "C" fn after_vm_setup_continuation() -> ! {
    let objects = objects();
    require(after_vm_setup(objects));
    handoff(objects)
}

/// Finishes `EntryPreludePhase.setup()` after `Vm.Setup` has switched address
/// spaces and returned through the virtual continuation path.
fn after_vm_setup(objects: &mut EntryPreludeObjects) -> EventResult {
    let result = objects.after_vm_setup();
    if !result.is_success() {
        return result;
    }

    checkpoint_ready(objects)
}

/// Implements the Phase handoff edge from `EntryPreludePhase` to the next
/// BootPhase child, `EntrySuccessorPhase`.
fn handoff(objects: &mut EntryPreludeObjects) -> ! {
    require(objects.cleanup_entry_prelude_phase());
    require(crate::phases::state::mark(
        &ENTRY_PRELUDE_PHASE_STATE,
        LifecycleEvent::Cleanup,
        State::Ready,
        State::Destroyed,
        Checkpoint::EntryPreludePhaseDestroyed,
    ));
    crate::phases::boot::entry_successor::setup(objects)
}

/// Checks the `EntryPreludePhase.Ready` model boundary before emitting its
/// checkpoint.  This is the coding counterpart of the phase invariant.
fn checkpoint_ready(objects: &EntryPreludeObjects) -> EventResult {
    if !objects.entry_prelude_phase_ready() {
        return EventResult::failed_condition(
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

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex entry prelude event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

fn objects() -> &'static mut EntryPreludeObjects {
    // SAFETY: the boot path is single-hart and system-exclusive here. This
    // static carrier keeps object state alive across Vm.Setup, which does not
    // return to its physical-address caller.
    unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_PRELUDE) }
}

pub fn objects_ref() -> &'static EntryPreludeObjects {
    // SAFETY: read-only access is used by PreparePhase before EntryPreludePhase
    // mutates the carrier. Later callers must keep using the mutable phase path.
    unsafe { &*core::ptr::addr_of!(ENTRY_PRELUDE) }
}
