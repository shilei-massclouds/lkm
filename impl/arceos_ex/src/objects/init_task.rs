use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    kernel_image::KernelImage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    vm::Vm,
};

pub struct InitTask {
    lifecycle: Lifecycle,
}

impl InitTask {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self, kernel_image: &KernelImage) -> EventResult {
        let Some(init_task_phys) =
            kernel_image.runtime_to_phys(core::ptr::addr_of!(init_task_storage) as usize)
        else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        if csr::read_tp() != init_task_phys {
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

    pub fn enable(&mut self, kernel_image: &KernelImage, vm: &Vm) -> EventResult {
        let Some(init_task_virt) =
            kernel_image.runtime_to_link(core::ptr::addr_of!(init_task_storage) as usize)
        else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        };

        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        }

        csr::write_tp(init_task_virt);
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Prepared,
            State::Online,
            Checkpoint::InitTaskOnline,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

#[repr(C)]
pub struct InitTaskStorage {
    reserved: usize,
}

#[unsafe(no_mangle)]
pub static init_task_storage: InitTaskStorage = InitTaskStorage { reserved: 0 };
