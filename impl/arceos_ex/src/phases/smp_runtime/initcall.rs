use crate::{
    context::Context,
    objects::{
        initcall::initcall_phase_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static INITCALL_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::InitcallPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex initcall event failed\n",
    );
    crate::phases::smp_runtime::setup_after_children()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::smp_runtime::runtime_core::is_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    ctx.cpuset_smp_trimmed.setup(&ctx.runtime_core_boundary)?;
    ctx.driver_core_deferred
        .setup(&ctx.cpuset_smp_trimmed, &ctx.page_allocator, &ctx.workqueue)?;
    ctx.irq_proc_view_deferred
        .setup(&ctx.driver_core_deferred, &ctx.irq_dispatch_tree)?;
    ctx.ctor_table
        .setup(&ctx.irq_proc_view_deferred, &ctx.static_objects)?;
    ctx.initcall_table.setup(
        &ctx.ctor_table,
        &ctx.static_objects,
        &ctx.saved_command_line,
        &ctx.kernel_init_task,
        &ctx.page_allocator,
    )?;
    ctx.initcall_boundary.setup(
        &ctx.cpuset_smp_trimmed,
        &ctx.driver_core_deferred,
        &ctx.irq_proc_view_deferred,
        &ctx.ctor_table,
        &ctx.initcall_table,
    )
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !initcall_phase_ready(
        &ctx.cpuset_smp_trimmed,
        &ctx.driver_core_deferred,
        &ctx.irq_proc_view_deferred,
        &ctx.ctor_table,
        &ctx.initcall_table,
        &ctx.initcall_boundary,
    ) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&INITCALL_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &INITCALL_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::InitcallPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&INITCALL_PHASE_STATE) == State::Ready
}
