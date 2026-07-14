use super::{
    memblock::MemBlock,
    raw_dtb::PhysRange,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    vm::Vm,
};
use crate::checkpoint::Checkpoint;

const DMA32_LIMIT: usize = 0x1_0000_0000;
const ZONE_KIND_COUNT: usize = 3;
const MIGRATION_TYPE_COUNT: usize = 3;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ZoneKind {
    Dma32,
    Normal,
    Movable,
}

impl ZoneKind {
    const fn index(self) -> usize {
        match self {
            Self::Dma32 => 0,
            Self::Normal => 1,
            Self::Movable => 2,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum MigrationType {
    Unmovable,
    Movable,
    Reclaimable,
}

#[derive(Clone, Copy)]
pub struct Zone {
    kind: ZoneKind,
    range: PhysRange,
    present_bytes: usize,
    migration_types: [MigrationType; MIGRATION_TYPE_COUNT],
    free_page_set_empty: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Zone {
    const fn empty(kind: ZoneKind) -> Self {
        Self {
            kind,
            range: PhysRange::empty(),
            present_bytes: 0,
            migration_types: [
                MigrationType::Unmovable,
                MigrationType::Movable,
                MigrationType::Reclaimable,
            ],
            free_page_set_empty: true,
        }
    }

    pub const fn kind(&self) -> ZoneKind {
        self.kind
    }

    pub const fn range(&self) -> PhysRange {
        self.range
    }

    pub const fn present_bytes(&self) -> usize {
        self.present_bytes
    }

    pub const fn is_empty(&self) -> bool {
        self.present_bytes == 0
    }

    pub const fn migration_type_count(&self) -> usize {
        self.migration_types.len()
    }

    pub fn has_migration_type(&self, migration_type: MigrationType) -> bool {
        self.migration_types.contains(&migration_type)
    }

    pub const fn free_page_set_empty(&self) -> bool {
        self.free_page_set_empty
    }

    fn add_range(&mut self, range: PhysRange) -> bool {
        if range.start() >= range.end() {
            return true;
        }

        if self.present_bytes == 0 {
            self.range = range;
        } else {
            self.range = PhysRange::new(
                self.range.start().min(range.start()),
                self.range.end().max(range.end()),
            );
        }

        let Some(present_bytes) = self.present_bytes.checked_add(range.size()) else {
            return false;
        };
        self.present_bytes = present_bytes;
        true
    }
}

pub struct Zones {
    lifecycle: Lifecycle,
    zones: [Zone; ZONE_KIND_COUNT],
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Zones {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            zones: [
                Zone::empty(ZoneKind::Dma32),
                Zone::empty(ZoneKind::Normal),
                Zone::empty(ZoneKind::Movable),
            ],
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn zone(&self, kind: ZoneKind) -> Option<&Zone> {
        self.zones.get(kind.index())
    }

    pub fn setup(&mut self, memblock: &MemBlock, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Base
            || memblock.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let Some(zones) = build_zones(memblock) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        self.zones = zones;

        if !zones_ready(&self.zones, memblock) {
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
            Checkpoint::ZonesReady,
        )
    }
}

fn build_zones(memblock: &MemBlock) -> Option<[Zone; ZONE_KIND_COUNT]> {
    let mut zones = [
        Zone::empty(ZoneKind::Dma32),
        Zone::empty(ZoneKind::Normal),
        Zone::empty(ZoneKind::Movable),
    ];
    let ranges = memblock.usable_ranges();
    let mut index = 0usize;

    while index < ranges.count() {
        let range = ranges.get(index)?;
        if range.start() < DMA32_LIMIT {
            let dma32_end = range.end().min(DMA32_LIMIT);
            if dma32_end > range.start()
                && !zones[ZoneKind::Dma32.index()]
                    .add_range(PhysRange::new(range.start(), dma32_end))
            {
                return None;
            }
        }
        if range.end() > DMA32_LIMIT {
            let normal_start = range.start().max(DMA32_LIMIT);
            if !zones[ZoneKind::Normal.index()].add_range(PhysRange::new(normal_start, range.end()))
            {
                return None;
            }
        }
        index += 1;
    }

    Some(zones)
}

fn zones_ready(zones: &[Zone; ZONE_KIND_COUNT], memblock: &MemBlock) -> bool {
    if zones[ZoneKind::Dma32.index()].kind() != ZoneKind::Dma32
        || zones[ZoneKind::Normal.index()].kind() != ZoneKind::Normal
        || zones[ZoneKind::Movable.index()].kind() != ZoneKind::Movable
    {
        return false;
    }

    if !zones[ZoneKind::Movable.index()].is_empty() {
        return false;
    }

    let mut zone_bytes = 0usize;
    for zone in zones {
        if zone.migration_type_count() != MIGRATION_TYPE_COUNT || !zone.free_page_set_empty() {
            return false;
        }
        let Some(next) = zone_bytes.checked_add(zone.present_bytes()) else {
            return false;
        };
        zone_bytes = next;
    }

    let mut memblock_bytes = 0usize;
    let ranges = memblock.usable_ranges();
    let mut index = 0usize;
    while index < ranges.count() {
        let Some(range) = ranges.get(index) else {
            return false;
        };
        let Some(next) = memblock_bytes.checked_add(range.size()) else {
            return false;
        };
        memblock_bytes = next;
        index += 1;
    }

    zone_bytes == memblock_bytes
}
