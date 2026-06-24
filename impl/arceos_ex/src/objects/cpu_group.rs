use super::{
    boot_args::BootArgs,
    cpu_control::BootCurrentCpu,
    cpu_id_map::CpuIdMap,
    device_tree::DeviceTree,
    fdt_reader::read_cells,
    sbi::Sbi,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub(crate) const MAX_CPUS: usize = 16;
const BOOT_CPU_LOGICAL_ID: usize = 0;

unsafe extern "C" {
    static head_boot_hartid: usize;
}

pub struct Cpu {
    lifecycle: Lifecycle,
    logical_id: usize,
    hartid: usize,
    role: CpuRole,
    possible: bool,
    present: bool,
    active: bool,
    online: bool,
}

impl Cpu {
    const fn empty() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            logical_id: usize::MAX,
            hartid: usize::MAX,
            role: CpuRole::Secondary,
            possible: false,
            present: false,
            active: false,
            online: false,
        }
    }

    const fn boot() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            logical_id: BOOT_CPU_LOGICAL_ID,
            hartid: usize::MAX,
            role: CpuRole::Boot,
            possible: false,
            present: false,
            active: false,
            online: false,
        }
    }

    const fn secondary(logical_id: usize, hartid: usize) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Ready),
            logical_id,
            hartid,
            role: CpuRole::Secondary,
            possible: true,
            present: true,
            active: false,
            online: false,
        }
    }

    fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        if self.role != CpuRole::Boot || self.logical_id != BOOT_CPU_LOGICAL_ID {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        let head_hartid = unsafe { core::ptr::addr_of!(head_boot_hartid).read_volatile() };
        if head_hartid != boot_args.boot_hartid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.hartid = head_hartid;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    fn setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !boot_hartid_valid {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.possible = true;
        self.present = true;
        self.active = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootCpuReady,
        )
    }

    fn enable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.role != CpuRole::Boot
            || !self.possible
            || !self.present
            || !self.active
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.online = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::BootCpuOnline,
        )
    }

    fn state(&self) -> State {
        self.lifecycle.state()
    }

    const fn cpu_ref(&self) -> CpuRef {
        CpuRef::new(self.logical_id)
    }

    const fn hartid(&self) -> usize {
        self.hartid
    }

    const fn role(&self) -> CpuRole {
        self.role
    }

    const fn is_online(&self) -> bool {
        self.online
    }

    fn mark_online(&mut self) {
        self.active = true;
        self.online = true;
    }

    fn view(&self) -> Option<CpuView> {
        if self.logical_id == usize::MAX || self.hartid == usize::MAX {
            return None;
        }

        Some(CpuView {
            cpu_ref: self.cpu_ref(),
            hartid: self.hartid,
            role: self.role,
            possible: self.possible,
            present: self.present,
            active: self.active,
            online: self.online,
        })
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuRole {
    Boot,
    Secondary,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CpuRef {
    logical_id: usize,
}

impl CpuRef {
    pub const fn invalid() -> Self {
        Self {
            logical_id: usize::MAX,
        }
    }

    pub const fn new(logical_id: usize) -> Self {
        Self { logical_id }
    }

    pub const fn logical_id(self) -> usize {
        self.logical_id
    }

    pub const fn is_boot_cpu(self) -> bool {
        self.logical_id == BOOT_CPU_LOGICAL_ID
    }
}

#[derive(Clone, Copy)]
pub struct CpuView {
    cpu_ref: CpuRef,
    hartid: usize,
    role: CpuRole,
    possible: bool,
    present: bool,
    active: bool,
    online: bool,
}

impl CpuView {
    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn logical_id(self) -> usize {
        self.cpu_ref.logical_id()
    }

    pub const fn hartid(self) -> usize {
        self.hartid
    }

    pub const fn role(self) -> CpuRole {
        self.role
    }

    pub const fn is_possible(self) -> bool {
        self.possible
    }

    pub const fn is_present(self) -> bool {
        self.present
    }

    pub const fn is_active(self) -> bool {
        self.active
    }

    pub const fn is_online(self) -> bool {
        self.online
    }
}

pub struct CpuGroup {
    lifecycle: Lifecycle,
    cpu_storage: [Cpu; MAX_CPUS],
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
            cpu_storage: {
                let mut cpus = [const { Cpu::empty() }; MAX_CPUS];
                cpus[BOOT_CPU_LOGICAL_ID] = Cpu::boot();
                cpus
            },
            cpu_refs: {
                let mut refs = [CpuRef::invalid(); MAX_CPUS];
                refs[BOOT_CPU_LOGICAL_ID] = CpuRef::new(BOOT_CPU_LOGICAL_ID);
                refs
            },
            cpu_ref_count: 1,
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

    pub fn adopt_boot_cpu_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        self.cpu_storage[BOOT_CPU_LOGICAL_ID].adopt_head_preset(boot_args)
    }

    pub fn preset(&mut self, current_cpu: &BootCurrentCpu) -> EventResult {
        let boot_cpu = &self.cpu_storage[BOOT_CPU_LOGICAL_ID];
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

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn boot_cpu_setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        let result = self.cpu_storage[BOOT_CPU_LOGICAL_ID].setup(boot_hartid_valid);
        if result.is_ok() {
            self.refresh_cpu_set_views();
        }
        result
    }

    pub fn boot_cpu_enable(&mut self) -> EventResult {
        let result = self.cpu_storage[BOOT_CPU_LOGICAL_ID].enable();
        if result.is_ok() {
            self.refresh_cpu_set_views();
        }
        result
    }

    pub fn setup_smp(
        &mut self,
        device_tree: &DeviceTree,
        cpu_id_map: &CpuIdMap,
        sbi: &Sbi,
    ) -> EventResult {
        let boot_cpu = &self.cpu_storage[BOOT_CPU_LOGICAL_ID];
        if self.lifecycle.state() != State::Prepared
            || boot_cpu.state() != State::Online
            || device_tree.state() != State::Ready
            || cpu_id_map.state() != State::Prepared
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
        self.reset_secondary_cpus(secondary_harts);
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
        self.cpu_instance(logical_id).and_then(|cpu| cpu.view())
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

    pub fn logical_id_index_ready(&self) -> bool {
        self.boot_cpu_ref().is_some()
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
            if self.cpu_storage[logical_id].hartid() == hartid {
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
            let cpu = &self.cpu_storage[logical_id];
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
            let cpu = &self.cpu_storage[logical_id];
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
        if self.lifecycle.state() != State::Ready
            || self.cpu_storage[BOOT_CPU_LOGICAL_ID].state() != State::Online
        {
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

    pub fn mark_secondary_cpus_online(&mut self) -> EventResult {
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

        let mut logical_id = 1usize;
        while logical_id < self.cpu_ref_count {
            self.cpu_storage[logical_id].mark_online();
            logical_id += 1;
        }
        self.refresh_cpu_set_views();
        self.smp_concurrency_open = true;
        crate::trace::checkpoint(Checkpoint::SecondaryCpusOnline);
        Ok(())
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn boot_cpu_state(&self) -> State {
        self.cpu_storage[BOOT_CPU_LOGICAL_ID].state()
    }

    fn cpu_instance(&self, logical_id: usize) -> Option<&Cpu> {
        if logical_id >= self.cpu_ref_count || logical_id >= MAX_CPUS {
            return None;
        }
        if self.cpu_refs[logical_id].logical_id() != logical_id {
            return None;
        }
        Some(&self.cpu_storage[logical_id])
    }

    fn reset_secondary_cpus(&mut self, secondary_harts: SecondaryHartSet) {
        let mut logical_id = 1usize;
        while logical_id < MAX_CPUS {
            self.cpu_storage[logical_id] = Cpu::empty();
            self.cpu_refs[logical_id] = CpuRef::invalid();
            logical_id += 1;
        }

        let mut index = 0usize;
        while index < secondary_harts.count {
            let logical_id = index + 1;
            self.cpu_storage[logical_id] =
                Cpu::secondary(logical_id, secondary_harts.hartids[index]);
            self.cpu_refs[logical_id] = CpuRef::new(logical_id);
            index += 1;
        }
        self.cpu_ref_count = 1 + secondary_harts.count;
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
            let cpu = &self.cpu_storage[logical_id];
            if cpu_ref.logical_id() == logical_id && cpu.logical_id == logical_id {
                if cpu.possible {
                    self.possible_refs[self.possible_count] = cpu_ref;
                    self.possible_count += 1;
                }
                if cpu.present {
                    self.present_refs[self.present_count] = cpu_ref;
                    self.present_count += 1;
                }
                if cpu.online {
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
