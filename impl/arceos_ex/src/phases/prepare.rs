use crate::objects::{
    boot_args::BootArgs,
    state::{failed_condition, EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static PREPARE_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn adopt_head_prefix(boot_args: &BootArgs) -> EventResult {
    let ctx = crate::context::context_ref();
    if boot_args.state() != State::Online || !prepare_inputs_ready(ctx) {
        return failed_condition(
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
    if result.is_err() {
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

fn prepare_inputs_ready(ctx: &crate::context::Context) -> bool {
    ctx.config.entry_prelude_ready()
        && ctx.lds.state() == State::Online
        && ctx.lds.entry_layout_ready()
        && ctx.static_objects.state() == State::Online
        && ctx.static_objects.storage_ready(&ctx.config)
}
