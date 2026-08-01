mod idle;
mod idle_entry;
mod preset;
mod rest_init;
mod schedule_handoff;
mod setup;

use core::arch::global_asm;

#[cfg(app_smoke)]
pub(crate) use preset::{
    boot_task_entry_bind_count, boot_task_entry_bind_diagnostic,
    boot_task_entry_preemption_initialized,
};

use crate::{
    checkpoint::Checkpoint,
    objects::{
        boot_task::BootTask,
        cpu_group::CpuGroup,
        rest_init::{KernelInitTask, KthreaddReadyGate, KthreaddTask},
        scheduler::Scheduler,
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::{Task, TaskEntry, TaskKind, TaskRef},
        task_flow::{TaskFlow, TaskFlowRef, task_flow_execution_guard_satisfied},
    },
};

global_asm!(
    r#"
    .section .text.boot_init_flow_preset_completion, "ax"
    .align 2
    .globl boot_init_flow_preset_completion
    .type boot_init_flow_preset_completion, @function
boot_init_flow_preset_completion:
    .option push
    .option norelax
    tail start_kernel
    .option pop
    .size boot_init_flow_preset_completion, . - boot_init_flow_preset_completion
"#,
);

unsafe extern "C" {
    fn boot_init_flow_preset_completion() -> !;
}

/// BootTask's immutable initial continuation. Every execution boundary reads
/// the owning Task's Task-only OnCpu projection.
pub struct BootInitFlow {
    flow: TaskFlow,
    pub(crate) idle: idle::IdleRuntime,
}

impl BootInitFlow {
    pub const fn new() -> Self {
        Self {
            flow: TaskFlow::new_static_bound(TaskFlowRef::BOOT_INIT, TaskRef::BOOT),
            idle: idle::IdleRuntime::new(),
        }
    }

    pub const fn state(&self) -> State {
        self.flow.state()
    }

    pub(crate) const fn core(&self) -> &TaskFlow {
        &self.flow
    }

    pub fn bind_cpu_ref(&mut self, cpu_ref: crate::objects::cpu::CpuRef) -> bool {
        self.flow.bind_cpu_ref(cpu_ref)
    }

    pub const fn cpu_ref(&self) -> Option<crate::objects::cpu::CpuRef> {
        self.flow.cpu_ref()
    }

    pub fn accept_initial_start_signal(&self, owner: &Task) -> EventResult {
        if self.flow.state() != State::Base
            || !owner.flow().same_identity(self.flow.flow_ref())
            || !owner.owns_flow(self.flow.flow_ref())
            || !task_flow_execution_guard_satisfied(&self.flow, owner)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.flow.state(),
                State::Base,
                State::Prepared,
            );
        }
        Ok(())
    }

    pub fn preset(&mut self, owner: &Task, checkpoint: Checkpoint) -> EventResult {
        self.flow.preset(owner, Some(checkpoint))
    }

    pub fn setup(&mut self, owner: &Task, checkpoint: Checkpoint) -> EventResult {
        self.flow.setup(owner, Some(checkpoint))
    }

    pub fn prepare_idle_runtime(
        &mut self,
        boot_task: &BootTask,
        scheduler: &Scheduler,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        kthreadd_ready_gate: &KthreaddReadyGate,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        self.idle.setup(
            &self.flow,
            boot_task,
            scheduler,
            kernel_init_task,
            kthreadd_task,
            kthreadd_ready_gate,
            cpu_group,
        )
    }

    pub fn enable(&mut self, owner: &Task, checkpoint: Checkpoint) -> EventResult {
        self.flow.enable(owner, Some(checkpoint))
    }
}

/// Adopts the BootInitFlow Preset boundary emitted by `_start`.
pub fn adopt_head_preset_start() -> EventResult {
    let ctx = crate::context::context();
    let state = ctx.boot_init_flow.state();
    if state != State::Base
        || ctx.boot_task.state() != State::OnCpu
        || ctx.boot_task.task_ref() != TaskRef::BOOT
        || ctx.boot_task.pid() != 0
        || ctx.boot_task.task().entry() != TaskEntry::None
        || ctx.boot_task.task().kind() != TaskKind::None
        || !ctx
            .boot_task
            .task()
            .flow()
            .same_identity(TaskFlowRef::BOOT_INIT)
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    ctx.boot_init_flow
        .accept_initial_start_signal(ctx.boot_task.task())
}

/// Completes BootInitFlow.Preset after all direct entry-object drives finish.
pub(super) fn preset_after_entry_objects() -> ! {
    let dependencies_ready = boot_task_on_cpu_and_canonical()
        && preset::entry_objects_ready(crate::context::context_ref());
    let ctx = crate::context::context();
    let result = if dependencies_ready {
        ctx.boot_init_flow
            .preset(ctx.boot_task.task(), Checkpoint::BootInitFlowPrepared)
    } else {
        failed_condition(
            LifecycleEvent::Preset,
            ctx.boot_init_flow.state(),
            State::Base,
            State::Prepared,
        )
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init preset failed\n");
    unsafe { boot_init_flow_preset_completion() }
}

/// Starts BootInitFlow.Setup with its first direct leaf.
#[unsafe(no_mangle)]
pub extern "C" fn start_kernel() -> ! {
    let result = if start_kernel_entry_guard_satisfied() {
        Ok(())
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex start_kernel guard failed\n");
    setup::run(crate::context::context())
}

pub fn setup_after_core_prepare() -> ! {
    require_setup_leaf(
        crate::phases::boot::core_prepare::is_online(),
        "core prepare",
    );
    crate::phases::boot::mm_core_init::preset(crate::context::context())
}

pub fn setup_after_mm_core_init() -> ! {
    require_setup_leaf(
        crate::phases::boot::mm_core_init::is_online(),
        "mm core init",
    );
    crate::phases::boot::sched_init::preset(crate::context::context())
}

pub fn setup_after_sched_init() -> ! {
    require_setup_leaf(crate::phases::boot::sched_init::is_online(), "sched init");
    crate::phases::interrupt::irq_time_init::preset(crate::context::context())
}

pub fn setup_after_irq_time_init() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::irq_time_init::is_online(),
        "irq time init",
    );
    crate::phases::interrupt::local_irq_enable::preset(crate::context::context())
}

pub fn setup_after_local_irq_enable() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::local_irq_enable::is_online(),
        "local irq enable",
    );
    crate::phases::interrupt::irq_open_prepare::preset(crate::context::context())
}

pub fn setup_after_irq_open_prepare() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::irq_open_prepare::is_online(),
        "irq open prepare",
    );
    crate::phases::interrupt::process_prepare::preset(crate::context::context())
}

pub fn setup_after_process_prepare() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::process_prepare::is_online(),
        "process prepare",
    );
    rest_init::preset(crate::context::context())
}

/// Completes BootInitFlow.Setup after the final direct leaf reaches Online.
pub fn setup_after_boot_init_rest_init() -> ! {
    let dependencies_ready = setup_leaves_online() && rest_init::is_online();
    let ctx = crate::context::context();
    let result = if dependencies_ready {
        ctx.boot_init_flow
            .setup(ctx.boot_task.task(), Checkpoint::BootInitFlowReady)
    } else {
        failed_condition(
            LifecycleEvent::Setup,
            ctx.boot_init_flow.state(),
            State::Prepared,
            State::Ready,
        )
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init setup failed\n");
    enable()
}

/// BootInitFlow.Enable drives only the reversible schedule-handoff leaf.
pub fn enable() -> ! {
    crate::phases::shutdown_on_error(
        require_guarded_state(LifecycleEvent::Enable, State::Ready, State::Online),
        "arceos_ex boot init enable start failed\n",
    );
    if !setup_leaves_online() || !rest_init::is_online() {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Enable, State::Ready, State::Online),
            "arceos_ex boot init enable dependency failed\n",
        );
    }
    schedule_handoff::preset(crate::context::context())
}

/// Commits BootInitFlow.Online at the last reversible boundary.
pub fn enable_after_boot_init_schedule_handoff() -> ! {
    let dependencies_ready = boot_task_on_cpu_and_canonical()
        && rest_init::is_online()
        && schedule_handoff::is_online()
        && schedule_handoff::precommit_ready();
    let ctx = crate::context::context();
    let result = if dependencies_ready {
        let crate::context::Context {
            boot_init_flow,
            boot_task,
            cpu_group,
            kernel_init_task,
            kthreadd_task,
            kthreadd_ready_gate,
            ..
        } = ctx;
        if let Some(scheduler) = cpu_group.boot_scheduler() {
            boot_init_flow
                .prepare_idle_runtime(
                    boot_task,
                    scheduler,
                    kernel_init_task,
                    kthreadd_task,
                    kthreadd_ready_gate,
                    cpu_group,
                )
                .and_then(|()| {
                    boot_init_flow.enable(boot_task.task(), Checkpoint::BootInitFlowOnline)
                })
        } else {
            failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Ready,
                State::Online,
            )
        }
    } else {
        failed_condition(
            LifecycleEvent::Enable,
            ctx.boot_init_flow.state(),
            State::Ready,
            State::Online,
        )
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init enable failed\n");
    schedule()
}

/// Lowers BootInitFlow's internal idle Schedule signal after the Flow has
/// committed Online. Kernel.Enable remains the enclosing continuation but is
/// not the Scheduler signal sender. The call returns only after a later switch
/// restores the original BootTask.
fn schedule() -> ! {
    if !crate::systems::kernel::enable_in_progress() || !is_online() {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Enable, State::Online, State::Online),
            "arceos_ex boot init schedule invariant failed\n",
        );
    }

    let ctx = crate::context::context();
    let schedule_result = ctx.schedule_current();
    crate::phases::shutdown_on_error(schedule_result, "arceos_ex first schedule failed\n");
    boot_task_restored()
}

/// Runs only if a later scheduler switch restores the original BootTask stack.
pub fn boot_task_restored() -> ! {
    idle::preset_entry(crate::context::context())
}

pub fn is_online() -> bool {
    crate::context::context_ref().boot_init_flow.state() == State::Online
        && setup_leaves_online()
        && rest_init::is_online()
        && schedule_handoff::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn rest_init_is_online() -> bool {
    rest_init::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn schedule_handoff_is_online() -> bool {
    schedule_handoff::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn idle_entry_is_online() -> bool {
    idle::entry_is_online()
}

/// Reports the post-handoff boundary observed while KernelInitTask owns the CPU.
pub fn dispatch_ready() -> bool {
    let ctx = crate::context::context_ref();
    is_online()
        && ctx.scheduler().schedule_passes() != 0
        && ctx.scheduler().current_runqueue_resolve_passes() != 0
        && ctx.scheduler().pick_next_task_passes() != 0
        && ctx.scheduler().switch_to_passes() != 0
        && ctx.scheduler().scheduler_finish_task_switch_count() != 0
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.kernel_init_task.task_ref()))
        && ctx.scheduler().kernel_init_stack_switch_started_count() == 1
}

pub fn is_prepared() -> bool {
    crate::context::context_ref().boot_init_flow.state() == State::Prepared
        && boot_task_on_cpu_and_canonical()
}

fn require_setup_leaf(child_online: bool, child: &str) {
    let result = if child_online
        && require_guarded_state(LifecycleEvent::Setup, State::Prepared, State::Ready).is_ok()
    {
        Ok(())
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    let message = match child {
        "core prepare" => "arceos_ex boot init after core prepare failed\n",
        "mm core init" => "arceos_ex boot init after mm core init failed\n",
        "sched init" => "arceos_ex boot init after sched init failed\n",
        "irq time init" => "arceos_ex boot init after irq time init failed\n",
        "local irq enable" => "arceos_ex boot init after local irq enable failed\n",
        "irq open prepare" => "arceos_ex boot init after irq open prepare failed\n",
        _ => "arceos_ex boot init after process prepare failed\n",
    };
    crate::phases::shutdown_on_error(result, message)
}

fn setup_leaves_online() -> bool {
    crate::phases::boot::core_prepare::is_online()
        && crate::phases::boot::mm_core_init::is_online()
        && crate::phases::boot::sched_init::is_online()
        && crate::phases::interrupt::irq_time_init::is_online()
        && crate::phases::interrupt::local_irq_enable::is_online()
        && crate::phases::interrupt::irq_open_prepare::is_online()
        && crate::phases::interrupt::process_prepare::is_online()
}

fn boot_task_on_cpu_and_canonical() -> bool {
    let ctx = crate::context::context_ref();
    ctx.boot_task.state() == State::OnCpu
        && ctx.boot_task.task_ref() == TaskRef::BOOT
        && ctx.boot_task.task().task_ref() == TaskRef::BOOT
        && ctx.boot_task.pid() == 0
}

fn start_kernel_entry_guard_satisfied() -> bool {
    let ctx = crate::context::context_ref();
    let flow = ctx.boot_init_flow.core();
    let cpu_ref = ctx.boot_init_flow.cpu_ref();
    crate::systems::kernel::enable_in_progress()
        && ctx.boot_init_flow.state() == State::Prepared
        && boot_task_on_cpu_and_canonical()
        && ctx.boot_task.task().flow().same_identity(flow.flow_ref())
        && task_flow_execution_guard_satisfied(flow, ctx.boot_task.task())
        && cpu_ref.is_some()
        && cpu_ref == ctx.cpu_group.boot_cpu_ref()
        && ctx
            .current_cpu()
            .is_ok_and(|current| Some(current.cpu_ref()) == cpu_ref)
}

fn require_guarded_state(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    let ctx = crate::context::context_ref();
    let actual = ctx.boot_init_flow.state();
    if actual != expected
        || !task_flow_execution_guard_satisfied(&ctx.boot_init_flow.flow, ctx.boot_task.task())
    {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::context::context_ref().boot_init_flow.state(),
        expected,
        target,
    )
}

pub(super) fn rest_init_facts_stable(ctx: &crate::context::Context) -> bool {
    rest_init::facts_stable(ctx)
}
