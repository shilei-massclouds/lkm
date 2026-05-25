use crate::{arch::riscv64::sbi, objects::printk};

use super::state::{EventResult, Lifecycle, LifecycleEvent, State};
use crate::trace::Checkpoint;

#[allow(dead_code)]
static mut EARLY_CON: EarlyCon = EarlyCon::new();

#[allow(dead_code)]
pub struct EarlyCon {
    lifecycle: Lifecycle,
}

#[allow(dead_code)]
impl EarlyCon {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn preset(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EarlyConPrepared,
        )
    }

    pub fn setup(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::EarlyConReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::EarlyConOnline,
        )
    }
}

#[allow(dead_code)]
pub fn preset() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().preset() }
}

#[allow(dead_code)]
pub fn setup() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().setup() }
}

#[allow(dead_code)]
pub fn enable() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().enable() }
}

#[allow(dead_code)]
pub fn drain_printk() {
    printk::drain_to(sbi::putchar);
}
