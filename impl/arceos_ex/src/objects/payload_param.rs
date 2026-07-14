use super::{
    boot_param::{BootParam, payload_arg_count_after_boundary},
    command_line::StaticCommandLine,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct PayloadParam {
    lifecycle: Lifecycle,
    arg_count: usize,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl PayloadParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            arg_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn arg_count(&self) -> usize {
        self.arg_count
    }

    pub fn arg<'a>(
        &self,
        cmdline: &'a [u8],
        boot_param: &BootParam,
        index: usize,
    ) -> Option<&'a [u8]> {
        let mut found = 0usize;
        let mut cursor = boot_param.payload_boundary()?;
        while let Some((token_start, token_end)) = next_token(cmdline, cursor) {
            if found == index {
                return Some(&cmdline[token_start..token_end]);
            }
            found += 1;
            cursor = token_end;
        }
        None
    }

    pub fn setup(
        &mut self,
        boot_param: &BootParam,
        static_command_line: &StaticCommandLine,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || boot_param.state() != State::Ready
            || static_command_line.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.arg_count = payload_arg_count_after_boundary(
            static_command_line.as_bytes(),
            boot_param.payload_boundary(),
        );
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PayloadParamReady,
        )
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
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
