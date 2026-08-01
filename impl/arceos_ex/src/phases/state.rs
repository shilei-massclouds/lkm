use core::sync::atomic::{AtomicU8, Ordering};

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};

const BASE: u8 = 0;
const PREPARED: u8 = 1;
const READY: u8 = 2;
const ONLINE: u8 = 3;
const OFFLINE: u8 = 4;
const DESTROYED: u8 = 5;
const ON_CPU: u8 = 6;
const SUSPENDED: u8 = 7;

pub const fn encode(state: State) -> u8 {
    match state {
        State::Base => BASE,
        State::Prepared => PREPARED,
        State::Ready => READY,
        State::Online => ONLINE,
        State::Offline => OFFLINE,
        State::Destroyed => DESTROYED,
        State::OnCpu => ON_CPU,
        State::Suspended => SUSPENDED,
    }
}

pub const fn decode(value: u8) -> State {
    match value {
        BASE => State::Base,
        PREPARED => State::Prepared,
        READY => State::Ready,
        ONLINE => State::Online,
        OFFLINE => State::Offline,
        DESTROYED => State::Destroyed,
        ON_CPU => State::OnCpu,
        SUSPENDED => State::Suspended,
        _ => State::Destroyed,
    }
}

pub fn load(state: &AtomicU8) -> State {
    decode(state.load(Ordering::Relaxed))
}

pub fn mark(
    state: &AtomicU8,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let actual = load(state);
    if actual != expected {
        return failed_condition(event, actual, expected, target);
    }

    state.store(encode(target), Ordering::Relaxed);
    crate::checkpoint::checkpoint(checkpoint);
    Ok(())
}

pub fn mark_checked(
    state: &AtomicU8,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    adopt(state, event, expected, target)?;

    let actual = load(state);
    if actual != target {
        return failed_condition(event, actual, target, target);
    }

    crate::checkpoint::checkpoint(checkpoint);
    Ok(())
}

pub fn adopt(
    state: &AtomicU8,
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    let actual = load(state);
    if actual != expected {
        return failed_condition(event, actual, expected, target);
    }

    state.store(encode(target), Ordering::Relaxed);
    Ok(())
}
