//! Generation-checked user process identity and fixed-CPU ownership registry.

use super::{
    cpu::{CpuRef, MAX_CPUS},
    cpu_group::CpuGroup,
    irq_spinlock::IrqSpinLock,
    task::TaskRef,
};

pub const USER_PROCESS_SLOT_COUNT: usize = 32;
pub const DYNAMIC_USER_PROCESS_SLOT_COUNT: usize = USER_PROCESS_SLOT_COUNT - 1;

const _: () = {
    assert!(DYNAMIC_USER_PROCESS_SLOT_COUNT == super::task::USER_TASK_SLOT_COUNT);
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessSlotState {
    Empty,
    Reserved,
    Published,
    Zombie,
    Reaping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessChildSelector {
    Any,
    ExactPid(usize),
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg(checkpoint_handler_user_syscall_error)]
pub(crate) struct UserProcessWaitDiagnostic {
    pub(crate) live_slot_count: usize,
    pub(crate) exact_child_count: usize,
    pub(crate) same_parent_ref_count: usize,
    pub(crate) same_parent_pid_count: usize,
    pub(crate) selector_candidate_count: usize,
    pub(crate) related_task_ref: TaskRef,
    pub(crate) related_pid: usize,
    pub(crate) related_parent_task_ref: TaskRef,
    pub(crate) related_parent_pid: usize,
    pub(crate) related_cpu_ref: CpuRef,
    pub(crate) related_state: UserProcessSlotState,
    pub(crate) related_scheduler_quiesced: bool,
}

impl UserProcessChildSelector {
    pub const fn matches(self, pid: usize) -> bool {
        match self {
            Self::Any => true,
            Self::ExactPid(expected) => expected == pid,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessSetSidResult {
    Updated(usize),
    PermissionDenied,
    InvalidCurrent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessSetPgidResult {
    Updated(usize),
    InvalidArgument,
    NoSuchProcess,
    PermissionDenied,
    InvalidCurrent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserProcessJobControlError {
    InvalidArgument,
    NoSuchProcess,
    PermissionDenied,
    NotControllingTty,
    InvalidCurrent,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct UserProcessSlot {
    state: UserProcessSlotState,
    generation: u32,
    task_ref: TaskRef,
    pid: usize,
    parent_task_ref: TaskRef,
    parent_pid: usize,
    process_group: usize,
    session_id: usize,
    session_leader: bool,
    controlling_tty_bound: bool,
    cpu_ref: CpuRef,
    resources_ready: bool,
    inbox_reserved: bool,
    lease_count: usize,
    wait_status: usize,
    scheduler_quiesced: bool,
}

impl UserProcessSlot {
    const fn empty() -> Self {
        Self {
            state: UserProcessSlotState::Empty,
            generation: 0,
            task_ref: TaskRef::NONE,
            pid: 0,
            parent_task_ref: TaskRef::NONE,
            parent_pid: 0,
            process_group: 0,
            session_id: 0,
            session_leader: false,
            controlling_tty_bound: false,
            cpu_ref: CpuRef::invalid(),
            resources_ready: false,
            inbox_reserved: false,
            lease_count: 0,
            wait_status: 0,
            scheduler_quiesced: false,
        }
    }
}

struct UserProcessRegistryInner {
    ready: bool,
    next_pid: usize,
    online_cpus: [CpuRef; MAX_CPUS],
    online_hartids: [usize; MAX_CPUS],
    online_cpu_count: usize,
    tty_session_id: usize,
    tty_foreground_process_group: usize,
    slots: [UserProcessSlot; USER_PROCESS_SLOT_COUNT],
}

impl UserProcessRegistryInner {
    const fn new() -> Self {
        Self {
            ready: false,
            next_pid: super::user_boot::USER_CHILD_PID,
            online_cpus: [CpuRef::invalid(); MAX_CPUS],
            online_hartids: [usize::MAX; MAX_CPUS],
            online_cpu_count: 0,
            tty_session_id: 0,
            tty_foreground_process_group: 0,
            slots: [UserProcessSlot::empty(); USER_PROCESS_SLOT_COUNT],
        }
    }
}

pub struct UserProcessRegistry {
    inner: IrqSpinLock<UserProcessRegistryInner>,
}

static USER_PROCESS_REGISTRY: UserProcessRegistry = UserProcessRegistry::new();

pub fn global_registry() -> &'static UserProcessRegistry {
    &USER_PROCESS_REGISTRY
}

impl UserProcessRegistry {
    pub const fn new() -> Self {
        Self {
            inner: IrqSpinLock::new(UserProcessRegistryInner::new()),
        }
    }

    pub fn setup(&self, cpu_group: Option<&CpuGroup>) -> bool {
        let mut inner = self.inner.lock();
        if inner.ready {
            return false;
        }
        let mut count = 0usize;
        if let Some(cpu_group) = cpu_group {
            let mut logical_id = 0usize;
            while logical_id < MAX_CPUS {
                let cpu_ref = CpuRef::new(logical_id);
                if cpu_group.online_contains(cpu_ref) {
                    let Some(cpu) = cpu_group.dereference(cpu_ref) else {
                        return false;
                    };
                    inner.online_cpus[count] = cpu_ref;
                    inner.online_hartids[count] = cpu.hartid();
                    count += 1;
                }
                logical_id += 1;
            }
        }
        if count == 0 {
            inner.online_cpus[0] = CpuRef::new(0);
            inner.online_hartids[0] = 0;
            count = 1;
        }
        if inner.online_cpus[0] != CpuRef::new(0) {
            return false;
        }
        inner.online_cpu_count = count;
        inner.tty_session_id = 1;
        inner.tty_foreground_process_group = 1;
        inner.slots[0] = UserProcessSlot {
            state: UserProcessSlotState::Published,
            generation: 1,
            task_ref: TaskRef::KERNEL_INIT,
            pid: 1,
            parent_task_ref: TaskRef::NONE,
            parent_pid: 0,
            process_group: 1,
            session_id: 1,
            session_leader: true,
            controlling_tty_bound: true,
            cpu_ref: CpuRef::new(0),
            resources_ready: true,
            inbox_reserved: false,
            lease_count: 0,
            wait_status: 0,
            scheduler_quiesced: false,
        };
        inner.ready = true;
        true
    }

    pub fn reserve(
        &self,
        pid: usize,
        parent_task_ref: TaskRef,
        parent_pid: usize,
        parent_cpu: CpuRef,
        same_cpu: bool,
    ) -> Option<(TaskRef, CpuRef)> {
        let mut inner = self.inner.lock();
        if !inner.ready {
            return None;
        }
        reserve_locked(
            &mut inner,
            pid,
            parent_task_ref,
            parent_pid,
            parent_cpu,
            same_cpu,
        )
    }

    pub fn reserve_next(
        &self,
        parent_task_ref: TaskRef,
        parent_pid: usize,
        parent_cpu: CpuRef,
        same_cpu: bool,
    ) -> Option<(usize, TaskRef, CpuRef)> {
        let mut inner = self.inner.lock();
        if !inner.ready {
            return None;
        }
        let pid = inner.next_pid;
        let (task_ref, cpu_ref) = reserve_locked(
            &mut inner,
            pid,
            parent_task_ref,
            parent_pid,
            parent_cpu,
            same_cpu,
        )?;
        Some((pid, task_ref, cpu_ref))
    }

    pub fn next_pid(&self) -> Option<usize> {
        let inner = self.inner.lock();
        inner.ready.then_some(inner.next_pid)
    }

    pub fn mark_prepared(&self, task_ref: TaskRef, inbox_reserved: bool) -> bool {
        let mut inner = self.inner.lock();
        let Some(slot) = slot_mut(&mut inner, task_ref) else {
            return false;
        };
        if slot.state != UserProcessSlotState::Reserved {
            return false;
        }
        slot.resources_ready = true;
        slot.inbox_reserved = inbox_reserved;
        true
    }

    pub fn publish(&self, task_ref: TaskRef) -> bool {
        let mut inner = self.inner.lock();
        let Some(slot) = slot_mut(&mut inner, task_ref) else {
            return false;
        };
        if slot.state != UserProcessSlotState::Reserved
            || !slot.resources_ready
            || !slot.inbox_reserved
        {
            return false;
        }
        slot.state = UserProcessSlotState::Published;
        true
    }

    pub fn rollback(&self, task_ref: TaskRef) -> bool {
        let mut inner = self.inner.lock();
        let Some(index) = registry_slot_for_ref(task_ref) else {
            return false;
        };
        let slot = &mut inner.slots[index];
        if slot.state != UserProcessSlotState::Reserved || slot.task_ref != task_ref {
            return false;
        }
        let generation = slot.generation;
        let pid = slot.pid;
        *slot = UserProcessSlot::empty();
        slot.generation = generation;
        if pid.checked_add(1) == Some(inner.next_pid) {
            inner.next_pid = pid;
        }
        true
    }

    #[allow(dead_code)]
    pub fn acquire(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> Option<UserProcessLease<'_>> {
        let mut inner = self.inner.lock();
        let slot = slot_mut(&mut inner, task_ref)?;
        if !matches!(
            slot.state,
            UserProcessSlotState::Published | UserProcessSlotState::Zombie
        ) || slot.cpu_ref != cpu_ref
        {
            return None;
        }
        slot.lease_count = slot.lease_count.checked_add(1)?;
        Some(UserProcessLease {
            registry: self,
            task_ref,
            cpu_ref,
        })
    }

    pub fn cpu_ref(&self, task_ref: TaskRef) -> Option<CpuRef> {
        let inner = self.inner.lock();
        let index = registry_slot_for_ref(task_ref)?;
        let slot = &inner.slots[index];
        (slot.task_ref == task_ref && slot.state != UserProcessSlotState::Empty)
            .then_some(slot.cpu_ref)
    }

    pub fn pid(&self, task_ref: TaskRef) -> Option<usize> {
        let inner = self.inner.lock();
        let index = registry_slot_for_ref(task_ref)?;
        let slot = &inner.slots[index];
        (slot.task_ref.same_identity(task_ref) && slot.state != UserProcessSlotState::Empty)
            .then_some(slot.pid)
    }

    pub fn lookup_published_pid(&self, pid: usize) -> Option<(TaskRef, CpuRef)> {
        let inner = self.inner.lock();
        if !inner.ready || pid == 0 {
            return None;
        }
        let mut index = 0usize;
        while index < USER_PROCESS_SLOT_COUNT {
            let slot = &inner.slots[index];
            if slot.state == UserProcessSlotState::Published && slot.pid == pid {
                return Some((slot.task_ref, slot.cpu_ref));
            }
            index += 1;
        }
        None
    }

    pub fn parent_pid(&self, task_ref: TaskRef) -> Option<usize> {
        let inner = self.inner.lock();
        let index = registry_slot_for_ref(task_ref)?;
        let slot = &inner.slots[index];
        (slot.task_ref.same_identity(task_ref)
            && matches!(
                slot.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            ))
        .then_some(slot.parent_pid)
    }

    pub fn parent_task_ref(&self, task_ref: TaskRef) -> Option<TaskRef> {
        let inner = self.inner.lock();
        let index = registry_slot_for_ref(task_ref)?;
        let slot = &inner.slots[index];
        (slot.task_ref.same_identity(task_ref)
            && matches!(
                slot.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            ))
        .then_some(slot.parent_task_ref)
    }

    pub(crate) fn wait_status(&self, task_ref: TaskRef) -> Option<usize> {
        let inner = self.inner.lock();
        let index = registry_slot_for_ref(task_ref)?;
        let slot = &inner.slots[index];
        (slot.task_ref.same_identity(task_ref) && slot.state == UserProcessSlotState::Zombie)
            .then_some(slot.wait_status)
    }

    /// Resolve the parent recorded by a private fork reservation.
    ///
    /// This is deliberately narrower than normal process lookup: the child
    /// must still be `Reserved`, while its exact parent occurrence must
    /// already be published (or await reap as a zombie).  Callers use the
    /// returned identity only to acquire a parent lease and prepare the
    /// child's unpublished aggregate.
    pub fn reserved_parent_task_ref(&self, task_ref: TaskRef) -> Option<TaskRef> {
        let inner = self.inner.lock();
        let child_index = registry_slot_for_ref(task_ref)?;
        let child = &inner.slots[child_index];
        if child.task_ref != task_ref || child.state != UserProcessSlotState::Reserved {
            return None;
        }
        let parent_index = registry_slot_for_ref(child.parent_task_ref)?;
        let parent = &inner.slots[parent_index];
        (parent.task_ref == child.parent_task_ref
            && parent.pid == child.parent_pid
            && matches!(
                parent.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            ))
        .then_some(parent.task_ref)
    }

    pub fn process_group(&self, current_task_ref: TaskRef, pid_arg: usize) -> Option<usize> {
        self.lookup_process_identity(current_task_ref, pid_arg, |slot| slot.process_group)
    }

    pub fn session_id(&self, current_task_ref: TaskRef, pid_arg: usize) -> Option<usize> {
        self.lookup_process_identity(current_task_ref, pid_arg, |slot| slot.session_id)
    }

    pub fn set_process_group(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
        pid_arg: usize,
        process_group_arg: usize,
    ) -> UserProcessSetPgidResult {
        let pid_arg = pid_arg as u32 as i32;
        let process_group_arg = process_group_arg as u32 as i32;
        if process_group_arg < 0 {
            return UserProcessSetPgidResult::InvalidArgument;
        }
        if pid_arg < 0 {
            return UserProcessSetPgidResult::NoSuchProcess;
        }

        let mut inner = self.inner.lock();
        let Some(current_index) = current_published_slot_index(&inner, task_ref, cpu_ref) else {
            return UserProcessSetPgidResult::InvalidCurrent;
        };
        let current_pid = inner.slots[current_index].pid;
        let current_session_id = inner.slots[current_index].session_id;
        if current_pid == 0 || current_session_id == 0 {
            return UserProcessSetPgidResult::InvalidCurrent;
        }
        let target_pid = if pid_arg == 0 {
            current_pid
        } else {
            pid_arg as usize
        };
        let Some(target_index) = inner.slots.iter().position(|slot| {
            slot.state == UserProcessSlotState::Published && slot.pid == target_pid
        }) else {
            return UserProcessSetPgidResult::NoSuchProcess;
        };
        let target = &inner.slots[target_index];
        let target_is_current = target_index == current_index;
        let target_is_child =
            target.parent_task_ref.same_identity(task_ref) && target.parent_pid == current_pid;
        if !target_is_current || target.pid != current_pid {
            if !target_is_child {
                return UserProcessSetPgidResult::NoSuchProcess;
            }
            if target.session_id != current_session_id {
                return UserProcessSetPgidResult::PermissionDenied;
            }
        }
        if target.session_leader {
            return UserProcessSetPgidResult::PermissionDenied;
        }

        let process_group = if process_group_arg == 0 {
            target_pid
        } else {
            process_group_arg as usize
        };
        if process_group != target_pid {
            let group_in_current_session = inner.slots.iter().any(|slot| {
                matches!(
                    slot.state,
                    UserProcessSlotState::Published | UserProcessSlotState::Zombie
                ) && slot.process_group == process_group
                    && slot.session_id == current_session_id
            });
            if !group_in_current_session {
                return UserProcessSetPgidResult::PermissionDenied;
            }
        }

        inner.slots[target_index].process_group = process_group;
        UserProcessSetPgidResult::Updated(process_group)
    }

    pub fn set_session_id(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> UserProcessSetSidResult {
        let mut inner = self.inner.lock();
        let Some(index) = registry_slot_for_ref(task_ref) else {
            return UserProcessSetSidResult::InvalidCurrent;
        };
        let current = &inner.slots[index];
        if !current.task_ref.same_identity(task_ref)
            || current.state != UserProcessSlotState::Published
            || current.cpu_ref != cpu_ref
            || current.pid == 0
            || current.session_id == 0
            || current.process_group == 0
        {
            return UserProcessSetSidResult::InvalidCurrent;
        }
        let pid = current.pid;
        if current.session_leader
            || inner.slots.iter().any(|slot| {
                matches!(
                    slot.state,
                    UserProcessSlotState::Published | UserProcessSlotState::Zombie
                ) && slot.process_group == pid
            })
        {
            return UserProcessSetSidResult::PermissionDenied;
        }

        let current = &mut inner.slots[index];
        current.process_group = pid;
        current.session_id = pid;
        current.session_leader = true;
        current.controlling_tty_bound = false;
        UserProcessSetSidResult::Updated(pid)
    }

    pub fn bind_controlling_tty(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
        arg: usize,
    ) -> Result<usize, UserProcessJobControlError> {
        let mut inner = self.inner.lock();
        let index = current_published_slot_index(&inner, task_ref, cpu_ref)
            .ok_or(UserProcessJobControlError::InvalidCurrent)?;
        let current = &inner.slots[index];
        if current.session_id == 0 || current.process_group == 0 {
            return Err(UserProcessJobControlError::InvalidCurrent);
        }
        let session_id = current.session_id;
        let process_group = current.process_group;
        if inner.tty_session_id == session_id {
            inner.slots[index].controlling_tty_bound = true;
            return Ok(session_id);
        }
        if !current.session_leader || current.controlling_tty_bound {
            return Err(UserProcessJobControlError::PermissionDenied);
        }
        if inner.tty_session_id != 0 && arg != 1 {
            return Err(UserProcessJobControlError::PermissionDenied);
        }

        let old_session_id = inner.tty_session_id;
        if old_session_id != 0 {
            for slot in &mut inner.slots {
                if matches!(
                    slot.state,
                    UserProcessSlotState::Published | UserProcessSlotState::Zombie
                ) && slot.session_id == old_session_id
                {
                    slot.controlling_tty_bound = false;
                }
            }
        }
        inner.tty_session_id = session_id;
        inner.tty_foreground_process_group = process_group;
        inner.slots[index].controlling_tty_bound = true;
        Ok(session_id)
    }

    pub fn tty_session_id(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> Result<usize, UserProcessJobControlError> {
        let inner = self.inner.lock();
        let index = current_published_slot_index(&inner, task_ref, cpu_ref)
            .ok_or(UserProcessJobControlError::InvalidCurrent)?;
        let current = &inner.slots[index];
        if !current.controlling_tty_bound
            || inner.tty_session_id == 0
            || current.session_id != inner.tty_session_id
        {
            return Err(UserProcessJobControlError::NotControllingTty);
        }
        Ok(inner.tty_session_id)
    }

    pub fn tty_foreground_process_group(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> Result<usize, UserProcessJobControlError> {
        let inner = self.inner.lock();
        let index = current_published_slot_index(&inner, task_ref, cpu_ref)
            .ok_or(UserProcessJobControlError::InvalidCurrent)?;
        let current = &inner.slots[index];
        if !current.controlling_tty_bound
            || inner.tty_session_id == 0
            || current.session_id != inner.tty_session_id
        {
            return Err(UserProcessJobControlError::NotControllingTty);
        }
        Ok(inner.tty_foreground_process_group)
    }

    pub fn set_tty_foreground_process_group(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
        process_group_arg: usize,
    ) -> Result<usize, UserProcessJobControlError> {
        let process_group_arg = process_group_arg as u32 as i32;
        if process_group_arg < 0 {
            return Err(UserProcessJobControlError::InvalidArgument);
        }
        let process_group = process_group_arg as usize;
        let mut inner = self.inner.lock();
        let index = current_published_slot_index(&inner, task_ref, cpu_ref)
            .ok_or(UserProcessJobControlError::InvalidCurrent)?;
        let current = &inner.slots[index];
        if !current.controlling_tty_bound
            || inner.tty_session_id == 0
            || current.session_id != inner.tty_session_id
        {
            return Err(UserProcessJobControlError::NotControllingTty);
        }
        let current_session_id = current.session_id;
        let Some(target) = inner.slots.iter().find(|slot| {
            matches!(
                slot.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            ) && slot.process_group == process_group
        }) else {
            return Err(UserProcessJobControlError::NoSuchProcess);
        };
        if target.session_id != current_session_id {
            return Err(UserProcessJobControlError::PermissionDenied);
        }
        inner.tty_foreground_process_group = process_group;
        Ok(process_group)
    }

    fn lookup_process_identity(
        &self,
        current_task_ref: TaskRef,
        pid_arg: usize,
        select: impl FnOnce(&UserProcessSlot) -> usize,
    ) -> Option<usize> {
        let pid_arg = pid_arg as u32 as i32;
        if pid_arg < 0 {
            return None;
        }
        let inner = self.inner.lock();
        let current_index = registry_slot_for_ref(current_task_ref)?;
        let current = &inner.slots[current_index];
        if !current.task_ref.same_identity(current_task_ref)
            || !matches!(
                current.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            )
        {
            return None;
        }
        let pid = if pid_arg == 0 {
            current.pid
        } else {
            pid_arg as usize
        };
        let target = inner.slots.iter().find(|slot| {
            matches!(
                slot.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            ) && slot.pid == pid
        })?;
        let value = select(target);
        (value != 0).then_some(value)
    }

    pub fn inbound_target_valid(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> bool {
        let inner = self.inner.lock();
        let Some(index) = registry_slot_for_ref(task_ref) else {
            return false;
        };
        let slot = &inner.slots[index];
        slot.task_ref.same_identity(task_ref)
            && slot.state == UserProcessSlotState::Published
            && slot.cpu_ref == cpu_ref
    }

    pub fn live_target_valid(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> bool {
        let inner = self.inner.lock();
        let Some(index) = registry_slot_for_ref(task_ref) else {
            return false;
        };
        let slot = &inner.slots[index];
        slot.task_ref.same_identity(task_ref)
            && matches!(
                slot.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            )
            && slot.cpu_ref == cpu_ref
    }

    pub fn inbound_target_reservable(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> bool {
        let inner = self.inner.lock();
        let Some(index) = registry_slot_for_ref(task_ref) else {
            return false;
        };
        let slot = &inner.slots[index];
        slot.task_ref.same_identity(task_ref)
            && slot.state == UserProcessSlotState::Reserved
            && slot.cpu_ref == cpu_ref
    }

    pub fn hartid_for_cpu(&self, cpu_ref: CpuRef) -> Option<usize> {
        let inner = self.inner.lock();
        let index = inner.online_cpus[..inner.online_cpu_count]
            .iter()
            .position(|online| *online == cpu_ref)?;
        let hartid = inner.online_hartids[index];
        (hartid != usize::MAX).then_some(hartid)
    }

    pub fn publish_zombie(&self, task_ref: TaskRef, wait_status: usize) -> bool {
        let mut inner = self.inner.lock();
        let Some(slot) = slot_mut(&mut inner, task_ref) else {
            return false;
        };
        if slot.state != UserProcessSlotState::Published {
            return false;
        }
        slot.wait_status = wait_status;
        slot.state = UserProcessSlotState::Zombie;
        true
    }

    /// Publish the terminal CPU-retirement boundary from the selected next
    /// Task's stack. A zombie may be observed before this point, but it cannot
    /// be reclaimed while the old SATP, CurrentTask binding, or kernel stack
    /// may still be live on its owner CPU.
    pub fn publish_scheduler_quiesced(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> Option<CpuRef> {
        let mut inner = self.inner.lock();
        let child_index = registry_slot_for_ref(task_ref)?;
        let child = &inner.slots[child_index];
        if child.task_ref != task_ref
            || child.state != UserProcessSlotState::Zombie
            || child.cpu_ref != cpu_ref
            || child.scheduler_quiesced
        {
            return None;
        }
        let parent_index = registry_slot_for_ref(child.parent_task_ref)?;
        let parent = &inner.slots[parent_index];
        if !parent.task_ref.same_identity(child.parent_task_ref)
            || parent.pid != child.parent_pid
            || !matches!(
                parent.state,
                UserProcessSlotState::Published | UserProcessSlotState::Zombie
            )
        {
            return None;
        };
        let parent_cpu_ref = parent.cpu_ref;
        inner.slots[child_index].scheduler_quiesced = true;
        Some(parent_cpu_ref)
    }

    #[allow(dead_code)]
    pub fn first_zombie_child(
        &self,
        parent_task_ref: TaskRef,
        parent_pid: usize,
        selector: UserProcessChildSelector,
    ) -> Option<(TaskRef, usize, usize)> {
        let inner = self.inner.lock();
        inner.slots[1..]
            .iter()
            .find(|slot| {
                slot.state == UserProcessSlotState::Zombie
                    && slot.scheduler_quiesced
                    && slot.parent_task_ref.same_identity(parent_task_ref)
                    && slot.parent_pid == parent_pid
                    && selector.matches(slot.pid)
            })
            .map(|slot| (slot.task_ref, slot.pid, slot.wait_status))
    }

    pub fn has_child(
        &self,
        parent_task_ref: TaskRef,
        parent_pid: usize,
        selector: UserProcessChildSelector,
    ) -> bool {
        let inner = self.inner.lock();
        inner.slots[1..].iter().any(|slot| {
            slot.state != UserProcessSlotState::Empty
                && slot.parent_task_ref.same_identity(parent_task_ref)
                && slot.parent_pid == parent_pid
                && selector.matches(slot.pid)
        })
    }

    #[cfg(checkpoint_handler_user_syscall_error)]
    pub(crate) fn wait_diagnostic(
        &self,
        parent_task_ref: TaskRef,
        parent_pid: usize,
        selector: UserProcessChildSelector,
    ) -> UserProcessWaitDiagnostic {
        let inner = self.inner.lock();
        let mut diagnostic = UserProcessWaitDiagnostic {
            live_slot_count: 0,
            exact_child_count: 0,
            same_parent_ref_count: 0,
            same_parent_pid_count: 0,
            selector_candidate_count: 0,
            related_task_ref: TaskRef::NONE,
            related_pid: 0,
            related_parent_task_ref: TaskRef::NONE,
            related_parent_pid: 0,
            related_cpu_ref: CpuRef::invalid(),
            related_state: UserProcessSlotState::Empty,
            related_scheduler_quiesced: false,
        };
        for slot in &inner.slots[1..] {
            if slot.state == UserProcessSlotState::Empty {
                continue;
            }
            diagnostic.live_slot_count += 1;
            let same_parent_ref = slot.parent_task_ref.same_identity(parent_task_ref);
            let same_parent_pid = slot.parent_pid == parent_pid;
            let selector_matches = selector.matches(slot.pid);
            diagnostic.same_parent_ref_count += usize::from(same_parent_ref);
            diagnostic.same_parent_pid_count += usize::from(same_parent_pid);
            diagnostic.selector_candidate_count += usize::from(selector_matches);
            diagnostic.exact_child_count +=
                usize::from(same_parent_ref && same_parent_pid && selector_matches);
            if !diagnostic.related_task_ref.is_valid()
                && selector_matches
                && (same_parent_ref || same_parent_pid)
            {
                diagnostic.related_task_ref = slot.task_ref;
                diagnostic.related_pid = slot.pid;
                diagnostic.related_parent_task_ref = slot.parent_task_ref;
                diagnostic.related_parent_pid = slot.parent_pid;
                diagnostic.related_cpu_ref = slot.cpu_ref;
                diagnostic.related_state = slot.state;
                diagnostic.related_scheduler_quiesced = slot.scheduler_quiesced;
            }
        }
        diagnostic
    }

    pub fn first_published_child_on_cpu(
        &self,
        parent_task_ref: TaskRef,
        parent_pid: usize,
        cpu_ref: CpuRef,
        selector: UserProcessChildSelector,
    ) -> Option<TaskRef> {
        let inner = self.inner.lock();
        inner.slots[1..]
            .iter()
            .find(|slot| {
                slot.state == UserProcessSlotState::Published
                    && slot.parent_task_ref.same_identity(parent_task_ref)
                    && slot.parent_pid == parent_pid
                    && slot.cpu_ref == cpu_ref
                    && selector.matches(slot.pid)
            })
            .map(|slot| slot.task_ref)
    }

    pub fn reap(&self, task_ref: TaskRef) -> bool {
        let mut inner = self.inner.lock();
        let Some(index) = registry_slot_for_ref(task_ref) else {
            return false;
        };
        let slot = &mut inner.slots[index];
        if slot.task_ref != task_ref
            || slot.state != UserProcessSlotState::Zombie
            || slot.lease_count != 0
            || !slot.scheduler_quiesced
        {
            return false;
        }
        slot.state = UserProcessSlotState::Reaping;
        let generation = slot.generation;
        *slot = UserProcessSlot::empty();
        slot.generation = generation;
        true
    }

    #[allow(dead_code)]
    pub fn free_slot_count(&self) -> usize {
        let inner = self.inner.lock();
        inner.slots[1..]
            .iter()
            .filter(|slot| slot.state == UserProcessSlotState::Empty)
            .count()
    }

    pub fn slot_state(&self, task_ref: TaskRef) -> Option<UserProcessSlotState> {
        let inner = self.inner.lock();
        let index = registry_slot_for_ref(task_ref)?;
        let slot = &inner.slots[index];
        (slot.task_ref == task_ref).then_some(slot.state)
    }

    #[allow(dead_code)]
    fn release_lease(&self, task_ref: TaskRef, cpu_ref: CpuRef) {
        let mut inner = self.inner.lock();
        let slot = slot_mut(&mut inner, task_ref).expect("live user process lease");
        assert!(slot.cpu_ref == cpu_ref);
        assert!(slot.lease_count != 0);
        slot.lease_count -= 1;
    }
}

#[allow(dead_code)]
pub struct UserProcessLease<'a> {
    registry: &'a UserProcessRegistry,
    task_ref: TaskRef,
    cpu_ref: CpuRef,
}

#[allow(dead_code)]
impl UserProcessLease<'_> {
    pub const fn task_ref(&self) -> TaskRef {
        self.task_ref
    }

    pub const fn cpu_ref(&self) -> CpuRef {
        self.cpu_ref
    }
}

impl Drop for UserProcessLease<'_> {
    fn drop(&mut self) {
        self.registry.release_lease(self.task_ref, self.cpu_ref);
    }
}

fn registry_slot_for_ref(task_ref: TaskRef) -> Option<usize> {
    if task_ref.same_identity(TaskRef::KERNEL_INIT) {
        Some(0)
    } else {
        task_ref.user_slot().map(|slot| slot + 1)
    }
}

fn current_published_slot_index(
    inner: &UserProcessRegistryInner,
    task_ref: TaskRef,
    cpu_ref: CpuRef,
) -> Option<usize> {
    let index = registry_slot_for_ref(task_ref)?;
    let slot = &inner.slots[index];
    (slot.task_ref.same_identity(task_ref)
        && slot.state == UserProcessSlotState::Published
        && slot.cpu_ref == cpu_ref)
        .then_some(index)
}

fn reserve_locked(
    inner: &mut UserProcessRegistryInner,
    pid: usize,
    parent_task_ref: TaskRef,
    parent_pid: usize,
    parent_cpu: CpuRef,
    same_cpu: bool,
) -> Option<(TaskRef, CpuRef)> {
    if pid == 0
        || inner.online_cpu_count == 0
        || inner
            .slots
            .iter()
            .any(|slot| slot.state != UserProcessSlotState::Empty && slot.pid == pid)
    {
        return None;
    }
    let registry_slot = (1..USER_PROCESS_SLOT_COUNT)
        .find(|slot| inner.slots[*slot].state == UserProcessSlotState::Empty)?;
    let parent_index = registry_slot_for_ref(parent_task_ref)?;
    let parent = &inner.slots[parent_index];
    if !parent.task_ref.same_identity(parent_task_ref)
        || parent.pid != parent_pid
        || !matches!(
            parent.state,
            UserProcessSlotState::Published | UserProcessSlotState::Zombie
        )
        || parent.process_group == 0
        || parent.session_id == 0
    {
        return None;
    }
    let process_group = parent.process_group;
    let session_id = parent.session_id;
    let controlling_tty_bound = parent.controlling_tty_bound;
    let generation = super::next_generation(inner.slots[registry_slot].generation);
    let cpu_ref = if same_cpu {
        if !inner.online_cpus[..inner.online_cpu_count].contains(&parent_cpu) {
            return None;
        }
        parent_cpu
    } else {
        inner.online_cpus[pid % inner.online_cpu_count]
    };
    let next_pid = if pid == inner.next_pid {
        pid.checked_add(1)?
    } else {
        inner.next_pid
    };
    let task_ref = TaskRef::user(registry_slot - 1, generation);
    inner.slots[registry_slot] = UserProcessSlot {
        state: UserProcessSlotState::Reserved,
        generation,
        task_ref,
        pid,
        parent_task_ref,
        parent_pid,
        process_group,
        session_id,
        session_leader: false,
        controlling_tty_bound,
        cpu_ref,
        resources_ready: false,
        inbox_reserved: false,
        lease_count: 0,
        wait_status: 0,
        scheduler_quiesced: false,
    };
    inner.next_pid = next_pid;
    Some((task_ref, cpu_ref))
}

fn slot_mut(
    inner: &mut UserProcessRegistryInner,
    task_ref: TaskRef,
) -> Option<&mut UserProcessSlot> {
    let index = registry_slot_for_ref(task_ref)?;
    let slot = &mut inner.slots[index];
    (slot.task_ref == task_ref).then_some(slot)
}
