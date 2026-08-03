use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use crate::{
    checkpoint::Checkpoint,
    objects::{cpu::MAX_CPUS, cpu_group::CpuGroup, state::State},
};

const ONLINE_IDLE_FACTS_READY: u8 = 1;

static STATES: [AtomicU8; MAX_CPUS] =
    [const { AtomicU8::new(crate::phases::state::encode(State::Base)) }; MAX_CPUS];
static FACTS: [AtomicU8; MAX_CPUS] = [const { AtomicU8::new(0) }; MAX_CPUS];
static PARK_LOOP_ENTERED: [AtomicU8; MAX_CPUS] = [const { AtomicU8::new(0) }; MAX_CPUS];
static IDLE_LOOP_READY: [AtomicU8; MAX_CPUS] = [const { AtomicU8::new(0) }; MAX_CPUS];
static WFI_ENTERED: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static WFI_WOKEN: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

pub(crate) fn reset_secondary(logical_id: usize) {
    super::smp_bringup::reset_ap_state(&STATES, logical_id);
    if logical_id < MAX_CPUS {
        FACTS[logical_id].store(0, Ordering::Release);
        PARK_LOOP_ENTERED[logical_id].store(0, Ordering::Release);
        IDLE_LOOP_READY[logical_id].store(0, Ordering::Release);
        WFI_ENTERED[logical_id].store(0, Ordering::Release);
        WFI_WOKEN[logical_id].store(0, Ordering::Release);
    }
}

pub(crate) fn state_for(logical_id: usize) -> State {
    super::smp_bringup::ap_state_for(&STATES, logical_id)
}

pub(crate) fn all_online(cpu_group: &CpuGroup) -> bool {
    super::smp_bringup::ap_family_all_online(&STATES, cpu_group)
}

pub(crate) fn all_online_idle_facts(cpu_group: &CpuGroup) -> bool {
    all_online(cpu_group) && all_facts(cpu_group)
}

pub(crate) fn all_park_loops_entered(cpu_group: &CpuGroup) -> bool {
    let mut logical_id = 1usize;
    while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
        if !park_loop_entered_for(logical_id) {
            return false;
        }
        logical_id += 1;
    }
    cpu_group.secondary_count() != 0 && cpu_group.secondary_count() < MAX_CPUS
}

pub(crate) fn park_loop_entered_for(logical_id: usize) -> bool {
    logical_id < MAX_CPUS && PARK_LOOP_ENTERED[logical_id].load(Ordering::Acquire) != 0
}

pub(crate) fn mark_park_loop_entered(logical_id: usize) {
    require_state(logical_id, State::Online, "park-source");
    PARK_LOOP_ENTERED[logical_id].store(1, Ordering::Release);
}

pub(crate) fn mark_idle_loop_ready(logical_id: usize) {
    require_state(logical_id, State::Online, "idle-loop-source");
    IDLE_LOOP_READY[logical_id].store(1, Ordering::Release);
}

pub(crate) fn record_wfi_enter(logical_id: usize) {
    if let Some(count) = WFI_ENTERED.get(logical_id) {
        count.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn record_wfi_wake(logical_id: usize) {
    if let Some(count) = WFI_WOKEN.get(logical_id) {
        count.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(app_smoke)]
pub(crate) fn idle_runtime_observation(logical_id: usize) -> Option<(bool, u64, u64)> {
    Some((
        IDLE_LOOP_READY.get(logical_id)?.load(Ordering::Acquire) != 0,
        WFI_ENTERED.get(logical_id)?.load(Ordering::Acquire),
        WFI_WOKEN.get(logical_id)?.load(Ordering::Acquire),
    ))
}

pub(crate) fn preset(logical_id: usize) -> ! {
    require_state(logical_id, State::Base, "preset-source");
    if !super::smp_bringup::ap_prerequisites_ready()
        || super::ap_smp_callin::state_for(logical_id) != State::Online
    {
        fail(logical_id, "callin-online");
    }

    crate::checkpoint::ap_checkpoint(Checkpoint::ApOnlineIdlePhaseStarted, logical_id);
    FACTS[logical_id].store(ONLINE_IDLE_FACTS_READY, Ordering::Release);
    crate::checkpoint::ap_checkpoint(Checkpoint::ApOnlineIdleDoneUpProduced, logical_id);
    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Base,
        State::Prepared,
        Checkpoint::ApOnlineIdlePhasePrepared,
        "ApOnlineIdlePhase",
    );
    setup(logical_id)
}

fn setup(logical_id: usize) -> ! {
    require_state(logical_id, State::Prepared, "setup-source");
    require_facts(logical_id, "online-idle-facts");
    if super::ap_smp_callin::state_for(logical_id) != State::Online {
        fail(logical_id, "callin-online");
    }

    super::smp_bringup::transition_ap_state(
        &STATES,
        logical_id,
        State::Prepared,
        State::Ready,
        Checkpoint::ApOnlineIdlePhaseReady,
        "ApOnlineIdlePhase",
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
        Checkpoint::ApOnlineIdlePhaseOnline,
        "ApOnlineIdlePhase",
    );
    super::smp_bringup::ap_after_online_idle(logical_id)
}

fn require_state(logical_id: usize, expected: State, check: &'static str) {
    if state_for(logical_id) != expected {
        fail(logical_id, check);
    }
}

fn require_facts(logical_id: usize, check: &'static str) {
    if logical_id >= MAX_CPUS
        || FACTS[logical_id].load(Ordering::Acquire) != ONLINE_IDLE_FACTS_READY
    {
        fail(logical_id, check);
    }
}

fn all_facts(cpu_group: &CpuGroup) -> bool {
    let mut logical_id = 1usize;
    while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
        if FACTS[logical_id].load(Ordering::Acquire) != ONLINE_IDLE_FACTS_READY {
            return false;
        }
        logical_id += 1;
    }
    cpu_group.secondary_count() != 0 && cpu_group.secondary_count() < MAX_CPUS
}

fn fail(logical_id: usize, check: &'static str) -> ! {
    super::smp_bringup::ap_phase_fail_stop(
        "ApOnlineIdlePhase",
        logical_id,
        state_for(logical_id),
        check,
    )
}
