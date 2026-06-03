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
    addi    sp, sp, -288
    sd      zero, 0(sp)
    sd      ra, 8(sp)
    sd      gp, 24(sp)
    sd      tp, 32(sp)
    sd      t0, 40(sp)
    addi    t0, sp, 288
    sd      t0, 16(sp)
    sd      t1, 48(sp)
    sd      t2, 56(sp)
    sd      s0, 64(sp)
    sd      s1, 72(sp)
    sd      a0, 80(sp)
    sd      a1, 88(sp)
    sd      a2, 96(sp)
    sd      a3, 104(sp)
    sd      a4, 112(sp)
    sd      a5, 120(sp)
    sd      a6, 128(sp)
    sd      a7, 136(sp)
    sd      s2, 144(sp)
    sd      s3, 152(sp)
    sd      s4, 160(sp)
    sd      s5, 168(sp)
    sd      s6, 176(sp)
    sd      s7, 184(sp)
    sd      s8, 192(sp)
    sd      s9, 200(sp)
    sd      s10, 208(sp)
    sd      s11, 216(sp)
    sd      t3, 224(sp)
    sd      t4, 232(sp)
    sd      t5, 240(sp)
    sd      t6, 248(sp)
    csrr    t0, sstatus
    sd      t0, 256(sp)
    csrr    t0, sepc
    sd      t0, 264(sp)
    csrr    t0, scause
    sd      t0, 272(sp)
    csrr    t0, stval
    sd      t0, 280(sp)

    mv      a0, sp
    call    formal_event_entry_rust

    ld      t0, 264(sp)
    csrw    sepc, t0
    ld      t0, 256(sp)
    csrw    sstatus, t0

    ld      ra, 8(sp)
    ld      gp, 24(sp)
    ld      tp, 32(sp)
    ld      t0, 40(sp)
    ld      t1, 48(sp)
    ld      t2, 56(sp)
    ld      s0, 64(sp)
    ld      s1, 72(sp)
    ld      a0, 80(sp)
    ld      a1, 88(sp)
    ld      a2, 96(sp)
    ld      a3, 104(sp)
    ld      a4, 112(sp)
    ld      a5, 120(sp)
    ld      a6, 128(sp)
    ld      a7, 136(sp)
    ld      s2, 144(sp)
    ld      s3, 152(sp)
    ld      s4, 160(sp)
    ld      s5, 168(sp)
    ld      s6, 176(sp)
    ld      s7, 184(sp)
    ld      s8, 192(sp)
    ld      s9, 200(sp)
    ld      s10, 208(sp)
    ld      s11, 216(sp)
    ld      t3, 224(sp)
    ld      t4, 232(sp)
    ld      t5, 240(sp)
    ld      t6, 248(sp)
    ld      sp, 16(sp)
    sret

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

#[repr(C)]
pub struct TrapFrame {
    regs: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
}

#[unsafe(no_mangle)]
extern "C" fn formal_event_entry_rust(frame: &mut TrapFrame) {
    dispatch_trap(frame)
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

pub fn dispatch_trap(frame: &mut TrapFrame) {
    if frame.scause & SCAUSE_INTERRUPT_BIT != 0 {
        crate::objects::interrupt_stream::dispatch_scause(frame.scause);
    } else {
        crate::objects::exception_stream::dispatch_trap(frame)
    }
}
