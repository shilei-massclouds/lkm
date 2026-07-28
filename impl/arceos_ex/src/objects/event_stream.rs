use core::arch::global_asm;

use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    exception_stream::ExceptionStream,
    interrupt_stream::InterruptStream,
    kernel_image::KernelImage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
    vm::Vm,
};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);
pub const TRAP_FRAME_SIZE: usize = 36 * core::mem::size_of::<usize>();
#[cfg_attr(not(any(app_smoke, app_user_boot)), allow(dead_code))]
pub const USER_TRAP_ENTRY_CONTEXT_SIZE: usize = 2 * core::mem::size_of::<usize>();
const USER_TRAP_ENTRY_TASK_IDENTITY_OFFSET: usize = 0;
const USER_TRAP_ENTRY_SAVED_TP_OFFSET: usize = core::mem::size_of::<usize>();
pub const KERNEL_TRAP_THREAD_SHIFT: usize = 14;
pub const KERNEL_TRAP_OVERFLOW_STACK_SIZE: usize = 4096;

global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl early_event_entry
early_event_entry:
    j early_event_entry_rust

    .globl formal_event_entry_save_context
formal_event_entry_save_context:
    addi    sp, sp, -{trap_frame_size}
    sd      zero, 0(sp)
    sd      ra, 8(sp)
    sd      gp, 24(sp)
    sd      tp, 32(sp)
    sd      t0, 40(sp)
    sd      t1, 48(sp)
    csrr    t0, sscratch
    beqz    t0, 1f
    ld      t1, {trap_frame_size_plus_saved_tp_offset}(sp)
    sd      t1, 32(sp)
    j       2f
1:
    addi    t0, sp, {trap_frame_size}
2:
    sd      t0, 16(sp)
    csrw    sscratch, zero
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
    li      t1, {sstatus_kernel_trap_clear}
    csrrc   zero, sstatus, t1
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
    andi    t1, t0, {sstatus_spp}
    bnez    t1, 3f
    addi    t0, sp, {trap_frame_size}
    sd      tp, {entry_task_identity_offset}(t0)
    csrw    sscratch, t0
    j       4f
3:
    csrw    sscratch, zero
4:

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
    .balign 16
    .globl arceos_ex_kernel_trap_overflow_stack
arceos_ex_kernel_trap_overflow_stack:
    .space {overflow_stack_size}
    .globl arceos_ex_kernel_trap_overflow_stack_end
arceos_ex_kernel_trap_overflow_stack_end:
    .balign 8
    .globl bss_anchor
bss_anchor:
    .space 8
"#,
    trap_frame_size = const TRAP_FRAME_SIZE,
    trap_frame_size_plus_saved_tp_offset = const (
        TRAP_FRAME_SIZE + USER_TRAP_ENTRY_SAVED_TP_OFFSET
    ),
    entry_task_identity_offset = const USER_TRAP_ENTRY_TASK_IDENTITY_OFFSET,
    overflow_stack_size = const KERNEL_TRAP_OVERFLOW_STACK_SIZE,
    sstatus_spp = const csr::SSTATUS_SPP,
    sstatus_kernel_trap_clear = const (csr::SSTATUS_SUM | csr::SSTATUS_FS_VS),
);

#[cfg(not(app_user_boot))]
global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl formal_event_entry
formal_event_entry:
    csrrw   sp, sscratch, sp
    bnez    sp, 1f
    csrrw   sp, sscratch, sp
1:
    j       formal_event_entry_save_context
"#,
);

#[cfg(app_user_boot)]
global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl formal_event_entry
formal_event_entry:
    /* User traps receive the entry-context boundary from sscratch. */
    csrrw   sp, sscratch, sp
    bnez    sp, .Lformal_event_entry_user

    /*
     * A kernel trap has sscratch == 0.  Restore the original stack pointer,
     * retain the prospective frame SP in sscratch, and perform Linux's VMAP
     * half-alignment bit test without touching another general register.
     */
    csrrw   sp, sscratch, sp
    addi    sp, sp, -{trap_frame_size}
    csrw    sscratch, sp
    srli    sp, sp, {thread_shift}
    andi    sp, sp, 1
    bnez    sp, .Lformal_event_entry_overflow

    csrr    sp, sscratch
    addi    sp, sp, {trap_frame_size}
    csrw    sscratch, zero
    j       formal_event_entry_save_context

.Lformal_event_entry_user:
    sd      tp, {entry_saved_tp_offset}(sp)
    ld      tp, {entry_task_identity_offset}(sp)
    j       formal_event_entry_save_context

.Lformal_event_entry_overflow:
    /*
     * sscratch holds prospective_sp.  The two exchanges preserve the
     * original t6 in t6 and leave the bad pre-trap SP in sscratch.
     */
    csrrw   t6, sscratch, t6
    addi    t6, t6, {trap_frame_size}
    csrrw   t6, sscratch, t6

    la      sp, arceos_ex_kernel_trap_overflow_stack_end
    addi    sp, sp, -{trap_frame_size}
    sd      zero, 0(sp)
    sd      ra, 8(sp)
    sd      gp, 24(sp)
    sd      tp, 32(sp)
    sd      t0, 40(sp)
    sd      t1, 48(sp)
    csrr    t0, sscratch
    sd      t0, 16(sp)
    csrw    sscratch, zero
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
    li      t1, {sstatus_kernel_trap_clear}
    csrrc   zero, sstatus, t1
    csrr    t0, sepc
    sd      t0, 264(sp)
    csrr    t0, scause
    sd      t0, 272(sp)
    csrr    t0, stval
    sd      t0, 280(sp)

    mv      a0, sp
    call    formal_event_entry_overflow_rust
.Lformal_event_entry_overflow_returned:
    wfi
    j       .Lformal_event_entry_overflow_returned
"#,
    trap_frame_size = const TRAP_FRAME_SIZE,
    thread_shift = const KERNEL_TRAP_THREAD_SHIFT,
    sstatus_kernel_trap_clear = const (csr::SSTATUS_SUM | csr::SSTATUS_FS_VS),
    entry_task_identity_offset = const USER_TRAP_ENTRY_TASK_IDENTITY_OFFSET,
    entry_saved_tp_offset = const USER_TRAP_ENTRY_SAVED_TP_OFFSET,
);

unsafe extern "C" {
    fn early_event_entry();
    fn formal_event_entry();
    #[cfg(any(app_smoke, app_user_boot))]
    static arceos_ex_kernel_trap_overflow_stack: u8;
    #[cfg(any(app_smoke, app_user_boot))]
    static arceos_ex_kernel_trap_overflow_stack_end: u8;
}

#[unsafe(no_mangle)]
extern "C" fn early_event_entry_rust() -> ! {
    crate::arch::riscv64::sbi::putstr("arceos_ex early trap\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    regs: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
}

const _: [(); TRAP_FRAME_SIZE] = [(); core::mem::size_of::<TrapFrame>()];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrapEntryOrigin {
    User,
    Kernel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrapEntryPrelude {
    pub origin: TrapEntryOrigin,
    pub stack_pointer: usize,
    pub scratch: usize,
    pub early_check_performed: bool,
    pub overflow: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub const fn formal_trap_entry_prelude(stack_pointer: usize, scratch: usize) -> TrapEntryPrelude {
    if scratch != 0 {
        TrapEntryPrelude {
            origin: TrapEntryOrigin::User,
            stack_pointer: scratch,
            scratch: stack_pointer,
            early_check_performed: false,
            overflow: false,
        }
    } else {
        TrapEntryPrelude {
            origin: TrapEntryOrigin::Kernel,
            stack_pointer,
            scratch: 0,
            early_check_performed: true,
            overflow: kernel_trap_frame_overflows(stack_pointer),
        }
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub const fn kernel_trap_frame_overflows(stack_pointer: usize) -> bool {
    let prospective_frame = stack_pointer.wrapping_sub(TRAP_FRAME_SIZE);
    ((prospective_frame >> KERNEL_TRAP_THREAD_SHIFT) & 1) != 0
}

#[cfg(any(app_smoke, app_user_boot))]
pub fn kernel_trap_overflow_stack_base() -> usize {
    core::ptr::addr_of!(arceos_ex_kernel_trap_overflow_stack) as usize
}

#[cfg(any(app_smoke, app_user_boot))]
pub fn kernel_trap_overflow_stack_top() -> usize {
    core::ptr::addr_of!(arceos_ex_kernel_trap_overflow_stack_end) as usize
}

impl TrapFrame {
    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn zeroed() -> Self {
        Self {
            regs: [0; 32],
            sstatus: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
        }
    }

    pub fn reg(&self, index: usize) -> usize {
        if index < self.regs.len() {
            self.regs[index]
        } else {
            0
        }
    }

    pub fn set_reg(&mut self, index: usize, value: usize) {
        if index != 0 && index < self.regs.len() {
            self.regs[index] = value;
        }
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn from_kernel_stack_overflow(
        mut regs: [usize; 32],
        bad_stack_pointer: usize,
        sstatus: usize,
        sepc: usize,
        scause: usize,
        stval: usize,
    ) -> Self {
        regs[0] = 0;
        regs[2] = bad_stack_pointer;
        Self {
            regs,
            sstatus,
            sepc,
            scause,
            stval,
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn formal_event_entry_rust(frame: &mut TrapFrame) {
    dispatch_trap(frame)
}

#[cfg(app_user_boot)]
#[unsafe(no_mangle)]
extern "C" fn formal_event_entry_overflow_rust(frame: &TrapFrame) -> ! {
    use crate::arch::riscv64::sbi;

    sbi::putstr("kernel trap stack overflow: terminal panic\n");
    sbi::putstr("bad_sp=");
    sbi_put_hex(frame.reg(2));
    sbi::putstr(" task_stack=[");
    sbi_put_hex(crate::objects::user_boot::user_kernel_trap_stack_base());
    sbi::putstr(",");
    sbi_put_hex(crate::objects::user_boot::user_kernel_trap_stack_top());
    sbi::putstr(") overflow_stack=[");
    sbi_put_hex(kernel_trap_overflow_stack_base());
    sbi::putstr(",");
    sbi_put_hex(kernel_trap_overflow_stack_top());
    sbi::putstr(")\nsepc=");
    sbi_put_hex(frame.sepc);
    sbi::putstr(" scause=");
    sbi_put_hex(frame.scause);
    sbi::putstr(" stval=");
    sbi_put_hex(frame.stval);
    sbi::putstr(" sstatus=");
    sbi_put_hex(frame.sstatus);
    sbi::putstr("\n");
    sbi::system_shutdown()
}

#[cfg(app_user_boot)]
fn sbi_put_hex(value: usize) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    crate::arch::riscv64::sbi::putstr("0x");
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        crate::arch::riscv64::sbi::putchar(DIGITS[(value >> shift) & 0xf]);
    }
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
        let Some(early_event_entry_phys) =
            kernel_image.runtime_to_phys(early_event_entry as *const () as usize)
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

        csr::write_stvec(formal_event_entry as *const () as usize);
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
