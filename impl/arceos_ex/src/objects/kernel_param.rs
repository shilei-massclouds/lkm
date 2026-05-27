use super::{
    earlycon,
    kernel_cmdline::KernelCmdline,
    printk,
    sbi::Sbi,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct KernelParam {
    lifecycle: Lifecycle,
}

impl KernelParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, kernel_cmdline: &KernelCmdline, sbi: &Sbi) -> EventResult {
        if kernel_cmdline.state() != State::Ready
            || sbi.state() != State::Ready
            || !printk::is_prepared()
            || !kernel_cmdline.has_earlycon_sbi()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let result = earlycon::preset(kernel_cmdline.has_earlycon_sbi());
        if result.is_err() {
            return result;
        }
        let result = earlycon::setup(sbi.state() == State::Ready);
        if result.is_err() {
            return result;
        }
        let result = earlycon::enable();
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KernelParamReady,
        )
    }
}
