use super::{
    fdt::BootCommandLine,
    fdt::FdtFacts,
    kernel_cmdline::KernelCmdline,
    memblock::MemBlock,
    raw_dtb::RawDtb,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

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
    cmdline: BootCommandLine,
}

impl SavedCommandLine {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cmdline: BootCommandLine::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn has_token(&self, token: &[u8]) -> bool {
        cmdline_has_token(self.cmdline.as_bytes(), token)
    }

    pub fn has_token_prefix(&self, prefix: &[u8]) -> bool {
        cmdline_has_token_prefix(self.cmdline.as_bytes(), prefix)
    }

    pub fn root_value_is(&self, value: &[u8]) -> bool {
        cmdline_token_value_is(self.cmdline.as_bytes(), b"root=", value)
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

        self.cmdline = kernel_cmdline.cmdline();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SavedCommandLineReady,
        )
    }
}

fn cmdline_has_token(cmdline: &[u8], token: &[u8]) -> bool {
    if token.is_empty() {
        return false;
    }

    let mut cursor = 0usize;
    while let Some((start, end)) = next_token(cmdline, cursor) {
        if bytes_eq(&cmdline[start..end], token) {
            return true;
        }
        cursor = end;
    }
    false
}

fn cmdline_has_token_prefix(cmdline: &[u8], prefix: &[u8]) -> bool {
    if prefix.is_empty() {
        return false;
    }

    let mut cursor = 0usize;
    while let Some((start, end)) = next_token(cmdline, cursor) {
        if token_starts_with(&cmdline[start..end], prefix) {
            return true;
        }
        cursor = end;
    }
    false
}

fn cmdline_token_value_is(cmdline: &[u8], prefix: &[u8], value: &[u8]) -> bool {
    if prefix.is_empty() {
        return false;
    }

    let mut cursor = 0usize;
    while let Some((start, end)) = next_token(cmdline, cursor) {
        let token = &cmdline[start..end];
        if token_starts_with(token, prefix) && bytes_eq(&token[prefix.len()..], value) {
            return true;
        }
        cursor = end;
    }
    false
}

fn next_token(cmdline: &[u8], mut cursor: usize) -> Option<(usize, usize)> {
    while cursor < cmdline.len() && cmdline[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= cmdline.len() {
        return None;
    }

    let start = cursor;
    while cursor < cmdline.len() && !cmdline[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    Some((start, cursor))
}

fn token_starts_with(token: &[u8], prefix: &[u8]) -> bool {
    if token.len() < prefix.len() {
        return false;
    }
    bytes_eq(&token[..prefix.len()], prefix)
}

fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut index = 0usize;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

pub struct StaticCommandLine {
    lifecycle: Lifecycle,
    cmdline: BootCommandLine,
}

impl StaticCommandLine {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cmdline: BootCommandLine::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.cmdline.as_bytes()
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

        self.cmdline = kernel_cmdline.cmdline();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::StaticCommandLineReady,
        )
    }
}
