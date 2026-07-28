mod preset;
mod rest_init;
mod schedule_handoff;

use crate::{
    checkpoint::Checkpoint,
    objects::{
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::{Task, TaskEntry, TaskKind, TaskRef},
        task_flow::{TaskFlow, TaskFlowRef, task_flow_execution_guard_satisfied},
    },
};

/// BootTask's immutable initial continuation. Every execution boundary reads
/// the owning Task's Task-only OnCpu projection.
pub struct BootInitFlow {
    flow: TaskFlow,
}

impl BootInitFlow {
    pub const fn new() -> Self {
        Self {
            flow: TaskFlow::new_static_bound(TaskFlowRef::BOOT_INIT, TaskRef::BOOT),
        }
    }

    pub const fn state(&self) -> State {
        self.flow.state()
    }

    pub(crate) fn core_mut(&mut self) -> &mut TaskFlow {
        &mut self.flow
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
            || !owner.initial_flow().same_identity(self.flow.flow_ref())
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

    pub fn setup_and_activate(&mut self, owner: &mut Task, checkpoint: Checkpoint) -> EventResult {
        self.flow.setup(owner, Some(checkpoint))?;
        owner.activate_initial_flow(&mut self.flow)
    }

    pub fn enable_with_successor(
        &mut self,
        owner: &mut Task,
        successor: &mut TaskFlow,
        checkpoint: Checkpoint,
    ) -> EventResult {
        self.flow
            .enable_after_successor_handoff(owner, successor, checkpoint)
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
        || ctx.boot_task.task().active_flow().is_valid()
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
    setup()
}

/// Starts BootInitFlow.Setup with its first direct leaf.
pub fn setup() -> ! {
    crate::phases::shutdown_on_error(
        require_guarded_state(LifecycleEvent::Setup, State::Prepared, State::Ready),
        "arceos_ex boot init setup start failed\n",
    );
    if !boot_task_on_cpu_and_canonical() {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready),
            "arceos_ex boot init setup dependency failed\n",
        );
    }
    crate::phases::boot::entry_successor::preset(crate::context::context())
}

pub fn setup_after_entry_successor() -> ! {
    if crate::context::context_ref().boot_init_flow.state() != State::Prepared
        || !boot_task_on_cpu_and_canonical()
        || !crate::phases::boot::entry_successor::is_online()
    {
        crate::arch::riscv64::sbi::putstr(
            "arceos_ex boot init after entry successor invariant failed\n",
        );
        crate::arch::riscv64::sbi::system_shutdown()
    }
    crate::phases::boot::core_prepare::preset(crate::context::context())
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
            .setup_and_activate(ctx.boot_task.task_mut(), Checkpoint::BootInitFlowReady)
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
        ctx.boot_init_flow.enable_with_successor(
            ctx.boot_task.task_mut(),
            ctx.boot_idle_flow.core_mut(),
            Checkpoint::BootInitFlowOnline,
        )
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

/// Lowers Kernel.Enable's Scheduler.Action::Schedule after BootInitFlow has
/// committed Online. The call returns only after a later switch restores the
/// original BootTask.
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
    crate::flows::boot_idle_flow::preset_entry(crate::context::context())
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

/// Reports the post-handoff boundary observed while KernelInitTask owns the CPU.
pub fn dispatch_ready() -> bool {
    let ctx = crate::context::context_ref();
    is_online()
        && ctx.scheduler.schedule_passes() != 0
        && ctx.scheduler.current_runqueue_resolve_passes() != 0
        && ctx.scheduler.pick_next_task_passes() != 0
        && ctx.scheduler.switch_to_passes() != 0
        && ctx.boot_cpu_current_task().switch_committed_count() != 0
        && ctx.boot_cpu_current_task().current_is_kernel_init()
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
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
    crate::phases::boot::entry_successor::is_online()
        && crate::phases::boot::core_prepare::is_online()
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
