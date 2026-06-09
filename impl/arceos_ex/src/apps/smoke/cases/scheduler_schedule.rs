use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{cpu_control::CurrentTaskRef, state::State},
};

const SCHEDULE_ATTEMPTS: usize = 4;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut CooperativeSwitchScenario::new());
    suite.result()
}

struct CooperativeSwitchScenario;

impl CooperativeSwitchScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for CooperativeSwitchScenario {
    fn name(&self) -> &'static str {
        "scheduler_schedule.cooperative_switch"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert("scheduler online", ctx.scheduler.state() == State::Online);
        assertions.assert(
            "current kernel init",
            ctx.boot_cpu_current_task.current_is_kernel_init(),
        );
        assertions.assert_ok(
            "setup smoke scheduler task",
            ctx.setup_smoke_scheduler_task(smoke_scheduler_task_entry),
        );
        assertions.assert_ok(
            "enqueue smoke scheduler task",
            ctx.enqueue_smoke_scheduler_task(),
        );
        assertions.assert_fail(
            "duplicate setup smoke scheduler task",
            ctx.setup_smoke_scheduler_task(smoke_scheduler_task_entry),
        );
        assertions.assert_fail(
            "duplicate enqueue smoke scheduler task",
            ctx.enqueue_smoke_scheduler_task(),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let mut attempts = 0usize;
        while attempts < SCHEDULE_ATTEMPTS {
            {
                let ctx = context();
                if ctx.scheduler.smoke_scheduler_task().yielded_back()
                    && ctx.boot_cpu_current_task.current_is_kernel_init()
                {
                    break;
                }
            }

            assertions.assert_ok("schedule current", context().schedule_current());
            attempts += 1;
        }

        let ctx = context();
        assertions.assert("bounded return", attempts < SCHEDULE_ATTEMPTS);
        assertions.assert(
            "smoke task ran",
            ctx.scheduler.smoke_scheduler_task().entry_ran(),
        );
        assertions.assert(
            "smoke task cpu",
            ctx.scheduler.smoke_scheduler_task().cpu_id() == ctx.scheduler.boot_runqueue().cpu_id(),
        );
        assertions.assert(
            "smoke task enqueued",
            ctx.scheduler
                .boot_runqueue()
                .contains_task(ctx.scheduler.smoke_scheduler_task().task_id()),
        );
        assertions.assert(
            "smoke task yielded back",
            ctx.scheduler.smoke_scheduler_task().yielded_back(),
        );
        assertions.assert(
            "current returned kernel init",
            ctx.boot_cpu_current_task.current() == CurrentTaskRef::KernelInit,
        );
        assertions.assert(
            "smoke switch saved",
            ctx.scheduler
                .smoke_scheduler_task()
                .thread_context()
                .core_saved_count()
                != 0,
        );
        assertions.assert(
            "smoke switch restored",
            ctx.scheduler
                .smoke_scheduler_task()
                .thread_context()
                .core_restored_count()
                != 0,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

extern "C" fn smoke_scheduler_task_entry() -> ! {
    let ctx = context();
    if ctx.mark_smoke_scheduler_entry_ran().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if ctx.mark_smoke_scheduler_yielded_back().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if ctx.schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    loop {
        core::hint::spin_loop();
    }
}
