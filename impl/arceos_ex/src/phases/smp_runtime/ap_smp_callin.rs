use core::sync::atomic::{AtomicU8, Ordering};

use crate::{
    checkpoint::Checkpoint,
    objects::{cpu::MAX_CPUS, cpu_group::CpuGroup, state::State},
};

const CALLIN_FACTS_READY: u8 = 1;

static STATES: [AtomicU8; MAX_CPUS] =
    [const { AtomicU8::new(crate::phases::state::encode(State::Base)) }; MAX_CPUS];
static FACTS: [AtomicU8; MAX_CPUS] = [const { AtomicU8::new(0) }; MAX_CPUS];

pub(crate) fn reset_secondary(logical_id: usize) {
    super::smp_bringup::reset_ap_state(&STATES, logical_id);
    if logical_id < MAX_CPUS {
        FACTS[logical_id].store(0, Ordering::Release);
    }
}

pub(crate) fn state_for(logical_id: usize) -> State {
    super::smp_bringup::ap_state_for(&STATES, logical_id)
}

pub(crate) fn all_online(cpu_group: &CpuGroup) -> bool {
    super::smp_bringup::ap_family_all_online(&STATES, cpu_group)
}

pub(crate) fn all_callin_facts(cpu_group: &CpuGroup) -> bool {
    all_online(cpu_group) && all_facts(cpu_group)
}

pub(crate) fn preset(logical_id: usize) -> ! {
    require_state(logical_id, State::Base, "preset-source");
    if !super::smp_bringup::ap_prerequisites_ready()
        || super::ap_entry_prelude::state_for(logical_id) != State::Online
    {
        fail(logical_id, "entry-online");
    }

    crate::checkpoint::ap_checkpoint(Checkpoint::ApSmpCallinPhaseStarted, logical_id);
    FACTS[logical_id].store(CALLIN_FACTS_READY, Ordering::Release);
    crate::checkpoint::ap_checkpoint(Checkpoint::ApSmpCallinCpuRunningProduced, logical_id);
    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Base,
        State::Prepared,
        Checkpoint::ApSmpCallinPhasePrepared,
        "ApSmpCallinPhase",
    );
    setup(logical_id)
}

fn setup(logical_id: usize) -> ! {
    require_state(logical_id, State::Prepared, "setup-source");
    require_facts(logical_id, "callin-facts");
    if super::ap_entry_prelude::state_for(logical_id) != State::Online {
        fail(logical_id, "entry-online");
    }

    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Prepared,
        State::Ready,
        Checkpoint::ApSmpCallinPhaseReady,
        "ApSmpCallinPhase",
    );
    enable(logical_id)
}

fn enable(logical_id: usize) -> ! {
    require_state(logical_id, State::Ready, "enable-source");
    require_facts(logical_id, "ready-invariant");

    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Ready,
        State::Online,
        Checkpoint::ApSmpCallinPhaseOnline,
        "ApSmpCallinPhase",
    );
    super::smp_bringup::ap_after_smp_callin(logical_id)
}

fn require_state(logical_id: usize, expected: State, check: &'static str) {
    if state_for(logical_id) != expected {
        fail(logical_id, check);
    }
}

fn require_facts(logical_id: usize, check: &'static str) {
    if logical_id >= MAX_CPUS || FACTS[logical_id].load(Ordering::Acquire) != CALLIN_FACTS_READY {
        fail(logical_id, check);
    }
}

fn all_facts(cpu_group: &CpuGroup) -> bool {
    let mut logical_id = 1usize;
    while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
        if FACTS[logical_id].load(Ordering::Acquire) != CALLIN_FACTS_READY {
            return false;
        }
        logical_id += 1;
    }
    cpu_group.secondary_count() != 0 && cpu_group.secondary_count() < MAX_CPUS
}

fn fail(logical_id: usize, check: &'static str) -> ! {
    super::smp_bringup::ap_phase_fail_stop(
        "ApSmpCallinPhase",
        logical_id,
        state_for(logical_id),
        check,
    )
}
