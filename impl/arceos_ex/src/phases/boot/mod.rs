pub mod core_prepare;
pub mod entry_prelude;
pub mod entry_successor;
pub mod mm_core_init;
pub mod sched_init;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static BOOT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn adopt_head_preset_start() -> EventResult {
    let state = crate::phases::state::load(&BOOT_PHASE_STATE);
    if state != State::Base || crate::arch::riscv64::csr::supervisor_interrupts_enabled() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    Ok(())
}

pub fn preset_after_entry_prelude() -> ! {
    let result = if entry_prelude::is_online() {
        commit_state(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::BootPhasePrepared,
        )
    } else {
        phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot preset event failed\n");
    setup()
}

fn setup() -> ! {
    crate::phases::shutdown_on_error(
        require_state(LifecycleEvent::Setup, State::Prepared, State::Ready),
        "arceos_ex boot setup start failed\n",
    );
    entry_successor::preset(crate::context::context())
}

pub fn setup_after_entry_successor() -> ! {
    let result = if entry_successor::is_online() {
        commit_state(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootPhaseReady,
        )
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot setup event failed\n");
    enable()
}

fn enable() -> ! {
    crate::phases::shutdown_on_error(
        require_state(LifecycleEvent::Enable, State::Ready, State::Online),
        "arceos_ex boot enable start failed\n",
    );
    core_prepare::preset(crate::context::context())
}

pub fn enable_after_core_prepare() -> ! {
    if !core_prepare::is_online()
        || require_state(LifecycleEvent::Enable, State::Ready, State::Online).is_err()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Enable, State::Ready, State::Online),
            "arceos_ex boot core prepare continuation failed\n",
        );
    }
    mm_core_init::preset(crate::context::context())
}

pub fn enable_after_mm_core_init() -> ! {
    if !core_prepare::is_online()
        || !mm_core_init::is_online()
        || require_state(LifecycleEvent::Enable, State::Ready, State::Online).is_err()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Enable, State::Ready, State::Online),
            "arceos_ex boot mm core init continuation failed\n",
        );
    }
    sched_init::preset(crate::context::context())
}

pub fn enable_after_sched_init() -> ! {
    let result =
        if core_prepare::is_online() && mm_core_init::is_online() && sched_init::is_online() {
            commit_state(
                LifecycleEvent::Enable,
                State::Ready,
                State::Online,
                Checkpoint::BootPhaseOnline,
            )
        } else {
            phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
        };
    crate::phases::shutdown_on_error(result, "arceos_ex boot enable event failed\n");
    crate::systems::kernel::preset_after_boot()
}

pub fn is_online() -> bool {
    crate::phases::state::load(&BOOT_PHASE_STATE) == State::Online
        && entry_prelude::is_online()
        && entry_successor::is_online()
        && core_prepare::is_online()
        && mm_core_init::is_online()
        && sched_init::is_online()
}

fn require_state(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    let actual = crate::phases::state::load(&BOOT_PHASE_STATE);
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
    crate::phases::state::mark_checked(&BOOT_PHASE_STATE, event, expected, target, checkpoint)
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&BOOT_PHASE_STATE),
        expected,
        target,
    )
}
