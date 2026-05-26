use crate::{
    objects::{
        boot_args::BootArgs,
        entry_prelude::EntryPreludeObjects,
        state::{EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

static ENTRY_PRELUDE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup_after_head_prefix(boot_args: &BootArgs, objects: &mut EntryPreludeObjects) -> ! {
    require(objects.adopt_head_prefix(boot_args));
    require(objects.event_stream_preset());
    require(objects.vm_preset(boot_args));
    objects.vm_setup()
}

pub fn after_vm_setup(objects: &mut EntryPreludeObjects) -> EventResult {
    let result = objects.after_vm_setup();
    if !result.is_success() {
        return result;
    }

    checkpoint_ready(objects)
}

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
