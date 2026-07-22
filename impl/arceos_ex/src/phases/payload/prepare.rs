use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};

#[unsafe(link_section = ".data.phase")]
static PAYLOAD_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Preset, State::Base, State::Prepared),
        "arceos_ex payload prepare preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::PayloadPreparePhaseStarted);

    let result = (|| {
        ctx.exec_sync_boundaries.setup(
            &ctx.kernel_init_task,
            &ctx.system_state,
            &ctx.binary_format_registry,
        )?;
        ctx.exec_transaction
            .setup(&ctx.binary_format_registry, &ctx.exec_sync_boundaries)?;
        ctx.user_clone_deferred_boundaries
            .setup(&ctx.exec_sync_boundaries)?;
        if ctx.exec_sync_boundaries.state() != State::Ready
            || ctx.exec_transaction.state() != State::Ready
            || ctx.user_clone_deferred_boundaries.state() != State::Ready
            || !super::mainline_ready(ctx)
        {
            return phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared);
        }
        crate::phases::state::mark_checked(
            &PAYLOAD_PREPARE_PHASE_STATE,
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PayloadPreparePhasePrepared,
        )
    })();
    crate::phases::shutdown_on_error(result, "arceos_ex payload prepare preset failed\n");
    setup()
}

fn setup() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Setup, State::Prepared, State::Ready),
        "arceos_ex payload prepare setup start failed\n",
    );
    let result = crate::apps::setup_selected_payload(ctx).and_then(|()| {
        if ctx.selected_payload_handoff.state() != State::Ready
            || !ctx.selected_payload_handoff.kind_bound()
            || !ctx.selected_payload_handoff.variant_setup_ready()
            || !super::mainline_ready(ctx)
        {
            return phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }
        crate::phases::state::mark_checked(
            &PAYLOAD_PREPARE_PHASE_STATE,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::PayloadPreparePhaseReady,
        )
    });
    crate::phases::shutdown_on_error(result, "arceos_ex payload prepare setup failed\n");
    enable()
}

fn enable() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Enable, State::Ready, State::Online),
        "arceos_ex payload prepare enable start failed\n",
    );
    let result = crate::phases::state::mark_checked(
        &PAYLOAD_PREPARE_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::PayloadPreparePhaseOnline,
    );
    crate::phases::shutdown_on_error(result, "arceos_ex payload prepare enable failed\n");
    crate::checkpoint::dispatch_after_trace(Checkpoint::PayloadPreparePhaseOnline, ctx);
    crate::phases::smp_runtime::setup_after_payload_prepare()
}

fn require_transition(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    let actual = state();
    if actual != expected
        || !super::common_dependencies_ready()
        || !super::mainline_ready(ctx)
        || ctx.kernel_init_flow.state() != State::Prepared
    {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(event, state(), expected, target)
}

pub fn state() -> State {
    crate::phases::state::load(&PAYLOAD_PREPARE_PHASE_STATE)
}

pub fn is_online() -> bool {
    state() == State::Online
}
