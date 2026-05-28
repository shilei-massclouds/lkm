use super::{
    fdt::FdtFacts,
    kernel_cmdline::KernelCmdline,
    memblock::MemBlock,
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct CommandLine {
    lifecycle: Lifecycle,
}

impl CommandLine {
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
        raw_dtb: &RawDtb,
        facts: &FdtFacts,
        kernel_cmdline: &mut KernelCmdline,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || raw_dtb.state() != State::Ready
            || kernel_cmdline.state() != State::Base
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        kernel_cmdline.preset(raw_dtb, facts)?;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CommandLinePrepared,
        )
    }

    pub fn setup(
        &mut self,
        kernel_cmdline: &KernelCmdline,
        saved_command_line: &mut SavedCommandLine,
        static_command_line: &mut StaticCommandLine,
        memblock: &MemBlock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || kernel_cmdline.state() != State::Ready
            || saved_command_line.state() != State::Base
            || static_command_line.state() != State::Base
            || memblock.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        saved_command_line.setup(kernel_cmdline, memblock)?;
        static_command_line.setup(kernel_cmdline, saved_command_line, memblock)?;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::CommandLineReady,
        )
    }
}

pub struct SavedCommandLine {
    lifecycle: Lifecycle,
}

impl SavedCommandLine {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, kernel_cmdline: &KernelCmdline, memblock: &MemBlock) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_cmdline.state() != State::Ready
            || memblock.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SavedCommandLineReady,
        )
    }
}

pub struct StaticCommandLine {
    lifecycle: Lifecycle,
}

impl StaticCommandLine {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(
        &mut self,
        kernel_cmdline: &KernelCmdline,
        saved_command_line: &SavedCommandLine,
        memblock: &MemBlock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_cmdline.state() != State::Ready
            || saved_command_line.state() != State::Ready
            || memblock.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::StaticCommandLineReady,
        )
    }
}
