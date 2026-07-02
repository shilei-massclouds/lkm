use core::arch::global_asm;

use super::{SUPERVISOR_EXTERNAL_IRQ, SUPERVISOR_TIMER_IRQ};

#[allow(dead_code)]
pub const SSTATUS_SIE: usize = 1 << 1;
pub const SSTATUS_SPIE: usize = 1 << 5;
pub const SSTATUS_SPP: usize = 1 << 8;
pub const SSTATUS_SUM: usize = 1 << 18;
const SIE_STIE: usize = 1 << SUPERVISOR_TIMER_IRQ;
const SIE_SEIE: usize = 1 << SUPERVISOR_EXTERNAL_IRQ;
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
     *
     * This follows the Linux/RISC-V relocation idea: after writing the
     * trampoline satp, the next physical PC is not mapped, so the CPU traps
     * to the virtual stvec label that is covered by the trampoline mapping.
     * The sfence.vma before writing trampoline satp and the sfence.vma after
     * writing early satp are the implementation facts required by
     * TrampolineVm.Enable and EarlyVm.Enable.
     */
    la      t0, 1f
    add     t0, t0, a5
    csrw    stvec, t0
    sfence.vma
    csrw    satp, a0
    .balign 4
1:
    mv      sp, a2
    mv      gp, a3
    csrw    satp, a1
    sfence.vma
    jr      a4

    .section .text.user_entry, "ax"
    .align 2
    .globl arceos_ex_enter_user_mode
arceos_ex_enter_user_mode:
    /*
     * a0 = user satp
     * a1 = user entry
     * a2 = user stack pointer
     * a3 = user sstatus
     * a4 = kernel trap stack top for sscratch
     */
    csrw    sscratch, a4
    csrw    sepc, a1
    csrw    sstatus, a3
    csrw    satp, a0
    sfence.vma
    mv      sp, a2
    sret
"#
);

unsafe extern "C" {
    fn arceos_ex_switch_to_early_vm(
        trampoline_satp: usize,
        early_satp: usize,
        stack_virt: usize,
        gp_virt: usize,
        continuation_virt: usize,
        kernel_virt_offset: usize,
    ) -> !;
    #[cfg(app_user_boot)]
    fn arceos_ex_enter_user_mode(
        user_satp: usize,
        user_entry: usize,
        user_sp: usize,
        user_sstatus: usize,
        kernel_trap_stack_top: usize,
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

pub fn clear_supervisor_interrupt_pending() {
    unsafe {
        core::arch::asm!("csrw sip, zero", options(nostack, nomem));
    }
}

#[allow(dead_code)]
pub fn disable_supervisor_interrupts() {
    clear_sstatus_bits(SSTATUS_SIE);
}

pub fn enable_supervisor_interrupts() {
    set_sstatus_bits(SSTATUS_SIE);
}

pub fn disable_supervisor_timer_interrupt() {
    clear_sie_bits(SIE_STIE);
}

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

#[allow(dead_code)]
pub fn clear_sscratch() {
    unsafe {
        core::arch::asm!("csrw sscratch, zero", options(nostack, nomem));
    }
}

pub unsafe fn switch_to_early_vm(
    trampoline_satp: usize,
    early_satp: usize,
    stack_virt: usize,
    gp_virt: usize,
    continuation_virt: usize,
    kernel_virt_offset: usize,
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
        )
    }
}

#[cfg(app_user_boot)]
pub unsafe fn enter_user_mode(
    user_satp: usize,
    user_entry: usize,
    user_sp: usize,
    user_sstatus: usize,
    kernel_trap_stack_top: usize,
) -> ! {
    unsafe {
        arceos_ex_enter_user_mode(
            user_satp,
            user_entry,
            user_sp,
            user_sstatus,
            kernel_trap_stack_top,
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
