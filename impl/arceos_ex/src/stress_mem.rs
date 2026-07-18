use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::arch::riscv64::sbi;

#[cfg(stress_mem_bytes_32768)]
const BUFFER_CAPACITY: usize = 32768;
#[cfg(stress_mem_bytes_65536)]
const BUFFER_CAPACITY: usize = 65536;
#[cfg(stress_mem_bytes_131072)]
const BUFFER_CAPACITY: usize = 131072;
#[cfg(stress_mem_bytes_262144)]
const BUFFER_CAPACITY: usize = 262144;
#[cfg(stress_mem_bytes_524288)]
const BUFFER_CAPACITY: usize = 524288;
#[cfg(not(any(
    stress_mem_bytes_32768,
    stress_mem_bytes_65536,
    stress_mem_bytes_131072,
    stress_mem_bytes_262144,
    stress_mem_bytes_524288
)))]
const BUFFER_CAPACITY: usize = 65536;

static WRITE: AtomicUsize = AtomicUsize::new(0);
static DROPPED: AtomicUsize = AtomicUsize::new(0);
static OVERFLOWED: AtomicBool = AtomicBool::new(false);
static DUMPING: AtomicBool = AtomicBool::new(false);
static FINISHED: AtomicBool = AtomicBool::new(false);

static mut BUFFER: [u8; BUFFER_CAPACITY] = [0; BUFFER_CAPACITY];

pub fn capture_bytes(bytes: &[u8]) {
    if DUMPING.load(Ordering::Acquire) {
        return;
    }

    for byte in bytes {
        capture_byte(*byte);
    }
}

pub fn capture_byte(byte: u8) {
    if DUMPING.load(Ordering::Acquire) {
        return;
    }

    sbi::putchar_raw(byte);

    let index = WRITE.fetch_add(1, Ordering::Relaxed);
    if index >= BUFFER_CAPACITY {
        OVERFLOWED.store(true, Ordering::Release);
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }

    unsafe {
        let buffer = core::ptr::addr_of_mut!(BUFFER) as *mut u8;
        buffer.add(index).write_volatile(byte);
    }
}

pub fn finish() {
    if FINISHED.swap(true, Ordering::AcqRel) {
        return;
    }

    DUMPING.store(true, Ordering::Release);
    let written = WRITE.load(Ordering::Acquire);
    let saved = if written < BUFFER_CAPACITY {
        written
    } else {
        BUFFER_CAPACITY
    };

    sbi::putstr_raw("stress_mem: v=1 encoding=hex bytes=");
    print_usize(saved);
    sbi::putstr_raw(" total=");
    print_usize(written);
    sbi::putstr_raw(" overflow=");
    print_bool(OVERFLOWED.load(Ordering::Acquire));
    sbi::putstr_raw(" dropped=");
    print_usize(DROPPED.load(Ordering::Acquire));
    sbi::putstr_raw(" data=");
    dump_hex(saved);
    sbi::putchar_raw(b'\n');
    DUMPING.store(false, Ordering::Release);
}

fn dump_hex(saved: usize) {
    let mut index = 0usize;
    while index < saved {
        let byte = unsafe {
            let buffer = core::ptr::addr_of!(BUFFER) as *const u8;
            buffer.add(index).read_volatile()
        };
        sbi::putchar_raw(hex_digit(byte >> 4));
        sbi::putchar_raw(hex_digit(byte & 0x0f));
        index += 1;
    }
}

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + (value - 10),
    }
}

fn print_bool(value: bool) {
    if value {
        sbi::putstr_raw("1");
    } else {
        sbi::putstr_raw("0");
    }
}

fn print_usize(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;

    if value == 0 {
        sbi::putchar_raw(b'0');
        return;
    }

    while value != 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len != 0 {
        len -= 1;
        sbi::putchar_raw(digits[len]);
    }
}
