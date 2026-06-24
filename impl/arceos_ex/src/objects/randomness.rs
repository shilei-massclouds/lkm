use super::{
    command_line::StaticCommandLine,
    cpu_group::CpuGroup,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct Randomness {
    lifecycle: Lifecycle,
    early_mix: u64,
    arch_entropy_bits: u16,
    fully_ready: bool,
}

impl Randomness {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            early_mix: 0,
            arch_entropy_bits: 0,
            fully_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn early_mix(&self) -> u64 {
        self.early_mix
    }

    #[allow(dead_code)]
    pub const fn arch_entropy_bits(&self) -> u16 {
        self.arch_entropy_bits
    }

    #[allow(dead_code)]
    pub const fn is_fully_ready(&self) -> bool {
        self.fully_ready
    }

    pub fn preset(&mut self, static_command_line: &StaticCommandLine) -> EventResult {
        if self.lifecycle.state() != State::Base || static_command_line.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.arch_entropy_bits = arch_entropy_bits();
        self.early_mix =
            mix_early_seed_material(static_command_line.as_bytes(), self.arch_entropy_bits);
        self.fully_ready = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::RandomnessPrepared,
        )
    }

    pub fn setup(&mut self, cpu_group: &CpuGroup, time_seed: u64) -> EventResult {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        };
        if self.lifecycle.state() != State::Prepared || cpu_group.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.early_mix = mix_bytes(self.early_mix, &time_seed.to_le_bytes());
        self.early_mix = mix_bytes(self.early_mix, &boot_cpu.hartid().to_le_bytes());
        self.fully_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::RandomnessReady,
        )
    }
}

fn arch_entropy_bits() -> u16 {
    0
}

fn mix_early_seed_material(command_line: &[u8], arch_entropy_bits: u16) -> u64 {
    let mut mix = 0xcbf2_9ce4_8422_2325u64;

    mix = mix_bytes(mix, b"arceos_ex.random_init_early");
    mix = mix_bytes(mix, &arch_entropy_bits.to_le_bytes());
    mix_bytes(mix, command_line)
}

fn mix_bytes(mut mix: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        mix ^= u64::from(*byte);
        mix = mix.wrapping_mul(0x0000_0100_0000_01b3);
    }
    mix
}
