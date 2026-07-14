use super::{
    command_line::StaticCommandLine,
    cpu_group::CpuGroup,
    irq_time::Timekeeper,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct Randomness {
    lifecycle: Lifecycle,
    early_mix: u64,
    arch_entropy_bits: u16,
    early_mix_without_input_pool_lock: bool,
    early_conditional_reseed_deferred: bool,
    fully_ready: bool,
    timekeeping_required: bool,
    cycle_entropy_mixed: bool,
    input_pool_lock_deferred: bool,
    base_crng_lock_deferred: bool,
    pm_notifier_deferred: bool,
}

impl Randomness {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            early_mix: 0,
            arch_entropy_bits: 0,
            early_mix_without_input_pool_lock: false,
            early_conditional_reseed_deferred: false,
            fully_ready: false,
            timekeeping_required: false,
            cycle_entropy_mixed: false,
            input_pool_lock_deferred: false,
            base_crng_lock_deferred: false,
            pm_notifier_deferred: false,
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

    pub const fn early_mix_without_input_pool_lock(&self) -> bool {
        self.early_mix_without_input_pool_lock
    }

    pub const fn early_conditional_reseed_deferred(&self) -> bool {
        self.early_conditional_reseed_deferred
    }

    #[allow(dead_code)]
    pub const fn is_fully_ready(&self) -> bool {
        self.fully_ready
    }

    pub const fn timekeeping_required(&self) -> bool {
        self.timekeeping_required
    }

    pub const fn cycle_entropy_mixed(&self) -> bool {
        self.cycle_entropy_mixed
    }

    pub const fn input_pool_lock_deferred(&self) -> bool {
        self.input_pool_lock_deferred
    }

    pub const fn base_crng_lock_deferred(&self) -> bool {
        self.base_crng_lock_deferred
    }

    pub const fn pm_notifier_deferred(&self) -> bool {
        self.pm_notifier_deferred
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
        self.early_mix_without_input_pool_lock = true;
        self.early_conditional_reseed_deferred = true;
        self.base_crng_lock_deferred = true;
        self.fully_ready = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::RandomnessPrepared,
        )
    }

    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        timekeeper: &Timekeeper,
        time_seed: u64,
    ) -> EventResult {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        };
        if self.lifecycle.state() != State::Prepared
            || cpu_group.state() != State::Ready
            || timekeeper.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.early_mix = mix_bytes(self.early_mix, &time_seed.to_le_bytes());
        self.early_mix = mix_bytes(self.early_mix, &boot_cpu.hartid().to_le_bytes());
        self.timekeeping_required = true;
        self.cycle_entropy_mixed = true;
        self.input_pool_lock_deferred = true;
        self.base_crng_lock_deferred = true;
        self.pm_notifier_deferred = true;
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
