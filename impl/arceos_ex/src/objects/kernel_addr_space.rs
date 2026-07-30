use crate::checkpoint::Checkpoint;

use super::{
    config::Config,
    fix_map::FixMap,
    kernel_image::KernelImage,
    lds::Lds,
    linear_map::LinearMap,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    swapper_vm::SwapperVm,
    user_space_reserve::UserSpaceReserve,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtRange {
    start: usize,
    end: usize,
}

impl VirtRange {
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }

    pub const fn valid(self) -> bool {
        self.start < self.end
    }

    pub const fn disjoint(self, other: Self) -> bool {
        self.end <= other.start || other.end <= self.start
    }
}

pub struct KernelAddrSpace {
    lifecycle: Lifecycle,
    kernel_image: VirtRange,
    fix_map: VirtRange,
    linear_map: LinearMap,
    user_space_reserve: UserSpaceReserve,
    final_swapper_mappings_published: bool,
}

impl KernelAddrSpace {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kernel_image: VirtRange::empty(),
            fix_map: VirtRange::empty(),
            linear_map: LinearMap::new(),
            user_space_reserve: UserSpaceReserve::new(),
            final_swapper_mappings_published: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn kernel_image_range(&self) -> VirtRange {
        self.kernel_image
    }

    #[allow(dead_code)]
    pub const fn fix_map_range(&self) -> VirtRange {
        self.fix_map
    }

    #[allow(dead_code)]
    pub const fn linear_map_range(&self) -> VirtRange {
        self.linear_map.range()
    }

    #[allow(dead_code)]
    pub const fn user_space_reserve_range(&self) -> VirtRange {
        self.user_space_reserve.range()
    }

    pub const fn final_swapper_mappings_published(&self) -> bool {
        self.final_swapper_mappings_published
    }

    pub fn preset(
        &mut self,
        config: &Config,
        lds: &Lds,
        kernel_image: &KernelImage,
    ) -> EventResult {
        let Some((kernel_start, kernel_end)) = kernel_image.loaded_virt_range(lds) else {
            return self.failed(LifecycleEvent::Preset, State::Base, State::Prepared);
        };
        let fdt_slot = config.fixmap().fdt();
        let Some(fixmap_end) = fdt_slot.virt_end() else {
            return self.failed(LifecycleEvent::Preset, State::Base, State::Prepared);
        };
        if self.lifecycle.state() != State::Base
            || config.state() != State::Online
            || lds.state() != State::Online
            || kernel_image.state() != State::Ready
        {
            return self.failed(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        self.linear_map.preset(config)?;
        self.user_space_reserve.preset(config)?;
        self.kernel_image = VirtRange::new(kernel_start, kernel_end);
        self.fix_map = VirtRange::new(fdt_slot.virt_start(), fixmap_end);
        if !self.declared_layout_valid(config) {
            return self.failed(LifecycleEvent::Preset, State::Base, State::Prepared);
        }
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::KernelAddrSpacePrepared,
        )
    }

    pub fn setup(&mut self, config: &Config, fix_map: &FixMap) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || fix_map.state() != State::Ready
            || fix_map.fdt_slot().virt_start() != self.fix_map.start
            || fix_map.fdt_slot().virt_end() != Some(self.fix_map.end)
            || !self.declared_layout_valid(config)
            || !self.regions_disjoint()
        {
            return self.failed(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KernelAddrSpaceReady,
        )
    }

    pub fn enable(&mut self, swapper_vm: &SwapperVm) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || swapper_vm.state() != State::Ready
            || !self.regions_disjoint()
        {
            return self.failed(LifecycleEvent::Enable, State::Ready, State::Online);
        }
        self.final_swapper_mappings_published = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KernelAddrSpaceOnline,
        )
    }

    #[allow(dead_code)]
    pub fn layout_ready(&self) -> bool {
        matches!(self.state(), State::Ready | State::Online) && self.regions_disjoint()
    }

    fn declared_layout_valid(&self, config: &Config) -> bool {
        self.kernel_image.valid()
            && self.fix_map.valid()
            && self.linear_map.state() == State::Ready
            && self.user_space_reserve.state() == State::Ready
            && self.linear_map.range().valid()
            && self.user_space_reserve.range().valid()
            && self.kernel_image.start() == config.kernel_link_addr()
            && self.linear_map.range().start() == config.linear_map_virt_start()
            && self.user_space_reserve.range().start() == 0
            && self.user_space_reserve.range().end() == config.canonical_user_virt_end()
    }

    fn regions_disjoint(&self) -> bool {
        let regions = [
            self.kernel_image,
            self.fix_map,
            self.linear_map.range(),
            self.user_space_reserve.range(),
        ];
        let mut left = 0usize;
        while left < regions.len() {
            let mut right = left + 1;
            while right < regions.len() {
                if !regions[left].disjoint(regions[right]) {
                    return false;
                }
                right += 1;
            }
            left += 1;
        }
        true
    }

    fn failed(&self, event: LifecycleEvent, expected: State, target: State) -> EventResult {
        failed_condition(event, self.lifecycle.state(), expected, target)
    }
}
