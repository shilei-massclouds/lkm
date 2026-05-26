use crate::{
    objects::{
        boot_args::BootArgs,
        entry_prelude::EntryPreludeObjects,
        entry_successor::EntrySuccessorObjects,
        state::{EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static BOOT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));
static mut ENTRY_PRELUDE: EntryPreludeObjects = EntryPreludeObjects::new();
static mut ENTRY_SUCCESSOR: EntrySuccessorObjects = EntrySuccessorObjects::new();

pub fn setup_after_head_prefix(boot_args: &BootArgs) -> ! {
    let entry_prelude = entry_prelude_objects();
    crate::phases::entry_prelude::setup_after_head_prefix(boot_args, entry_prelude);
}

pub fn after_vm_setup_continuation() -> ! {
    let entry_prelude = entry_prelude_objects();
    require(crate::phases::entry_prelude::after_vm_setup(entry_prelude));
    crate::phases::entry_prelude::handoff(entry_prelude)
}

pub fn setup_after_children() -> ! {
    require(crate::phases::state::mark(
        &BOOT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::BootPhaseReady,
    ));
    handoff()
}

fn handoff() -> ! {
    crate::startup_timeline_ready()
}

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex boot event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

fn entry_prelude_objects() -> &'static mut EntryPreludeObjects {
    // SAFETY: the boot path is single-hart and system-exclusive here. This
    // static carrier keeps object state alive across Vm.Setup, which does not
    // return to its physical-address caller.
    unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_PRELUDE) }
}

pub fn entry_prelude_objects_ref() -> &'static EntryPreludeObjects {
    // SAFETY: read-only access is used by PreparePhase before BootPhase mutates
    // the carrier. Later callers must keep using the mutable phase path.
    unsafe { &*core::ptr::addr_of!(ENTRY_PRELUDE) }
}

pub fn entry_successor_objects() -> &'static mut EntrySuccessorObjects {
    // SAFETY: same boot-exclusive context as ENTRY_PRELUDE.
    unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_SUCCESSOR) }
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&BOOT_PHASE_STATE) == State::Ready
}
