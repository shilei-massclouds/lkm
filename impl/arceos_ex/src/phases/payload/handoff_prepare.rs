use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};

#[unsafe(link_section = ".data.phase")]
static PAYLOAD_HANDOFF_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Preset, State::Base, State::Prepared),
        "arceos_ex payload handoff prepare preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::PayloadHandoffPreparePhaseStarted);
    let result = crate::apps::prepare_selected_payload(ctx).and_then(|()| {
        if ctx.selected_payload_handoff.state() != State::Online
            || !ctx.selected_payload_handoff.variant_prepare_ready()
            || !ctx.selected_payload_handoff.no_return_entry_bound()
            || ctx.kernel_init_task.flow_state() != State::Online
            || !super::mainline_ready(ctx)
        {
            return phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared);
        }
        crate::phases::state::mark_checked(
            &PAYLOAD_HANDOFF_PREPARE_PHASE_STATE,
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PayloadHandoffPreparePhasePrepared,
        )
    });
    crate::phases::shutdown_on_error(result, "arceos_ex payload handoff prepare preset failed\n");
    setup()
}

fn setup() -> ! {
    let ctx = crate::context::context_ref();
    let result =
        if require_transition(ctx, LifecycleEvent::Setup, State::Prepared, State::Ready).is_ok() {
            crate::phases::state::mark_checked(
                &PAYLOAD_HANDOFF_PREPARE_PHASE_STATE,
                LifecycleEvent::Setup,
                State::Prepared,
                State::Ready,
                Checkpoint::PayloadHandoffPreparePhaseReady,
            )
        } else {
            phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
        };
    crate::phases::shutdown_on_error(result, "arceos_ex payload handoff prepare setup failed\n");
    enable()
}

fn enable() -> ! {
    let ctx = crate::context::context_ref();
    let result =
        if require_transition(ctx, LifecycleEvent::Enable, State::Ready, State::Online).is_ok() {
            crate::phases::state::mark_checked(
                &PAYLOAD_HANDOFF_PREPARE_PHASE_STATE,
                LifecycleEvent::Enable,
                State::Ready,
                State::Online,
                Checkpoint::PayloadHandoffPreparePhaseOnline,
            )
        } else {
            phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
        };
    crate::phases::shutdown_on_error(result, "arceos_ex payload handoff prepare enable failed\n");
    crate::checkpoint::dispatch_after_trace(Checkpoint::PayloadHandoffPreparePhaseOnline, ctx);
    crate::phases::smp_runtime::enable_after_payload_handoff_prepare()
}

fn require_transition(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    let actual = state();
    let payload_boundary_ready = match event {
        LifecycleEvent::Preset => {
            ctx.selected_payload_handoff.state() == State::Ready
                && ctx.selected_payload_handoff.kind_bound()
                && ctx.selected_payload_handoff.variant_setup_ready()
        }
        LifecycleEvent::Setup | LifecycleEvent::Enable => {
            ctx.selected_payload_handoff.state() == State::Online
                && ctx.selected_payload_handoff.variant_prepare_ready()
                && ctx.selected_payload_handoff.no_return_entry_bound()
        }
        _ => false,
    };
    if actual != expected
        || !super::prepare::is_online()
        || !super::mainline_ready(ctx)
        || ctx.kernel_init_task.flow_state() != State::Online
        || !payload_boundary_ready
    {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(event, state(), expected, target)
}

pub fn state() -> State {
    crate::phases::state::load(&PAYLOAD_HANDOFF_PREPARE_PHASE_STATE)
}

pub fn is_online() -> bool {
    state() == State::Online
}
