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

const MAX_CPUS: usize = 16;
const BOOT_CPU_LOGICAL_ID: usize = 0;

unsafe extern "C" {
    static head_boot_hartid: usize;
}

pub struct BootCpu {
    lifecycle: Lifecycle,
    hartid: usize,
}

impl BootCpu {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hartid: usize::MAX,
        }
    }

    fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
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

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootCpuReady,
        )
    }

    fn enable(&mut self) -> EventResult {
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

#[derive(Clone, Copy)]
pub struct SecondaryCpu {
    hartid: usize,
    possible: bool,
    present: bool,
    online: bool,
}

impl SecondaryCpu {
    const fn empty() -> Self {
        Self {
            hartid: usize::MAX,
            possible: false,
            present: false,
            online: false,
        }
    }

    const fn new(hartid: usize) -> Self {
        Self {
            hartid,
            possible: true,
            present: true,
            online: false,
        }
    }

    pub const fn hartid(self) -> usize {
        self.hartid
    }

    pub const fn is_possible(self) -> bool {
        self.possible
    }

    pub const fn is_present(self) -> bool {
        self.present
    }

    pub const fn is_online(self) -> bool {
        self.online
    }
}

pub struct CpuGroup {
    lifecycle: Lifecycle,
    /*
     * Transitional lowering: the formal model is one CPU instance per logical
     * id. Until the internal storage is flattened, boot_cpu and secondary_cpus
     * are exposed through CpuRef/CpuView below as the formal CpuGroup.Cpu[id]
     * indexed view.
     */
    boot_cpu: BootCpu,
    secondary_cpus: [SecondaryCpu; MAX_CPUS - 1],
    secondary_count: usize,
    pre_smp_topology_ready: bool,
    boot_cpu_topology_recorded: bool,
    smp_concurrency_open: bool,
}

impl CpuGroup {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_cpu: BootCpu::new(),
            secondary_cpus: [SecondaryCpu::empty(); MAX_CPUS - 1],
            secondary_count: 0,
            pre_smp_topology_ready: false,
            boot_cpu_topology_recorded: false,
            smp_concurrency_open: false,
        }
    }

    pub fn adopt_boot_cpu_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        self.boot_cpu.adopt_head_preset(boot_args)
    }

    pub fn preset(&mut self, current_cpu: &BootCurrentCpu) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.boot_cpu.state() != State::Prepared
            || current_cpu.state() != State::Ready
            || current_cpu.hartid() != self.boot_cpu.hartid
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
        self.boot_cpu.setup(boot_hartid_valid)
    }

    pub fn boot_cpu_enable(&mut self) -> EventResult {
        self.boot_cpu.enable()
    }

    pub fn setup_smp(
        &mut self,
        device_tree: &DeviceTree,
        cpu_id_map: &CpuIdMap,
        sbi: &Sbi,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || self.boot_cpu.state() != State::Online
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

        let Some(secondary_cpus) = collect_secondary_cpus(device_tree, self.boot_cpu.hartid) else {
            return self.failed_setup();
        };
        self.secondary_cpus = secondary_cpus.cpus;
        self.secondary_count = secondary_cpus.count;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::CpuGroupReady,
        )
    }

    pub fn boot_hartid(&self) -> usize {
        self.boot_cpu.hartid
    }

    pub fn boot_cpu_ref(&self) -> Option<CpuRef> {
        self.cpu_ref_at(BOOT_CPU_LOGICAL_ID)
    }

    pub fn cpu_ref_at(&self, logical_id: usize) -> Option<CpuRef> {
        self.cpu(logical_id).map(|cpu| cpu.cpu_ref())
    }

    pub fn cpu(&self, logical_id: usize) -> Option<CpuView> {
        if logical_id == BOOT_CPU_LOGICAL_ID {
            return self.boot_cpu_view();
        }

        let secondary_index = logical_id.checked_sub(1)?;
        let cpu = self.secondary_cpu(secondary_index)?;
        Some(CpuView {
            cpu_ref: CpuRef::new(logical_id),
            hartid: cpu.hartid(),
            role: CpuRole::Secondary,
            possible: cpu.is_possible(),
            present: cpu.is_present(),
            active: cpu.is_online(),
            online: cpu.is_online(),
        })
    }

    pub fn possible_contains(&self, cpu_ref: CpuRef) -> bool {
        self.cpu(cpu_ref.logical_id())
            .map(|cpu| cpu.is_possible())
            .unwrap_or(false)
    }

    pub fn present_contains(&self, cpu_ref: CpuRef) -> bool {
        self.cpu(cpu_ref.logical_id())
            .map(|cpu| cpu.is_present())
            .unwrap_or(false)
    }

    pub fn online_contains(&self, cpu_ref: CpuRef) -> bool {
        self.cpu(cpu_ref.logical_id())
            .map(|cpu| cpu.is_online())
            .unwrap_or(false)
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
        self.secondary_count
    }

    pub fn secondary_cpu(&self, index: usize) -> Option<SecondaryCpu> {
        if index < self.secondary_count {
            Some(self.secondary_cpus[index])
        } else {
            None
        }
    }

    pub fn has_hartid(&self, hartid: usize) -> bool {
        if self.boot_cpu.hartid == hartid {
            return true;
        }

        let mut index = 0usize;
        while index < self.secondary_count {
            if self.secondary_cpus[index].hartid == hartid {
                return true;
            }
            index += 1;
        }
        false
    }

    pub const fn possible_cpu_count(&self) -> usize {
        1 + self.secondary_count
    }

    pub const fn pre_smp_topology_ready(&self) -> bool {
        self.pre_smp_topology_ready
    }

    pub const fn boot_cpu_topology_recorded(&self) -> bool {
        self.boot_cpu_topology_recorded
    }

    pub fn secondary_cpus_present_not_online(&self) -> bool {
        let mut index = 0usize;
        while index < self.secondary_count {
            let cpu = self.secondary_cpus[index];
            if !cpu.is_present() || cpu.is_online() {
                return false;
            }
            index += 1;
        }
        true
    }

    pub fn secondary_cpus_online(&self) -> bool {
        let mut index = 0usize;
        while index < self.secondary_count {
            let cpu = self.secondary_cpus[index];
            if !cpu.is_present() || !cpu.is_online() {
                return false;
            }
            index += 1;
        }
        self.secondary_count != 0
    }

    pub const fn smp_concurrency_open(&self) -> bool {
        self.smp_concurrency_open
    }

    pub fn prepare_pre_smp(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.boot_cpu.state() != State::Online {
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

        let mut index = 0usize;
        while index < self.secondary_count {
            self.secondary_cpus[index].online = true;
            index += 1;
        }
        self.smp_concurrency_open = true;
        crate::trace::checkpoint(Checkpoint::SecondaryCpusOnline);
        Ok(())
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn boot_cpu_state(&self) -> State {
        self.boot_cpu.state()
    }

    fn boot_cpu_view(&self) -> Option<CpuView> {
        if self.boot_cpu.hartid == usize::MAX {
            return None;
        }

        let state = self.boot_cpu.state();
        let active = state == State::Ready || state == State::Online;
        Some(CpuView {
            cpu_ref: CpuRef::new(BOOT_CPU_LOGICAL_ID),
            hartid: self.boot_cpu.hartid,
            role: CpuRole::Boot,
            possible: active,
            present: active,
            active,
            online: state == State::Online,
        })
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

struct SecondaryCpuSet {
    cpus: [SecondaryCpu; MAX_CPUS - 1],
    count: usize,
}

impl SecondaryCpuSet {
    const fn empty() -> Self {
        Self {
            cpus: [SecondaryCpu::empty(); MAX_CPUS - 1],
            count: 0,
        }
    }

    fn push(&mut self, hartid: usize) -> bool {
        if self.count >= self.cpus.len() || self.contains(hartid) {
            return false;
        }

        self.cpus[self.count] = SecondaryCpu::new(hartid);
        self.count += 1;
        true
    }

    fn contains(&self, hartid: usize) -> bool {
        let mut index = 0usize;
        while index < self.count {
            if self.cpus[index].hartid == hartid {
                return true;
            }
            index += 1;
        }
        false
    }
}

fn collect_secondary_cpus(device_tree: &DeviceTree, boot_hartid: usize) -> Option<SecondaryCpuSet> {
    let cpus = device_tree.find_node(b"/cpus")?;
    let address_cells = cpu_address_cells(cpus.property(b"#address-cells")?.raw_value())?;
    let mut secondary_cpus = SecondaryCpuSet::empty();
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
        } else if !secondary_cpus.push(hartid) {
            return None;
        }
    }

    if saw_boot_cpu {
        Some(secondary_cpus)
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
