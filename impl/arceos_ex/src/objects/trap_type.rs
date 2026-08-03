use core::{
    arch::global_asm,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    cpu::{CpuRef, MAX_CPUS},
    current_task::CurrentTaskError,
    exception_type::ExceptionType,
    interrupt_type::InterruptType,
    kernel_image::KernelImage,
    state::{
        EventError, EventErrorCode, EventResult, FailureDiagnostic, Lifecycle, LifecycleEvent,
        State, failed_condition,
    },
    static_objects::StaticObjects,
    task::TaskRef,
    task_flow::TaskFlowRef,
    trap_flow_type::{
        TRAP_RETURN_TOKEN_MAGIC, TrapCauseClass, TrapEntrySnapshot, TrapExecutionRecord,
    },
    vm::Vm,
};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);
pub const TRAP_FRAME_SIZE: usize = 36 * core::mem::size_of::<usize>();
pub const TRAP_EXECUTION_RECORD_SIZE: usize = core::mem::size_of::<TrapExecutionRecord>();
pub const TRAP_STACK_RECORD_SIZE: usize = (TRAP_FRAME_SIZE + TRAP_EXECUTION_RECORD_SIZE + 15) & !15;
#[cfg_attr(not(app_user_boot), allow(dead_code))]
pub const USER_TRAP_ENTRY_CONTEXT_SIZE: usize = core::mem::size_of::<TrapEntryContext>();
#[cfg_attr(not(any(app_smoke, app_user_boot)), allow(dead_code))]
pub const KERNEL_TRAP_THREAD_SHIFT: usize = 14;
pub const KERNEL_TRAP_OVERFLOW_STACK_SIZE: usize = 4096;
const TRAP_ENTRY_CONTEXT_MAGIC: usize = 0x5452_4150_4354_5838;
const TRAP_ENTRY_CONTEXT_MAGIC_OFFSET: usize = 0;
const TRAP_ENTRY_CONTEXT_CPU_OFFSET: usize = 8;
pub(crate) const TRAP_ENTRY_CONTEXT_TASK_OFFSET: usize = 16;
pub(crate) const TRAP_ENTRY_CONTEXT_STACK_BASE_OFFSET: usize = 24;
pub(crate) const TRAP_ENTRY_CONTEXT_STACK_TOP_OFFSET: usize = 32;
const TRAP_ENTRY_CONTEXT_EMERGENCY_BASE_OFFSET: usize = 40;
const TRAP_ENTRY_CONTEXT_EMERGENCY_TOP_OFFSET: usize = 48;
pub(crate) const TRAP_ENTRY_CONTEXT_ROOT_ADDRESS_OFFSET: usize = 56;
pub(crate) const TRAP_ENTRY_CONTEXT_ROOT_GENERATION_OFFSET: usize = 64;
const TRAP_ENTRY_CONTEXT_EMERGENCY_ACTIVE_OFFSET: usize = 72;
const TRAP_ENTRY_CONTEXT_SAVED_SP_OFFSET: usize = 80;
const TRAP_ENTRY_CONTEXT_SAVED_TP_OFFSET: usize = 88;
const TRAP_ENTRY_CONTEXT_SAVED_T0_OFFSET: usize = 96;
const TRAP_ENTRY_CONTEXT_SAVED_T1_OFFSET: usize = 104;
const TRAP_ENTRY_CONTEXT_SAVED_T6_OFFSET: usize = 112;

#[unsafe(no_mangle)]
static FORMAL_TRAP_ENTRY_CONTEXTS: [AtomicUsize; MAX_CPUS] =
    [const { AtomicUsize::new(0) }; MAX_CPUS];

#[repr(C, align(16))]
pub struct TrapEntryContext {
    magic: usize,
    cpu_logical_id: usize,
    task_identity: usize,
    kernel_stack_base: usize,
    kernel_stack_top: usize,
    emergency_stack_base: usize,
    emergency_stack_top: usize,
    root_flow_address: usize,
    root_flow_generation: usize,
    emergency_active: usize,
    saved_sp: usize,
    saved_tp: usize,
    saved_t0: usize,
    saved_t1: usize,
    saved_t6: usize,
}

#[cfg_attr(not(app_user_boot), allow(dead_code))]
impl TrapEntryContext {
    const fn new() -> Self {
        Self {
            magic: 0,
            cpu_logical_id: usize::MAX,
            task_identity: 0,
            kernel_stack_base: 0,
            kernel_stack_top: 0,
            emergency_stack_base: 0,
            emergency_stack_top: 0,
            root_flow_address: 0,
            root_flow_generation: 0,
            emergency_active: 0,
            saved_sp: 0,
            saved_tp: 0,
            saved_t0: 0,
            saved_t1: 0,
            saved_t6: 0,
        }
    }

    pub const fn cpu_logical_id(&self) -> usize {
        self.cpu_logical_id
    }

    pub const fn task_identity(&self) -> usize {
        self.task_identity
    }

    pub const fn kernel_stack_base(&self) -> usize {
        self.kernel_stack_base
    }

    pub const fn kernel_stack_top(&self) -> usize {
        self.kernel_stack_top
    }

    pub const fn emergency_stack_base(&self) -> usize {
        self.emergency_stack_base
    }

    pub const fn emergency_stack_top(&self) -> usize {
        self.emergency_stack_top
    }

    pub const fn emergency_active(&self) -> bool {
        self.emergency_active != 0
    }

    pub const fn root_flow_ref(&self) -> super::trap_flow_type::TrapFlowRef {
        super::trap_flow_type::TrapFlowRef::new(
            self.root_flow_address,
            self.root_flow_generation as u32,
        )
    }

    fn install(
        &mut self,
        cpu_ref: CpuRef,
        task_identity: usize,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
        emergency_stack_base: usize,
        emergency_stack_top: usize,
    ) -> bool {
        if !cpu_ref.is_valid()
            || task_identity == 0
            || kernel_stack_base == 0
            || kernel_stack_top <= kernel_stack_base
            || emergency_stack_base == 0
            || emergency_stack_top - emergency_stack_base != KERNEL_TRAP_OVERFLOW_STACK_SIZE
            || !emergency_stack_base.is_multiple_of(16)
        {
            return false;
        }
        self.cpu_logical_id = cpu_ref.logical_id();
        self.task_identity = task_identity;
        self.kernel_stack_base = kernel_stack_base;
        self.kernel_stack_top = kernel_stack_top;
        self.emergency_stack_base = emergency_stack_base;
        self.emergency_stack_top = emergency_stack_top;
        self.root_flow_address = 0;
        self.root_flow_generation = 0;
        self.emergency_active = 0;
        self.magic = TRAP_ENTRY_CONTEXT_MAGIC;
        true
    }

    fn refresh_task_stack(
        &mut self,
        task_identity: usize,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
    ) -> bool {
        if self.magic != TRAP_ENTRY_CONTEXT_MAGIC
            || task_identity == 0
            || kernel_stack_base == 0
            || kernel_stack_top <= kernel_stack_base
            || self.emergency_active()
        {
            return false;
        }
        self.task_identity = task_identity;
        self.kernel_stack_base = kernel_stack_base;
        self.kernel_stack_top = kernel_stack_top;
        true
    }

    fn refresh_task_root(
        &mut self,
        task_identity: usize,
        root_ref: super::trap_flow_type::TrapFlowRef,
    ) -> bool {
        if self.magic != TRAP_ENTRY_CONTEXT_MAGIC
            || task_identity == 0
            || self.emergency_active()
            || (!root_ref.is_valid() && root_ref != super::trap_flow_type::TrapFlowRef::NONE)
        {
            return false;
        }
        self.task_identity = task_identity;
        self.root_flow_address = root_ref.address();
        self.root_flow_generation = root_ref.generation() as usize;
        true
    }

    fn entry_authority_matches(
        &self,
        cpu_ref: CpuRef,
        task_identity: usize,
        task_root: super::trap_flow_type::TrapFlowRef,
        frame_address: usize,
    ) -> bool {
        self.magic == TRAP_ENTRY_CONTEXT_MAGIC
            && cpu_ref.is_valid()
            && self.cpu_logical_id == cpu_ref.logical_id()
            && task_identity != 0
            && self.task_identity == task_identity
            && self.root_flow_ref().same_identity(task_root)
            && !self.emergency_active()
            && self.emergency_stack_top > self.emergency_stack_base
            && self.emergency_stack_top - self.emergency_stack_base
                == KERNEL_TRAP_OVERFLOW_STACK_SIZE
            && trap_record_fits(
                frame_address + TRAP_STACK_RECORD_SIZE,
                self.kernel_stack_base,
                self.kernel_stack_top,
            )
            && frame_address.is_multiple_of(16)
    }
}

#[repr(C, align(16))]
struct TrapEmergencyStack {
    bytes: [u8; KERNEL_TRAP_OVERFLOW_STACK_SIZE],
}

impl TrapEmergencyStack {
    const fn new() -> Self {
        Self {
            bytes: [0; KERNEL_TRAP_OVERFLOW_STACK_SIZE],
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<TrapEntryContext>() == 128);
    assert!(core::mem::offset_of!(TrapEntryContext, magic) == TRAP_ENTRY_CONTEXT_MAGIC_OFFSET);
    assert!(
        core::mem::offset_of!(TrapEntryContext, cpu_logical_id) == TRAP_ENTRY_CONTEXT_CPU_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, task_identity) == TRAP_ENTRY_CONTEXT_TASK_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, kernel_stack_base)
            == TRAP_ENTRY_CONTEXT_STACK_BASE_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, kernel_stack_top)
            == TRAP_ENTRY_CONTEXT_STACK_TOP_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, emergency_stack_base)
            == TRAP_ENTRY_CONTEXT_EMERGENCY_BASE_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, emergency_stack_top)
            == TRAP_ENTRY_CONTEXT_EMERGENCY_TOP_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, root_flow_address)
            == TRAP_ENTRY_CONTEXT_ROOT_ADDRESS_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, root_flow_generation)
            == TRAP_ENTRY_CONTEXT_ROOT_GENERATION_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, emergency_active)
            == TRAP_ENTRY_CONTEXT_EMERGENCY_ACTIVE_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, saved_sp) == TRAP_ENTRY_CONTEXT_SAVED_SP_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, saved_tp) == TRAP_ENTRY_CONTEXT_SAVED_TP_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, saved_t0) == TRAP_ENTRY_CONTEXT_SAVED_T0_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, saved_t1) == TRAP_ENTRY_CONTEXT_SAVED_T1_OFFSET
    );
    assert!(
        core::mem::offset_of!(TrapEntryContext, saved_t6) == TRAP_ENTRY_CONTEXT_SAVED_T6_OFFSET
    );
};

global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl early_event_entry
early_event_entry:
    j early_event_entry_rust

    .globl formal_event_entry_save_context
formal_event_entry_save_context:
    sd      zero, 0(sp)
    sd      ra, 8(sp)
    sd      gp, 24(sp)
    ld      t0, {context_saved_sp_offset}(t6)
    sd      t0, 16(sp)
    ld      t0, {context_saved_tp_offset}(t6)
    sd      t0, 32(sp)
    ld      t0, {context_saved_t0_offset}(t6)
    sd      t0, 40(sp)
    ld      t0, {context_saved_t1_offset}(t6)
    sd      t0, 48(sp)
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
    ld      t0, {context_saved_t6_offset}(t6)
    sd      t0, 248(sp)
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

    mv      s11, t6
    mv      a0, sp
    mv      a1, t6
    call    formal_event_entry_rust
    mv      t6, s11
    li      t0, {trap_return_token_magic}
    bne     a0, t0, .Lformal_trap_return_token_rejected

    ld      t0, 264(sp)
    csrw    sepc, t0
    ld      t0, 256(sp)
    csrw    sstatus, t0
    andi    t1, t0, {sstatus_spp}
    bnez    t1, 3f
    sd      tp, {context_task_offset}(t6)
    csrw    sscratch, tp
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

.Lformal_trap_return_token_rejected:
    call    formal_trap_return_token_rejected_rust
    j       .Lformal_trap_return_token_rejected

    .section .bss.objects, "aw", @nobits
    .balign 8
    .globl bss_anchor
bss_anchor:
    .space 8
"#,
    context_task_offset = const TRAP_ENTRY_CONTEXT_TASK_OFFSET,
    context_saved_sp_offset = const TRAP_ENTRY_CONTEXT_SAVED_SP_OFFSET,
    context_saved_tp_offset = const TRAP_ENTRY_CONTEXT_SAVED_TP_OFFSET,
    context_saved_t0_offset = const TRAP_ENTRY_CONTEXT_SAVED_T0_OFFSET,
    context_saved_t1_offset = const TRAP_ENTRY_CONTEXT_SAVED_T1_OFFSET,
    context_saved_t6_offset = const TRAP_ENTRY_CONTEXT_SAVED_T6_OFFSET,
    trap_return_token_magic = const TRAP_RETURN_TOKEN_MAGIC,
    sstatus_spp = const csr::SSTATUS_SPP,
    sstatus_kernel_trap_clear = const (csr::SSTATUS_SUM | csr::SSTATUS_FS_VS),
);

global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl formal_event_entry
formal_event_entry:
    csrrw   tp, sscratch, tp
    la      tp, FORMAL_TRAP_ENTRY_CONTEXTS
    ld      tp, 0(tp)
    j       .Lformal_event_entry_common

    .macro FORMAL_CPU_ENTRY index, offset
    .align 2
    .globl formal_event_entry_cpu\index
formal_event_entry_cpu\index:
    csrrw   tp, sscratch, tp
    la      tp, FORMAL_TRAP_ENTRY_CONTEXTS
    ld      tp, \offset(tp)
    j       .Lformal_event_entry_common
    .endm

    FORMAL_CPU_ENTRY 1, 8
    FORMAL_CPU_ENTRY 2, 16
    FORMAL_CPU_ENTRY 3, 24
    FORMAL_CPU_ENTRY 4, 32
    FORMAL_CPU_ENTRY 5, 40
    FORMAL_CPU_ENTRY 6, 48
    FORMAL_CPU_ENTRY 7, 56
    FORMAL_CPU_ENTRY 8, 64
    FORMAL_CPU_ENTRY 9, 72
    FORMAL_CPU_ENTRY 10, 80
    FORMAL_CPU_ENTRY 11, 88
    FORMAL_CPU_ENTRY 12, 96
    FORMAL_CPU_ENTRY 13, 104
    FORMAL_CPU_ENTRY 14, 112
    FORMAL_CPU_ENTRY 15, 120

.Lformal_event_entry_common:
    /* sscratch now holds the interrupted tp; tp locates this CPU's context. */
    beqz    tp, .Lformal_event_entry_direct_shutdown
    sd      sp, {context_saved_sp_offset}(tp)
    sd      t6, {context_saved_t6_offset}(tp)
    sd      t0, {context_saved_t0_offset}(tp)
    sd      t1, {context_saved_t1_offset}(tp)
    csrr    t0, sscratch
    sd      t0, {context_saved_tp_offset}(tp)
    mv      t6, tp
    ld      tp, {context_task_offset}(t6)
    csrw    sscratch, zero

    ld      t0, {context_magic_offset}(t6)
    li      t1, {context_magic}
    bne     t0, t1, .Lformal_event_entry_direct_shutdown
    csrr    t0, sstatus
    andi    t0, t0, {sstatus_spp}
    bnez    t0, .Lformal_event_entry_kernel
    ld      sp, {context_stack_top_offset}(t6)
    ld      tp, {context_task_offset}(t6)
    j       .Lformal_event_entry_check_capacity

.Lformal_event_entry_kernel:
    ld      sp, {context_saved_sp_offset}(t6)

.Lformal_event_entry_check_capacity:
    ld      t0, {context_stack_base_offset}(t6)
    ld      t1, {context_stack_top_offset}(t6)
    bltu    sp, t0, .Lformal_event_entry_overflow
    bltu    t1, sp, .Lformal_event_entry_overflow
    addi    sp, sp, -{trap_stack_record_size}
    bltu    sp, t0, .Lformal_event_entry_overflow
    j       formal_event_entry_save_context

.Lformal_event_entry_overflow:
    ld      t0, {context_emergency_active_offset}(t6)
    bnez    t0, .Lformal_event_entry_direct_shutdown
    li      t0, 1
    sd      t0, {context_emergency_active_offset}(t6)
    ld      sp, {context_emergency_top_offset}(t6)
    addi    sp, sp, -{trap_frame_size}
    sd      zero, 0(sp)
    sd      ra, 8(sp)
    sd      gp, 24(sp)
    ld      t0, {context_saved_sp_offset}(t6)
    sd      t0, 16(sp)
    ld      t0, {context_saved_tp_offset}(t6)
    sd      t0, 32(sp)
    ld      t0, {context_saved_t0_offset}(t6)
    sd      t0, 40(sp)
    ld      t0, {context_saved_t1_offset}(t6)
    sd      t0, 48(sp)
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
    ld      t0, {context_saved_t6_offset}(t6)
    sd      t0, 248(sp)
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
    mv      a1, t6
    call    formal_event_entry_overflow_rust
.Lformal_event_entry_overflow_returned:
    wfi
    j       .Lformal_event_entry_overflow_returned

.Lformal_event_entry_direct_shutdown:
    li      a7, 0x53525354
    li      a6, 0
    li      a0, 0
    li      a1, 1
    ecall
1:
    wfi
    j       1b
"#,
    trap_frame_size = const TRAP_FRAME_SIZE,
    trap_stack_record_size = const TRAP_STACK_RECORD_SIZE,
    context_magic = const TRAP_ENTRY_CONTEXT_MAGIC,
    context_magic_offset = const TRAP_ENTRY_CONTEXT_MAGIC_OFFSET,
    context_task_offset = const TRAP_ENTRY_CONTEXT_TASK_OFFSET,
    context_stack_base_offset = const TRAP_ENTRY_CONTEXT_STACK_BASE_OFFSET,
    context_stack_top_offset = const TRAP_ENTRY_CONTEXT_STACK_TOP_OFFSET,
    context_emergency_top_offset = const TRAP_ENTRY_CONTEXT_EMERGENCY_TOP_OFFSET,
    context_emergency_active_offset = const TRAP_ENTRY_CONTEXT_EMERGENCY_ACTIVE_OFFSET,
    context_saved_sp_offset = const TRAP_ENTRY_CONTEXT_SAVED_SP_OFFSET,
    context_saved_tp_offset = const TRAP_ENTRY_CONTEXT_SAVED_TP_OFFSET,
    context_saved_t0_offset = const TRAP_ENTRY_CONTEXT_SAVED_T0_OFFSET,
    context_saved_t1_offset = const TRAP_ENTRY_CONTEXT_SAVED_T1_OFFSET,
    context_saved_t6_offset = const TRAP_ENTRY_CONTEXT_SAVED_T6_OFFSET,
    sstatus_kernel_trap_clear = const (csr::SSTATUS_SUM | csr::SSTATUS_FS_VS),
    sstatus_spp = const csr::SSTATUS_SPP,
);

unsafe extern "C" {
    fn early_event_entry();
    fn formal_event_entry();
    fn formal_event_entry_cpu1();
    fn formal_event_entry_cpu2();
    fn formal_event_entry_cpu3();
    fn formal_event_entry_cpu4();
    fn formal_event_entry_cpu5();
    fn formal_event_entry_cpu6();
    fn formal_event_entry_cpu7();
    fn formal_event_entry_cpu8();
    fn formal_event_entry_cpu9();
    fn formal_event_entry_cpu10();
    fn formal_event_entry_cpu11();
    fn formal_event_entry_cpu12();
    fn formal_event_entry_cpu13();
    fn formal_event_entry_cpu14();
    fn formal_event_entry_cpu15();
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
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub enum TrapEntryOrigin {
    User,
    Kernel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrapEntryPrelude {
    pub origin: TrapEntryOrigin,
    pub stack_pointer: usize,
    pub entry_context: usize,
    pub capacity_check_performed: bool,
    pub overflow: bool,
}

pub const fn trap_record_fits(
    stack_pointer: usize,
    kernel_stack_base: usize,
    kernel_stack_top: usize,
) -> bool {
    if kernel_stack_base == 0
        || kernel_stack_top <= kernel_stack_base
        || stack_pointer < kernel_stack_base
        || stack_pointer > kernel_stack_top
    {
        return false;
    }
    match stack_pointer.checked_sub(TRAP_STACK_RECORD_SIZE) {
        Some(record_base) => record_base >= kernel_stack_base,
        None => false,
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub const fn formal_trap_entry_prelude(
    origin: TrapEntryOrigin,
    interrupted_stack_pointer: usize,
    entry_context: usize,
    kernel_stack_base: usize,
    kernel_stack_top: usize,
) -> TrapEntryPrelude {
    let stack_pointer = match origin {
        TrapEntryOrigin::User => kernel_stack_top,
        TrapEntryOrigin::Kernel => interrupted_stack_pointer,
    };
    TrapEntryPrelude {
        origin,
        stack_pointer,
        entry_context,
        capacity_check_performed: true,
        overflow: kernel_trap_frame_overflows(stack_pointer, kernel_stack_base, kernel_stack_top),
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub const fn kernel_trap_frame_overflows(
    stack_pointer: usize,
    kernel_stack_base: usize,
    kernel_stack_top: usize,
) -> bool {
    !trap_record_fits(stack_pointer, kernel_stack_base, kernel_stack_top)
}

#[cfg(any(app_smoke, app_user_boot))]
pub fn kernel_trap_overflow_stack_base() -> usize {
    crate::context::context_ref()
        .cpu_group
        .boot_cpu_trap()
        .map(TrapType::emergency_stack_base)
        .unwrap_or(0)
}

#[cfg(any(app_smoke, app_user_boot))]
pub fn kernel_trap_overflow_stack_top() -> usize {
    crate::context::context_ref()
        .cpu_group
        .boot_cpu_trap()
        .map(TrapType::emergency_stack_top)
        .unwrap_or(0)
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
extern "C" fn formal_event_entry_rust(
    frame: &mut TrapFrame,
    entry_context: &TrapEntryContext,
) -> usize {
    if frame.scause == SCAUSE_INTERRUPT_BIT | crate::arch::riscv64::SUPERVISOR_SOFTWARE_IRQ {
        crate::arch::riscv64::csr::clear_supervisor_software_interrupt();
        crate::objects::kernel_task::handle_reschedule_ipi(entry_context.cpu_logical_id());
        return TRAP_RETURN_TOKEN_MAGIC;
    }
    let record_ptr = unsafe {
        (frame as *mut TrapFrame)
            .cast::<u8>()
            .add(TRAP_FRAME_SIZE)
            .cast::<TrapExecutionRecord>()
    };
    unsafe { record_ptr.write(TrapExecutionRecord::new()) };
    let record = unsafe { &mut *record_ptr };
    let entry = capture_entry_authority(frame, record);
    match dispatch_trap_occurrence(record, frame, entry) {
        Ok(token) => token,
        Err(error) => trap_occurrence_failed(frame, record, entry, error),
    }
}

#[unsafe(no_mangle)]
extern "C" fn formal_trap_return_token_rejected_rust() -> ! {
    crate::arch::riscv64::sbi::putstr("trap return token rejected after Rust return\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

#[unsafe(no_mangle)]
extern "C" fn formal_event_entry_overflow_rust(
    frame: &TrapFrame,
    entry_context: &TrapEntryContext,
) -> ! {
    use crate::arch::riscv64::sbi;

    sbi::putstr("kernel trap stack overflow: terminal panic\n");
    sbi::putstr("cpu=");
    sbi_put_hex(entry_context.cpu_logical_id());
    sbi::putstr(" task_identity=");
    sbi_put_hex(entry_context.task_identity());
    sbi::putstr("\n");
    sbi::putstr("bad_sp=");
    sbi_put_hex(frame.reg(2));
    sbi::putstr(" task_stack=[");
    sbi_put_hex(entry_context.kernel_stack_base());
    sbi::putstr(",");
    sbi_put_hex(entry_context.kernel_stack_top());
    sbi::putstr(") overflow_stack=[");
    sbi_put_hex(entry_context.emergency_stack_base());
    sbi::putstr(",");
    sbi_put_hex(entry_context.emergency_stack_top());
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

fn sbi_put_hex(value: usize) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    crate::arch::riscv64::sbi::putstr("0x");
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        crate::arch::riscv64::sbi::putchar(DIGITS[(value >> shift) & 0xf]);
    }
}

pub struct TrapType {
    lifecycle: Lifecycle,
    interrupt: InterruptType,
    exception: ExceptionType,
    entry_context: TrapEntryContext,
    emergency_stack: TrapEmergencyStack,
    service_online: bool,
    next_occurrence_generation: AtomicU32,
}

impl TrapType {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            interrupt: InterruptType::new(),
            exception: ExceptionType::new(),
            entry_context: TrapEntryContext::new(),
            emergency_stack: TrapEmergencyStack::new(),
            service_online: false,
            next_occurrence_generation: AtomicU32::new(0),
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
            Checkpoint::TrapTypePrepared,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn interrupt(&self) -> &InterruptType {
        &self.interrupt
    }

    pub fn interrupt_mut(&mut self) -> &mut InterruptType {
        &mut self.interrupt
    }

    pub const fn exception(&self) -> &ExceptionType {
        &self.exception
    }

    pub fn exception_mut(&mut self) -> &mut ExceptionType {
        &mut self.exception
    }

    #[allow(dead_code)]
    pub const fn entry_context(&self) -> &TrapEntryContext {
        &self.entry_context
    }

    pub fn entry_context_address(&self) -> usize {
        core::ptr::addr_of!(self.entry_context) as usize
    }

    pub fn emergency_stack_base(&self) -> usize {
        core::ptr::addr_of!(self.emergency_stack.bytes) as usize
    }

    pub fn emergency_stack_top(&self) -> usize {
        self.emergency_stack_base() + KERNEL_TRAP_OVERFLOW_STACK_SIZE
    }

    pub fn install_entry_context(
        &mut self,
        cpu_ref: CpuRef,
        task_identity: usize,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
    ) -> bool {
        let emergency_base = self.emergency_stack_base();
        let emergency_top = self.emergency_stack_top();
        let installed = self.entry_context.install(
            cpu_ref,
            task_identity,
            kernel_stack_base,
            kernel_stack_top,
            emergency_base,
            emergency_top,
        ) && self.entry_context_address().is_multiple_of(16);
        let Some(slot) = FORMAL_TRAP_ENTRY_CONTEXTS.get(cpu_ref.logical_id()) else {
            return false;
        };
        if !installed {
            return false;
        }
        slot.store(self.entry_context_address(), Ordering::Release);
        true
    }

    pub(crate) fn installed_entry_context_address(logical_id: usize) -> usize {
        FORMAL_TRAP_ENTRY_CONTEXTS
            .get(logical_id)
            .map(|slot| slot.load(Ordering::Acquire))
            .unwrap_or(0)
    }

    #[cfg_attr(not(app_user_boot), allow(dead_code))]
    pub fn refresh_entry_task_stack(
        &mut self,
        task_identity: usize,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
    ) -> bool {
        self.entry_context
            .refresh_task_stack(task_identity, kernel_stack_base, kernel_stack_top)
    }

    pub fn refresh_entry_task_root(
        &mut self,
        task_identity: usize,
        root_ref: super::trap_flow_type::TrapFlowRef,
    ) -> bool {
        self.entry_context
            .refresh_task_root(task_identity, root_ref)
    }

    pub(crate) fn prepare_secondary_entry(
        &mut self,
        cpu_ref: CpuRef,
        task_identity: usize,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || !self.install_entry_context(
                cpu_ref,
                task_identity,
                kernel_stack_base,
                kernel_stack_top,
            )
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Base,
                State::Online,
            );
        }
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)?;
        self.interrupt.adopt_secondary_online()?;
        self.exception.adopt_secondary_ready()?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub(crate) fn formal_entry_address(cpu_ref: CpuRef) -> Option<usize> {
        let entry = match cpu_ref.logical_id() {
            0 => formal_event_entry,
            1 => formal_event_entry_cpu1,
            2 => formal_event_entry_cpu2,
            3 => formal_event_entry_cpu3,
            4 => formal_event_entry_cpu4,
            5 => formal_event_entry_cpu5,
            6 => formal_event_entry_cpu6,
            7 => formal_event_entry_cpu7,
            8 => formal_event_entry_cpu8,
            9 => formal_event_entry_cpu9,
            10 => formal_event_entry_cpu10,
            11 => formal_event_entry_cpu11,
            12 => formal_event_entry_cpu12,
            13 => formal_event_entry_cpu13,
            14 => formal_event_entry_cpu14,
            15 => formal_event_entry_cpu15,
            _ => return None,
        };
        Some(entry as *const () as usize)
    }

    pub fn setup(
        &mut self,
        vm: &Vm,
        static_objects: &StaticObjects,
        cpu_ref: CpuRef,
        task_identity: usize,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
    ) -> EventResult {
        let formal_entry = Self::formal_entry_address(cpu_ref).unwrap_or(0);
        if self.lifecycle.state() != State::Prepared
            || vm.state() != State::Ready
            || static_objects.state() != State::Online
            || self.exception.state() != State::Base
            || self.interrupt.state() != State::Ready
            || formal_entry == 0
            || !self.install_entry_context(
                cpu_ref,
                task_identity,
                kernel_stack_base,
                kernel_stack_top,
            )
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.exception.preset(self.lifecycle.state())?;
        csr::write_stvec(formal_entry);
        csr::clear_sscratch();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::TrapTypeReady,
        )
    }

    #[cfg_attr(not(app_user_boot), allow(dead_code))]
    pub fn enable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.interrupt.state() != State::Online
            || self.exception.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.service_online = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    #[cfg(app_user_boot)]
    pub(crate) fn enable_secondary_service(
        &mut self,
        syscall_table: &super::exception_type::SyscallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.interrupt.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.exception.enable_secondary(syscall_table)?;
        self.enable()
    }

    #[cfg_attr(not(app_user_boot), allow(dead_code))]
    pub const fn service_online(&self) -> bool {
        self.service_online
    }

    fn allocate_occurrence_generation(&self) -> u32 {
        let mut current = self.next_occurrence_generation.load(Ordering::Relaxed);
        loop {
            let next = super::next_generation(current);
            match self.next_occurrence_generation.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return next,
                Err(observed) => current = observed,
            }
        }
    }
}

#[derive(Clone, Copy)]
struct TrapEntryAuthority {
    task_ref: TaskRef,
    task_flow_ref: TaskFlowRef,
    cpu_ref: CpuRef,
    generation: u32,
    trap_state: State,
    interrupt_state: State,
    exception_state: State,
    page_fault_state: State,
    syscall_state: State,
    breakpoint_state: State,
    unexpected_state: State,
}

#[derive(Clone, Copy)]
struct TrapReturnAuthority {
    cpu_ref: CpuRef,
    committed_terminal_switch: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConcreteExceptionKind {
    PageFault,
    Syscall,
    Breakpoint,
    Unexpected,
}

fn capture_entry_authority(frame: &TrapFrame, record: &TrapExecutionRecord) -> TrapEntryAuthority {
    let ctx = crate::context::context_ref();
    let task_ref = match ctx.current_task_ref() {
        Ok(task_ref) => task_ref,
        Err(error) => trap_selector_failed(frame, record, error),
    };
    let task_flow_ref = match ctx.current_task_flow_ref() {
        Ok(flow_ref) => flow_ref,
        Err(error) => trap_selector_failed(frame, record, error),
    };
    let cpu_ref = match ctx.current_cpu() {
        Ok(current_cpu) => current_cpu.cpu_ref(),
        Err(error) => trap_selector_failed(frame, record, error),
    };
    let Some(cpu) = ctx.cpu_group.dereference(cpu_ref) else {
        trap_authority_failed(frame, record, task_ref, task_flow_ref, cpu_ref)
    };
    let trap = cpu.trap();
    let task_identity = csr::read_tp();
    let task_root = ctx.task_root_trap_flow_ref_raw(task_ref);
    if csr::read_sscratch() != 0
        || TrapType::installed_entry_context_address(cpu_ref.logical_id())
            != trap.entry_context_address()
        || !trap.entry_context.entry_authority_matches(
            cpu_ref,
            task_identity,
            task_root,
            frame as *const TrapFrame as usize,
        )
    {
        crate::arch::riscv64::sbi::putstr("trap entry context mismatch sscratch=");
        sbi_put_hex(csr::read_sscratch());
        crate::arch::riscv64::sbi::putstr(" installed_context=");
        sbi_put_hex(TrapType::installed_entry_context_address(
            cpu_ref.logical_id(),
        ));
        crate::arch::riscv64::sbi::putstr(" expected_context=");
        sbi_put_hex(trap.entry_context_address());
        crate::arch::riscv64::sbi::putstr(" context_task=");
        sbi_put_hex(trap.entry_context.task_identity());
        crate::arch::riscv64::sbi::putstr(" context_stack=[");
        sbi_put_hex(trap.entry_context.kernel_stack_base());
        crate::arch::riscv64::sbi::putstr(",");
        sbi_put_hex(trap.entry_context.kernel_stack_top());
        crate::arch::riscv64::sbi::putstr(") context_root=");
        sbi_put_hex(trap.entry_context.root_flow_ref().address());
        crate::arch::riscv64::sbi::putstr(":");
        sbi_put_hex(trap.entry_context.root_flow_ref().generation() as usize);
        trap_authority_failed(frame, record, task_ref, task_flow_ref, cpu_ref)
    }
    let exception = trap.exception();
    TrapEntryAuthority {
        task_ref,
        task_flow_ref,
        cpu_ref,
        generation: trap.allocate_occurrence_generation(),
        trap_state: trap.state(),
        interrupt_state: trap.interrupt().state(),
        exception_state: exception.state(),
        page_fault_state: exception.page_fault_state(),
        syscall_state: exception.syscall_state(),
        breakpoint_state: exception.breakpoint_state(),
        unexpected_state: exception.unexpected_state(),
    }
}

fn dispatch_trap_occurrence(
    record: &mut TrapExecutionRecord,
    frame: &mut TrapFrame,
    entry: TrapEntryAuthority,
) -> Result<usize, EventError> {
    let root_address = core::ptr::addr_of!(record.root) as usize;
    record.root.declare_and_bind(
        entry.generation,
        root_address,
        entry.cpu_ref,
        matches!(entry.trap_state, State::Ready | State::Online),
    )?;
    let root_ref = record.root.flow_ref();
    let root_binding_created = crate::context::context()
        .bind_task_root_trap_flow(entry.task_ref, root_ref)
        .map_err(|first_failed| {
            trap_return_condition_error().with_diagnostic(FailureDiagnostic::new(
                "TrapOccurrence",
                "bind_root",
                "TaskThreadContext",
                "root TrapFlowRef binding",
                first_failed,
            ))
        });
    let Ok(root_binding_created) = root_binding_created else {
        return Err(root_binding_created.expect_err("failed root binding must carry a diagnostic"));
    };
    let cause_class = if frame.scause & SCAUSE_INTERRUPT_BIT != 0 {
        TrapCauseClass::Interrupt
    } else {
        TrapCauseClass::Exception
    };
    record.root.preset(TrapEntrySnapshot {
        cause_class,
        scause: frame.scause,
        sepc: frame.sepc,
        sstatus: frame.sstatus,
        stval: frame.stval,
        entry_task: entry.task_ref,
        effective_task_flow: entry.task_flow_ref,
    })?;

    match cause_class {
        TrapCauseClass::Interrupt => dispatch_interrupt_occurrence(record, frame, entry)?,
        TrapCauseClass::Exception => dispatch_exception_occurrence(record, frame, entry)?,
    }

    let return_authority = capture_return_cpu(frame, record, entry, true);
    record.root.setup()?;
    let token = record.root.enable()?;
    record.root.disable()?;
    record.root.cleanup(return_authority.cpu_ref)?;
    let root_binding_valid = if root_binding_created {
        match crate::context::context().clear_task_root_trap_flow(entry.task_ref, root_ref) {
            Some(cleared) => cleared,
            None => return_authority.committed_terminal_switch,
        }
    } else {
        match crate::context::context().task_root_trap_flow_resolves(entry.task_ref) {
            Some(resolves) => resolves,
            None => return_authority.committed_terminal_switch,
        }
    };
    if !root_binding_valid {
        let step = if root_binding_created {
            "clear_installed_root"
        } else {
            "resolve_outer_root"
        };
        return Err(
            trap_return_condition_error().with_diagnostic(FailureDiagnostic::new(
                "TrapOccurrence",
                step,
                "TaskThreadContext",
                "root TrapFlowRef cleanup",
                "stored root missing, replaced, stale, or released",
            )),
        );
    }
    token
        .consume_after_cleanup(&record.root)
        .ok_or_else(trap_return_condition_error)
}

fn dispatch_interrupt_occurrence(
    record: &mut TrapExecutionRecord,
    frame: &mut TrapFrame,
    entry: TrapEntryAuthority,
) -> EventResult {
    let address = core::ptr::addr_of!(record.interrupt) as usize;
    record.interrupt.declare_and_bind(
        entry.generation,
        address,
        entry.cpu_ref,
        entry.interrupt_state,
    )?;
    record.interrupt.preset()?;
    if !record.interrupt.hardirq_schedule_forbidden()
        || !record.interrupt.ordinary_reentry_forbidden()
    {
        return failed_condition(
            LifecycleEvent::Preset,
            record.interrupt.state(),
            State::Prepared,
            State::Prepared,
        );
    }
    let child_ref = record.interrupt.flow_ref();
    record.root.select_child(child_ref)?;
    crate::objects::interrupt_type::dispatch_scause(frame.scause);
    capture_return_cpu(frame, record, entry, false);
    record.interrupt.setup_after_handler()?;
    record.interrupt.enable()?;
    record.interrupt.disable()?;
    record.interrupt.cleanup()?;
    record.root.mark_child_completed(child_ref)
}

fn dispatch_exception_occurrence(
    record: &mut TrapExecutionRecord,
    frame: &mut TrapFrame,
    entry: TrapEntryAuthority,
) -> EventResult {
    let address = core::ptr::addr_of!(record.exception) as usize;
    record.exception.declare_and_bind(
        entry.generation,
        address,
        entry.cpu_ref,
        entry.exception_state,
    )?;
    record.exception.preset(frame.scause)?;
    let exception_ref = record.exception.flow_ref();
    record.root.select_child(exception_ref)?;

    let kind = classify_exception(frame.scause);
    let migration_allowed = matches!(
        kind,
        ConcreteExceptionKind::PageFault | ConcreteExceptionKind::Syscall
    );
    let concrete_ref = match kind {
        ConcreteExceptionKind::PageFault => {
            let address = core::ptr::addr_of!(record.page_fault) as usize;
            record.page_fault.declare_and_bind(
                entry.generation,
                address,
                entry.cpu_ref,
                entry.page_fault_state,
            )?;
            let from_user = frame.sstatus & csr::SSTATUS_SPP == 0;
            record
                .page_fault
                .preset(frame.scause, frame.stval, from_user)?;
            let child_ref = record.page_fault.flow_ref();
            record.exception.select_child(child_ref)?;
            record.page_fault.setup_context(!from_user, false)?;
            crate::objects::exception_type::dispatch_trap(frame);
            record.page_fault.enable_after_handler()?;
            record.page_fault.disable()?;
            record.page_fault.cleanup()?;
            child_ref
        }
        ConcreteExceptionKind::Syscall => {
            let address = core::ptr::addr_of!(record.syscall) as usize;
            record.syscall.declare_and_bind(
                entry.generation,
                address,
                entry.cpu_ref,
                entry.syscall_state,
            )?;
            record.syscall.preset()?;
            let child_ref = record.syscall.flow_ref();
            record.exception.select_child(child_ref)?;
            record.syscall.setup()?;
            crate::objects::exception_type::dispatch_trap(frame);
            record.syscall.enable_after_handler()?;
            record.syscall.disable()?;
            record.syscall.cleanup()?;
            child_ref
        }
        ConcreteExceptionKind::Breakpoint => {
            let address = core::ptr::addr_of!(record.breakpoint) as usize;
            record.breakpoint.declare_and_bind(
                entry.generation,
                address,
                entry.cpu_ref,
                entry.breakpoint_state,
            )?;
            record.breakpoint.preset()?;
            let child_ref = record.breakpoint.flow_ref();
            record.exception.select_child(child_ref)?;
            crate::objects::exception_type::dispatch_trap(frame);
            record.breakpoint.setup_after_hook()?;
            record.breakpoint.enable()?;
            record.breakpoint.disable()?;
            record.breakpoint.cleanup()?;
            child_ref
        }
        ConcreteExceptionKind::Unexpected => {
            let address = core::ptr::addr_of!(record.unexpected) as usize;
            record.unexpected.declare_and_bind(
                entry.generation,
                address,
                entry.cpu_ref,
                entry.unexpected_state,
            )?;
            record.unexpected.preset()?;
            let child_ref = record.unexpected.flow_ref();
            record.exception.select_child(child_ref)?;
            record.unexpected.setup()?;
            crate::objects::exception_type::dispatch_trap(frame);
            record.unexpected.enable()?;
            record.unexpected.disable()?;
            record.unexpected.cleanup()?;
            child_ref
        }
    };

    capture_return_cpu(frame, record, entry, migration_allowed);
    record.exception.mark_child_completed(concrete_ref)?;
    record.exception.setup()?;
    record.exception.enable()?;
    record.exception.disable()?;
    record.exception.cleanup()?;
    record.root.mark_child_completed(exception_ref)
}

const fn classify_exception(scause: usize) -> ConcreteExceptionKind {
    match scause & !SCAUSE_INTERRUPT_BIT {
        12 | 13 | 15 => ConcreteExceptionKind::PageFault,
        8 | 9 => ConcreteExceptionKind::Syscall,
        3 => ConcreteExceptionKind::Breakpoint,
        _ => ConcreteExceptionKind::Unexpected,
    }
}

fn capture_return_cpu(
    frame: &TrapFrame,
    record: &TrapExecutionRecord,
    entry: TrapEntryAuthority,
    migration_allowed: bool,
) -> TrapReturnAuthority {
    let ctx = crate::context::context_ref();
    let task_ref = match ctx.current_task_ref() {
        Ok(task_ref) => task_ref,
        Err(error) => trap_selector_failed(frame, record, error),
    };
    let task_flow_ref = match ctx.current_task_flow_ref() {
        Ok(flow_ref) => flow_ref,
        Err(error) => trap_selector_failed(frame, record, error),
    };
    let return_cpu = match ctx.current_cpu() {
        Ok(current_cpu) => current_cpu.cpu_ref(),
        Err(error) => trap_selector_failed(frame, record, error),
    };
    let same_continuation =
        task_ref.same_identity(entry.task_ref) && task_flow_ref.same_identity(entry.task_flow_ref);
    let committed_terminal_switch = migration_allowed
        && !task_ref.same_identity(entry.task_ref)
        && ctx.scheduler().switch_to_entry_prev_ref() == entry.task_ref
        && ctx.scheduler().switch_to_entry_next_ref() == task_ref
        && ctx.scheduler().switch_to_exit_prev_ref() == entry.task_ref
        && ctx.scheduler().switch_to_exit_next_ref() == task_ref
        && ctx.scheduler().switch_to_exit_current_ref() == task_ref
        && ctx.scheduler().switch_to_exit_count() != 0;
    if (!same_continuation && !committed_terminal_switch)
        || !return_cpu.is_valid()
        || (!migration_allowed && return_cpu != entry.cpu_ref)
    {
        trap_return_authority_failed(
            frame,
            record,
            entry,
            task_ref,
            task_flow_ref,
            return_cpu,
            migration_allowed,
        )
    }
    TrapReturnAuthority {
        cpu_ref: return_cpu,
        committed_terminal_switch,
    }
}

const fn trap_return_condition_error() -> EventError {
    EventError::failed(
        EventErrorCode::ConditionFailed,
        LifecycleEvent::Cleanup,
        State::Destroyed,
        State::Destroyed,
        State::Destroyed,
    )
}

fn trap_selector_failed(
    frame: &TrapFrame,
    record: &TrapExecutionRecord,
    error: CurrentTaskError,
) -> ! {
    let diagnostic = error.diagnostic();
    crate::arch::riscv64::sbi::putstr("trap selector resolution failure code=");
    sbi_put_hex(error.code() as usize);
    crate::arch::riscv64::sbi::putstr(" tp=");
    sbi_put_hex(diagnostic.tp());
    print_trap_identity(
        frame,
        record,
        diagnostic.task_ref(),
        diagnostic.flow_ref(),
        diagnostic.cpu_ref(),
    )
}

fn trap_authority_failed(
    frame: &TrapFrame,
    record: &TrapExecutionRecord,
    task_ref: TaskRef,
    task_flow_ref: TaskFlowRef,
    cpu_ref: CpuRef,
) -> ! {
    crate::arch::riscv64::sbi::putstr("trap execution authority mismatch");
    print_trap_identity(frame, record, task_ref, task_flow_ref, cpu_ref)
}

fn trap_return_authority_failed(
    frame: &TrapFrame,
    record: &TrapExecutionRecord,
    entry: TrapEntryAuthority,
    task_ref: TaskRef,
    task_flow_ref: TaskFlowRef,
    cpu_ref: CpuRef,
    migration_allowed: bool,
) -> ! {
    let mismatch = ((!task_ref.same_identity(entry.task_ref)) as usize)
        | (((!task_flow_ref.same_identity(entry.task_flow_ref)) as usize) << 1)
        | (((!cpu_ref.is_valid()) as usize) << 2)
        | (((!migration_allowed && cpu_ref != entry.cpu_ref) as usize) << 3);
    crate::arch::riscv64::sbi::putstr("trap return authority mismatch mask=");
    sbi_put_hex(mismatch);
    crate::arch::riscv64::sbi::putstr(" entry_cpu=");
    sbi_put_hex(entry.cpu_ref.logical_id());
    crate::arch::riscv64::sbi::putstr(" entry_task=");
    sbi_put_hex(entry.task_ref.slot());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(entry.task_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" entry_task_flow=");
    sbi_put_hex(entry.task_flow_ref.slot());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(entry.task_flow_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" return");
    print_trap_identity(frame, record, task_ref, task_flow_ref, cpu_ref)
}

fn trap_occurrence_failed(
    frame: &TrapFrame,
    record: &TrapExecutionRecord,
    entry: TrapEntryAuthority,
    error: EventError,
) -> ! {
    crate::arch::riscv64::sbi::putstr("trap occurrence lifecycle failure event=");
    sbi_put_hex(error.event_code() as usize);
    crate::arch::riscv64::sbi::putstr(" code=");
    sbi_put_hex(error.error_code() as usize);
    if let Some(diagnostic) = error.diagnostic() {
        crate::arch::riscv64::sbi::putstr(" stage=");
        crate::arch::riscv64::sbi::putstr(diagnostic.step);
        crate::arch::riscv64::sbi::putstr(" first_failed=");
        crate::arch::riscv64::sbi::putstr(diagnostic.first_failed);
    }
    let stored_root = crate::context::context().task_root_trap_flow_ref(entry.task_ref);
    crate::arch::riscv64::sbi::putstr(" stored_root=");
    sbi_put_hex(stored_root.address());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(stored_root.generation() as usize);
    print_trap_identity(
        frame,
        record,
        entry.task_ref,
        entry.task_flow_ref,
        entry.cpu_ref,
    )
}

fn print_trap_identity(
    frame: &TrapFrame,
    record: &TrapExecutionRecord,
    task_ref: TaskRef,
    task_flow_ref: TaskFlowRef,
    cpu_ref: CpuRef,
) -> ! {
    let root_ref = record.root.flow_ref();
    let child_ref = record.root.active_child();
    let leaf_ref = record.exception.active_child();
    crate::arch::riscv64::sbi::putstr(" cpu=");
    sbi_put_hex(cpu_ref.logical_id());
    crate::arch::riscv64::sbi::putstr(" task=");
    sbi_put_hex(task_ref.slot());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(task_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" task_flow=");
    sbi_put_hex(task_flow_ref.slot());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(task_flow_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" root=");
    sbi_put_hex(root_ref.address());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(root_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" child=");
    sbi_put_hex(child_ref.address());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(child_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" leaf=");
    sbi_put_hex(leaf_ref.address());
    crate::arch::riscv64::sbi::putstr(":");
    sbi_put_hex(leaf_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" stack_record=[");
    sbi_put_hex(frame as *const TrapFrame as usize);
    crate::arch::riscv64::sbi::putstr(",");
    sbi_put_hex(frame as *const TrapFrame as usize + TRAP_STACK_RECORD_SIZE);
    crate::arch::riscv64::sbi::putstr(") scause=");
    sbi_put_hex(frame.scause);
    crate::arch::riscv64::sbi::putstr(" sepc=");
    sbi_put_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=");
    sbi_put_hex(frame.stval);
    crate::arch::riscv64::sbi::putstr("\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

const _: () =
    assert!(core::mem::align_of::<TrapExecutionRecord>() <= core::mem::align_of::<usize>());
const _: () = assert!(TRAP_STACK_RECORD_SIZE <= i16::MAX as usize);
