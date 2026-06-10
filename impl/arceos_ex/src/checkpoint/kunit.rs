use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::{objects::printk, trace::Checkpoint};

const SUITE_NAME: &str = "arceos_ex";

static STARTED: AtomicBool = AtomicBool::new(false);
static FINISHED: AtomicBool = AtomicBool::new(false);
static NEXT_CASE: AtomicUsize = AtomicUsize::new(1);
static FAILED: AtomicUsize = AtomicUsize::new(0);

pub fn start_case(total: usize, prefix: &str, name: &str, checkpoint: Checkpoint) {
    start(total);
    putstr("  # checkpoint: ");
    putstr(checkpoint.name());
    putchar(b'\n');
    putstr("  # running: ");
    putstr(prefix);
    putstr(name);
    putchar(b'\n');
}

pub fn pass(total: usize, prefix: &str, name: &str) {
    result(total, true, prefix, name);
}

pub fn fail(total: usize, prefix: &str, name: &str, reason: &str) {
    diag(reason);
    result(total, false, prefix, name);
}

pub fn diag(message: &str) {
    putstr("  # ");
    putstr(message);
    putchar(b'\n');
}

pub fn diag_hex_pair(label: &str, first: usize, second: usize) {
    putstr("  # ");
    putstr(label);
    putstr("=0x");
    put_hex(first);
    putstr(",0x");
    put_hex(second);
    putchar(b'\n');
}

pub fn diag_usize(label: &str, value: usize) {
    putstr("  # ");
    putstr(label);
    putstr("=");
    put_usize(value);
    putchar(b'\n');
}

pub fn drain_printk_diag() {
    let mut at_line_start = true;
    let mut wrote = false;
    printk::drain_to(|byte| {
        if at_line_start {
            putstr("  # ");
            at_line_start = false;
            wrote = true;
        }
        putchar(byte);
        if byte == b'\n' {
            at_line_start = true;
        }
    });
    if wrote && !at_line_start {
        putchar(b'\n');
    }
}

fn start(total: usize) {
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }

    putstr("KTAP version 1\n");
    putstr("1..1\n");
    putstr("  KTAP version 1\n");
    putstr("  # Subtest: ");
    putstr(SUITE_NAME);
    putchar(b'\n');
    putstr("  1..");
    put_usize(total);
    putchar(b'\n');
}

fn result(total: usize, passed: bool, prefix: &str, name: &str) {
    let case_no = NEXT_CASE.fetch_add(1, Ordering::AcqRel);
    if !passed {
        FAILED.fetch_add(1, Ordering::AcqRel);
    }

    putstr("  ");
    if !passed {
        putstr("not ");
    }
    putstr("ok ");
    put_usize(case_no);
    putchar(b' ');
    putstr(prefix);
    putstr(name);
    putchar(b'\n');

    if case_no == total && !FINISHED.swap(true, Ordering::AcqRel) {
        if FAILED.load(Ordering::Acquire) == 0 {
            putstr("ok 1 ");
        } else {
            putstr("not ok 1 ");
        }
        putstr(SUITE_NAME);
        putchar(b'\n');
    }
}

fn putstr(message: &str) {
    crate::arch::riscv64::sbi::putstr(message);
}

fn putchar(byte: u8) {
    crate::arch::riscv64::sbi::putchar(byte);
}

fn put_usize(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;

    if value == 0 {
        putchar(b'0');
        return;
    }

    while value != 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len != 0 {
        len -= 1;
        putchar(digits[len]);
    }
}

fn put_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        putchar(HEX[(value >> shift) & 0xf]);
    }
}
