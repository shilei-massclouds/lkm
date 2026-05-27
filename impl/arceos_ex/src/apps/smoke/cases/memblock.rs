use crate::{apps::smoke::SmokeResult, context::context, objects::printk};

pub fn run() -> SmokeResult {
    let ctx = context();
    let page_size = ctx.config.page_size();

    let Some(first) = ctx.memblock.alloc_phys(page_size, page_size) else {
        printk::write_str("memblock alloc first page failed\n");
        return SmokeResult::Failed;
    };
    let Some(second) = ctx.memblock.alloc_phys(page_size, page_size) else {
        printk::write_str("memblock alloc second page failed\n");
        return SmokeResult::Failed;
    };

    if first.end() > second.start() || first.size() != page_size || second.size() != page_size {
        printk::write_str("memblock allocated ranges overlap or have bad size\n");
        return SmokeResult::Failed;
    }

    let Some(first_va) = ctx.config.phys_to_linear(first.start()) else {
        printk::write_str("memblock linear address conversion failed\n");
        return SmokeResult::Failed;
    };

    let word = first_va as *mut usize;
    unsafe {
        core::ptr::write_volatile(word, 0x4d45_4d42usize);
        if core::ptr::read_volatile(word) != 0x4d45_4d42usize {
            printk::write_str("memblock allocated page readback failed\n");
            return SmokeResult::Failed;
        }
    }

    printk::write_fmt(format_args!(
        "allocated pages phys={:#x},{:#x}\n",
        first.start(),
        second.start()
    ));
    SmokeResult::Passed
}
