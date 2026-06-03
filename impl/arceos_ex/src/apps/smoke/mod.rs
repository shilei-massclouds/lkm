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

const CASES: &[SmokeCase] = &[
    SmokeCase {
        name: "breakpoint",
        run: cases::breakpoint::run,
    },
    SmokeCase {
        name: "hello",
        run: cases::hello::run,
    },
    SmokeCase {
        name: "print",
        run: cases::print::run,
    },
    SmokeCase {
        name: "resource_tree",
        run: cases::resource_tree::run,
    },
    SmokeCase {
        name: "cpu_id_map",
        run: cases::cpu_id_map::run,
    },
    SmokeCase {
        name: "cache_block_info",
        run: cases::cache_block_info::run,
    },
    SmokeCase {
        name: "cpu_capabilities",
        run: cases::cpu_capabilities::run,
    },
    SmokeCase {
        name: "memblock",
        run: cases::memblock::run,
    },
    SmokeCase {
        name: "per_cpu",
        run: cases::per_cpu::run,
    },
    SmokeCase {
        name: "params",
        run: cases::params::run,
    },
    SmokeCase {
        name: "device_tree",
        run: cases::device_tree::run,
    },
    SmokeCase {
        name: "zones",
        run: cases::zones::run,
    },
    SmokeCase {
        name: "page_allocator",
        run: cases::page_allocator::run,
    },
    SmokeCase {
        name: "slub",
        run: cases::slub::run,
    },
    SmokeCase {
        name: "vmalloc",
        run: cases::vmalloc::run,
    },
    SmokeCase {
        name: "scheduler",
        run: cases::scheduler::run,
    },
    SmokeCase {
        name: "irq_time",
        run: cases::irq_time::run,
    },
    SmokeCase {
        name: "irq_open_prepare",
        run: cases::irq_open_prepare::run,
    },
    SmokeCase {
        name: "delay_loop",
        run: cases::delay_loop::run,
    },
    SmokeCase {
        name: "process_prepare",
        run: cases::process_prepare::run,
    },
    SmokeCase {
        name: "rest_init",
        run: cases::rest_init::run,
    },
    SmokeCase {
        name: "fdt",
        run: cases::fdt::run,
    },
];

pub fn run() -> ! {
    printk::write_str("arceos_ex smoke start\n");

    let mut passed = 0usize;
    let mut failed = 0usize;

    let total = CASES.len();

    for (index, case) in CASES.iter().enumerate() {
        printk::write_str("[");
        write_usize(index + 1);
        printk::write_str("/");
        write_usize(total);
        printk::write_str("] ");
        printk::write_str(case.name);
        printk::write_str("\n");

        match (case.run)() {
            SmokeResult::Passed => {
                passed += 1;
                printk::write_str("  ");
                printk::write_str("\x1b[32m");
                printk::write_str("ok\n");
                printk::write_str("\x1b[0m");
            }
            SmokeResult::Failed => {
                failed += 1;
                printk::write_str("  ");
                printk::write_str("\x1b[31m");
                printk::write_str("FAILED\n");
                printk::write_str("\x1b[0m");
            }
        }
    }

    if failed == 0 {
        printk::write_str("result: \x1b[32mok\x1b[0m. passed=");
    } else {
        printk::write_str("result: \x1b[31mFAILED\x1b[0m. passed=");
    }
    write_usize(passed);
    printk::write_str(" failed=");
    write_usize(failed);
    printk::write_str(" total=");
    write_usize(total);
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
