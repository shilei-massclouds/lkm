use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        state::State,
        task::{
            TASK_SIGNAL_SEGMENTATION_FAULT, Task, TaskBreakpointState, TaskEntry,
            TaskExecutionAuthority, TaskKind, TaskRef, TaskSegvCode, TaskTerminalReason,
        },
        task_flow::TaskFlowRef,
    },
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
        assertions.assert("scheduler online", ctx.scheduler().state() == State::Online);
        assertions.assert(
            "current kernel init",
            ctx.current_task_ref()
                .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT),
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
                if ctx.smoke_scheduler_task().yielded_back()
                    && ctx
                        .current_task_ref()
                        .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT)
                {
                    break;
                }
            }

            let result = context().schedule_current();
            if let Err(error) = result {
                print_schedule_error("schedule current", error);
            }
            assertions.assert_ok("schedule current", result);
            attempts += 1;
        }

        let ctx = context();
        assertions.assert("bounded return", attempts < SCHEDULE_ATTEMPTS);
        assertions.assert("smoke task ran", ctx.smoke_scheduler_task().entry_ran());
        assertions.assert(
            "smoke task cpu",
            ctx.smoke_scheduler_task().cpu_id()
                == ctx
                    .scheduler()
                    .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
                    .map(|view| view.runqueue().cpu_id())
                    .unwrap_or(usize::MAX),
        );
        assertions.assert(
            "sleeping smoke task deactivated before pick",
            ctx.scheduler()
                .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
                .map(|view| !view.runqueue_contains_task_id(ctx.smoke_scheduler_task().task_id()))
                .unwrap_or(false)
                && ctx.smoke_scheduler_task().state() == State::Online
                && ctx.smoke_scheduler_task().scheduler_sleep_declared()
                && !ctx.smoke_scheduler_task().pending_wake_signal()
                && ctx.scheduler().last_prev_disposition()
                    == crate::objects::scheduler::PrevDisposition::Blocked
                && ctx.scheduler().prepare_prev_passes() != 0
                && ctx.scheduler().prepare_prev_blocked_passes() != 0,
        );
        assertions.assert(
            "smoke task yielded back",
            ctx.smoke_scheduler_task().yielded_back(),
        );
        assertions.assert(
            "smoke task uses unified task and flow carriers",
            ctx.smoke_scheduler_task().unified_carrier_ready()
                && ctx.smoke_scheduler_task().task_ref() == TaskRef::SMOKE_SCHEDULER
                && ctx.smoke_scheduler_task().flow_ref().is_valid(),
        );
        assertions.assert(
            "current returned kernel init",
            ctx.current_task_ref()
                .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT),
        );
        assertions.assert(
            "smoke switch saved",
            ctx.smoke_scheduler_task()
                .thread_context()
                .core_saved_count()
                != 0,
        );
        assertions.assert(
            "smoke switch restored",
            ctx.smoke_scheduler_task()
                .thread_context()
                .core_restored_count()
                != 0,
        );
        assertions.assert(
            "assembly switch preserves ra/sp/s0-s11 sentinels",
            ctx.scheduler().task_switch_register_sentinel_passes() != 0
                && ctx.kernel_init_task.switch_context().sp() != 0
                && ctx
                    .kernel_init_task
                    .switch_context()
                    .saved_register(11)
                    .is_some()
                && ctx
                    .kernel_init_task
                    .switch_context()
                    .saved_register(12)
                    .is_none(),
        );
        assertions.assert(
            "tp follows selected Task identity outside switch context",
            ctx.scheduler().task_switch_tp_identity_passes() != 0,
        );
        assertions.assert(
            "class callback and fallback protocols observed in Linux order",
            ctx.scheduler().combined_pick_passes() != 0
                && ctx.scheduler().fallback_pick_passes() != 0
                && ctx.scheduler().pick_task_passes() != 0
                && ctx.scheduler().put_prev_task_passes() != 0
                && ctx.scheduler().set_next_task_passes() != 0
                && ctx.scheduler().prepare_prev_sequence() < ctx.scheduler().pick_task_sequence()
                && ctx.scheduler().pick_task_sequence() < ctx.scheduler().put_prev_task_sequence()
                && ctx.scheduler().put_prev_task_sequence()
                    < ctx.scheduler().set_next_task_sequence()
                && ctx.scheduler().last_picked_class()
                    == Some(crate::objects::scheduler::SchedClassRef::Fair)
                && ctx.scheduler().task_dispatch_signal_passes() != 0,
        );
        assertions.assert(
            "nonidentity switch preflights then saves suspends restores finishes and dispatches fixed carriers",
            ctx.scheduler().switch_preflight_passes() == ctx.scheduler().switch_to_passes()
                && ctx.scheduler().save_core_context_sequence() != 0
                && ctx.scheduler().save_core_context_sequence()
                    < ctx.scheduler().suspend_task_sequence()
                && ctx.scheduler().suspend_task_sequence()
                    < ctx.scheduler().restore_core_context_sequence()
                && ctx.scheduler().restore_core_context_sequence()
                    < ctx.scheduler().finish_task_switch_sequence()
                && ctx.scheduler().finish_task_switch_sequence()
                    < ctx.scheduler().task_dispatch_sequence()
                && ctx.scheduler().task_dispatch_sequence()
                    < ctx.scheduler().flow_signal_sequence()
                && ctx.scheduler().flow_enter_signal_passes() != 0
                && ctx.scheduler().task_dispatch_signal_passes()
                    == ctx.scheduler().flow_enter_signal_passes(),
        );

        let before_state = ctx.kernel_init_task.state();
        let before_authority = ctx.kernel_init_task.task().execution_authority();
        let before_breakpoint = ctx.kernel_init_task.task().breakpoint_state();
        let before_saved = ctx
            .kernel_init_task
            .task()
            .thread_context()
            .core_saved_count();
        let before_restored = ctx
            .kernel_init_task
            .task()
            .thread_context()
            .core_restored_count();
        let before_prepare = ctx.scheduler().scheduler_prepare_task_switch_count();
        let before_finish = ctx.scheduler().scheduler_finish_task_switch_count();
        let before_identity = ctx.scheduler().identity_switch_passes();
        let before_flow_enter = ctx.scheduler().flow_enter_signal_passes();
        let before_task_dispatch = ctx.scheduler().task_dispatch_signal_passes();
        let identity_result = ctx.schedule_current();
        if let Err(error) = identity_result {
            print_schedule_error("identity schedule accepted", error);
        }
        assertions.assert_ok("identity schedule accepted", identity_result);
        assertions.assert(
            "identity schedule consumes return without Dispatch/Enter delivery",
            ctx.scheduler().identity_switch_passes() == before_identity + 1
                && ctx.scheduler().flow_enter_signal_passes() == before_flow_enter
                && ctx.scheduler().task_dispatch_signal_passes() == before_task_dispatch
                && ctx.kernel_init_task.state() == before_state
                && ctx.kernel_init_task.task().execution_authority() == before_authority
                && ctx.kernel_init_task.task().breakpoint_state() == before_breakpoint
                && ctx
                    .kernel_init_task
                    .task()
                    .thread_context()
                    .core_saved_count()
                    == before_saved
                && ctx
                    .kernel_init_task
                    .task()
                    .thread_context()
                    .core_restored_count()
                    == before_restored
                && ctx.scheduler().scheduler_prepare_task_switch_count() == before_prepare
                && ctx.scheduler().scheduler_finish_task_switch_count() == before_finish
                && ctx
                    .current_task_ref()
                    .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT),
        );
        let before_identity = ctx.scheduler().identity_switch_passes();
        assertions.assert_ok("identity switch accepted", ctx.smoke_identity_switch());
        assertions.assert(
            "identity switch has no lifecycle breakpoint or context event",
            ctx.scheduler().identity_switch_passes() == before_identity + 1
                && ctx.kernel_init_task.state() == before_state
                && ctx.kernel_init_task.task().execution_authority() == before_authority
                && ctx.kernel_init_task.task().breakpoint_state() == before_breakpoint
                && ctx
                    .kernel_init_task
                    .task()
                    .thread_context()
                    .core_saved_count()
                    == before_saved
                && ctx
                    .kernel_init_task
                    .task()
                    .thread_context()
                    .core_restored_count()
                    == before_restored
                && ctx.scheduler().scheduler_prepare_task_switch_count() == before_prepare
                && ctx.scheduler().scheduler_finish_task_switch_count() == before_finish
                && ctx
                    .current_task_ref()
                    .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT),
        );
        assertions.assert(
            "prepared enable dispatch enter suspend and terminal contract",
            task_breakpoint_contract_smoke(),
        );
        assertions.assert(
            "SIGSEGV terminal records exact code address and rejects duplicates",
            task_sigsegv_terminal_smoke(),
        );
        assertions.assert(
            "stale cross-cpu inactive and wrong-binding Schedule senders leave scheduler unchanged",
            schedule_sender_rejection_smoke(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn print_schedule_error(label: &'static str, error: crate::objects::state::EventError) {
    use crate::objects::printk;

    printk::write_str("scheduler smoke error label=");
    printk::write_str(label);
    if let Some(diagnostic) = error.diagnostic() {
        printk::write_str(" phase=");
        printk::write_str(diagnostic.phase);
        printk::write_str(" step=");
        printk::write_str(diagnostic.step);
        printk::write_str(" first_failed=");
        printk::write_str(diagnostic.first_failed);
    }
    printk::write_fmt(format_args!(
        " error={} event={} actual={} expected={} target={}\n",
        error.error_code() as char,
        error.event_code() as char,
        error.actual_state_code() as char,
        error.expected_state_code() as char,
        error.target_state_code() as char,
    ));
}

fn schedule_sender_rejection_smoke() -> bool {
    let (
        sender_flow_ref,
        current_task_ref,
        current_cpu_ref,
        schedule_passes,
        preemption_disables,
        prepare_prev_passes,
        pick_next_passes,
        switch_entries,
        flow_enters,
        task_dispatches,
        task_state,
        task_authority,
        task_breakpoint,
    ) = {
        let ctx = context();
        let Ok(sender_flow_ref) = ctx.current_task_flow_ref() else {
            return false;
        };
        let Ok(current_task_ref) = ctx.current_task_ref() else {
            return false;
        };
        let Ok(current_cpu) = ctx.current_cpu() else {
            return false;
        };
        (
            sender_flow_ref,
            current_task_ref,
            current_cpu.cpu_ref(),
            ctx.scheduler().schedule_passes(),
            ctx.scheduler().schedule_preemption_disable_count(),
            ctx.scheduler().prepare_prev_passes(),
            ctx.scheduler().pick_next_task_passes(),
            ctx.scheduler().switch_to_entry_count(),
            ctx.scheduler().flow_enter_signal_passes(),
            ctx.scheduler().task_dispatch_signal_passes(),
            ctx.kernel_init_task.state(),
            ctx.kernel_init_task.task().execution_authority(),
            ctx.kernel_init_task.task().breakpoint_state(),
        )
    };
    if current_task_ref != TaskRef::KERNEL_INIT {
        return false;
    }

    let stale_sender =
        sender_flow_ref.with_generation_for_test(sender_flow_ref.generation().wrapping_add(1));
    if context()
        .schedule_from_refs_for_test(stale_sender, current_task_ref, current_cpu_ref)
        .is_ok()
        || context()
            .schedule_from_refs_for_test(TaskFlowRef::BOOT_INIT, current_task_ref, current_cpu_ref)
            .is_ok()
        || context()
            .schedule_from_refs_for_test(
                sender_flow_ref,
                current_task_ref,
                crate::objects::cpu::CpuRef::new(current_cpu_ref.logical_id() + 1),
            )
            .is_ok()
        || context()
            .schedule_from_refs_for_test(sender_flow_ref, TaskRef::BOOT, current_cpu_ref)
            .is_ok()
    {
        return false;
    }

    let ctx = context();
    ctx.scheduler().schedule_passes() == schedule_passes
        && ctx.scheduler().schedule_preemption_disable_count() == preemption_disables
        && ctx.scheduler().prepare_prev_passes() == prepare_prev_passes
        && ctx.scheduler().pick_next_task_passes() == pick_next_passes
        && ctx.scheduler().switch_to_entry_count() == switch_entries
        && ctx.scheduler().flow_enter_signal_passes() == flow_enters
        && ctx.scheduler().task_dispatch_signal_passes() == task_dispatches
        && ctx.kernel_init_task.state() == task_state
        && ctx.kernel_init_task.task().execution_authority() == task_authority
        && ctx.kernel_init_task.task().breakpoint_state() == task_breakpoint
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref == current_task_ref)
}

fn task_breakpoint_contract_smoke() -> bool {
    let mut task = Task::new_user(TaskRef::user(0, 77), 0);
    if task
        .set_identity_metadata(77, TaskEntry::UserChild, TaskKind::TestOnly)
        .is_err()
        || task.adopt_preset().is_err()
        || task.declare_and_bind_embedded_flow().is_err()
    {
        return false;
    }
    task.init_dummy_switch_context();
    if task.breakpoint_state() != TaskBreakpointState::Prepared
        || task.adopt_setup().is_err()
        || !task.bind_flow_cpu_ref(crate::objects::cpu::CpuRef::new(0))
        || task.publish_embedded_flow().is_err()
        || task.set_runtime_running().is_err()
        || task.publish_runqueue_binding().is_err()
        || task.adopt_enable().is_err()
    {
        return false;
    }
    let flow_ref = task.flow_ref();
    let stale_flow = flow_ref.with_generation_for_test(flow_ref.generation() + 1);
    if task.state() != State::Online
        || task.execution_authority() != TaskExecutionAuthority::None
        || task.breakpoint_state() != TaskBreakpointState::Valid
        || task.breakpoint_flow() != flow_ref
        || !task.breakpoint_matches(flow_ref)
        || task.breakpoint_matches(stale_flow)
        || !task.switch_in_ready()
        || !task.dispatch_rejected_for_test(
            stale_flow,
            task.context_epoch(),
            crate::objects::cpu::CpuRef::new(0),
        )
        || !task.dispatch_rejected_for_test(
            flow_ref,
            task.context_epoch().wrapping_add(1),
            crate::objects::cpu::CpuRef::new(0),
        )
        || !task.dispatch_rejected_for_test(
            flow_ref,
            task.context_epoch(),
            crate::objects::cpu::CpuRef::new(1),
        )
        || task.dispatch_and_enter_for_test().is_err()
        || !task.duplicate_enter_rejected_for_test()
        || task.switch_in_ready()
        || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
            task.embedded_flow(),
            &task,
        )
        || task.declare_scheduler_sleep().is_err()
        || task.post_pending_wake_signal().is_err()
        || !task.scheduler_sleep_declared()
        || !task.pending_wake_signal()
        || !task.prepare_prev_runnable()
        || task.scheduler_sleep_declared()
        || task.pending_wake_signal()
    {
        return false;
    }

    if task.breakpoint_state() != TaskBreakpointState::Invalid
        || task.declare_scheduler_sleep().is_err()
        || task.prepare_prev_runnable()
        || task.deactivate_from_scheduler().is_err()
        || task.suspend_from_cpu().is_err()
        || task.breakpoint_state() != TaskBreakpointState::Valid
        || task.breakpoint_flow() != flow_ref
        || !task.breakpoint_matches(flow_ref)
        || task.breakpoint_matches(stale_flow)
        || task.wake_for_scheduler_enqueue().is_err()
        || task.dispatch_and_enter_for_test().is_err()
        || task.cleanup_embedded_flow().is_err()
        || task.disable().is_err()
        || task.state() != State::Offline
        || task.breakpoint_state() != TaskBreakpointState::Invalid
        || task.cleanup().is_err()
    {
        return false;
    }
    task.state() == State::Destroyed
}

fn task_sigsegv_terminal_smoke() -> bool {
    let mut task = Task::new_user(TaskRef::user(0, 78), 0);
    if task
        .set_identity_metadata(78, TaskEntry::UserChild, TaskKind::TestOnly)
        .is_err()
        || task.adopt_preset().is_err()
        || task.declare_and_bind_embedded_flow().is_err()
    {
        return false;
    }
    task.init_dummy_switch_context();
    if task.adopt_setup().is_err()
        || !task.bind_flow_cpu_ref(crate::objects::cpu::CpuRef::new(0))
        || task.publish_embedded_flow().is_err()
        || task.set_runtime_running().is_err()
        || task.publish_runqueue_binding().is_err()
        || task.adopt_enable().is_err()
        || task.dispatch_and_enter_for_test().is_err()
        || !task.record_segmentation_fault_terminal(TaskSegvCode::Maperr, 0x1234_5000)
    {
        return false;
    }
    let Some(info) = task.segv_info() else {
        return false;
    };
    task.terminal_reason() == TaskTerminalReason::SegmentationFault
        && info.signal() == TASK_SIGNAL_SEGMENTATION_FAULT
        && info.code() == TaskSegvCode::Maperr
        && info.address() == 0x1234_5000
        && task.terminal_wait_status(0xff) == TASK_SIGNAL_SEGMENTATION_FAULT
        && !task.record_segmentation_fault_terminal(TaskSegvCode::Accerr, 0x5678_9000)
        && !task.record_out_of_memory_terminal()
        && task.segv_info() == Some(info)
}

extern "C" fn smoke_scheduler_task_entry() -> ! {
    let ctx = context();
    if ctx.mark_smoke_scheduler_entry_ran().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if ctx.mark_smoke_scheduler_yielded_back().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if ctx.declare_current_scheduler_sleep().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if let Err(error) = ctx.schedule_current() {
        print_schedule_error("smoke task schedule back", error);
        crate::arch::riscv64::sbi::system_shutdown();
    }

    loop {
        core::hint::spin_loop();
    }
}
