use super::{
    cpu::{CpuRole, CpuView},
    cpu_group::CpuGroup,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

const MAX_CPUS: usize = 16;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuIdMapEntryKind {
    BootCpu,
    SecondaryCpu,
}

#[derive(Clone, Copy)]
pub struct CpuIdMapEntry {
    logical_id: usize,
    hartid: usize,
    kind: CpuIdMapEntryKind,
}

impl CpuIdMapEntry {
    const fn empty() -> Self {
        Self {
            logical_id: usize::MAX,
            hartid: usize::MAX,
            kind: CpuIdMapEntryKind::BootCpu,
        }
    }

    pub const fn logical_id(self) -> usize {
        self.logical_id
    }

    pub const fn hartid(self) -> usize {
        self.hartid
    }

    pub const fn kind(self) -> CpuIdMapEntryKind {
        self.kind
    }
}

pub struct CpuIdMap {
    lifecycle: Lifecycle,
    entries: [CpuIdMapEntry; MAX_CPUS],
    count: usize,
}

impl CpuIdMap {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            entries: [CpuIdMapEntry::empty(); MAX_CPUS],
            count: 0,
        }
    }

    pub fn preset(&mut self, cpu_group: &CpuGroup) -> EventResult {
        let Some(boot_cpu) = cpu_group.cpu(0) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        if self.lifecycle.state() != State::Base
            || boot_cpu.role() != CpuRole::Boot
            || boot_cpu.logical_id() != 0
            || boot_cpu.hartid() == usize::MAX
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.entries = [CpuIdMapEntry::empty(); MAX_CPUS];
        self.entries[0] = entry_from_cpu(boot_cpu);
        self.count = 1;

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CpuIdMapPrepared,
        )
    }

    pub fn setup(&mut self, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || cpu_group.state() != State::Ready
            || !cpu_group.logical_id_index_ready()
            || !cpu_group.boot_cpu_index_zero()
            || self.entry(0).map(|entry| entry.hartid()) != cpu_group.cpu(0).map(|cpu| cpu.hartid())
            || self.entry(0).map(|entry| entry.kind()) != Some(CpuIdMapEntryKind::BootCpu)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        let mut entries = [CpuIdMapEntry::empty(); MAX_CPUS];
        entries[0] = self.entries[0];
        let mut count = 1usize;

        let mut logical_id = 1usize;
        while logical_id < cpu_group.possible_cpu_count() {
            let Some(cpu) = cpu_group.cpu(logical_id) else {
                return self.failed_setup();
            };
            if count >= MAX_CPUS || contains_hartid(&entries, count, cpu.hartid()) {
                return self.failed_setup();
            }
            entries[count] = entry_from_cpu(cpu);
            count += 1;
            logical_id += 1;
        }

        self.entries = entries;
        self.count = count;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::CpuIdMapReady,
        )
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub fn entry(&self, logical_id: usize) -> Option<CpuIdMapEntry> {
        if logical_id < self.count && self.entries[logical_id].logical_id == logical_id {
            Some(self.entries[logical_id])
        } else {
            None
        }
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

fn entry_from_cpu(cpu: CpuView) -> CpuIdMapEntry {
    CpuIdMapEntry {
        logical_id: cpu.logical_id(),
        hartid: cpu.hartid(),
        kind: match cpu.role() {
            CpuRole::Boot => CpuIdMapEntryKind::BootCpu,
            CpuRole::Secondary => CpuIdMapEntryKind::SecondaryCpu,
        },
    }
}

fn contains_hartid(entries: &[CpuIdMapEntry; MAX_CPUS], count: usize, hartid: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if entries[index].hartid == hartid {
            return true;
        }
        index += 1;
    }
    false
}
