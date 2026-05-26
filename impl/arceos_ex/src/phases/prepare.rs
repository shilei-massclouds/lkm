use crate::objects::{
    boot_args::BootArgs,
    entry_prelude::EntryPreludeObjects,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
};

#[unsafe(link_section = ".data.phase")]
static mut PREPARE_PHASE: Lifecycle = Lifecycle::new(State::Base);

pub fn adopt_head_prefix(boot_args: &BootArgs) -> EventResult {
    if boot_args.state() != State::Online || !entry_prelude_objects().prepare_inputs_ready() {
        return EventResult::failed_condition(
            LifecycleEvent::Setup,
            prepare_phase().state(),
            State::Base,
            State::Ready,
        );
    }

    let result = prepare_phase().adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready);
    if !result.is_success() {
        return result;
    }

    prepare_phase().adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
}

fn prepare_phase() -> &'static mut Lifecycle {
    // SAFETY: early boot is single-hart and phase state is only mutated by the
    // startup timeline in specification order.
    unsafe { &mut *core::ptr::addr_of_mut!(PREPARE_PHASE) }
}

pub fn is_online() -> bool {
    prepare_phase().state() == State::Online
}

fn entry_prelude_objects() -> &'static EntryPreludeObjects {
    crate::phases::boot::entry_prelude_objects_ref()
}
