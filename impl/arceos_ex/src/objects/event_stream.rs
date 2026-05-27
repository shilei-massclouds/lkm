use core::arch::global_asm;

use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    entry_prelude::KernelImage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    vm::Vm,
};

global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl early_event_entry
early_event_entry:
    j early_event_entry_rust

    .section .bss.objects, "aw", @nobits
    .align 3
    .globl bss_anchor
bss_anchor:
    .space 8
"#
);

unsafe extern "C" {
    fn early_event_entry();
}

#[unsafe(no_mangle)]
extern "C" fn early_event_entry_rust() -> ! {
    crate::arch::riscv64::sbi::putstr("arceos_ex early trap\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

pub struct EventStream {
    lifecycle: Lifecycle,
}

impl EventStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn preset(&mut self, kernel_image: &KernelImage) -> EventResult {
        let Some(early_event_entry_phys) = kernel_image.runtime_to_phys(early_event_entry as usize)
        else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        csr::write_stvec(early_event_entry_phys);
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EventStreamPrepared,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn enable(&mut self, vm: &Vm, static_objects: &StaticObjects) -> EventResult {
        let _ = static_objects.state();
        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        }

        csr::write_stvec(early_event_entry as usize);
        csr::clear_sscratch();
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Prepared,
            State::Online,
            Checkpoint::EventStreamOnline,
        )
    }
}
