use super::{
    kernel_cmdline::KernelCmdline,
    memblock::MemBlock,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

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
