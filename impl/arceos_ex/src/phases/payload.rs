use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        printk,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};

#[unsafe(link_section = ".data.phase")]
static PAYLOAD_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Preset, State::Base, State::Prepared),
        "arceos_ex payload preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::PayloadPhaseStarted);

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
            || !mainline_ready(ctx)
        {
            return phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared);
        }
        crate::phases::state::mark_checked(
            &PAYLOAD_PHASE_STATE,
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PayloadPhasePrepared,
        )
    })();
    crate::phases::shutdown_on_error(result, "arceos_ex payload preset failed\n");
    setup()
}

fn setup() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Setup, State::Prepared, State::Ready),
        "arceos_ex payload setup start failed\n",
    );

    let result = crate::apps::setup_selected_payload(ctx).and_then(|()| {
        if ctx.selected_payload_handoff.state() != State::Ready
            || !ctx.selected_payload_handoff.kind_bound()
            || !ctx.selected_payload_handoff.variant_setup_ready()
            || !mainline_ready(ctx)
        {
            return phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }
        crate::phases::state::mark_checked(
            &PAYLOAD_PHASE_STATE,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::PayloadPhaseReady,
        )
    });
    crate::phases::shutdown_on_error(result, "arceos_ex payload setup failed\n");
    enable()
}

fn enable() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_transition(ctx, LifecycleEvent::Enable, State::Ready, State::Online),
        "arceos_ex payload enable start failed\n",
    );

    let result = crate::apps::prepare_selected_payload(ctx).and_then(|()| {
        if ctx.selected_payload_handoff.state() != State::Online
            || !ctx.selected_payload_handoff.variant_prepare_ready()
            || !ctx.selected_payload_handoff.no_return_entry_bound()
            || !mainline_ready(ctx)
        {
            return phase_failure(LifecycleEvent::Enable, State::Ready, State::Online);
        }
        crate::phases::state::mark_checked(
            &PAYLOAD_PHASE_STATE,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::PayloadPhaseOnline,
        )
    });
    crate::phases::shutdown_on_error(result, "arceos_ex payload enable failed\n");

    crate::checkpoint::dispatch_after_trace(Checkpoint::PayloadPhaseOnline, ctx);
    crate::phases::shutdown_on_error(
        crate::systems::kernel::mark_online(),
        "arceos_ex kernel enable after payload failed\n",
    );
    crate::apps::enter_selected_payload(ctx)
}

fn require_transition(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    let actual = crate::phases::state::load(&PAYLOAD_PHASE_STATE);
    if actual != expected || !payload_phase_dependencies_ready() || !mainline_ready(ctx) {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn payload_phase_dependencies_ready() -> bool {
    crate::phases::prepare::is_online()
        && crate::phases::boot::is_online()
        && crate::phases::interrupt::is_online()
        && crate::phases::boot_init::is_online()
        && crate::phases::smp_runtime::is_online()
        && crate::phases::boot::core_prepare::is_online()
        && crate::phases::boot::mm_core_init::is_online()
        && printk::is_ready()
        && (printk::boot_console_online() || printk::console_handoff_complete())
}

fn mainline_ready(ctx: &Context) -> bool {
    ctx.kernel_init_task.state() == State::Online
        && ctx.boot_cpu_current_task.current_is_kernel_init()
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
        && ctx.kernel_init_task.entry_started_count() == 1
        && ctx.kernel_init_task.entry_stack_verified()
        && ctx.kernel_init_task.current_stack_pointer_in_range()
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&PAYLOAD_PHASE_STATE),
        expected,
        target,
    )
}

pub fn state() -> State {
    crate::phases::state::load(&PAYLOAD_PHASE_STATE)
}

pub fn is_online() -> bool {
    state() == State::Online
}
