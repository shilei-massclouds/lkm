use crate::{
    objects::{
        entry_prelude::EntryPreludeObjects,
        entry_successor::EntrySuccessorObjects,
        state::{EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static ENTRY_SUCCESSOR_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));
static mut ENTRY_SUCCESSOR: EntrySuccessorObjects = EntrySuccessorObjects::new();

pub fn setup(entry_prelude: &mut EntryPreludeObjects) -> ! {
    let objects = objects();
    crate::trace::checkpoint(Checkpoint::EntrySuccessorPhaseStarted);
    require(objects.setup(entry_prelude));
    require(checkpoint_ready(objects, entry_prelude));
    handoff()
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
}

fn checkpoint_ready(
    objects: &EntrySuccessorObjects,
    entry_prelude: &EntryPreludeObjects,
) -> EventResult {
    if !objects.entry_successor_phase_ready(entry_prelude) {
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

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex entry successor event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

fn objects() -> &'static mut EntrySuccessorObjects {
    // SAFETY: the boot path is single-hart and system-exclusive here. The
    // successor phase owns this object carrier for the duration of its setup().
    unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_SUCCESSOR) }
}
