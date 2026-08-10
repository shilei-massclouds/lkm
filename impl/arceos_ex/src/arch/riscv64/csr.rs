use core::arch::global_asm;

use super::{SUPERVISOR_EXTERNAL_IRQ, SUPERVISOR_SOFTWARE_IRQ, SUPERVISOR_TIMER_IRQ};

#[allow(dead_code)]
pub const SSTATUS_SIE: usize = 1 << 1;
pub const SSTATUS_SPIE: usize = 1 << 5;
pub const SSTATUS_SPP: usize = 1 << 8;
pub const SSTATUS_SUM: usize = 1 << 18;
const SIE_STIE: usize = 1 << SUPERVISOR_TIMER_IRQ;
const SIE_SEIE: usize = 1 << SUPERVISOR_EXTERNAL_IRQ;
const SIE_SSIE: usize = 1 << SUPERVISOR_SOFTWARE_IRQ;
pub const SSTATUS_VS: usize = 0b11 << 9;
pub const SSTATUS_FS: usize = 0b11 << 13;
pub const SSTATUS_FS_INITIAL: usize = 0b01 << 13;
pub const SSTATUS_FS_VS: usize = SSTATUS_FS | SSTATUS_VS;

pub const SATP_MODE_SV39: usize = 8usize << 60;

global_asm!(
    r#"
    .section .head.text.vm_switch, "ax"
    .align 2
    .globl arceos_ex_switch_to_early_vm
arceos_ex_switch_to_early_vm:
    /*
     * a0 = trampoline satp
     * a1 = early satp
     * a2 = virtual stack pointer
     * a3 = virtual gp
     * a4 = virtual continuation
     * a5 = kernel virtual offset
     * a6 = physical address of the current CPU translation-state storage
     *
     * This follows the Linux/RISC-V relocation idea: after writing the
     * trampoline satp, the next physical PC is not mapped, so the CPU traps
     * to the virtual stvec label that is covered by the trampoline mapping.
     * The sfence.vma before writing trampoline satp and the sfence.vma after
     * writing early satp are the implementation facts required by
     * TrampolineVm.ActivateOnCpu and EarlyVm.ActivateOnCpu.
     */
    la      t0, 1f
    add     t0, t0, a5
    csrw    stvec, t0
    csrr    t0, satp
    bnez    t0, .Lbp_translation_fail
    lbu     t0, {state_controller_offset}(a6)
    li      t1, {physical_controller}
    bne     t0, t1, .Lbp_translation_fail
    ld      t0, {state_count_offset}(a6)
    li      t1, 1
    bne     t0, t1, .Lbp_translation_fail
    beqz    a0, .Lbp_translation_fail
    beqz    a1, .Lbp_translation_fail
    sfence.vma
    csrw    satp, a0
    .balign 4
1:
    mv      sp, a2
    mv      gp, a3
    add     a6, a6, a5
    csrr    t0, satp
    bne     t0, a0, .Lbp_translation_fail

    li      t0, {physical_controller}
    sb      t0, {bp_trampoline_old_offset}(a6)
    li      t0, {trampoline_controller}
    sb      t0, {bp_trampoline_new_offset}(a6)
    li      t0, 1
    sb      t0, {bp_trampoline_sync_offset}(a6)
    li      t0, {handoff_kind}
    sb      t0, {bp_trampoline_kind_offset}(a6)
    sd      a0, {bp_trampoline_satp_offset}(a6)
    li      t0, 2
    sd      t0, {bp_trampoline_sequence_offset}(a6)
    fence   rw, w
    li      t0, {trampoline_controller}
    sb      t0, {state_controller_offset}(a6)
    fence   rw, w
    li      t0, 2
    sd      t0, {state_count_offset}(a6)

    lbu     t0, {state_controller_offset}(a6)
    li      t1, {trampoline_controller}
    bne     t0, t1, .Lbp_translation_fail
    ld      t0, {state_count_offset}(a6)
    li      t1, 2
    bne     t0, t1, .Lbp_translation_fail
    csrw    satp, a1
    sfence.vma
    csrr    t0, satp
    bne     t0, a1, .Lbp_translation_fail

    li      t0, {trampoline_controller}
    sb      t0, {bp_early_old_offset}(a6)
    li      t1, {early_controller}
    sb      t1, {bp_early_new_offset}(a6)
    li      t0, 1
    sb      t0, {bp_early_sync_offset}(a6)
    li      t0, {handoff_kind}
    sb      t0, {bp_early_kind_offset}(a6)
    sd      a1, {bp_early_satp_offset}(a6)
    li      t0, 3
    sd      t0, {bp_early_sequence_offset}(a6)
    fence   rw, w
    sb      t1, {state_controller_offset}(a6)
    fence   rw, w
    li      t0, 3
    sd      t0, {state_count_offset}(a6)
    jr      a4

.Lbp_translation_fail:
    li      a7, 8
    ecall
2:
    wfi
    j       2b

    .section .text.user_entry, "ax"
    .align 2
    .globl arceos_ex_enter_user_mode
arceos_ex_enter_user_mode:
    /*
     * a0 = user satp
     * a1 = user entry
     * a2 = user stack pointer
     * a3 = user sstatus
     * a4 = current CPU TrapEntryContext
     */
    sd      tp, {trap_context_task_offset}(a4)
    csrw    sscratch, tp
    csrw    sepc, a1
    csrw    sstatus, a3
    csrw    satp, a0
    sfence.vma
    mv      sp, a2
    sret

    .align 2
    .globl arceos_ex_enter_user_mode_from_trap_frame
arceos_ex_enter_user_mode_from_trap_frame:
    /*
     * a0 = user satp
     * a1 = complete TrapFrame copied by fork
     * a2 = current CPU TrapEntryContext
     *
     * A fork continuation is not an ELF entry point: every user register is
     * observable state.  Keep the Task identity in sscratch, then restore the
     * complete saved frame exactly as the formal trap-return path does.
     */
    sd      tp, {trap_context_task_offset}(a2)
    csrw    sscratch, tp
    mv      sp, a1
    ld      t0, {trap_frame_sepc_offset}(sp)
    csrw    sepc, t0
    ld      t0, {trap_frame_sstatus_offset}(sp)
    csrw    sstatus, t0
    csrw    satp, a0
    sfence.vma

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
"#,
    state_controller_offset = const crate::objects::cpu::TRANSLATION_STATE_CONTROLLER_OFFSET,
    state_count_offset = const crate::objects::cpu::TRANSLATION_STATE_COMMITTED_COUNT_OFFSET,
    bp_trampoline_old_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_OLD_CONTROLLER_OFFSET,
    bp_trampoline_new_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_NEW_CONTROLLER_OFFSET,
    bp_trampoline_sync_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_SYNC_COMPLETE_OFFSET,
    bp_trampoline_kind_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_KIND_OFFSET,
    bp_trampoline_satp_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_SATP_OFFSET,
    bp_trampoline_sequence_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_SEQUENCE_OFFSET,
    bp_early_old_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + 2 * crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_OLD_CONTROLLER_OFFSET,
    bp_early_new_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + 2 * crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_NEW_CONTROLLER_OFFSET,
    bp_early_sync_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + 2 * crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_SYNC_COMPLETE_OFFSET,
    bp_early_kind_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + 2 * crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_KIND_OFFSET,
    bp_early_satp_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + 2 * crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_SATP_OFFSET,
    bp_early_sequence_offset = const crate::objects::cpu::TRANSLATION_STATE_JOURNAL_OFFSET + 2 * crate::objects::cpu::TRANSLATION_RECEIPT_SIZE + crate::objects::cpu::TRANSLATION_RECEIPT_SEQUENCE_OFFSET,
    early_controller = const crate::objects::cpu::TranslationController::EarlyVm as u8,
    physical_controller = const crate::objects::cpu::TranslationController::PhysicalDirect as u8,
    trampoline_controller = const crate::objects::cpu::TranslationController::TrampolineVm as u8,
    handoff_kind = const crate::objects::cpu::TranslationActivationKind::Handoff as u8,
    trap_context_task_offset = const crate::objects::trap_type::TRAP_ENTRY_CONTEXT_TASK_OFFSET,
    trap_frame_sstatus_offset = const crate::objects::trap_type::TRAP_FRAME_SSTATUS_OFFSET,
    trap_frame_sepc_offset = const crate::objects::trap_type::TRAP_FRAME_SEPC_OFFSET,
);

unsafe extern "C" {
    fn arceos_ex_switch_to_early_vm(
        trampoline_satp: usize,
        early_satp: usize,
        stack_virt: usize,
        gp_virt: usize,
        continuation_virt: usize,
        kernel_virt_offset: usize,
        translation_state_phys: usize,
    ) -> !;
    #[cfg(app_user_boot)]
    fn arceos_ex_enter_user_mode(
        user_satp: usize,
        user_entry: usize,
        user_sp: usize,
        user_sstatus: usize,
        user_trap_entry_context: usize,
    ) -> !;
    #[cfg(app_user_boot)]
    fn arceos_ex_enter_user_mode_from_trap_frame(
        user_satp: usize,
        frame: *const crate::objects::trap_type::TrapFrame,
        user_trap_entry_context: usize,
    ) -> !;
}

pub fn kernel_fpu_vector_disabled() -> bool {
    read_sstatus() & SSTATUS_FS_VS == 0
}

pub fn read_sie() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("csrr {value}, sie", value = out(reg) value, options(nostack, nomem));
    }

    value
}

#[allow(dead_code)]
pub fn disable_supervisor_interrupts() {
    clear_sstatus_bits(SSTATUS_SIE);
}

pub fn enable_supervisor_interrupts() {
    set_sstatus_bits(SSTATUS_SIE);
}

pub fn enable_supervisor_software_interrupt() {
    set_sie_bits(SIE_SSIE);
}

pub fn supervisor_software_interrupt_enabled() -> bool {
    read_sie() & SIE_SSIE != 0
}

pub fn clear_supervisor_software_interrupt() {
    unsafe {
        core::arch::asm!("csrrc zero, sip, {mask}", mask = in(reg) SIE_SSIE, options(nostack, nomem));
    }
}

pub fn disable_supervisor_timer_interrupt() {
    clear_sie_bits(SIE_STIE);
}

// Timer-interrupt exercises are selected by smoke/KUnit configurations.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub fn enable_supervisor_timer_interrupt() {
    set_sie_bits(SIE_STIE);
}

#[allow(dead_code)]
pub fn disable_supervisor_external_interrupt() {
    clear_sie_bits(SIE_SEIE);
}

#[allow(dead_code)]
pub fn enable_supervisor_external_interrupt() {
    set_sie_bits(SIE_SEIE);
}

pub fn read_gp() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("mv {value}, gp", value = out(reg) value, options(nostack, nomem));
    }

    value
}

pub fn read_tp() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("mv {value}, tp", value = out(reg) value, options(nostack, nomem));
    }

    value
}

pub fn read_sp() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("mv {value}, sp", value = out(reg) value, options(nostack, nomem));
    }

    value
}

pub fn read_sstatus() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("csrr {value}, sstatus", value = out(reg) value, options(nostack, nomem));
    }

    value
}

pub fn supervisor_interrupts_enabled() -> bool {
    read_sstatus() & SSTATUS_SIE != 0
}

pub fn save_and_disable_supervisor_interrupts() -> usize {
    let saved = read_sstatus();
    disable_supervisor_interrupts();
    saved
}

pub fn restore_supervisor_interrupts(saved_sstatus: usize) {
    if saved_sstatus & SSTATUS_SIE != 0 {
        enable_supervisor_interrupts();
    } else {
        disable_supervisor_interrupts();
    }
}

pub fn save_and_enable_user_memory_access() -> usize {
    let saved = read_sstatus();
    set_sstatus_bits(SSTATUS_SUM);
    saved
}

pub fn restore_user_memory_access(saved_sstatus: usize) {
    if saved_sstatus & SSTATUS_SUM != 0 {
        set_sstatus_bits(SSTATUS_SUM);
    } else {
        clear_sstatus_bits(SSTATUS_SUM);
    }
}

pub fn supervisor_external_interrupt_enabled() -> bool {
    read_sie() & SIE_SEIE != 0
}

pub fn read_satp() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("csrr {value}, satp", value = out(reg) value, options(nostack, nomem));
    }

    value
}

pub fn write_tp(value: usize) {
    unsafe {
        core::arch::asm!("mv tp, {value}", value = in(reg) value, options(nostack, nomem));
    }
}

pub fn write_stvec(value: usize) {
    unsafe {
        core::arch::asm!("csrw stvec, {value}", value = in(reg) value, options(nostack, nomem));
    }
}

#[cfg(app_smoke)]
pub fn read_stvec() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("csrr {value}, stvec", value = out(reg) value, options(nostack, nomem));
    }

    value
}

pub fn write_satp(value: usize) {
    unsafe {
        core::arch::asm!("csrw satp, {value}", value = in(reg) value, options(nostack, nomem));
    }
}

pub fn sfence_vma() {
    unsafe {
        core::arch::asm!("sfence.vma", options(nostack, nomem));
    }
}

pub fn sfence_vma_addr(addr: usize) {
    unsafe {
        core::arch::asm!("sfence.vma {addr}, zero", addr = in(reg) addr, options(nostack, nomem));
    }
}

pub fn clear_sscratch() {
    unsafe {
        core::arch::asm!("csrw sscratch, zero", options(nostack, nomem));
    }
}

#[allow(dead_code)]
pub fn read_sscratch() -> usize {
    let value: usize;
    unsafe {
        core::arch::asm!("csrr {value}, sscratch", value = out(reg) value, options(nostack, nomem));
    }
    value
}

pub unsafe fn switch_to_early_vm(
    trampoline_satp: usize,
    early_satp: usize,
    stack_virt: usize,
    gp_virt: usize,
    continuation_virt: usize,
    kernel_virt_offset: usize,
    translation_state_phys: usize,
) -> ! {
    // SAFETY: this is the architecture boundary for Vm.Setup. The caller must
    // provide satp values for initialized TrampolineVm/EarlyVm page tables and
    // virtual continuation state covered by EarlyVm.
    unsafe {
        arceos_ex_switch_to_early_vm(
            trampoline_satp,
            early_satp,
            stack_virt,
            gp_virt,
            continuation_virt,
            kernel_virt_offset,
            translation_state_phys,
        )
    }
}

#[cfg(app_user_boot)]
pub unsafe fn enter_user_mode(
    user_satp: usize,
    user_entry: usize,
    user_sp: usize,
    user_sstatus: usize,
    user_trap_entry_context: usize,
) -> ! {
    unsafe {
        arceos_ex_enter_user_mode(
            user_satp,
            user_entry,
            user_sp,
            user_sstatus,
            user_trap_entry_context,
        )
    }
}

#[cfg(app_user_boot)]
pub unsafe fn enter_user_mode_from_trap_frame(
    user_satp: usize,
    frame: &crate::objects::trap_type::TrapFrame,
    user_trap_entry_context: usize,
) -> ! {
    unsafe {
        arceos_ex_enter_user_mode_from_trap_frame(
            user_satp,
            core::ptr::from_ref(frame),
            user_trap_entry_context,
        )
    }
}

fn clear_sstatus_bits(mask: usize) {
    unsafe {
        core::arch::asm!("csrrc zero, sstatus, {mask}", mask = in(reg) mask, options(nostack, nomem));
    }
}

fn set_sstatus_bits(mask: usize) {
    unsafe {
        core::arch::asm!("csrrs zero, sstatus, {mask}", mask = in(reg) mask, options(nostack, nomem));
    }
}

fn set_sie_bits(mask: usize) {
    unsafe {
        core::arch::asm!("csrrs zero, sie, {mask}", mask = in(reg) mask, options(nostack, nomem));
    }
}

fn clear_sie_bits(mask: usize) {
    unsafe {
        core::arch::asm!("csrrc zero, sie, {mask}", mask = in(reg) mask, options(nostack, nomem));
    }
}
