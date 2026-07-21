pub mod rest_init;

use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    objects::{
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::{TaskEntry, TaskKind, TaskRef},
    },
};

#[unsafe(link_section = ".data.phase")]
static BOOT_INIT_FLOW_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

/// Adopts the BootInitFlow Preset boundary emitted by `_start`.
pub fn adopt_head_preset_start() -> EventResult {
    let state = crate::phases::state::load(&BOOT_INIT_FLOW_STATE);
    let ctx = crate::context::context_ref();
    if state != State::Base
        || ctx.boot_task.state() != State::Online
        || ctx.boot_task.task_ref() != TaskRef::BOOT
        || ctx.boot_task.pid() != 0
        || ctx.boot_task.task().entry() != TaskEntry::None
        || ctx.boot_task.task().kind() != TaskKind::None
        || ctx.boot_task.task().active_flow().is_valid()
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

/// Completes BootInitFlow.Preset after EntryPreludePhase reaches Online.
pub fn preset_after_entry_prelude() -> ! {
    let result =
        if boot_task_online_and_canonical() && crate::phases::boot::entry_prelude::is_online() {
            commit_state(
                LifecycleEvent::Preset,
                State::Base,
                State::Prepared,
                Checkpoint::BootInitFlowPrepared,
            )
        } else {
            phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared)
        };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init preset failed\n");
    crate::systems::kernel::preset_after_boot_init()
}

/// Starts BootInitFlow.Setup by driving BootPhase.
pub fn setup() -> ! {
    crate::phases::shutdown_on_error(
        require_state(LifecycleEvent::Setup, State::Prepared, State::Ready),
        "arceos_ex boot init setup start failed\n",
    );
    if !boot_task_online_and_canonical() || !crate::phases::boot::entry_prelude::is_online() {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready),
            "arceos_ex boot init setup dependency failed\n",
        );
    }
    crate::phases::boot::preset()
}

/// Continues BootInitFlow.Setup from BootPhase to InterruptPhase.
pub fn setup_after_boot() -> ! {
    if crate::phases::state::load(&BOOT_INIT_FLOW_STATE) != State::Prepared
        || !boot_task_online_and_canonical()
        || !crate::phases::boot::entry_prelude::is_online()
        || !crate::phases::boot::is_online()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex boot init after boot invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
    crate::phases::interrupt::setup()
}

/// Completes BootInitFlow.Setup after InterruptPhase reaches Online.
pub fn setup_after_interrupt() -> ! {
    let result = if boot_task_online_and_canonical()
        && crate::phases::boot::entry_prelude::is_online()
        && crate::phases::boot::is_online()
        && crate::phases::interrupt::is_online()
    {
        commit_state(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootInitFlowReady,
        )
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init setup failed\n");
    crate::systems::kernel::setup_after_boot_init()
}

/// Starts BootInitFlow.Enable with the rest_init creation leaf.
pub fn enable() -> ! {
    crate::phases::shutdown_on_error(
        require_state(LifecycleEvent::Enable, State::Ready, State::Online),
        "arceos_ex boot init enable start failed\n",
    );
    if !boot_task_online_and_canonical()
        || !crate::phases::boot::entry_prelude::is_online()
        || !crate::phases::boot::is_online()
        || !crate::phases::interrupt::is_online()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Enable, State::Ready, State::Online),
            "arceos_ex boot init enable dependency failed\n",
        );
    }
    rest_init::preset(crate::context::context())
}

/// Continues BootInitFlow.Enable with the pre-commit handoff leaf.
pub fn enable_after_boot_init_rest_init() -> ! {
    if crate::phases::state::load(&BOOT_INIT_FLOW_STATE) != State::Ready
        || !boot_task_online_and_canonical()
        || !rest_init::boot_init_rest_init_is_online()
    {
        crate::arch::riscv64::sbi::putstr("arceos_ex boot init rest-init invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
    rest_init::setup(crate::context::context())
}

/// Commits BootInitFlow.Online at the last reversible boundary.
pub fn enable_after_boot_init_schedule_handoff() -> ! {
    let result = if boot_task_online_and_canonical()
        && rest_init::boot_init_rest_init_is_online()
        && rest_init::boot_init_schedule_handoff_is_online()
        && rest_init::precommit_ready()
    {
        commit_state(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::BootInitFlowOnline,
        )
    } else {
        phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init enable failed\n");
    crate::systems::kernel::switch_after_boot_init()
}

/// Runs only if a later scheduler switch restores the original BootTask stack.
pub fn boot_task_restored() -> ! {
    rest_init::enable(crate::context::context())
}

pub fn is_online() -> bool {
    crate::phases::state::load(&BOOT_INIT_FLOW_STATE) == State::Online
        && boot_task_online_and_canonical()
        && rest_init::boot_init_rest_init_is_online()
        && rest_init::boot_init_schedule_handoff_is_online()
}

pub fn is_prepared() -> bool {
    crate::phases::state::load(&BOOT_INIT_FLOW_STATE) == State::Prepared
        && boot_task_online_and_canonical()
        && crate::phases::boot::entry_prelude::is_online()
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&BOOT_INIT_FLOW_STATE) == State::Ready
        && boot_task_online_and_canonical()
        && crate::phases::boot::entry_prelude::is_online()
        && crate::phases::boot::is_online()
        && crate::phases::interrupt::is_online()
}

fn boot_task_online_and_canonical() -> bool {
    let ctx = crate::context::context_ref();
    ctx.boot_task.state() == State::Online
        && ctx.boot_task.task_ref() == TaskRef::BOOT
        && ctx.boot_task.task().task_ref() == TaskRef::BOOT
        && ctx.boot_task.pid() == 0
}

fn require_state(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    let actual = crate::phases::state::load(&BOOT_INIT_FLOW_STATE);
    if actual != expected {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn commit_state(
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    crate::phases::state::mark_checked(&BOOT_INIT_FLOW_STATE, event, expected, target, checkpoint)
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&BOOT_INIT_FLOW_STATE),
        expected,
        target,
    )
}
