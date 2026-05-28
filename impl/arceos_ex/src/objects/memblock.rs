use super::{
    config::Config,
    early_dtb::EarlyDtb,
    fdt::PhysRangeSet,
    kernel_image::KernelImage,
    lds::Lds,
    physical_memory::PhysicalMemory,
    raw_dtb::{PhysRange, RawDtb},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct MemBlock {
    lifecycle: Lifecycle,
    usable: PhysRangeSet,
    reserved: PhysRangeSet,
    #[allow(dead_code)]
    alloc_cursor: usize,
}

impl MemBlock {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            usable: PhysRangeSet::empty(),
            reserved: PhysRangeSet::empty(),
            alloc_cursor: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn usable_ranges(&self) -> &PhysRangeSet {
        &self.usable
    }

    #[allow(dead_code)]
    pub fn alloc_phys(&mut self, size: usize, align: usize) -> Option<PhysRange> {
        if self.lifecycle.state() != State::Online
            || size == 0
            || align == 0
            || !align.is_power_of_two()
        {
            return None;
        }

        let mut range_index = 0;
        while range_index < self.usable.count() {
            let range = self.usable.get(range_index)?;
            let mut start = align_up(range.start().max(self.alloc_cursor), align)?;
            loop {
                let end = start.checked_add(size)?;
                if end > range.end() {
                    break;
                }

                let candidate = PhysRange::new(start, end);
                if !self.reserved.overlaps_range(candidate) {
                    self.alloc_cursor = end;
                    if self.reserved.push(candidate) {
                        return Some(candidate);
                    }
                    return None;
                }
                start = align_up(end, align)?;
            }
            range_index += 1;
        }

        None
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
        let Some(kernel_start) = kernel_image.runtime_to_phys(lds.kernel_start()) else {
            return self.failed_setup();
        };
        let Some(kernel_end) = kernel_image.runtime_to_phys(lds.kernel_end()) else {
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

        self.reserved = early_dtb.reserved_ranges();
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

#[allow(dead_code)]
fn align_up(value: usize, align: usize) -> Option<usize> {
    Some(value.checked_add(align - 1)? & !(align - 1))
}
