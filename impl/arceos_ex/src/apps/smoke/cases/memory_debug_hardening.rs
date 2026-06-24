use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State, static_branch::StaticKey},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let hardening = &ctx.memory_debug_hardening;

    if hardening.state() != State::Ready {
        printk::write_str("memory debug hardening is not ready\n");
        return SmokeResult::Failed;
    }
    if ctx.early_param.state() != State::Ready
        || !hardening.early_params_scanned()
        || !hardening.early_param_policy_trimmed()
        || !hardening.default_policy_selected()
        || !hardening.static_keys_resolved_from_default_policy()
    {
        printk::write_str("memory debug hardening policy facts missing\n");
        return SmokeResult::Failed;
    }
    if hardening.init_on_alloc()
        || hardening.init_on_free()
        || hardening.debug_pagealloc()
        || hardening.debug_guardpage()
        || !hardening.check_pages()
    {
        printk::write_str("memory debug hardening default policy invalid\n");
        return SmokeResult::Failed;
    }
    if ctx.static_branch.enabled(StaticKey::InitOnAlloc) != Some(false)
        || ctx.static_branch.enabled(StaticKey::InitOnFree) != Some(false)
        || ctx.static_branch.enabled(StaticKey::DebugPageAlloc) != Some(false)
        || ctx.static_branch.enabled(StaticKey::DebugGuardPage) != Some(false)
        || ctx.static_branch.enabled(StaticKey::CheckPages) != Some(true)
    {
        printk::write_str("memory debug hardening static keys invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "memory_debug_hardening default init_on_alloc={} init_on_free={} debug_pagealloc={} guardpage={} check_pages={} trimmed={}\n",
        hardening.init_on_alloc(),
        hardening.init_on_free(),
        hardening.debug_pagealloc(),
        hardening.debug_guardpage(),
        hardening.check_pages(),
        hardening.early_param_policy_trimmed()
    ));
    SmokeResult::Passed
}
