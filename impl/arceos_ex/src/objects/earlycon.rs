use crate::{arch::riscv64::sbi, objects::printk};

use super::state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition};
use crate::checkpoint::Checkpoint;

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

    pub fn preset(&mut self, earlycon_sbi_config: bool) -> EventResult {
        if !earlycon_sbi_config {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EarlyConPrepared,
        )
    }

    pub fn setup(&mut self, sbi_ready: bool) -> EventResult {
        if !sbi_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::EarlyConReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        let result = self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::EarlyConOnline,
        );
        if result.is_ok() {
            printk::register_boot_console();
            printk::drain_to(sbi::putchar);
        }
        result
    }

    pub fn disable_after_handoff(&mut self) -> EventResult {
        match self.lifecycle.state() {
            State::Online => self.lifecycle.transition(
                LifecycleEvent::Disable,
                State::Online,
                State::Offline,
                Checkpoint::EarlyConOffline,
            ),
            State::Offline => Ok(()),
            state => failed_condition(
                LifecycleEvent::Disable,
                state,
                State::Online,
                State::Offline,
            ),
        }
    }
}

#[allow(dead_code)]
pub fn preset(earlycon_sbi_config: bool) -> EventResult {
    unsafe {
        (&raw mut EARLY_CON)
            .as_mut()
            .unwrap()
            .preset(earlycon_sbi_config)
    }
}

#[allow(dead_code)]
pub fn setup(sbi_ready: bool) -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().setup(sbi_ready) }
}

#[allow(dead_code)]
pub fn enable() -> EventResult {
    unsafe { (&raw mut EARLY_CON).as_mut().unwrap().enable() }
}

#[allow(dead_code)]
pub fn disable_after_handoff() -> EventResult {
    unsafe {
        (&raw mut EARLY_CON)
            .as_mut()
            .unwrap()
            .disable_after_handoff()
    }
}

#[allow(dead_code)]
pub fn drain_printk() {
    if !is_online() || !printk::earlycon_drain_allowed() {
        panic!("earlycon drain after console handoff");
    }
    printk::drain_to(sbi::putchar);
}

pub fn is_online() -> bool {
    unsafe { (&raw const EARLY_CON).as_ref().unwrap().lifecycle.state() == State::Online }
}
