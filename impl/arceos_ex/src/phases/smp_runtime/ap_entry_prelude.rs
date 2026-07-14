use core::sync::atomic::{AtomicU8, Ordering};

use crate::{
    checkpoint::Checkpoint,
    objects::{cpu::MAX_CPUS, cpu_group::CpuGroup, smp_bringup::ApEntryAdoption, state::State},
};

const BOOT_DATA_VERIFIED: u8 = 1 << 0;
const STACK_VERIFIED: u8 = 1 << 1;
const TASK_POINTER_VERIFIED: u8 = 1 << 2;
const ALL_ADOPTION_FACTS: u8 = BOOT_DATA_VERIFIED | STACK_VERIFIED | TASK_POINTER_VERIFIED;

static STATES: [AtomicU8; MAX_CPUS] =
    [const { AtomicU8::new(crate::phases::state::encode(State::Base)) }; MAX_CPUS];
static ADOPTION_FACTS: [AtomicU8; MAX_CPUS] = [const { AtomicU8::new(0) }; MAX_CPUS];

pub(crate) fn reset_secondary(logical_id: usize) {
    super::smp_bringup::reset_ap_state(&STATES, logical_id);
    if logical_id < MAX_CPUS {
        ADOPTION_FACTS[logical_id].store(0, Ordering::Release);
    }
}

pub(crate) fn state_for(logical_id: usize) -> State {
    super::smp_bringup::ap_state_for(&STATES, logical_id)
}

pub(crate) fn all_online(cpu_group: &CpuGroup) -> bool {
    super::smp_bringup::ap_family_all_online(&STATES, cpu_group)
}

pub(crate) fn all_adoption_facts(cpu_group: &CpuGroup) -> bool {
    all_online(cpu_group) && all_facts(cpu_group, ALL_ADOPTION_FACTS)
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn all_boot_data_verified(cpu_group: &CpuGroup) -> bool {
    all_online(cpu_group) && all_facts(cpu_group, BOOT_DATA_VERIFIED)
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn all_stacks_verified(cpu_group: &CpuGroup) -> bool {
    all_online(cpu_group) && all_facts(cpu_group, STACK_VERIFIED)
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn all_task_pointers_verified(cpu_group: &CpuGroup) -> bool {
    all_online(cpu_group) && all_facts(cpu_group, TASK_POINTER_VERIFIED)
}

pub(crate) fn preset(adoption: ApEntryAdoption) -> ! {
    let logical_id = adoption.logical_id();
    require_state(logical_id, State::Base, "preset-source");
    require_prerequisites(logical_id);
    crate::checkpoint::ap_checkpoint(Checkpoint::ApEntryPreludePhaseStarted, logical_id);

    if !adoption.boot_data_matches_target() {
        fail(logical_id, "boot-data-target");
    }
    if !adoption.stack_matches_target() {
        fail(logical_id, "stack-pointer");
    }
    if !adoption.task_pointer_matches_target() {
        fail(logical_id, "task-pointer");
    }

    ADOPTION_FACTS[logical_id].store(ALL_ADOPTION_FACTS, Ordering::Release);
    crate::checkpoint::ap_checkpoint(Checkpoint::ApEntryPreludeBootDataConsumed, logical_id);
    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Base,
        State::Prepared,
        Checkpoint::ApEntryPreludePhasePrepared,
        "ApEntryPreludePhase",
    );
    setup(logical_id)
}

fn setup(logical_id: usize) -> ! {
    require_state(logical_id, State::Prepared, "setup-source");
    if facts_for(logical_id) != ALL_ADOPTION_FACTS {
        fail(logical_id, "adoption-facts");
    }

    crate::checkpoint::ap_checkpoint(
        Checkpoint::ApEntryPreludeCurrentStackEstablished,
        logical_id,
    );
    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Prepared,
        State::Ready,
        Checkpoint::ApEntryPreludePhaseReady,
        "ApEntryPreludePhase",
    );
    enable(logical_id)
}

fn enable(logical_id: usize) -> ! {
    require_state(logical_id, State::Ready, "enable-source");
    if facts_for(logical_id) != ALL_ADOPTION_FACTS {
        fail(logical_id, "ready-invariant");
    }

    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Ready,
        State::Online,
        Checkpoint::ApEntryPreludePhaseOnline,
        "ApEntryPreludePhase",
    );
    super::smp_bringup::ap_after_entry_prelude(logical_id)
}

fn require_prerequisites(logical_id: usize) {
    if !super::smp_bringup::ap_prerequisites_ready() {
        fail(logical_id, "bp-prerequisites");
    }
}

fn require_state(logical_id: usize, expected: State, check: &'static str) {
    if state_for(logical_id) != expected {
        fail(logical_id, check);
    }
}

fn facts_for(logical_id: usize) -> u8 {
    if logical_id >= MAX_CPUS {
        return 0;
    }
    ADOPTION_FACTS[logical_id].load(Ordering::Acquire)
}

fn all_facts(cpu_group: &CpuGroup, mask: u8) -> bool {
    let mut logical_id = 1usize;
    while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
        if facts_for(logical_id) & mask != mask {
            return false;
        }
        logical_id += 1;
    }
    cpu_group.secondary_count() != 0 && cpu_group.secondary_count() < MAX_CPUS
}

fn fail(logical_id: usize, check: &'static str) -> ! {
    super::smp_bringup::ap_phase_fail_stop(
        "ApEntryPreludePhase",
        logical_id,
        state_for(logical_id),
        check,
    )
}
