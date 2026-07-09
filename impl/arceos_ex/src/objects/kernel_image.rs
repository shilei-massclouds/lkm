use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    config::Config,
    lds::Lds,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};

pub struct KernelImage {
    lifecycle: Lifecycle,
    phys_start: usize,
    virt_start: usize,
    virt_offset: usize,
}

impl KernelImage {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            phys_start: 0,
            virt_start: 0,
            virt_offset: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn phys_start(&self) -> usize {
        self.phys_start
    }

    pub const fn virt_start(&self) -> usize {
        self.virt_start
    }

    pub const fn virt_offset(&self) -> usize {
        self.virt_offset
    }

    pub fn link_to_phys(&self, addr: usize) -> Option<usize> {
        addr.checked_sub(self.virt_offset)
    }

    pub fn phys_to_link(&self, addr: usize) -> Option<usize> {
        addr.checked_add(self.virt_offset)
    }

    pub fn runtime_to_phys(&self, addr: usize) -> Option<usize> {
        if addr >= self.virt_start {
            self.link_to_phys(addr)
        } else {
            Some(addr)
        }
    }

    pub fn runtime_to_link(&self, addr: usize) -> Option<usize> {
        if addr >= self.virt_start {
            Some(addr)
        } else {
            self.phys_to_link(addr)
        }
    }

    pub fn adopt_head_preset(&mut self, config: &Config, lds: &Lds) -> EventResult {
        let runtime_start = lds.kernel_start();
        let virt_start = config.kernel_link_addr();
        let Some(virt_offset) = virt_start.checked_sub(runtime_start) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        self.phys_start = runtime_start;
        self.virt_start = virt_start;
        self.virt_offset = virt_offset;

        if lds.state() != State::Online
            || !lds.entry_layout_ready()
            || runtime_start == 0
            || runtime_start >= virt_start
            || !runtime_start.is_multiple_of(config.pmd_size())
            || lds.current_global_pointer(self) != Some(csr::read_gp())
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn adopt_head_setup(&mut self, lds: &Lds) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !lds.bss_zeroed(self) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self, lds: &Lds) -> EventResult {
        if self.lifecycle.state() != State::Ready || csr::read_gp() != lds.global_pointer() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KernelImageOnline,
        )
    }
}
