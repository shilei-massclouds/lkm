#[allow(dead_code)]
const SSTATUS_SIE: usize = 1 << 1;
const SSTATUS_VS: usize = 0b11 << 9;
const SSTATUS_FS: usize = 0b11 << 13;

pub fn disable_kernel_fpu_vector() {
    clear_sstatus_bits(SSTATUS_FS | SSTATUS_VS);
}

pub fn close_interrupt_stream() {
    unsafe {
        core::arch::asm!("csrw sie, zero", "csrw sip, zero", options(nostack, nomem));
    }
}

#[allow(dead_code)]
pub fn disable_supervisor_interrupts() {
    clear_sstatus_bits(SSTATUS_SIE);
}

pub fn read_gp() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!("mv {value}, gp", value = out(reg) value, options(nostack, nomem));
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

#[allow(dead_code)]
pub fn clear_sscratch() {
    unsafe {
        core::arch::asm!("csrw sscratch, zero", options(nostack, nomem));
    }
}

fn clear_sstatus_bits(mask: usize) {
    unsafe {
        core::arch::asm!("csrrc zero, sstatus, {mask}", mask = in(reg) mask, options(nostack, nomem));
    }
}
