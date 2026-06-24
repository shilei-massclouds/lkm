use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let trimmed = &ctx.mm_core_trimmed_paths;

    if trimmed.state() != State::Ready {
        printk::write_str("mm core trimmed paths are not ready\n");
        return SmokeResult::Failed;
    }

    if !trimmed.page_ext_flatmem_trimmed()
        || !trimmed.kfence_pool_trimmed()
        || !trimmed.kmsan_shadow_trimmed()
        || !trimmed.page_ext_flatmem_late_trimmed()
        || !trimmed.kmemleak_init_trimmed()
        || !trimmed.debug_objects_mem_trimmed()
        || !trimmed.page_ext_final_trimmed()
        || !trimmed.x86_espfix_not_applicable()
        || !trimmed.x86_pti_not_applicable()
        || !trimmed.kmsan_runtime_trimmed()
        || !trimmed.execmem_init_trimmed_noop()
        || !trimmed.execmem_trimmed_because_config_execmem_disabled()
        || !trimmed.position_preserved()
    {
        printk::write_str("mm core trimmed path facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_str(
        "mm_core_trimmed_paths page_ext/kfence/kmsan/kmemleak/debug_objects/execmem recorded\n",
    );
    SmokeResult::Passed
}
