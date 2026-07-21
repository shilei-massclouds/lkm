use core::cell::UnsafeCell;

use crate::{
    arch::riscv64::{csr, task_switch::TaskSwitchContext},
    checkpoint::Checkpoint,
};

use super::{
    kernel_image::KernelImage,
    state::{EventResult, LifecycleEvent, State, failed_condition},
    task::{Task, TaskEntry, TaskKind, TaskRef},
    vm::Vm,
};

/// Role metadata for the linker-visible PID 0 Task carrier.
///
/// The lifecycle, PID, CPU and switch context live only in
/// `init_task_storage`; this wrapper never mirrors them.
pub struct BootTask {
    idle_role_bound: bool,
}

#[allow(dead_code)]
impl BootTask {
    pub const fn new() -> Self {
        Self {
            idle_role_bound: false,
        }
    }

    fn task_ptr() -> *mut Task {
        init_task_storage.0.get()
    }

    pub fn carrier_address(&self) -> usize {
        Self::task_ptr() as usize
    }

    pub const fn task_ref(&self) -> TaskRef {
        TaskRef::BOOT
    }

    pub fn pid(&self) -> usize {
        Self::canonical_pid()
    }

    pub fn cpu_id(&self) -> usize {
        Self::canonical_cpu_id()
    }

    pub fn idle_role_bound(&self) -> bool {
        self.idle_role_bound
    }

    pub fn adopt_head_preset(&mut self, kernel_image: &KernelImage) -> EventResult {
        let Some(init_task_phys) = kernel_image.runtime_to_phys(Self::task_ptr() as usize) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.state(),
                State::Base,
                State::Prepared,
            );
        };

        if csr::read_tp() != init_task_phys {
            return failed_condition(
                LifecycleEvent::Preset,
                self.state(),
                State::Base,
                State::Prepared,
            );
        }

        let task = unsafe { &mut *Self::task_ptr() };
        task.set_identity_metadata(0, TaskEntry::None, TaskKind::Idle)?;
        task.adopt_preset()
    }

    pub fn enable(&mut self, kernel_image: &KernelImage, vm: &Vm) -> EventResult {
        let Some(init_task_virt) = kernel_image.runtime_to_link(Self::task_ptr() as usize) else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.state(),
                State::Prepared,
                State::Online,
            );
        };

        if self.state() != State::Prepared || vm.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.state(),
                State::Prepared,
                State::Online,
            );
        }

        csr::write_tp(init_task_virt);
        unsafe { &mut *Self::task_ptr() }.enable_from_prepared(Checkpoint::BootTaskOnline)
    }

    pub fn bind_idle_metadata(&mut self, cpu_id: usize) -> EventResult {
        let task = unsafe { &mut *Self::task_ptr() };
        if task.state() != State::Online
            || task.task_ref() != TaskRef::BOOT
            || task.pid() != 0
            || !task.set_task_cpu(cpu_id)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                task.state(),
                State::Online,
                State::Online,
            );
        }
        task.set_role_metadata(TaskEntry::BootIdle, TaskKind::Idle);
        task.init_dummy_switch_context();
        self.idle_role_bound = true;
        Ok(())
    }

    pub fn state(&self) -> State {
        unsafe { (*Self::task_ptr()).state() }
    }

    pub fn switch_context(&self) -> &TaskSwitchContext {
        Self::canonical_switch_context()
    }

    pub fn switch_context_mut(&mut self) -> &mut TaskSwitchContext {
        Self::canonical_switch_context_mut()
    }

    pub fn task(&self) -> &Task {
        unsafe { &*Self::task_ptr() }
    }

    pub fn task_mut(&mut self) -> &mut Task {
        unsafe { &mut *Self::task_ptr() }
    }

    pub(crate) fn canonical_pid() -> usize {
        unsafe { (*Self::task_ptr()).pid() }
    }

    pub(crate) fn canonical_cpu_id() -> usize {
        unsafe { (*Self::task_ptr()).cpu_id() }
    }

    pub(crate) fn canonical_switch_context() -> &'static TaskSwitchContext {
        unsafe { (*Self::task_ptr()).switch_context() }
    }

    pub(crate) fn canonical_switch_context_mut() -> &'static mut TaskSwitchContext {
        unsafe { (*Self::task_ptr()).switch_context_mut() }
    }
}

#[repr(transparent)]
pub struct InitTaskStorage(UnsafeCell<Task>);

unsafe impl Sync for InitTaskStorage {}

#[unsafe(no_mangle)]
pub static init_task_storage: InitTaskStorage =
    InitTaskStorage(UnsafeCell::new(Task::with_ref(TaskRef::BOOT)));
