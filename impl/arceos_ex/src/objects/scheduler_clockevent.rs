//! Per-CPU 10 ms scheduler clockevent state.

#![cfg_attr(not(app_smoke), allow(dead_code))]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::cpu::MAX_CPUS;

pub const SCHEDULER_SLICE_MILLISECONDS: u64 = 10;

struct CpuSchedulerClockevent {
    slice_ticks: AtomicU64,
    deadline: AtomicU64,
    need_resched: AtomicBool,
    slice_begin_count: AtomicU64,
    interrupt_count: AtomicU64,
    missed_period_count: AtomicU64,
    identity_renewal_count: AtomicU64,
}

impl CpuSchedulerClockevent {
    const fn new() -> Self {
        Self {
            slice_ticks: AtomicU64::new(0),
            deadline: AtomicU64::new(0),
            need_resched: AtomicBool::new(false),
            slice_begin_count: AtomicU64::new(0),
            interrupt_count: AtomicU64::new(0),
            missed_period_count: AtomicU64::new(0),
            identity_renewal_count: AtomicU64::new(0),
        }
    }
}

static CLOCKEVENTS: [CpuSchedulerClockevent; MAX_CPUS] =
    [const { CpuSchedulerClockevent::new() }; MAX_CPUS];

pub fn setup_cpu(logical_id: usize, timebase_hz: u64) -> bool {
    let Some(clockevent) = CLOCKEVENTS.get(logical_id) else {
        return false;
    };
    let slice_ticks = timebase_hz / (1000 / SCHEDULER_SLICE_MILLISECONDS);
    if slice_ticks == 0 {
        return false;
    }
    match clockevent.slice_ticks.compare_exchange(
        0,
        slice_ticks,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => true,
        Err(existing) => existing == slice_ticks,
    }
}

pub fn begin_slice(logical_id: usize, now: u64) -> Option<u64> {
    let clockevent = CLOCKEVENTS.get(logical_id)?;
    let slice_ticks = clockevent.slice_ticks.load(Ordering::Acquire);
    if slice_ticks == 0 {
        return None;
    }
    clockevent.need_resched.store(false, Ordering::Release);
    let deadline = now.checked_add(slice_ticks)?;
    clockevent.deadline.store(deadline, Ordering::Release);
    clockevent.slice_begin_count.fetch_add(1, Ordering::Relaxed);
    Some(deadline)
}

pub fn next_deadline(logical_id: usize) -> Option<u64> {
    let deadline = CLOCKEVENTS
        .get(logical_id)?
        .deadline
        .load(Ordering::Acquire);
    (deadline != 0).then_some(deadline)
}

/// Acknowledge only the scheduler side of the clockevent mux. The hardirq
/// records a pending reschedule and advances from the previous deadline; it
/// never invokes the scheduler.
pub fn handle_timer(logical_id: usize, now: u64) -> bool {
    let Some(clockevent) = CLOCKEVENTS.get(logical_id) else {
        return false;
    };
    let deadline = clockevent.deadline.load(Ordering::Acquire);
    let slice_ticks = clockevent.slice_ticks.load(Ordering::Acquire);
    if deadline == 0 || slice_ticks == 0 || now < deadline {
        return false;
    }
    let elapsed = now - deadline;
    let periods = elapsed / slice_ticks + 1;
    let Some(next_deadline) = deadline.checked_add(periods.saturating_mul(slice_ticks)) else {
        return false;
    };
    clockevent.deadline.store(next_deadline, Ordering::Release);
    clockevent.need_resched.store(true, Ordering::Release);
    clockevent.interrupt_count.fetch_add(1, Ordering::Relaxed);
    clockevent
        .missed_period_count
        .fetch_add(periods.saturating_sub(1), Ordering::Relaxed);
    true
}

pub fn need_resched_pending(logical_id: usize) -> bool {
    CLOCKEVENTS
        .get(logical_id)
        .is_some_and(|clockevent| clockevent.need_resched.load(Ordering::Acquire))
}

/// Consume a tick at a user-return boundary when no other runnable Task can
/// use the slice. The already advanced deadline remains the renewed slice.
pub fn renew_identity_slice(logical_id: usize) -> bool {
    let Some(clockevent) = CLOCKEVENTS.get(logical_id) else {
        return false;
    };
    if !clockevent.need_resched.swap(false, Ordering::AcqRel) {
        return false;
    }
    clockevent
        .identity_renewal_count
        .fetch_add(1, Ordering::Relaxed);
    true
}

#[allow(dead_code)]
pub fn take_need_resched(logical_id: usize) -> bool {
    CLOCKEVENTS
        .get(logical_id)
        .is_some_and(|clockevent| clockevent.need_resched.swap(false, Ordering::AcqRel))
}

pub fn counters(logical_id: usize) -> Option<(u64, u64, u64, u64)> {
    let clockevent = CLOCKEVENTS.get(logical_id)?;
    Some((
        clockevent.interrupt_count.load(Ordering::Acquire),
        clockevent.missed_period_count.load(Ordering::Acquire),
        clockevent.identity_renewal_count.load(Ordering::Acquire),
        clockevent.slice_begin_count.load(Ordering::Acquire),
    ))
}
