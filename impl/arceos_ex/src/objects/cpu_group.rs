use super::{
    boot_args::BootArgs,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

unsafe extern "C" {
    static head_boot_hartid: usize;
}

pub struct BootCpu {
    lifecycle: Lifecycle,
    hartid: usize,
}

impl BootCpu {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hartid: usize::MAX,
        }
    }

    fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        let head_hartid = unsafe { core::ptr::addr_of!(head_boot_hartid).read_volatile() };
        if head_hartid != boot_args.boot_hartid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.hartid = head_hartid;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    fn setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !boot_hartid_valid {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootCpuReady,
        )
    }

    fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::BootCpuOnline,
        )
    }

    fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct CpuGroup {
    lifecycle: Lifecycle,
    boot_cpu: BootCpu,
}

impl CpuGroup {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_cpu: BootCpu::new(),
        }
    }

    pub fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        let result = self.boot_cpu.adopt_head_preset(boot_args);
        if result.is_err() {
            return result;
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn boot_cpu_setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        self.boot_cpu.setup(boot_hartid_valid)
    }

    pub fn boot_cpu_enable(&mut self) -> EventResult {
        self.boot_cpu.enable()
    }

    pub fn boot_hartid(&self) -> usize {
        self.boot_cpu.hartid
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn boot_cpu_state(&self) -> State {
        self.boot_cpu.state()
    }
}
