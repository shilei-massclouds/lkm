use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context_ref,
    objects::{
        current_task::CurrentTaskErrorCode,
        task::{Task, TaskRef},
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut ResolutionScenario);
    suite.result()
}

struct ResolutionScenario;

impl SmokeScenario for ResolutionScenario {
    fn name(&self) -> &'static str {
        "current_task.resolution"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        let current = ctx.current_task();
        assertions.assert(
            "kernel init resolves from tp",
            current.is_ok_and(|capability| capability.task_ref() == TaskRef::KERNEL_INIT),
        );
        assertions.assert(
            "current cpu derives through effective flow",
            ctx.current_cpu().is_ok_and(|current_cpu| {
                ctx.kernel_init_flow.core().cpu_ref() == Some(current_cpu.cpu_ref())
                    && current_cpu.logical_id() == 0
            }),
        );

        let kernel_identity = ctx.kernel_init_task.task() as *const Task as usize;
        let stale_ref = TaskRef::KERNEL_INIT.with_generation_for_test(2);
        let stale = ctx.resolve_current_task_for_test(kernel_identity, stale_ref);
        assertions.assert(
            "stale generation rejected",
            stale.is_err_and(|error| {
                let diagnostic = error.diagnostic();
                error.code() == CurrentTaskErrorCode::StaleTaskRef
                    && diagnostic.tp() == kernel_identity
                    && diagnostic.task_ref() == TaskRef::KERNEL_INIT
                    && diagnostic.flow_ref() == ctx.kernel_init_task.task().active_flow()
                    && Some(diagnostic.cpu_ref()) == ctx.cpu_group.boot_cpu_ref()
            }),
        );

        let unknown_identity = usize::MAX;
        let unknown = ctx.resolve_current_task_for_test(unknown_identity, TaskRef::KERNEL_INIT);
        assertions.assert(
            "unknown identity rejected with diagnostics",
            unknown.is_err_and(|error| {
                let diagnostic = error.diagnostic();
                error.code() == CurrentTaskErrorCode::UnknownIdentity
                    && diagnostic.tp() == unknown_identity
                    && diagnostic.task_ref() == TaskRef::KERNEL_INIT
                    && diagnostic.flow_ref() == ctx.kernel_init_task.task().active_flow()
                    && Some(diagnostic.cpu_ref()) == ctx.cpu_group.boot_cpu_ref()
            }),
        );

        let boot_address = ctx.boot_task.carrier_address();
        let boot_physical = ctx.kernel_image.runtime_to_phys(boot_address);
        let boot_virtual = ctx.kernel_image.runtime_to_link(boot_address);
        assertions.assert(
            "boot physical identity remains canonical",
            boot_physical.is_some_and(|identity| {
                ctx.resolve_current_task_for_test(identity, TaskRef::BOOT)
                    .is_err_and(|error| error.code() == CurrentTaskErrorCode::NotOnCpuLive)
            }),
        );
        assertions.assert(
            "boot virtual identity remains canonical",
            boot_virtual.is_some_and(|identity| {
                ctx.resolve_current_task_for_test(identity, TaskRef::BOOT)
                    .is_err_and(|error| error.code() == CurrentTaskErrorCode::NotOnCpuLive)
            }),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
