use crate::objects::{
    boot_args::BootArgs,
    entry_prelude::EntryPreludeObjects,
    state::{EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static PREPARE_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn adopt_head_prefix(boot_args: &BootArgs) -> EventResult {
    if boot_args.state() != State::Online || !entry_prelude_objects().prepare_inputs_ready() {
        return EventResult::failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&PREPARE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    let result = crate::phases::state::adopt(
        &PREPARE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
    );
    if !result.is_success() {
        return result;
    }

    crate::phases::state::adopt(
        &PREPARE_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&PREPARE_PHASE_STATE) == State::Online
}

fn entry_prelude_objects() -> &'static EntryPreludeObjects {
    crate::phases::boot::entry_prelude_objects_ref()
}
