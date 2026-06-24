use super::{
    cpu::{CpuRef, CpuRole, CpuView, SecondaryCpuStore, BOOT_CPU_LOGICAL_ID, MAX_CPUS},
    cpu_control::BootCurrentCpu,
    device_tree::DeviceTree,
    fdt_reader::read_cells,
    sbi::Sbi,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub struct CpuGroup {
    lifecycle: Lifecycle,
    cpu_views: [CpuView; MAX_CPUS],
    cpu_refs: [CpuRef; MAX_CPUS],
    cpu_ref_count: usize,
    possible_refs: [CpuRef; MAX_CPUS],
    possible_count: usize,
    present_refs: [CpuRef; MAX_CPUS],
    present_count: usize,
    online_refs: [CpuRef; MAX_CPUS],
    online_count: usize,
    pre_smp_topology_ready: bool,
    boot_cpu_topology_recorded: bool,
    smp_concurrency_open: bool,
}

impl CpuGroup {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cpu_views: [CpuView::invalid(); MAX_CPUS],
            cpu_refs: [CpuRef::invalid(); MAX_CPUS],
            cpu_ref_count: 0,
            possible_refs: [CpuRef::invalid(); MAX_CPUS],
            possible_count: 0,
            present_refs: [CpuRef::invalid(); MAX_CPUS],
            present_count: 0,
            online_refs: [CpuRef::invalid(); MAX_CPUS],
            online_count: 0,
            pre_smp_topology_ready: false,
            boot_cpu_topology_recorded: false,
            smp_concurrency_open: false,
        }
    }

    pub fn preset(&mut self, current_cpu: &BootCurrentCpu) -> EventResult {
        let Some(boot_cpu) = current_cpu.boot_cpu() else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        if self.lifecycle.state() != State::Base
            || boot_cpu.state() != State::Prepared
            || current_cpu.state() != State::Ready
            || current_cpu.hartid() != boot_cpu.hartid()
            || !current_cpu.owns_boot_cpu()
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.register_cpu_view(boot_cpu)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn register_boot_cpu(&mut self, current_cpu: &BootCurrentCpu) -> EventResult {
        let Some(boot_cpu) = current_cpu.boot_cpu() else {
            return self.failed_setup();
        };
        if !boot_cpu.cpu_ref().is_boot_cpu()
            || boot_cpu.hartid() != current_cpu.hartid()
            || !current_cpu.owns_boot_cpu()
        {
            return self.failed_setup();
        }

        self.register_cpu_view(boot_cpu)
    }

    pub fn setup_smp(
        &mut self,
        device_tree: &DeviceTree,
        sbi: &Sbi,
        secondary_cpus: &mut SecondaryCpuStore,
    ) -> EventResult {
        let Some(boot_cpu) = self.boot_cpu() else {
            return self.failed_setup();
        };
        if self.lifecycle.state() != State::Prepared
            || boot_cpu.state() != State::Online
            || device_tree.state() != State::Ready
            || sbi.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        let Some(secondary_harts) = collect_secondary_harts(device_tree, boot_cpu.hartid()) else {
            return self.failed_setup();
        };
        if !secondary_cpus.reset_from_harts(&secondary_harts.hartids, secondary_harts.count) {
            return self.failed_setup();
        }
        self.register_secondary_cpu_views(secondary_cpus)?;
        self.refresh_cpu_set_views();

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::CpuGroupReady,
        )
    }

    pub fn boot_cpu_ref(&self) -> Option<CpuRef> {
        self.cpu_ref_at(BOOT_CPU_LOGICAL_ID)
    }

    pub fn boot_cpu(&self) -> Option<CpuView> {
        self.cpu(BOOT_CPU_LOGICAL_ID)
    }

    pub fn cpu_ref_at(&self, logical_id: usize) -> Option<CpuRef> {
        if logical_id < self.cpu_ref_count && self.cpu_refs[logical_id].logical_id() == logical_id {
            Some(self.cpu_refs[logical_id])
        } else {
            None
        }
    }

    pub fn possible_cpu_ref_at(&self, logical_id: usize) -> Option<CpuRef> {
        let cpu_ref = self.cpu_ref_at(logical_id)?;
        if contains_cpu_ref(&self.possible_refs, self.possible_count, cpu_ref) {
            Some(cpu_ref)
        } else {
            None
        }
    }

    pub fn cpu(&self, logical_id: usize) -> Option<CpuView> {
        self.cpu_view(logical_id)
    }

    pub fn possible_contains(&self, cpu_ref: CpuRef) -> bool {
        contains_cpu_ref(&self.possible_refs, self.possible_count, cpu_ref)
    }

    pub fn present_contains(&self, cpu_ref: CpuRef) -> bool {
        contains_cpu_ref(&self.present_refs, self.present_count, cpu_ref)
    }

    pub fn online_contains(&self, cpu_ref: CpuRef) -> bool {
        contains_cpu_ref(&self.online_refs, self.online_count, cpu_ref)
    }

    pub fn boot_cpu_index_zero(&self) -> bool {
        self.cpu(BOOT_CPU_LOGICAL_ID)
            .map(|cpu| cpu.role() == CpuRole::Boot && cpu.logical_id() == BOOT_CPU_LOGICAL_ID)
            .unwrap_or(false)
    }

    pub const fn secondary_count(&self) -> usize {
        self.cpu_ref_count.saturating_sub(1)
    }

    pub fn has_hartid(&self, hartid: usize) -> bool {
        let mut logical_id = 0usize;
        while logical_id < self.cpu_ref_count {
            if self.cpu_views[logical_id].hartid() == hartid {
                return true;
            }
            logical_id += 1;
        }
        false
    }

    pub const fn possible_cpu_count(&self) -> usize {
        self.possible_count
    }

    pub const fn pre_smp_topology_ready(&self) -> bool {
        self.pre_smp_topology_ready
    }

    pub const fn boot_cpu_topology_recorded(&self) -> bool {
        self.boot_cpu_topology_recorded
    }

    pub fn secondary_cpus_present_not_online(&self) -> bool {
        let mut logical_id = 1usize;
        while logical_id < self.cpu_ref_count {
            let Some(cpu_ref) = self.cpu_ref_at(logical_id) else {
                return false;
            };
            let Some(cpu) = self.cpu_view(logical_id) else {
                return false;
            };
            if cpu.role() != CpuRole::Secondary
                || !self.present_contains(cpu_ref)
                || self.online_contains(cpu_ref)
                || cpu.is_online()
            {
                return false;
            }
            logical_id += 1;
        }
        true
    }

    pub fn secondary_cpus_online(&self) -> bool {
        let mut logical_id = 1usize;
        while logical_id < self.cpu_ref_count {
            let Some(cpu_ref) = self.cpu_ref_at(logical_id) else {
                return false;
            };
            let Some(cpu) = self.cpu_view(logical_id) else {
                return false;
            };
            if cpu.role() != CpuRole::Secondary
                || !self.present_contains(cpu_ref)
                || !self.online_contains(cpu_ref)
                || !cpu.is_online()
            {
                return false;
            }
            logical_id += 1;
        }
        self.secondary_count() != 0
    }

    pub const fn smp_concurrency_open(&self) -> bool {
        self.smp_concurrency_open
    }

    pub fn prepare_pre_smp(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.boot_cpu_state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        if !self.secondary_cpus_present_not_online() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.pre_smp_topology_ready = true;
        self.boot_cpu_topology_recorded = true;
        crate::trace::checkpoint(Checkpoint::CpuGroupPreSmpReady);
        Ok(())
    }

    pub fn mark_secondary_cpus_online(
        &mut self,
        secondary_cpus: &mut SecondaryCpuStore,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.pre_smp_topology_ready
            || !self.boot_cpu_topology_recorded
            || !self.secondary_cpus_present_not_online()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        secondary_cpus.mark_all_online();
        self.register_secondary_cpu_views(secondary_cpus)?;
        self.refresh_cpu_set_views();
        self.smp_concurrency_open = true;
        crate::trace::checkpoint(Checkpoint::SecondaryCpusOnline);
        Ok(())
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn boot_cpu_state(&self) -> State {
        self.boot_cpu()
            .map(|cpu| cpu.state())
            .unwrap_or(State::Base)
    }

    fn cpu_view(&self, logical_id: usize) -> Option<CpuView> {
        if logical_id >= self.cpu_ref_count || logical_id >= MAX_CPUS {
            return None;
        }
        if self.cpu_refs[logical_id].logical_id() != logical_id {
            return None;
        }
        let cpu = self.cpu_views[logical_id];
        if cpu.logical_id() == logical_id {
            Some(cpu)
        } else {
            None
        }
    }

    fn register_secondary_cpu_views(&mut self, secondary_cpus: &SecondaryCpuStore) -> EventResult {
        let boot_cpu = self.boot_cpu();
        self.cpu_views = [CpuView::invalid(); MAX_CPUS];
        self.cpu_refs = [CpuRef::invalid(); MAX_CPUS];
        self.cpu_ref_count = 0;

        let Some(boot_cpu) = boot_cpu else {
            return self.failed_setup();
        };
        self.register_cpu_view(boot_cpu)?;

        let mut logical_id = 1usize;
        while logical_id <= secondary_cpus.count() {
            let Some(cpu) = secondary_cpus.cpu(logical_id) else {
                return self.failed_setup();
            };
            self.register_cpu_view(cpu)?;
            logical_id += 1;
        }
        Ok(())
    }

    pub fn possible_cpu_boundary_ready(&self) -> bool {
        if self.lifecycle.state() != State::Ready
            || self.possible_count == 0
            || self.possible_count > MAX_CPUS
            || self.boot_cpu_ref().is_none()
            || !self.boot_cpu_index_zero()
        {
            return false;
        }

        let mut logical_id = 0usize;
        while logical_id < self.possible_count {
            let Some(cpu_ref) = self.possible_cpu_ref_at(logical_id) else {
                return false;
            };
            let Some(cpu) = self.cpu(logical_id) else {
                return false;
            };
            if cpu.cpu_ref() != cpu_ref
                || cpu_ref.logical_id() != logical_id
                || cpu.logical_id() != logical_id
                || !cpu.is_possible()
                || self.present_contains(cpu_ref) != cpu.is_present()
                || self.online_contains(cpu_ref) != cpu.is_online()
                || contains_hartid_before(&self.cpu_views, logical_id, cpu.hartid())
            {
                return false;
            }
            if logical_id == BOOT_CPU_LOGICAL_ID {
                if cpu.role() != CpuRole::Boot || !cpu.is_online() {
                    return false;
                }
            } else if cpu.role() != CpuRole::Secondary {
                return false;
            }
            logical_id += 1;
        }

        self.cpu_ref_at(self.possible_count).is_none()
            && self.possible_cpu_ref_at(self.possible_count).is_none()
    }

    fn register_cpu_view(&mut self, cpu: CpuView) -> EventResult {
        let logical_id = cpu.logical_id();
        if logical_id >= MAX_CPUS || cpu.cpu_ref().logical_id() != logical_id {
            return self.failed_setup();
        }
        if logical_id < self.cpu_ref_count {
            if self.cpu_refs[logical_id] != cpu.cpu_ref() {
                return self.failed_setup();
            }
            self.cpu_views[logical_id] = cpu;
            self.refresh_cpu_set_views();
            return Ok(());
        }
        if logical_id != self.cpu_ref_count {
            return self.failed_setup();
        }

        self.cpu_views[logical_id] = cpu;
        self.cpu_refs[logical_id] = cpu.cpu_ref();
        self.cpu_ref_count += 1;
        self.refresh_cpu_set_views();
        Ok(())
    }

    fn refresh_cpu_set_views(&mut self) {
        self.possible_refs = [CpuRef::invalid(); MAX_CPUS];
        self.present_refs = [CpuRef::invalid(); MAX_CPUS];
        self.online_refs = [CpuRef::invalid(); MAX_CPUS];
        self.possible_count = 0;
        self.present_count = 0;
        self.online_count = 0;

        let mut logical_id = 0usize;
        while logical_id < self.cpu_ref_count {
            let cpu_ref = self.cpu_refs[logical_id];
            let cpu = self.cpu_views[logical_id];
            if cpu_ref.logical_id() == logical_id && cpu.logical_id() == logical_id {
                if cpu.is_possible() {
                    self.possible_refs[self.possible_count] = cpu_ref;
                    self.possible_count += 1;
                }
                if cpu.is_present() {
                    self.present_refs[self.present_count] = cpu_ref;
                    self.present_count += 1;
                }
                if cpu.is_online() {
                    self.online_refs[self.online_count] = cpu_ref;
                    self.online_count += 1;
                }
            }
            logical_id += 1;
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

fn contains_hartid_before(cpu_views: &[CpuView; MAX_CPUS], end: usize, hartid: usize) -> bool {
    let mut index = 0usize;
    while index < end {
        if cpu_views[index].hartid() == hartid {
            return true;
        }
        index += 1;
    }
    false
}

struct SecondaryHartSet {
    hartids: [usize; MAX_CPUS - 1],
    count: usize,
}

impl SecondaryHartSet {
    const fn empty() -> Self {
        Self {
            hartids: [usize::MAX; MAX_CPUS - 1],
            count: 0,
        }
    }

    fn push(&mut self, hartid: usize) -> bool {
        if self.count >= self.hartids.len() || self.contains(hartid) {
            return false;
        }

        self.hartids[self.count] = hartid;
        self.count += 1;
        true
    }

    fn contains(&self, hartid: usize) -> bool {
        let mut index = 0usize;
        while index < self.count {
            if self.hartids[index] == hartid {
                return true;
            }
            index += 1;
        }
        false
    }
}

fn collect_secondary_harts(
    device_tree: &DeviceTree,
    boot_hartid: usize,
) -> Option<SecondaryHartSet> {
    let cpus = device_tree.find_node(b"/cpus")?;
    let address_cells = cpu_address_cells(cpus.property(b"#address-cells")?.raw_value())?;
    let mut secondary_harts = SecondaryHartSet::empty();
    let mut saw_boot_cpu = false;

    for cpu in cpus.children() {
        let Some(reg) = cpu.property(b"reg") else {
            continue;
        };
        let value = reg.raw_value();
        let base = value.as_ptr() as usize;
        let (hartid, _) = read_cells(base, value.len(), address_cells)?;
        let hartid = usize::try_from(hartid).ok()?;
        if hartid == boot_hartid {
            saw_boot_cpu = true;
        } else if !secondary_harts.push(hartid) {
            return None;
        }
    }

    if saw_boot_cpu {
        Some(secondary_harts)
    } else {
        None
    }
}

fn cpu_address_cells(value: &[u8]) -> Option<usize> {
    if value.len() < 4 {
        return None;
    }
    let (cells, _) = read_cells(value.as_ptr() as usize, value.len(), 1)?;
    let cells = usize::try_from(cells).ok()?;
    if cells == 0 || cells > 2 {
        None
    } else {
        Some(cells)
    }
}

fn contains_cpu_ref(entries: &[CpuRef; MAX_CPUS], count: usize, cpu_ref: CpuRef) -> bool {
    let mut index = 0usize;
    while index < count {
        if entries[index] == cpu_ref {
            return true;
        }
        index += 1;
    }
    false
}
