use super::{
    fdt::{self, BootCommandLine, FdtFacts, HartSet, PhysRangeSet},
    fix_map::FixMap,
    kernel_cmdline::KernelCmdline,
    kernel_param::KernelParam,
    memblock::MemBlock,
    physical_memory::PhysicalMemory,
    platform_cpu_info::PlatformCpuInfo,
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct EarlyDtb {
    lifecycle: Lifecycle,
    facts: FdtFacts,
}

impl EarlyDtb {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            facts: FdtFacts {
                harts: HartSet::empty(),
                memory: PhysRangeSet::empty(),
                reserved: PhysRangeSet::empty(),
                cmdline: BootCommandLine::empty(),
            },
        }
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn reserved_ranges(&self) -> PhysRangeSet {
        self.facts.reserved
    }

    pub fn preset(
        &mut self,
        raw_dtb: &RawDtb,
        fix_map: &FixMap,
        boot_hartid: usize,
        platform_cpu_info: &mut PlatformCpuInfo,
        physical_memory: &mut PhysicalMemory,
    ) -> EventResult {
        if raw_dtb.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let Some(facts) = fdt::parse(raw_dtb, fix_map) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        self.facts = facts;

        let result = platform_cpu_info.preset(raw_dtb, &self.facts, boot_hartid);
        if result.is_err() {
            return result;
        }
        let result = platform_cpu_info.enable();
        if result.is_err() {
            return result;
        }

        let result = physical_memory.preset(raw_dtb, &self.facts);
        if result.is_err() {
            return result;
        }
        let result = physical_memory.enable();
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EarlyDtbPrepared,
        )
    }

    pub fn setup(
        &mut self,
        raw_dtb: &RawDtb,
        memblock: &mut MemBlock,
        kernel_cmdline: &mut KernelCmdline,
        physical_memory: &PhysicalMemory,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || raw_dtb.state() != State::Ready
            || physical_memory.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        let result = memblock.preset(self, physical_memory, raw_dtb);
        if result.is_err() {
            return result;
        }

        let result = kernel_cmdline.preset(raw_dtb, &self.facts);
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::EarlyDtbReady,
        )
    }

    pub fn cleanup(&mut self, memblock: &MemBlock, kernel_param: &KernelParam) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || memblock.state() != State::Online
            || kernel_param.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Cleanup,
                self.lifecycle.state(),
                State::Ready,
                State::Destroyed,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Cleanup,
            State::Ready,
            State::Destroyed,
            Checkpoint::EarlyDtbDestroyed,
        )
    }
}
