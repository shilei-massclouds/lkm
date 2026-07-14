const EID_LEGACY_CONSOLE_PUTCHAR: usize = 1;
#[cfg_attr(not(app_smoke), allow(dead_code))]
const EID_LEGACY_SET_TIMER: usize = 0;
const EID_HSM: usize = 0x0048_534d;
const EID_SRST: usize = 0x5352_5354;
const FID_HSM_HART_START: usize = 0;
const FID_SYSTEM_RESET: usize = 0;
const RESET_TYPE_SHUTDOWN: usize = 0;
const RESET_REASON_NONE: usize = 0;

// Direct timer programming is exercised by smoke/KUnit configurations.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub fn set_timer(stime_value: u64) {
    let _ = sbi_call_1(EID_LEGACY_SET_TIMER, 0, stime_value as usize);
}

pub fn read_time() -> u64 {
    let value: usize;

    unsafe {
        core::arch::asm!("rdtime {value}", value = out(reg) value, options(nostack, nomem));
    }

    value as u64
}

pub fn hart_start(hartid: usize, start_addr: usize, opaque: usize) -> Result<usize, usize> {
    let (error, value) = sbi_call_3(EID_HSM, FID_HSM_HART_START, hartid, start_addr, opaque);
    if error == 0 { Ok(value) } else { Err(error) }
}

pub fn putchar(byte: u8) {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::stress_mem::capture_byte(byte);
    #[cfg(not(checkpoint_handler_stress_mem))]
    putchar_raw(byte);
}

pub fn putchar_raw(byte: u8) {
    let _ = sbi_call_1(EID_LEGACY_CONSOLE_PUTCHAR, 0, byte as usize);
}

pub fn putstr(message: &str) {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::stress_mem::capture_bytes(message.as_bytes());
    #[cfg(not(checkpoint_handler_stress_mem))]
    putstr_raw(message);
}

pub fn putstr_raw(message: &str) {
    for byte in message.bytes() {
        putchar_raw(byte);
    }
}

pub fn system_shutdown() -> ! {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::stress_mem::finish();

    let _ = sbi_call_2(
        EID_SRST,
        FID_SYSTEM_RESET,
        RESET_TYPE_SHUTDOWN,
        RESET_REASON_NONE,
    );

    loop {
        core::hint::spin_loop();
    }
}

#[inline(always)]
fn sbi_call_1(eid: usize, fid: usize, arg0: usize) -> (usize, usize) {
    let error: usize;
    let value: usize;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            lateout("a1") value,
            in("a6") fid,
            in("a7") eid,
            options(nostack)
        );
    }

    (error, value)
}

#[inline(always)]
fn sbi_call_2(eid: usize, fid: usize, arg0: usize, arg1: usize) -> (usize, usize) {
    let error: usize;
    let value: usize;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            inlateout("a1") arg1 => value,
            in("a6") fid,
            in("a7") eid,
            options(nostack)
        );
    }

    (error, value)
}

#[inline(always)]
fn sbi_call_3(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> (usize, usize) {
    let error: usize;
    let value: usize;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            inlateout("a1") arg1 => value,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid,
            options(nostack)
        );
    }

    (error, value)
}
