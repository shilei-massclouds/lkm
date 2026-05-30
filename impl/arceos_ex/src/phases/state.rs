use core::sync::atomic::{AtomicU8, Ordering};

use crate::{
    objects::state::{failed_condition, EventResult, LifecycleEvent, State},
    trace::{self, Checkpoint},
};

const BASE: u8 = 0;
const PREPARED: u8 = 1;
const READY: u8 = 2;
const ONLINE: u8 = 3;
const OFFLINE: u8 = 4;
const DESTROYED: u8 = 5;

pub const fn encode(state: State) -> u8 {
    match state {
        State::Base => BASE,
        State::Prepared => PREPARED,
        State::Ready => READY,
        State::Online => ONLINE,
        State::Offline => OFFLINE,
        State::Destroyed => DESTROYED,
    }
}

pub fn load(state: &AtomicU8) -> State {
    match state.load(Ordering::Relaxed) {
        BASE => State::Base,
        PREPARED => State::Prepared,
        READY => State::Ready,
        ONLINE => State::Online,
        OFFLINE => State::Offline,
        DESTROYED => State::Destroyed,
        _ => State::Destroyed,
    }
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
    trace::checkpoint(checkpoint);
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
