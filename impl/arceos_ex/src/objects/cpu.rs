use super::{
    boot_args::BootArgs,
    cpu_control::LocalInterruptControl,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub(crate) const MAX_CPUS: usize = 16;
pub(crate) const BOOT_CPU_LOGICAL_ID: usize = 0;

unsafe extern "C" {
    static head_boot_hartid: usize;
}

/// The compact key used by the CpuGroup-owned indexed collection.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct LogicId(u16);

impl LogicId {
    pub const INVALID: Self = Self(u16::MAX);

    pub const fn new(value: usize) -> Option<Self> {
        if value < MAX_CPUS {
            Some(Self(value as u16))
        } else {
            None
        }
    }

    pub const fn get(self) -> usize {
        self.0 as usize
    }

    pub const fn is_valid(self) -> bool {
        self.0 != u16::MAX && (self.0 as usize) < MAX_CPUS
    }
}

/// Stable typed reference to `CpuGroup.cpus[logic_id]`.
///
/// Construction validates only the compact key range. Dereference through
/// CpuGroup additionally verifies that the indexed element is published.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CpuRef {
    logical_id: LogicId,
}

impl CpuRef {
    pub const fn invalid() -> Self {
        Self {
            logical_id: LogicId::INVALID,
        }
    }

    pub const fn new(logical_id: usize) -> Self {
        match LogicId::new(logical_id) {
            Some(logical_id) => Self { logical_id },
            None => Self::invalid(),
        }
    }

    pub const fn logic_id(self) -> LogicId {
        self.logical_id
    }

    pub const fn logical_id(self) -> usize {
        if self.logical_id.is_valid() {
            self.logical_id.get()
        } else {
            usize::MAX
        }
    }

    pub const fn is_valid(self) -> bool {
        self.logical_id.is_valid()
    }

    pub const fn is_boot_cpu(self) -> bool {
        self.logical_id.get() == BOOT_CPU_LOGICAL_ID
    }
}

pub struct Cpu {
    lifecycle: Lifecycle,
    logical_id: LogicId,
    hartid: usize,
    role: CpuRole,
    possible: bool,
    present: bool,
    active: bool,
    online: bool,
    local_interrupt: LocalInterruptControl,
}

impl Cpu {
    pub const fn boot() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            logical_id: LogicId(BOOT_CPU_LOGICAL_ID as u16),
            hartid: usize::MAX,
            role: CpuRole::Boot,
            possible: false,
            present: false,
            active: false,
            online: false,
            local_interrupt: LocalInterruptControl::new(),
        }
    }

    pub const fn secondary(logical_id: LogicId, hartid: usize) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Ready),
            logical_id,
            hartid,
            role: CpuRole::Secondary,
            possible: true,
            present: true,
            active: false,
            online: false,
            local_interrupt: LocalInterruptControl::new(),
        }
    }

    pub fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        if self.role != CpuRole::Boot || self.logical_id.get() != BOOT_CPU_LOGICAL_ID {
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

    pub const fn cpu_ref(&self) -> CpuRef {
        CpuRef {
            logical_id: self.logical_id,
        }
    }

    pub const fn logical_id(&self) -> usize {
        self.logical_id.get()
    }

    pub const fn hartid(&self) -> usize {
        self.hartid
    }

    pub const fn role(&self) -> CpuRole {
        self.role
    }

    pub const fn is_possible(&self) -> bool {
        self.possible
    }

    pub const fn is_present(&self) -> bool {
        self.present
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub const fn is_online(&self) -> bool {
        self.online
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn local_interrupt(&self) -> &LocalInterruptControl {
        &self.local_interrupt
    }

    pub fn local_interrupt_mut(&mut self) -> &mut LocalInterruptControl {
        &mut self.local_interrupt
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuRole {
    Boot,
    Secondary,
}
