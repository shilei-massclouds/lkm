use crate::{arch::riscv64::sbi, objects::printk};

use super::state::{EventResult, State};

static mut EARLY_CON: EarlyCon = EarlyCon::new();

pub struct EarlyCon {
    state: State,
}

impl EarlyCon {
    pub const fn new() -> Self {
        Self { state: State::Base }
    }

    pub fn preset(&mut self) -> EventResult {
        if self.state != State::Base {
            return EventResult::Failed;
        }
        self.state = State::Prepared;
        EventResult::Success
    }

    pub fn setup(&mut self) -> EventResult {
        if self.state != State::Prepared {
            return EventResult::Blocked;
        }
        self.state = State::Ready;
        EventResult::Success
    }

    pub fn enable(&mut self) -> EventResult {
        if self.state != State::Ready {
            return EventResult::Blocked;
        }
        self.state = State::Online;
        EventResult::Success
    }
}

pub fn preset() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().preset() }
}

pub fn setup() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().setup() }
}

pub fn enable() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().enable() }
}

pub fn drain_printk() {
    printk::drain_to(sbi::putchar);
}
