use super::{
    cpu::{CpuRef, MAX_CPUS},
    cpu_group::{CpuGroup, PossibleCpuInventory},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct DefaultSchedRootDomain {
    lifecycle: Lifecycle,
    covered_cpu_refs: [CpuRef; MAX_CPUS],
    covered_cpu_count: usize,
    smp_topology_deferred: bool,
}

impl DefaultSchedRootDomain {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            covered_cpu_refs: [CpuRef::invalid(); MAX_CPUS],
            covered_cpu_count: 0,
            smp_topology_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn covered_cpu_count(&self) -> usize {
        self.covered_cpu_count
    }

    pub const fn possible_cpu_count(&self) -> usize {
        self.covered_cpu_count
    }

    pub const fn covered_cpu_ref(&self, index: usize) -> Option<CpuRef> {
        if index < self.covered_cpu_count {
            Some(self.covered_cpu_refs[index])
        } else {
            None
        }
    }

    pub fn covers_cpu_ref(&self, cpu_ref: CpuRef) -> bool {
        if self.lifecycle.state() != State::Ready {
            return false;
        }

        let mut index = 0usize;
        while index < self.covered_cpu_count {
            if self.covered_cpu_refs[index] == cpu_ref {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn covers_cpu_group_possible(&self, cpu_group: &CpuGroup) -> bool {
        if self.lifecycle.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !cpu_group.possible_cpu_boundary_ready()
            || self.covered_cpu_count != cpu_group.possible_cpu_count()
        {
            return false;
        }

        let mut logical_id = 0usize;
        while logical_id < cpu_group.possible_cpu_count() {
            let Some(cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
                return false;
            };
            if self.covered_cpu_refs[logical_id] != cpu_ref || !self.covers_cpu_ref(cpu_ref) {
                return false;
            }
            logical_id += 1;
        }
        true
    }

    pub const fn smp_topology_deferred(&self) -> bool {
        self.smp_topology_deferred
    }

    pub(crate) fn setup_inventory(&mut self, inventory: PossibleCpuInventory) -> EventResult {
        if self.lifecycle.state() != State::Base || inventory.count() == 0 {
            return self.failed_setup();
        }

        self.covered_cpu_refs = [CpuRef::invalid(); MAX_CPUS];
        self.covered_cpu_count = 0;
        let mut logical_id = 0usize;
        while logical_id < inventory.count() {
            let Some(cpu_ref) = inventory.cpu_ref(logical_id) else {
                return self.failed_setup();
            };
            if self.covers_duplicate_before(cpu_ref, logical_id) {
                return self.failed_setup();
            }
            self.covered_cpu_refs[logical_id] = cpu_ref;
            self.covered_cpu_count += 1;
            logical_id += 1;
        }
        self.smp_topology_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DefaultSchedRootDomainReady,
        )
    }

    fn covers_duplicate_before(&self, cpu_ref: CpuRef, limit: usize) -> bool {
        let mut index = 0usize;
        while index < limit {
            if self.covered_cpu_refs[index] == cpu_ref {
                return true;
            }
            index += 1;
        }
        false
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}
