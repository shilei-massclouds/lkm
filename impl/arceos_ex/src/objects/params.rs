use super::{
    boot_param::BootParam,
    command_line::{CommandLine, StaticCommandLine},
    early_param::EarlyParam,
    kernel_cmdline::KernelCmdline,
    payload_param::PayloadParam,
    printk,
    sbi::Sbi,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

pub struct Params {
    lifecycle: Lifecycle,
}

impl Params {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn preset(
        &mut self,
        command_line: &CommandLine,
        kernel_cmdline: &KernelCmdline,
        sbi: &Sbi,
        early_param: &mut EarlyParam,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || command_line.state() != State::Prepared
            || kernel_cmdline.state() != State::Ready
            || sbi.state() != State::Ready
            || !printk::is_prepared()
            || early_param.state() != State::Base
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        early_param.setup(command_line, kernel_cmdline, sbi)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(
        &mut self,
        early_param: &EarlyParam,
        static_command_line: &StaticCommandLine,
        boot_param: &mut BootParam,
        payload_param: &mut PayloadParam,
        after_boot_param: fn(&BootParam) -> EventResult,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || early_param.state() != State::Ready
            || static_command_line.state() != State::Ready
            || boot_param.state() != State::Base
            || payload_param.state() != State::Base
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        boot_param.setup(early_param, static_command_line)?;
        after_boot_param(boot_param)?;
        payload_param.setup(boot_param, static_command_line)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }
}
