#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
use core::fmt::{self, Write};

const EID_LEGACY_CONSOLE_PUTCHAR: usize = 1;
#[cfg_attr(not(app_smoke), allow(dead_code))]
const EID_LEGACY_SET_TIMER: usize = 0;
const EID_HSM: usize = 0x0048_534d;
#[cfg_attr(not(app_smoke), allow(dead_code))]
const EID_IPI: usize = 0x0073_5049;
const EID_SRST: usize = 0x5352_5354;
const FID_HSM_HART_START: usize = 0;
#[cfg_attr(not(app_smoke), allow(dead_code))]
const FID_IPI_SEND_IPI: usize = 0;
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

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub fn send_ipi(hartid: usize) -> Result<(), usize> {
    if hartid >= usize::BITS as usize {
        return Err(usize::MAX);
    }
    let (error, _) = sbi_call_2(EID_IPI, FID_IPI_SEND_IPI, 1usize << hartid, 0);
    if error == 0 { Ok(()) } else { Err(error) }
}

pub fn putchar(byte: u8) {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::stress_mem::capture_byte(byte);
    #[cfg(not(checkpoint_handler_stress_mem))]
    {
        if crate::objects::ns16550a::uart8250_interrupt_driven_configured() {
            let _ = crate::objects::ns16550a::write_console_bytes(&[byte]);
        } else {
            putchar_raw(byte);
        }
    }
}

pub fn putchar_raw(byte: u8) {
    let _ = sbi_call_1(EID_LEGACY_CONSOLE_PUTCHAR, 0, byte as usize);
}

pub fn putstr(message: &str) {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::stress_mem::capture_bytes(message.as_bytes());
    #[cfg(not(checkpoint_handler_stress_mem))]
    {
        if crate::objects::ns16550a::uart8250_interrupt_driven_configured() {
            for chunk in message.as_bytes().chunks(256) {
                let _ = crate::objects::ns16550a::write_console_bytes(chunk);
            }
        } else {
            putstr_raw(message);
        }
    }
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
const RUNTIME_DIAGNOSTIC_RECORD_SIZE: usize = 480;

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
struct RuntimeDiagnosticRecord {
    bytes: [u8; RUNTIME_DIAGNOSTIC_RECORD_SIZE],
    len: usize,
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
impl RuntimeDiagnosticRecord {
    const fn new() -> Self {
        Self {
            bytes: [0; RUNTIME_DIAGNOSTIC_RECORD_SIZE],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
impl Write for RuntimeDiagnosticRecord {
    fn write_str(&mut self, message: &str) -> fmt::Result {
        let end = self.len.checked_add(message.len()).ok_or(fmt::Error)?;
        let destination = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        destination.copy_from_slice(message.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
pub fn write_record(args: fmt::Arguments<'_>) {
    let mut record = RuntimeDiagnosticRecord::new();
    if record.write_fmt(args).is_err() {
        write_record_bytes(b"runtime diagnostic record overflow\n");
        return;
    }
    write_record_bytes(record.as_bytes());
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_user_scheduler_trace,
    checkpoint_handler_user_syscall_trace
))]
fn write_record_bytes(bytes: &[u8]) {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::objects::printk::write_bytes(bytes);
    #[cfg(not(checkpoint_handler_stress_mem))]
    {
        if crate::objects::ns16550a::uart8250_interrupt_driven_configured() {
            crate::objects::printk::write_bytes(bytes);
        } else {
            for byte in bytes {
                putchar_raw(*byte);
            }
        }
    }
}

pub fn putstr_raw(message: &str) {
    for byte in message.bytes() {
        putchar_raw(byte);
    }
}

pub fn system_shutdown() -> ! {
    #[cfg(checkpoint_handler_stress_mem)]
    crate::stress_mem::finish();
    #[cfg(not(checkpoint_handler_stress_mem))]
    let _ = crate::objects::ns16550a::flush_runtime_console();

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
