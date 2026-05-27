pub mod cases;

use crate::{
    arch::riscv64::sbi,
    objects::{earlycon, printk},
};

pub enum SmokeResult {
    Passed,
    #[allow(dead_code)]
    Failed,
}

pub struct SmokeCase {
    pub name: &'static str,
    pub run: fn() -> SmokeResult,
}

const CASES: &[SmokeCase] = &[SmokeCase {
    name: "hello_output",
    run: cases::hello::run,
}];

pub fn run() -> ! {
    printk::write_str("arceos_ex smoke start\n");

    let mut passed = 0usize;
    let mut failed = 0usize;

    for case in CASES {
        printk::write_str("[smoke] ");
        printk::write_str(case.name);
        printk::write_str(": ");

        match (case.run)() {
            SmokeResult::Passed => {
                passed += 1;
                printk::write_str("ok\n");
            }
            SmokeResult::Failed => {
                failed += 1;
                printk::write_str("failed\n");
            }
        }
    }

    printk::write_str("arceos_ex smoke passed=");
    write_usize(passed);
    printk::write_str(" failed=");
    write_usize(failed);
    printk::write_str("\n");
    earlycon::drain_printk();
    sbi::system_shutdown()
}

fn write_usize(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;

    if value == 0 {
        printk::write_str("0");
        return;
    }

    while value != 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len != 0 {
        len -= 1;
        printk::write_byte(digits[len]);
    }
}
