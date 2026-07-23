use super::{
    boot_task::BootTask,
    kernel_image::KernelImage,
    mutex::Mutex,
    percpu_rw_semaphore::{PerCpuRwSemaphore, PerCpuRwSemaphoreReadOutcome},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    vm::Vm,
};
use crate::checkpoint::Checkpoint;

const MAX_STATIC_KEYS: usize = 8;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum StaticKey {
    InitOnAlloc,
    InitOnFree,
    DebugPageAlloc,
    DebugGuardPage,
    CheckPages,
}

#[derive(Clone, Copy)]
struct StaticKeyEntry {
    key: StaticKey,
    enabled: bool,
}

impl StaticKeyEntry {
    const fn new(key: StaticKey) -> Self {
        Self {
            key,
            enabled: false,
        }
    }
}

pub struct StaticBranch {
    lifecycle: Lifecycle,
    entries: [StaticKeyEntry; MAX_STATIC_KEYS],
    count: usize,
    cpu_hotplug_read_guard_used: bool,
    text_patch_sync_deferred: bool,
}

impl StaticBranch {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            entries: [
                StaticKeyEntry::new(StaticKey::InitOnAlloc),
                StaticKeyEntry::new(StaticKey::InitOnFree),
                StaticKeyEntry::new(StaticKey::DebugPageAlloc),
                StaticKeyEntry::new(StaticKey::DebugGuardPage),
                StaticKeyEntry::new(StaticKey::CheckPages),
                StaticKeyEntry::new(StaticKey::InitOnAlloc),
                StaticKeyEntry::new(StaticKey::InitOnAlloc),
                StaticKeyEntry::new(StaticKey::InitOnAlloc),
            ],
            count: 0,
            cpu_hotplug_read_guard_used: false,
            text_patch_sync_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn key_count(&self) -> usize {
        self.count
    }

    pub const fn cpu_hotplug_read_guard_used(&self) -> bool {
        self.cpu_hotplug_read_guard_used
    }

    pub fn cpu_hotplug_read_guard_used_by(&self, cpu_hotplug_lock: &PerCpuRwSemaphore) -> bool {
        self.lifecycle.state() == State::Ready
            && cpu_hotplug_lock.boot_init_task_read_guard_completed()
    }

    pub fn jump_label_mutex_guard_used(&self, jump_label_mutex: &Mutex) -> bool {
        self.lifecycle.state() == State::Ready && jump_label_mutex.boot_init_task_guard_completed()
    }

    pub const fn text_patch_sync_deferred(&self) -> bool {
        self.text_patch_sync_deferred
    }

    pub fn setup(
        &mut self,
        kernel_image: &KernelImage,
        vm: &Vm,
        cpu_hotplug_lock: &mut PerCpuRwSemaphore,
        jump_label_mutex: &mut Mutex,
        boot_task: &BootTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_image.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || !cpu_hotplug_lock.ready()
            || !jump_label_mutex.ready()
            || boot_task.state() != State::OnCpu
        {
            return self.failed_setup();
        }

        /*
         * CpuHotplugReadContext:
         * CpuHotplugLock.ReadLock(BootTaskRef) preserves the
         * cpus_read_lock() boundary even though the surrounding
         * SingleTaskContext already contributes single CPU/task facts.
         */
        if cpu_hotplug_lock.read_lock_owner(
            super::percpu_rw_semaphore::PerCpuRwSemaphoreOwner::BootTask,
            0,
        )? != PerCpuRwSemaphoreReadOutcome::AcquiredFast
        {
            return self.failed_setup();
        }

        /*
         * StaticBranchJumpLabelContext:
         * JumpLabelMutex.Lock(BootTaskRef) preserves the jump_label_lock()
         * boundary for setup.
         */
        jump_label_mutex.lock_boot_init_task(boot_task)?;
        self.entries = [
            StaticKeyEntry::new(StaticKey::InitOnAlloc),
            StaticKeyEntry::new(StaticKey::InitOnFree),
            StaticKeyEntry::new(StaticKey::DebugPageAlloc),
            StaticKeyEntry::new(StaticKey::DebugGuardPage),
            StaticKeyEntry::new(StaticKey::CheckPages),
            StaticKeyEntry::new(StaticKey::InitOnAlloc),
            StaticKeyEntry::new(StaticKey::InitOnAlloc),
            StaticKeyEntry::new(StaticKey::InitOnAlloc),
        ];
        self.count = 5;
        self.cpu_hotplug_read_guard_used = true;
        self.text_patch_sync_deferred = true;
        /*
         * StaticBranchJumpLabelContext:
         * JumpLabelMutex.Unlock(BootTaskRef) exits the same modeled mutex
         * boundary.
         */
        jump_label_mutex.unlock_boot_init_task(boot_task)?;
        /*
         * CpuHotplugReadContext:
         * CpuHotplugLock.ReadUnlock(BootTaskRef) exits the same modeled
         * read-side boundary.
         */
        cpu_hotplug_lock.read_unlock_owner(
            super::percpu_rw_semaphore::PerCpuRwSemaphoreOwner::BootTask,
            0,
        )?;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::StaticBranchReady,
        )
    }

    pub fn set(&mut self, key: StaticKey, enabled: bool) -> EventResult {
        if self.lifecycle.state() != State::Ready && self.lifecycle.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        let mut index = 0usize;
        while index < self.count {
            if self.entries[index].key == key {
                self.entries[index].enabled = enabled;
                return Ok(());
            }
            index += 1;
        }

        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Ready,
            State::Ready,
        )
    }

    pub fn enabled(&self, key: StaticKey) -> Option<bool> {
        let mut index = 0usize;
        while index < self.count {
            if self.entries[index].key == key {
                return Some(self.entries[index].enabled);
            }
            index += 1;
        }
        None
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
