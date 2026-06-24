use super::{
    cpu::{CpuRef, CpuRole, CpuView, MAX_CPUS},
    cpu_group::CpuGroup,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuIdMapEntryKind {
    BootCpu,
    SecondaryCpu,
}

#[derive(Clone, Copy)]
pub struct CpuIdMapEntry {
    cpu_ref: CpuRef,
    logical_id: usize,
    hartid: usize,
    kind: CpuIdMapEntryKind,
}

impl CpuIdMapEntry {
    const fn empty() -> Self {
        Self {
            cpu_ref: CpuRef::invalid(),
            logical_id: usize::MAX,
            hartid: usize::MAX,
            kind: CpuIdMapEntryKind::BootCpu,
        }
    }

    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
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
            || cpu_group.cpu_ref_at(0) != Some(boot_cpu.cpu_ref())
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.entries = [CpuIdMapEntry::empty(); MAX_CPUS];
        self.entries[0] = entry_from_cpu_ref(boot_cpu.cpu_ref(), boot_cpu);
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
            || self.entry(0).map(|entry| entry.cpu_ref()) != cpu_group.cpu_ref_at(0)
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
        let mut count = 0usize;

        let mut logical_id = 0usize;
        while logical_id < cpu_group.possible_cpu_count() {
            let Some(cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
                return self.failed_setup();
            };
            let Some(cpu) = cpu_group.cpu(logical_id) else {
                return self.failed_setup();
            };
            if cpu.cpu_ref() != cpu_ref
                || count >= MAX_CPUS
                || contains_cpu_ref(&entries, count, cpu_ref)
                || contains_hartid(&entries, count, cpu.hartid())
            {
                return self.failed_setup();
            }
            entries[count] = entry_from_cpu_ref(cpu_ref, cpu);
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
        if logical_id < self.count
            && self.entries[logical_id].logical_id == logical_id
            && self.entries[logical_id].cpu_ref.logical_id() == logical_id
        {
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

fn entry_from_cpu_ref(cpu_ref: CpuRef, cpu: CpuView) -> CpuIdMapEntry {
    CpuIdMapEntry {
        cpu_ref,
        logical_id: cpu.logical_id(),
        hartid: cpu.hartid(),
        kind: match cpu.role() {
            CpuRole::Boot => CpuIdMapEntryKind::BootCpu,
            CpuRole::Secondary => CpuIdMapEntryKind::SecondaryCpu,
        },
    }
}

fn contains_cpu_ref(entries: &[CpuIdMapEntry; MAX_CPUS], count: usize, cpu_ref: CpuRef) -> bool {
    let mut index = 0usize;
    while index < count {
        if entries[index].cpu_ref == cpu_ref {
            return true;
        }
        index += 1;
    }
    false
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
