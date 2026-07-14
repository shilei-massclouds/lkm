use super::{
    command_line::StaticCommandLine,
    early_param::EarlyParam,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

#[derive(Clone, Copy)]
pub struct BootParamSummary {
    pub boot_arg_count: usize,
    pub unknown_count: usize,
    pub payload_boundary: Option<usize>,
    pub init_value: Option<(usize, usize)>,
}

pub struct BootParam {
    lifecycle: Lifecycle,
    summary: BootParamSummary,
}

impl BootParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            summary: BootParamSummary {
                boot_arg_count: 0,
                unknown_count: 0,
                payload_boundary: None,
                init_value: None,
            },
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn boot_arg_count(&self) -> usize {
        self.summary.boot_arg_count
    }

    pub const fn unknown_count(&self) -> usize {
        self.summary.unknown_count
    }

    pub const fn payload_boundary(&self) -> Option<usize> {
        self.summary.payload_boundary
    }

    pub const fn init_value_range(&self) -> Option<(usize, usize)> {
        self.summary.init_value
    }

    pub fn init_value<'a>(&self, cmdline: &'a [u8]) -> Option<&'a [u8]> {
        let (start, end) = self.summary.init_value?;
        if start <= end && end <= cmdline.len() {
            Some(&cmdline[start..end])
        } else {
            None
        }
    }

    pub fn boot_arg<'a>(&self, cmdline: &'a [u8], index: usize) -> Option<&'a [u8]> {
        let mut found = 0usize;
        let mut cursor = 0usize;
        while let Some((token_start, token_end)) = next_token(cmdline, cursor) {
            if bytes_eq(&cmdline[token_start..token_end], b"--") {
                return None;
            }
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
        early_param: &EarlyParam,
        static_command_line: &StaticCommandLine,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || early_param.state() != State::Ready
            || static_command_line.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.summary = parse_boot_params(static_command_line.as_bytes());
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootParamReady,
        )
    }
}

fn parse_boot_params(cmdline: &[u8]) -> BootParamSummary {
    let mut summary = BootParamSummary {
        boot_arg_count: 0,
        unknown_count: 0,
        payload_boundary: None,
        init_value: None,
    };

    let mut cursor = 0usize;
    while let Some((token_start, token_end)) = next_token(cmdline, cursor) {
        if bytes_eq(&cmdline[token_start..token_end], b"--") {
            summary.payload_boundary = Some(token_end);
            break;
        }

        summary.boot_arg_count += 1;
        if token_starts_with(&cmdline[token_start..token_end], b"init=") {
            summary.init_value = Some((token_start + b"init=".len(), token_end));
        }
        if !is_known_boot_param(&cmdline[token_start..token_end]) {
            summary.unknown_count += 1;
        }
        cursor = token_end;
    }

    summary
}

pub fn payload_arg_count_after_boundary(cmdline: &[u8], boundary: Option<usize>) -> usize {
    let Some(boundary) = boundary else {
        return 0;
    };

    let mut count = 0usize;
    let mut cursor = boundary;
    while let Some((_token_start, token_end)) = next_token(cmdline, cursor) {
        count += 1;
        cursor = token_end;
    }
    count
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

fn is_known_boot_param(token: &[u8]) -> bool {
    token_starts_with(token, b"earlycon=")
        || token_starts_with(token, b"console=")
        || token_starts_with(token, b"root=")
        || token_starts_with(token, b"init=")
        || token_starts_with(token, b"rdinit=")
        || bytes_eq(token, b"ro")
        || bytes_eq(token, b"rw")
        || bytes_eq(token, b"quiet")
        || bytes_eq(token, b"debug")
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
