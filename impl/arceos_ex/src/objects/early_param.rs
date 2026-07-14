use super::{
    command_line::CommandLine,
    earlycon,
    kernel_cmdline::KernelCmdline,
    printk,
    sbi::Sbi,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct EarlyParam {
    lifecycle: Lifecycle,
}

impl EarlyParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(
        &mut self,
        command_line: &CommandLine,
        kernel_cmdline: &KernelCmdline,
        sbi: &Sbi,
    ) -> EventResult {
        if command_line.state() != State::Prepared
            || kernel_cmdline.state() != State::Ready
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

        earlycon::preset(kernel_cmdline.has_earlycon_sbi())?;
        earlycon::setup(sbi.state() == State::Ready)?;
        earlycon::enable()?;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::EarlyParamReady,
        )
    }
}
