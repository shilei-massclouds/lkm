use super::{
    boot_args::BootArgs,
    cpu::{BOOT_CPU_LOGICAL_ID, Cpu, CpuRef, CpuRole, LogicId, MAX_CPUS},
    cpu_control::{CurrentTaskSlot, LocalInterruptControl},
    device_tree::DeviceTree,
    fdt_reader::read_cells,
    sbi::Sbi,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    task_flow::TaskFlow,
};
use crate::checkpoint::Checkpoint;

/// The sole authority for CPU instances. Every CpuRef resolves into this
/// indexed collection; aliases such as BootCPU never carry separate state.
pub struct CpuGroup {
    lifecycle: Lifecycle,
    cpus: [Option<Cpu>; MAX_CPUS],
    cpu_count: usize,
    pre_smp_topology_ready: bool,
    boot_cpu_topology_recorded: bool,
    smp_concurrency_open: bool,
}

impl CpuGroup {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cpus: [const { None }; MAX_CPUS],
            cpu_count: 0,
            pre_smp_topology_ready: false,
            boot_cpu_topology_recorded: false,
            smp_concurrency_open: false,
        }
    }

    /// Adopts the OpenSBI-to-kernel entry effect. CPU0 and its parent are
    /// published together only after the complete boot-hart validation passes.
    pub fn preset(&mut self, boot_args: &BootArgs) -> EventResult {
        if self.lifecycle.state() != State::Base || self.cpu_count != 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let mut boot_cpu = Cpu::boot();
        boot_cpu.adopt_head_preset(boot_args)?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)?;
        self.cpus[BOOT_CPU_LOGICAL_ID] = Some(boot_cpu);
        self.cpu_count = 1;
        Ok(())
    }

    pub fn setup_boot_cpu(&mut self, boot_hartid_valid: bool) -> EventResult {
        let state = self.lifecycle.state();
        let Some(boot_cpu) = self.cpus[BOOT_CPU_LOGICAL_ID].as_mut() else {
            return failed_condition(LifecycleEvent::Setup, state, State::Prepared, State::Ready);
        };
        boot_cpu.setup_boot(boot_hartid_valid)
    }

    pub fn enable_boot_cpu(&mut self) -> EventResult {
        let state = self.lifecycle.state();
        let Some(boot_cpu) = self.cpus[BOOT_CPU_LOGICAL_ID].as_mut() else {
            return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
        };
        boot_cpu.enable_boot()
    }

    /// Atomically publishes the AP portion of the indexed collection.
    pub fn setup_smp(&mut self, device_tree: &DeviceTree, sbi: &Sbi) -> EventResult {
        let Some(boot_cpu) = self.boot_cpu() else {
            return self.failed_setup();
        };
        if self.lifecycle.state() != State::Prepared
            || self.cpu_count != 1
            || boot_cpu.state() != State::Online
            || device_tree.state() != State::Ready
            || sbi.state() != State::Ready
        {
            return self.failed_setup();
        }

        let Some(secondary_harts) = collect_secondary_harts(device_tree, boot_cpu.hartid()) else {
            return self.failed_setup();
        };
        if secondary_harts.count + 1 > MAX_CPUS {
            return self.failed_setup();
        }

        // Collection, key and duplicate validation is complete. None of the
        // following construction steps can fail, so parent and children
        // become visible as one stable publication.
        let mut index = 0usize;
        while index < secondary_harts.count {
            let logical_id = index + 1;
            let Some(logic_id) = LogicId::new(logical_id) else {
                return self.failed_setup();
            };
            self.cpus[logical_id] = Some(Cpu::secondary(logic_id, secondary_harts.hartids[index]));
            index += 1;
        }
        self.cpu_count = secondary_harts.count + 1;
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

    pub fn boot_cpu(&self) -> Option<&Cpu> {
        self.cpu(BOOT_CPU_LOGICAL_ID)
    }

    pub fn cpu_ref_at(&self, logical_id: usize) -> Option<CpuRef> {
        let cpu = self.cpu(logical_id)?;
        let cpu_ref = cpu.cpu_ref();
        if cpu_ref.is_valid() && cpu_ref.logical_id() == logical_id {
            Some(cpu_ref)
        } else {
            None
        }
    }

    pub fn dereference(&self, cpu_ref: CpuRef) -> Option<&Cpu> {
        if !cpu_ref.is_valid() {
            return None;
        }
        let cpu = self.cpu(cpu_ref.logic_id().get())?;
        if cpu.cpu_ref() == cpu_ref {
            Some(cpu)
        } else {
            None
        }
    }

    pub fn current_cpu(&self, flow: &TaskFlow) -> Option<CurrentCpu> {
        let cpu_ref = flow.cpu_ref()?;
        self.current_cpu_ref(cpu_ref)
    }

    pub fn current_cpu_ref(&self, cpu_ref: CpuRef) -> Option<CurrentCpu> {
        self.dereference(cpu_ref)?;
        Some(CurrentCpu { cpu_ref })
    }

    pub fn possible_cpu_ref_at(&self, logical_id: usize) -> Option<CpuRef> {
        let cpu = self.cpu(logical_id)?;
        if cpu.is_possible() {
            Some(cpu.cpu_ref())
        } else {
            None
        }
    }

    pub fn cpu(&self, logical_id: usize) -> Option<&Cpu> {
        self.cpus.get(logical_id)?.as_ref()
    }

    pub fn cpu_mut(&mut self, logical_id: usize) -> Option<&mut Cpu> {
        self.cpus.get_mut(logical_id)?.as_mut()
    }

    pub fn possible_contains(&self, cpu_ref: CpuRef) -> bool {
        self.dereference(cpu_ref)
            .map(Cpu::is_possible)
            .unwrap_or(false)
    }

    pub fn present_contains(&self, cpu_ref: CpuRef) -> bool {
        self.dereference(cpu_ref)
            .map(Cpu::is_present)
            .unwrap_or(false)
    }

    pub fn online_contains(&self, cpu_ref: CpuRef) -> bool {
        self.dereference(cpu_ref)
            .map(Cpu::is_online)
            .unwrap_or(false)
    }

    pub fn boot_cpu_index_zero(&self) -> bool {
        self.boot_cpu()
            .map(|cpu| cpu.role() == CpuRole::Boot && cpu.logical_id() == BOOT_CPU_LOGICAL_ID)
            .unwrap_or(false)
    }

    pub const fn secondary_count(&self) -> usize {
        self.cpu_count.saturating_sub(1)
    }

    pub fn has_hartid(&self, hartid: usize) -> bool {
        self.logical_id_for_hartid(hartid).is_some()
    }

    pub fn logical_id_for_hartid(&self, hartid: usize) -> Option<LogicId> {
        let mut logical_id = 0usize;
        while logical_id < self.cpu_count {
            if self.cpu(logical_id).map(Cpu::hartid) == Some(hartid) {
                return LogicId::new(logical_id);
            }
            logical_id += 1;
        }
        None
    }

    pub fn possible_cpu_count(&self) -> usize {
        self.count_matching(Cpu::is_possible)
    }

    pub const fn pre_smp_topology_ready(&self) -> bool {
        self.pre_smp_topology_ready
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn boot_cpu_topology_recorded(&self) -> bool {
        self.boot_cpu_topology_recorded
    }

    pub fn secondary_cpus_present_not_online(&self) -> bool {
        let mut logical_id = 1usize;
        while logical_id < self.cpu_count {
            let Some(cpu) = self.cpu(logical_id) else {
                return false;
            };
            if cpu.role() != CpuRole::Secondary
                || !cpu.is_present()
                || cpu.is_active()
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
        while logical_id < self.cpu_count {
            let Some(cpu) = self.cpu(logical_id) else {
                return false;
            };
            if cpu.role() != CpuRole::Secondary || !cpu.is_present() || !cpu.is_online() {
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
        crate::checkpoint::checkpoint(Checkpoint::CpuGroupPreSmpReady);
        Ok(())
    }

    pub fn mark_secondary_cpus_online_after_ap_ack(&mut self) -> EventResult {
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
        while logical_id < self.cpu_count {
            let Some(cpu) = self.cpu_mut(logical_id) else {
                return failed_condition(
                    LifecycleEvent::Enable,
                    self.lifecycle.state(),
                    State::Ready,
                    State::Ready,
                );
            };
            cpu.mark_online();
            logical_id += 1;
        }
        self.smp_concurrency_open = true;
        crate::checkpoint::checkpoint(Checkpoint::SecondaryCpusOnline);
        Ok(())
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn boot_cpu_state(&self) -> State {
        self.boot_cpu().map(Cpu::state).unwrap_or(State::Base)
    }

    pub fn boot_hartid(&self) -> Option<usize> {
        self.boot_cpu().map(Cpu::hartid)
    }

    pub fn boot_cpu_local_interrupt(&self) -> Option<&LocalInterruptControl> {
        Some(self.boot_cpu()?.local_interrupt())
    }

    pub fn boot_cpu_local_interrupt_mut(&mut self) -> Option<&mut LocalInterruptControl> {
        Some(self.cpu_mut(BOOT_CPU_LOGICAL_ID)?.local_interrupt_mut())
    }

    pub fn boot_cpu_current_task(&self) -> Option<&CurrentTaskSlot> {
        Some(self.boot_cpu()?.current_task())
    }

    pub fn boot_cpu_current_task_mut(&mut self) -> Option<&mut CurrentTaskSlot> {
        Some(self.cpu_mut(BOOT_CPU_LOGICAL_ID)?.current_task_mut())
    }

    pub fn boot_cpu_controls_mut(
        &mut self,
    ) -> Option<(&mut LocalInterruptControl, &mut CurrentTaskSlot)> {
        Some(self.cpu_mut(BOOT_CPU_LOGICAL_ID)?.controls_mut())
    }

    pub fn possible_cpu_boundary_ready(&self) -> bool {
        let possible_count = self.possible_cpu_count();
        if self.lifecycle.state() != State::Ready
            || possible_count == 0
            || possible_count > MAX_CPUS
            || self.boot_cpu_ref().is_none()
            || !self.boot_cpu_index_zero()
        {
            return false;
        }

        let mut logical_id = 0usize;
        while logical_id < possible_count {
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
                || self.contains_hartid_before(logical_id, cpu.hartid())
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

        self.cpu_ref_at(possible_count).is_none()
            && self.possible_cpu_ref_at(possible_count).is_none()
    }

    fn count_matching(&self, predicate: fn(&Cpu) -> bool) -> usize {
        let mut count = 0usize;
        let mut logical_id = 0usize;
        while logical_id < self.cpu_count {
            if self.cpu(logical_id).map(predicate).unwrap_or(false) {
                count += 1;
            }
            logical_id += 1;
        }
        count
    }

    fn contains_hartid_before(&self, end: usize, hartid: usize) -> bool {
        let mut logical_id = 0usize;
        while logical_id < end {
            if self.cpu(logical_id).map(Cpu::hartid) == Some(hartid) {
                return true;
            }
            logical_id += 1;
        }
        false
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

/// Stateless capability proving that a concrete TaskFlow selected a live
/// canonical element of CpuGroup.cpus[]. It is never stored in Context.
#[derive(Clone, Copy)]
pub struct CurrentCpu {
    cpu_ref: CpuRef,
}

impl CurrentCpu {
    pub const fn cpu_ref(&self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn logical_id(&self) -> usize {
        self.cpu_ref.logical_id()
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
            if saw_boot_cpu {
                return None;
            }
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
