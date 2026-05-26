use core::sync::atomic::{AtomicU8, Ordering};

use crate::{
    objects::state::{EventResult, LifecycleEvent, State},
    trace::{self, Checkpoint},
};

const BASE: u8 = 0;
const PREPARED: u8 = 1;
const READY: u8 = 2;
const ONLINE: u8 = 3;
const DESTROYED: u8 = 4;

pub const fn encode(state: State) -> u8 {
    match state {
        State::Base => BASE,
        State::Prepared => PREPARED,
        State::Ready => READY,
        State::Online => ONLINE,
        State::Destroyed => DESTROYED,
    }
}

pub fn load(state: &AtomicU8) -> State {
    match state.load(Ordering::Relaxed) {
        BASE => State::Base,
        PREPARED => State::Prepared,
        READY => State::Ready,
        ONLINE => State::Online,
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
        return EventResult::failed_condition(event, actual, expected, target);
    }

    state.store(encode(target), Ordering::Relaxed);
    trace::checkpoint(checkpoint);
    EventResult::Success
}
