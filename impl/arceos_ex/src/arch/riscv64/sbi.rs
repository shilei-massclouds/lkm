const EID_LEGACY_CONSOLE_PUTCHAR: usize = 1;
const EID_LEGACY_SET_TIMER: usize = 0;
const EID_SRST: usize = 0x5352_5354;
const FID_SYSTEM_RESET: usize = 0;
const RESET_TYPE_SHUTDOWN: usize = 0;
const RESET_REASON_NONE: usize = 0;

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

pub fn putchar(byte: u8) {
    let _ = sbi_call_1(EID_LEGACY_CONSOLE_PUTCHAR, 0, byte as usize);
}

pub fn putstr(message: &str) {
    for byte in message.bytes() {
        putchar(byte);
    }
}

pub fn system_shutdown() -> ! {
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
