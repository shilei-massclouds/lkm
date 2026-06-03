use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::MemBlockOnline];
const MEMBLOCK_TEST_WORD: usize = 0x4d45_4d42;
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "memblock",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(checkpoint: Checkpoint, ctx: &mut Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    kunit::start_case(total, "", HANDLER.name, checkpoint);
    let page_size = ctx.config.page_size();

    let Some(first) = ctx.memblock.alloc_phys(page_size, page_size) else {
        kunit::fail(total, "", HANDLER.name, "first alloc failed");
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(second) = ctx.memblock.alloc_phys(page_size, page_size) else {
        kunit::fail(total, "", HANDLER.name, "second alloc failed");
        return CheckpointOutcome::FailAndShutdown;
    };

    if first.end() > second.start() || first.size() != page_size || second.size() != page_size {
        kunit::fail(total, "", HANDLER.name, "allocated ranges invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    let Some(first_va) = ctx.config.phys_to_linear(first.start()) else {
        kunit::fail(total, "", HANDLER.name, "linear address conversion failed");
        return CheckpointOutcome::FailAndShutdown;
    };

    let word = first_va as *mut usize;
    unsafe {
        core::ptr::write_volatile(word, MEMBLOCK_TEST_WORD);
        if core::ptr::read_volatile(word) != MEMBLOCK_TEST_WORD {
            kunit::fail(total, "", HANDLER.name, "readback failed");
            return CheckpointOutcome::FailAndShutdown;
        }
    }

    kunit::diag_hex_pair("allocated_pages_phys", first.start(), second.start());
    kunit::pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}
