#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskEntry {
    None,
    KernelInit,
    Kthreadd,
    UserChild,
    SmokeScheduler,
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
