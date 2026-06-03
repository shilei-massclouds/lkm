use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    context::Context,
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::MemBlockOnline];
const MEMBLOCK_TEST_WORD: usize = 0x4d45_4d42;

pub const HANDLER: Handler = Handler {
    name: "memblock",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(_checkpoint: Checkpoint, ctx: &mut Context) -> CheckpointOutcome {
    let page_size = ctx.config.page_size();

    let Some(first) = ctx.memblock.alloc_phys(page_size, page_size) else {
        putstr("kunit memblock: first alloc failed\n");
        return CheckpointOutcome::FailAndShutdown;
    };
    let Some(second) = ctx.memblock.alloc_phys(page_size, page_size) else {
        putstr("kunit memblock: second alloc failed\n");
        return CheckpointOutcome::FailAndShutdown;
    };

    if first.end() > second.start() || first.size() != page_size || second.size() != page_size {
        putstr("kunit memblock: allocated ranges invalid\n");
        return CheckpointOutcome::FailAndShutdown;
    }

    let Some(first_va) = ctx.config.phys_to_linear(first.start()) else {
        putstr("kunit memblock: linear address conversion failed\n");
        return CheckpointOutcome::FailAndShutdown;
    };

    let word = first_va as *mut usize;
    unsafe {
        core::ptr::write_volatile(word, MEMBLOCK_TEST_WORD);
        if core::ptr::read_volatile(word) != MEMBLOCK_TEST_WORD {
            putstr("kunit memblock: readback failed\n");
            return CheckpointOutcome::FailAndShutdown;
        }
    }

    putstr("kunit memblock: allocated pages phys=0x");
    put_hex(first.start());
    putstr(",0x");
    put_hex(second.start());
    putstr("\n");
    putstr("kunit memblock: passed\n");
    CheckpointOutcome::Passed
}

fn putstr(message: &str) {
    crate::arch::riscv64::sbi::putstr(message);
}

fn put_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        crate::arch::riscv64::sbi::putchar(HEX[(value >> shift) & 0xf]);
    }
}
