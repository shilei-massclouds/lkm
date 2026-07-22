//! Runtime lifecycle boundary for the running `Kernel` system instance.

use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};

#[unsafe(link_section = ".data.phase")]
static KERNEL_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn adopt_head_preset_start() -> EventResult {
    let state = crate::phases::state::load(&KERNEL_STATE);
    if state != State::Base || !crate::phases::prepare::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    Ok(())
}

pub fn preset_after_boot_init() -> ! {
    if crate::phases::state::load(&KERNEL_STATE) != State::Base
        || !crate::phases::prepare::is_online()
        || !crate::phases::boot_init::is_prepared()
        || !crate::phases::boot::entry_prelude::is_online()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex kernel preset invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }

    crate::phases::shutdown_on_error(mark_prepared(), "arceos_ex kernel preset event failed\n");
    crate::phases::boot_init::setup()
}

pub fn setup_after_boot_init() -> ! {
    if crate::phases::state::load(&KERNEL_STATE) != State::Prepared
        || !crate::phases::prepare::is_online()
        || !crate::phases::boot_init::is_ready()
        || !crate::phases::boot::entry_prelude::is_online()
        || !crate::phases::boot::is_online()
        || !crate::phases::interrupt::is_online()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex kernel setup invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }

    crate::phases::shutdown_on_error(mark_ready(), "arceos_ex kernel setup event failed\n");
    crate::phases::boot_init::enable()
}

pub fn switch_after_boot_init() -> ! {
    if crate::phases::state::load(&KERNEL_STATE) != State::Ready
        || !crate::phases::boot_init::is_online()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex kernel boot init switch invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }

    let ctx = crate::context::context();
    let schedule_result = ctx.scheduler.schedule(
        &ctx.cpu_group,
        &mut ctx.kernel_init_task,
        &mut ctx.kernel_init_flow,
        &mut ctx.kthreadd_task,
        &mut ctx.kthreadd_flow,
        &mut ctx.user_task_set,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.boot_cpu_current_task,
        &mut ctx.boot_dispatch_window,
    );
    crate::phases::shutdown_on_error(schedule_result, "arceos_ex first schedule failed\n");
    crate::checkpoint::dispatch(Checkpoint::SchedulerPickNextTaskExit, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerSwitchToEntry, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerSwitchToExit, ctx);
    crate::checkpoint::dispatch(Checkpoint::SchedulerScheduleExit, ctx);
    crate::phases::shutdown_on_error(
        ctx.scheduler
            .handoff_boot_idle_to_kernel_init(&ctx.kernel_init_task, &ctx.boot_cpu_current_task),
        "arceos_ex kernel_init task handoff failed\n",
    );
    crate::phases::boot_init::boot_task_restored()
}

pub fn enable_after_smp_runtime() -> ! {
    if !crate::phases::prepare::is_online()
        || !crate::phases::boot::entry_prelude::is_online()
        || !crate::phases::boot::is_online()
        || !crate::phases::interrupt::is_online()
        || !crate::phases::boot_init::is_online()
        || !crate::phases::smp_runtime::is_online()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex kernel enable invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }

    crate::phases::payload::preset()
}

pub fn enable_after_boot_init() -> ! {
    let ctx = crate::context::context_ref();
    if crate::phases::state::load(&KERNEL_STATE) != State::Ready
        || !crate::phases::boot::entry_prelude::is_online()
        || !crate::phases::boot::is_online()
        || !crate::phases::interrupt::is_online()
        || !crate::phases::boot_init::is_online()
        || ctx.kernel_init_task.state() != State::Online
        || ctx.scheduler.kernel_init_stack_switch_started_count() != 1
        || ctx.kernel_init_task.entry_started_count() != 1
        || !ctx.kernel_init_task.entry_stack_verified()
    {
        crate::arch::riscv64::sbi::putstr(
            "arceos_ex kernel enable after boot init invariant failed\n",
        );
        crate::arch::riscv64::sbi::system_shutdown()
    }

    crate::phases::smp_runtime::setup()
}

fn mark_prepared() -> EventResult {
    crate::phases::state::adopt(
        &KERNEL_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
    )
}

fn mark_ready() -> EventResult {
    crate::phases::state::adopt(
        &KERNEL_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
    )
}

pub fn mark_online() -> EventResult {
    if !crate::phases::prepare::is_online()
        || !crate::phases::boot::entry_prelude::is_online()
        || !crate::phases::boot::is_online()
        || !crate::phases::interrupt::is_online()
        || !crate::phases::boot_init::is_online()
        || !crate::phases::smp_runtime::is_online()
        || !crate::phases::payload::is_online()
    {
        return failed_condition(
            LifecycleEvent::Enable,
            crate::phases::state::load(&KERNEL_STATE),
            State::Ready,
            State::Online,
        );
    }

    crate::phases::state::mark(
        &KERNEL_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::KernelOnline,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&KERNEL_STATE) == State::Online
}

pub fn is_prepared() -> bool {
    crate::phases::state::load(&KERNEL_STATE) == State::Prepared
}
