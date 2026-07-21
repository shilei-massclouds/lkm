pub mod irq_open_prepare;
pub mod irq_time_init;
pub mod local_irq_enable;
pub mod process_prepare;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static INTERRUPT_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    crate::phases::shutdown_on_error(
        require_state(LifecycleEvent::Preset, State::Base, State::Prepared),
        "arceos_ex interrupt preset start failed\n",
    );
    if !crate::phases::boot::is_online()
        || crate::arch::riscv64::csr::supervisor_interrupts_enabled()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared),
            "arceos_ex interrupt preset dependency failed\n",
        );
    }
    crate::checkpoint::checkpoint(Checkpoint::InterruptPhaseStarted);
    irq_time_init::preset(crate::context::context())
}

pub fn preset_after_irq_time_init() -> ! {
    if !irq_time_init::is_online()
        || require_state(LifecycleEvent::Preset, State::Base, State::Prepared).is_err()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared),
            "arceos_ex interrupt irq time continuation failed\n",
        );
    }
    local_irq_enable::preset(crate::context::context())
}

pub fn preset_after_local_irq_enable() -> ! {
    if !irq_time_init::is_online()
        || !local_irq_enable::is_online()
        || require_state(LifecycleEvent::Preset, State::Base, State::Prepared).is_err()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared),
            "arceos_ex interrupt local irq continuation failed\n",
        );
    }
    irq_open_prepare::preset(crate::context::context())
}

pub fn preset_after_irq_open_prepare() -> ! {
    if !irq_time_init::is_online()
        || !local_irq_enable::is_online()
        || !irq_open_prepare::is_online()
        || require_state(LifecycleEvent::Preset, State::Base, State::Prepared).is_err()
    {
        crate::phases::shutdown_on_error(
            phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared),
            "arceos_ex interrupt irq open continuation failed\n",
        );
    }
    process_prepare::preset(crate::context::context())
}

pub fn preset_after_process_prepare() -> ! {
    let result = if children_online() {
        commit_state(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::InterruptPhasePrepared,
        )
    } else {
        phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex interrupt preset event failed\n");
    setup_after_children()
}

fn setup_after_children() -> ! {
    let result = if children_online() {
        commit_state(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InterruptPhaseReady,
        )
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex interrupt setup event failed\n");
    enable()
}

fn enable() -> ! {
    let result = if children_online() {
        commit_state(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::InterruptPhaseOnline,
        )
    } else {
        phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex interrupt enable event failed\n");
    crate::phases::boot_init::setup_after_interrupt()
}

pub fn is_online() -> bool {
    crate::phases::state::load(&INTERRUPT_PHASE_STATE) == State::Online && children_online()
}

fn children_online() -> bool {
    irq_time_init::is_online()
        && local_irq_enable::is_online()
        && irq_open_prepare::is_online()
        && process_prepare::is_online()
}

fn require_state(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    let actual = crate::phases::state::load(&INTERRUPT_PHASE_STATE);
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
    crate::phases::state::mark_checked(&INTERRUPT_PHASE_STATE, event, expected, target, checkpoint)
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&INTERRUPT_PHASE_STATE),
        expected,
        target,
    )
}
