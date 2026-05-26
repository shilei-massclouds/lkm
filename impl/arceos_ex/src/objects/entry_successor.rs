use super::{
    config::Config,
    earlycon,
    entry_prelude::{KernelImage, Lds},
    fdt::{self, BootCommandLine, FdtFacts, HartSet, PhysRangeSet},
    fix_map::FixMap,
    printk,
    raw_dtb::{PhysRange, RawDtb},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct PlatformCpuInfo {
    lifecycle: Lifecycle,
    harts: HartSet,
}

impl PlatformCpuInfo {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            harts: HartSet::empty(),
        }
    }

    pub fn contains(&self, hartid: usize) -> bool {
        self.lifecycle.state() == State::Online && self.harts.contains(hartid)
    }

    pub fn preset(
        &mut self,
        raw_dtb: &RawDtb,
        facts: &FdtFacts,
        boot_hartid: usize,
    ) -> EventResult {
        if raw_dtb.state() != State::Ready
            || facts.harts.count() == 0
            || !facts.harts.contains(boot_hartid)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.harts = facts.harts;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::PlatformCpuInfoReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::PlatformCpuInfoOnline,
        )
    }
}

pub struct PhysicalMemory {
    lifecycle: Lifecycle,
    ram: PhysRangeSet,
}

impl PhysicalMemory {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ram: PhysRangeSet::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ram(&self) -> &PhysRangeSet {
        &self.ram
    }

    pub fn preset(&mut self, raw_dtb: &RawDtb, facts: &FdtFacts) -> EventResult {
        if raw_dtb.state() != State::Ready || facts.memory.count() == 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.ram = facts.memory;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::PhysicalMemoryReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::PhysicalMemoryOnline,
        )
    }
}

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

pub struct CpuIdMap {
    lifecycle: Lifecycle,
    boot_hartid: usize,
}

impl CpuIdMap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_hartid: usize::MAX,
        }
    }

    pub fn preset(&mut self, boot_hartid: usize) -> EventResult {
        self.boot_hartid = boot_hartid;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::CpuIdMapReady,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct KernelCmdline {
    lifecycle: Lifecycle,
    cmdline: BootCommandLine,
}

impl KernelCmdline {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cmdline: BootCommandLine::empty(),
        }
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn has_earlycon_sbi(&self) -> bool {
        self.cmdline.contains(b"earlycon=sbi")
    }

    pub fn preset(&mut self, raw_dtb: &RawDtb, facts: &FdtFacts) -> EventResult {
        if raw_dtb.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.cmdline = facts.cmdline;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::KernelCmdlineReady,
        )
    }
}

pub struct InitMm {
    lifecycle: Lifecycle,
    kernel_start: usize,
    kernel_end: usize,
}

impl InitMm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kernel_start: 0,
            kernel_end: 0,
        }
    }

    pub fn setup(&mut self, lds: &Lds) -> EventResult {
        if lds.state() != State::Online || lds.kernel_start() >= lds.kernel_end() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.kernel_start = lds.kernel_start();
        self.kernel_end = lds.kernel_end();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::InitMmReady,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct EarlyIoremap {
    lifecycle: Lifecycle,
}

impl EarlyIoremap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn setup(&mut self, fix_map: &FixMap) -> EventResult {
        if fix_map.state() != State::Ready {
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
            Checkpoint::EarlyIoremapReady,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct Sbi {
    lifecycle: Lifecycle,
}

impl Sbi {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SbiReady,
        )
    }
}

pub struct KernelParam {
    lifecycle: Lifecycle,
}

impl KernelParam {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, kernel_cmdline: &KernelCmdline, sbi: &Sbi) -> EventResult {
        if kernel_cmdline.state() != State::Ready
            || sbi.state() != State::Ready
            || !printk::is_prepared()
            || !kernel_cmdline.has_earlycon_sbi()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let result = earlycon::preset(kernel_cmdline.has_earlycon_sbi());
        if result.is_err() {
            return result;
        }
        let result = earlycon::setup(sbi.state() == State::Ready);
        if result.is_err() {
            return result;
        }
        let result = earlycon::enable();
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KernelParamReady,
        )
    }
}

pub struct MemBlock {
    lifecycle: Lifecycle,
    usable: PhysRangeSet,
    reserved: PhysRangeSet,
}

impl MemBlock {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            usable: PhysRangeSet::empty(),
            reserved: PhysRangeSet::empty(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn usable_ranges(&self) -> &PhysRangeSet {
        &self.usable
    }

    pub fn preset(
        &mut self,
        early_dtb: &EarlyDtb,
        physical_memory: &PhysicalMemory,
        raw_dtb: &RawDtb,
    ) -> EventResult {
        if early_dtb.state() != State::Prepared
            || physical_memory.state() != State::Online
            || raw_dtb.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.usable = *physical_memory.ram();
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::MemBlockPrepared,
        )
    }

    pub fn setup(
        &mut self,
        early_dtb: &EarlyDtb,
        kernel_image: &KernelImage,
        raw_dtb: &RawDtb,
        config: &Config,
        lds: &Lds,
        physical_memory: &PhysicalMemory,
    ) -> EventResult {
        let Some(kernel_start) = config.runtime_to_phys(lds.kernel_start()) else {
            return self.failed_setup();
        };
        let Some(kernel_end) = config.runtime_to_phys(lds.kernel_end()) else {
            return self.failed_setup();
        };
        if early_dtb.state() != State::Ready
            || self.lifecycle.state() != State::Prepared
            || kernel_image.state() != State::Online
            || raw_dtb.state() != State::Ready
            || config.state() != State::Online
            || physical_memory.state() != State::Online
            || !self
                .usable
                .contains_range(PhysRange::new(kernel_start, kernel_end))
            || !self.usable.contains_range(raw_dtb.range())
        {
            return self.failed_setup();
        }

        self.reserved = early_dtb.facts.reserved;
        if !self.reserved.push(PhysRange::new(kernel_start, kernel_end))
            || !self.reserved.push(raw_dtb.range())
        {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::MemBlockReady,
        )
    }

    pub fn enable(&mut self, vm_state: State) -> EventResult {
        if vm_state != State::Online {
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
            Checkpoint::MemBlockOnline,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }
}
