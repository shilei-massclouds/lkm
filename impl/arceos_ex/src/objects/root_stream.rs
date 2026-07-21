use crate::arch::riscv64::csr;

use super::{
    boot_task::BootTask,
    state::{EventResult, LifecycleEvent, State, failed_condition},
    task::{TaskFlow, TaskFlowRef},
};

pub struct RootStream {
    flow: TaskFlow,
}

impl RootStream {
    pub const fn new() -> Self {
        Self {
            flow: TaskFlow::new_static(TaskFlowRef::ROOT_STREAM),
        }
    }

    pub fn adopt_head_preset(&mut self, boot_task: &mut BootTask) -> EventResult {
        if !csr::kernel_fpu_vector_disabled() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.flow
            .preset(boot_task.task_mut(), TaskFlowRef::NONE, None)?;
        boot_task
            .task_mut()
            .bind_initial_prepared_flow(&mut self.flow)
    }

    pub fn prepare_for_handoff(&mut self, boot_task: &mut BootTask) -> EventResult {
        if self.flow.state() != State::Prepared
            || self.flow.owner() != boot_task.task_ref()
            || !self.flow.active()
            || boot_task.task().active_flow() != self.flow.flow_ref()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.flow.state(),
                State::Prepared,
                State::Offline,
            );
        }
        self.flow.setup(None)?;
        self.flow.enable(boot_task.task(), None)?;
        self.flow.disable(boot_task.task(), None)
    }

    pub fn cleanup_after_handoff(&mut self, boot_task: &mut BootTask) -> EventResult {
        self.flow.cleanup(boot_task.task(), None)?;
        boot_task.task_mut().retire_destroyed_flow(&self.flow)
    }

    pub const fn state(&self) -> State {
        self.flow.state()
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.flow.flow_ref()
    }

    pub const fn active(&self) -> bool {
        self.flow.active()
    }

    pub const fn owner(&self) -> super::task::TaskRef {
        self.flow.owner()
    }

    #[allow(dead_code)]
    pub const fn destroyed(&self) -> bool {
        matches!(self.flow.state(), State::Destroyed)
    }

    pub(crate) const fn core(&self) -> &TaskFlow {
        &self.flow
    }
}
