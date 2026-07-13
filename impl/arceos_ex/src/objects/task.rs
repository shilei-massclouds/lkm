use crate::arch::riscv64::task_switch::TaskSwitchContext;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskEntry {
    None,
    KernelInit,
    Kthreadd,
    UserChild,
    SmokeScheduler,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskKind {
    None,
    UserModeThread,
    KernelThread,
}

pub struct TaskCpuState {
    cpu_id: usize,
}

impl TaskCpuState {
    pub const fn new() -> Self {
        Self { cpu_id: usize::MAX }
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu_id
    }

    pub fn set_task_cpu(&mut self, cpu_id: usize) -> bool {
        if cpu_id == usize::MAX {
            return false;
        }

        self.cpu_id = cpu_id;
        true
    }
}

pub struct Task {
    pub entry: TaskEntry,
    pub pid: usize,
    pub kind: TaskKind,
    pub cpu: TaskCpuState,
    pub running: bool,
    pub switch_ctx: TaskSwitchContext,
}

impl Task {
    pub const fn new() -> Self {
        Self {
            entry: TaskEntry::None,
            pid: 0,
            kind: TaskKind::None,
            cpu: TaskCpuState::new(),
            running: false,
            switch_ctx: TaskSwitchContext::new(),
        }
    }

    pub fn init_switch_context(&mut self, entry: extern "C" fn() -> !, stack_top: usize) {
        let tp = self as *const Task as usize;
        self.switch_ctx.init(entry, stack_top, tp);
    }
}
