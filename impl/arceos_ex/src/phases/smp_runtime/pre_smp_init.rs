use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        pre_smp_init::pre_smp_runtime_ready,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static PRE_SMP_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(ctx),
        "arceos_ex pre-smp init preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::PreSmpInitPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared(ctx)),
        "arceos_ex pre-smp init preset failed\n",
    );
    setup(ctx)
}

fn preset_start(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&PRE_SMP_INIT_PHASE_STATE);
    if state != State::Base
        || !crate::phases::boot_init::is_online()
        || !crate::phases::boot_init::rest_init::dispatch_ready()
        || !ctx.kernel_init_task.current_stack_pointer_in_range()
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.kernel_init_task
        .observe_kthreadd_done_release(&mut ctx.kthreadd_ready_gate)?;
    ctx.page_allocator.open_full_gfp_mask()?;
    ctx.cpu_group.prepare_pre_smp()?;
    ctx.workqueue
        .setup(&ctx.page_allocator, &ctx.cpu_group, &ctx.kernel_init_task)?;
    ctx.vmstat_core.preset(
        &ctx.workqueue,
        &ctx.page_allocator,
        &ctx.kernel_init_task,
        &ctx.scheduler,
    )?;
    ctx.rcu_core.tasks_rcu_mut().setup()?;
    ctx.pre_smp_initcalls.setup(
        &ctx.kernel_init_task,
        &ctx.rcu_core,
        &ctx.softirq,
        &ctx.scheduler,
        &ctx.cpu_group,
    )?;
    ctx.pre_smp_boundary.setup(
        &ctx.kernel_init_task,
        &ctx.pre_smp_initcalls,
        &ctx.scheduler,
        &ctx.cpu_group,
    )
}

fn adopt_prepared(ctx: &Context) -> EventResult {
    transition_if_ready(
        ctx,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::PreSmpInitPhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::PreSmpInitPhaseReady,
        ),
        "arceos_ex pre-smp init setup failed\n",
    );
    enable(ctx)
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::PreSmpInitPhaseOnline,
        ),
        "arceos_ex pre-smp init enable failed\n",
    );
    crate::phases::smp_runtime::preset_after_pre_smp_init()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let state = crate::phases::state::load(&PRE_SMP_INIT_PHASE_STATE);
    if state != expected || !pre_smp_phase_ready(ctx) {
        return failed_condition(event, state, expected, target);
    }
    crate::phases::state::mark_checked(
        &PRE_SMP_INIT_PHASE_STATE,
        event,
        expected,
        target,
        checkpoint,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&PRE_SMP_INIT_PHASE_STATE) == State::Online
}

fn pre_smp_phase_ready(ctx: &Context) -> bool {
    pre_smp_runtime_ready(
        &ctx.kernel_init_task,
        &ctx.kthreadd_task,
        &ctx.scheduler,
        &ctx.page_allocator,
        &ctx.cpu_group,
        &ctx.workqueue,
        &ctx.rcu_core,
        &ctx.vmstat_core,
        &ctx.pre_smp_initcalls,
        &ctx.pre_smp_boundary,
    )
}
