use crate::{
    context::Context,
    objects::{
        pre_smp_init::pre_smp_runtime_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static PRE_SMP_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::PreSmpInitPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex pre-smp init event failed\n",
    );
    crate::phases::smp_runtime::smp_bringup::setup(ctx)
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::up_multitask::rest_init::dispatch_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

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

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !pre_smp_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&PRE_SMP_INIT_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &PRE_SMP_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::PreSmpInitPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&PRE_SMP_INIT_PHASE_STATE) == State::Ready
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
