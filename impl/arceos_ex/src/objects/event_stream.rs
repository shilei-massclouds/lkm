use core::arch::global_asm;

use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    exception_stream::ExceptionStream,
    interrupt_stream::InterruptStream,
    kernel_image::KernelImage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    vm::Vm,
};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);

global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl early_event_entry
early_event_entry:
    j early_event_entry_rust

    .globl formal_event_entry
formal_event_entry:
    j formal_event_entry_rust

    .section .bss.objects, "aw", @nobits
    .align 3
    .globl bss_anchor
bss_anchor:
    .space 8
"#
);

unsafe extern "C" {
    fn early_event_entry();
    fn formal_event_entry();
}

#[unsafe(no_mangle)]
extern "C" fn early_event_entry_rust() -> ! {
    crate::arch::riscv64::sbi::putstr("arceos_ex early trap\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

#[unsafe(no_mangle)]
extern "C" fn formal_event_entry_rust() -> ! {
    let scause = csr::read_scause();
    dispatch_scause(scause)
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

    pub fn setup(
        &mut self,
        vm: &Vm,
        static_objects: &StaticObjects,
        exception_stream: &ExceptionStream,
        interrupt_stream: &InterruptStream,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || vm.state() != State::Ready
            || static_objects.state() != State::Online
            || exception_stream.state() != State::Prepared
            || interrupt_stream.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        csr::write_stvec(formal_event_entry as usize);
        csr::clear_sscratch();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::EventStreamReady,
        )
    }
}

pub fn dispatch_scause(scause: usize) -> ! {
    if scause & SCAUSE_INTERRUPT_BIT != 0 {
        crate::objects::interrupt_stream::dispatch_scause(scause)
    } else {
        crate::objects::exception_stream::dispatch_scause(scause)
    }
}
