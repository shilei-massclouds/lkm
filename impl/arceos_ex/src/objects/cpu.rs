use super::{
    boot_args::BootArgs,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::trace::Checkpoint;

pub(crate) const MAX_CPUS: usize = 16;
pub(crate) const BOOT_CPU_LOGICAL_ID: usize = 0;

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
    pub const fn empty() -> Self {
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

    pub const fn boot() -> Self {
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

    pub const fn secondary(logical_id: usize, hartid: usize) -> Self {
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

    pub fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
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

    pub fn setup_boot(&mut self, boot_hartid_valid: bool) -> EventResult {
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

    pub fn enable_boot(&mut self) -> EventResult {
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

    pub fn mark_online(&mut self) {
        self.active = true;
        self.online = true;
    }

    pub fn view(&self) -> Option<CpuView> {
        if self.logical_id == usize::MAX || self.hartid == usize::MAX {
            return None;
        }

        Some(CpuView {
            cpu_ref: CpuRef::new(self.logical_id),
            hartid: self.hartid,
            role: self.role,
            possible: self.possible,
            present: self.present,
            active: self.active,
            online: self.online,
            state: self.lifecycle.state(),
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
    state: State,
}

impl CpuView {
    pub const fn invalid() -> Self {
        Self {
            cpu_ref: CpuRef::invalid(),
            hartid: usize::MAX,
            role: CpuRole::Secondary,
            possible: false,
            present: false,
            active: false,
            online: false,
            state: State::Base,
        }
    }

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

    pub const fn state(self) -> State {
        self.state
    }
}

pub struct SecondaryCpuStore {
    cpus: [Cpu; MAX_CPUS],
    count: usize,
}

impl SecondaryCpuStore {
    pub const fn new() -> Self {
        Self {
            cpus: [const { Cpu::empty() }; MAX_CPUS],
            count: 0,
        }
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub fn reset_from_harts(&mut self, hartids: &[usize; MAX_CPUS - 1], count: usize) -> bool {
        if count >= MAX_CPUS {
            return false;
        }

        let mut logical_id = 0usize;
        while logical_id < MAX_CPUS {
            self.cpus[logical_id] = Cpu::empty();
            logical_id += 1;
        }

        let mut index = 0usize;
        while index < count {
            let logical_id = index + 1;
            self.cpus[logical_id] = Cpu::secondary(logical_id, hartids[index]);
            index += 1;
        }
        self.count = count;
        true
    }

    pub fn cpu(&self, logical_id: usize) -> Option<CpuView> {
        if logical_id == BOOT_CPU_LOGICAL_ID || logical_id > self.count {
            return None;
        }
        self.cpus[logical_id].view()
    }

    pub fn all_match_cpu_group_views(&self, cpu_group: &super::cpu_group::CpuGroup) -> bool {
        if self.count != cpu_group.secondary_count() {
            return false;
        }

        let mut logical_id = 1usize;
        while logical_id <= self.count {
            let Some(stored_cpu) = self.cpu(logical_id) else {
                return false;
            };
            let Some(group_cpu) = cpu_group.cpu(logical_id) else {
                return false;
            };
            if stored_cpu.cpu_ref() != group_cpu.cpu_ref()
                || stored_cpu.hartid() != group_cpu.hartid()
                || stored_cpu.role() != group_cpu.role()
                || stored_cpu.is_possible() != group_cpu.is_possible()
                || stored_cpu.is_present() != group_cpu.is_present()
                || stored_cpu.is_active() != group_cpu.is_active()
                || stored_cpu.is_online() != group_cpu.is_online()
                || stored_cpu.state() != group_cpu.state()
            {
                return false;
            }
            logical_id += 1;
        }
        true
    }

    pub fn mark_all_online(&mut self) {
        let mut logical_id = 1usize;
        while logical_id <= self.count {
            self.cpus[logical_id].mark_online();
            logical_id += 1;
        }
    }
}
