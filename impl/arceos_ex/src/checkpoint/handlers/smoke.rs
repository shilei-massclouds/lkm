use crate::{
    apps::smoke::{cases, SmokeResult},
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
pub const KUNIT_CASE_COUNT: usize = CASES.len();

struct SmokeKunitCase {
    name: &'static str,
    run: fn() -> SmokeResult,
}

const CASES: &[SmokeKunitCase] = &[
    SmokeKunitCase {
        name: "breakpoint",
        run: cases::breakpoint::run,
    },
    SmokeKunitCase {
        name: "hello",
        run: cases::hello::run,
    },
    SmokeKunitCase {
        name: "print",
        run: cases::print::run,
    },
    SmokeKunitCase {
        name: "resource_tree",
        run: cases::resource_tree::run,
    },
    SmokeKunitCase {
        name: "cpu_id_map",
        run: cases::cpu_id_map::run,
    },
    SmokeKunitCase {
        name: "cache_block_info",
        run: cases::cache_block_info::run,
    },
    SmokeKunitCase {
        name: "cpu_capabilities",
        run: cases::cpu_capabilities::run,
    },
    SmokeKunitCase {
        name: "memblock",
        run: cases::memblock::run,
    },
    SmokeKunitCase {
        name: "per_cpu",
        run: cases::per_cpu::run,
    },
    SmokeKunitCase {
        name: "params",
        run: cases::params::run,
    },
    SmokeKunitCase {
        name: "device_tree",
        run: cases::device_tree::run,
    },
    SmokeKunitCase {
        name: "zones",
        run: cases::zones::run,
    },
    SmokeKunitCase {
        name: "page_allocator",
        run: cases::page_allocator::run,
    },
    SmokeKunitCase {
        name: "slub",
        run: cases::slub::run,
    },
    SmokeKunitCase {
        name: "vmalloc",
        run: cases::vmalloc::run,
    },
    SmokeKunitCase {
        name: "scheduler",
        run: cases::scheduler::run,
    },
    SmokeKunitCase {
        name: "irq_time",
        run: cases::irq_time::run,
    },
    SmokeKunitCase {
        name: "irq_open_prepare",
        run: cases::irq_open_prepare::run,
    },
    SmokeKunitCase {
        name: "delay_loop",
        run: cases::delay_loop::run,
    },
    SmokeKunitCase {
        name: "process_prepare",
        run: cases::process_prepare::run,
    },
    SmokeKunitCase {
        name: "rest_init",
        run: cases::rest_init::run,
    },
    SmokeKunitCase {
        name: "pre_smp_init",
        run: cases::pre_smp_init::run,
    },
    SmokeKunitCase {
        name: "smp_bringup",
        run: cases::smp_bringup::run,
    },
    SmokeKunitCase {
        name: "runtime_core",
        run: cases::runtime_core::run,
    },
    SmokeKunitCase {
        name: "initcall",
        run: cases::initcall::run,
    },
    SmokeKunitCase {
        name: "fdt",
        run: cases::fdt::run,
    },
];

pub const HANDLER: Handler = Handler {
    name: "smoke",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, _ctx: &mut Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    let mut index = 0usize;
    while index < CASES.len() {
        let case = &CASES[index];
        kunit::start_case(total, "smoke.", case.name, checkpoint);
        match (case.run)() {
            SmokeResult::Passed => {
                kunit::drain_printk_diag();
                kunit::pass(total, "smoke.", case.name);
            }
            SmokeResult::Failed => {
                kunit::drain_printk_diag();
                kunit::fail(total, "smoke.", case.name, "smoke case failed");
                return CheckpointOutcome::FailAndShutdown;
            }
        }
        index += 1;
    }
    CheckpointOutcome::Continue
}
