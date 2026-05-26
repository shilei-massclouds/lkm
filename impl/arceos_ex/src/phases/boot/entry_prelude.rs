use crate::{
    objects::{
        boot_args::BootArgs,
        entry_prelude::EntryPreludeObjects,
        state::{EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static ENTRY_PRELUDE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

/// Continues `EntryPreludePhase.setup()` after the `_start` head segment.
///
/// The head segment is implemented by the `_start` assembly block in
/// `main.rs`, because Rust requires the naked entry symbol to be declared at
/// crate scope.  That assembly has already performed and checkpointed the
/// pre-Rust lifecycle events:
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
/// order until `Vm.Setup` switches to the early virtual address space.
pub fn setup(boot_args: &BootArgs, objects: &mut EntryPreludeObjects) -> ! {
    require(objects.adopt_head_prefix(boot_args));
    require(objects.event_stream_preset());
    require(objects.vm_preset(boot_args));
    objects.vm_setup()
}

/// Finishes `EntryPreludePhase.setup()` after `Vm.Setup` has switched address
/// spaces and returned through the virtual continuation path.
pub fn after_vm_setup(objects: &mut EntryPreludeObjects) -> EventResult {
    let result = objects.after_vm_setup();
    if !result.is_success() {
        return result;
    }

    checkpoint_ready(objects)
}

/// Implements the Phase handoff edge from `EntryPreludePhase` to the next
/// BootPhase child, `EntrySuccessorPhase`.
pub fn handoff(objects: &mut EntryPreludeObjects) -> ! {
    require(objects.cleanup_entry_prelude_phase());
    require(crate::phases::state::mark(
        &ENTRY_PRELUDE_PHASE_STATE,
        LifecycleEvent::Cleanup,
        State::Ready,
        State::Destroyed,
        Checkpoint::EntryPreludePhaseDestroyed,
    ));
    let entry_successor = crate::phases::boot::entry_successor_objects();
    crate::phases::entry_successor::setup(entry_successor, objects)
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
