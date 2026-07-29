use crate::checkpoint::Checkpoint;

use super::{
    config::Config,
    raw_dtb::{PhysRange, RawDtb},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct FixMapSlot {
    page_size: usize,
    page_count: usize,
    virt_start: usize,
    mapped_range: PhysRange,
}

impl FixMapSlot {
    pub const fn new(page_size: usize, page_count: usize, virt_start: usize) -> Self {
        Self {
            page_size,
            page_count,
            virt_start,
            mapped_range: PhysRange::empty(),
        }
    }

    const fn empty() -> Self {
        Self {
            page_size: 0,
            page_count: 0,
            virt_start: 0,
            mapped_range: PhysRange::empty(),
        }
    }

    pub const fn page_size(self) -> usize {
        self.page_size
    }

    pub const fn virt_start(self) -> usize {
        self.virt_start
    }

    pub fn virt_end(self) -> Option<usize> {
        self.page_count
            .checked_mul(self.page_size)
            .and_then(|bytes| self.virt_start.checked_add(bytes))
    }

    pub fn virt_for_phys(self, phys: usize) -> Option<usize> {
        if self.page_size == 0
            || !self.page_size.is_power_of_two()
            || phys < self.mapped_range.start()
            || phys >= self.mapped_range.end()
        {
            return None;
        }

        let page_offset = self.mapped_range.start() & (self.page_size - 1);
        let mapped_phys_base = self.mapped_range.start() - page_offset;
        let delta = phys.checked_sub(mapped_phys_base)?;
        self.virt_start.checked_add(delta)
    }

    fn can_contain(self, range: PhysRange) -> bool {
        if self.page_size == 0 || !self.page_size.is_power_of_two() {
            return false;
        }
        let Some(bytes) = range.end().checked_sub(range.start()) else {
            return false;
        };
        if bytes == 0 {
            return false;
        }
        page_cover_count(range.start(), bytes, self.page_size) <= self.page_count
    }

    fn contains(self, range: PhysRange) -> bool {
        self.mapped_range == range && self.can_contain(range)
    }

    fn map(&mut self, range: PhysRange) {
        self.mapped_range = range;
    }
}

pub struct FixMap {
    lifecycle: Lifecycle,
    fdt_slot: FixMapSlot,
}

impl FixMap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fdt_slot: FixMapSlot::empty(),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fdt_slot(&self) -> FixMapSlot {
        self.fdt_slot
    }

    pub fn contains_raw_dtb(&self, raw_dtb: &RawDtb) -> bool {
        self.fdt_slot.contains(raw_dtb.range())
    }

    pub fn preset(&mut self, config: &Config, raw_dtb: &RawDtb) -> EventResult {
        let fdt_slot = config.fixmap().fdt();
        if config.state() != State::Online
            || raw_dtb.state() != State::Ready
            || config.page_size() == 0
            || fdt_slot.page_size() != config.page_size()
            || !fdt_slot.can_contain(raw_dtb.range())
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.fdt_slot = fdt_slot;
        self.fdt_slot.map(raw_dtb.range());
        if !self.fdt_slot.contains(raw_dtb.range()) {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::FixMapReady,
        )
    }
}

fn page_cover_count(start: usize, bytes: usize, page_size: usize) -> usize {
    let page_offset = start & (page_size - 1);
    let Some(covered_bytes) = bytes.checked_add(page_offset) else {
        return usize::MAX;
    };
    covered_bytes.div_ceil(page_size)
}
