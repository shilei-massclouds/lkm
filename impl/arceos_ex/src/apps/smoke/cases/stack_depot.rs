use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let stack_depot = &ctx.stack_depot;

    if stack_depot.state() != State::Ready {
        printk::write_str("stack depot is not ready\n");
        return SmokeResult::Failed;
    }
    if !ctx.config.stack_depot_enabled() || ctx.config.stack_depot_always_init() {
        printk::write_str("stack depot config facts invalid\n");
        return SmokeResult::Failed;
    }
    if !stack_depot.config_enabled()
        || !stack_depot.early_init_passed()
        || stack_depot.early_init_requested()
        || !stack_depot.early_table_allocation_not_required()
        || stack_depot.early_table_allocated()
        || !stack_depot.late_init_deferred()
    {
        printk::write_str("stack depot early init facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "stack_depot enabled={} always_init={} requested={} table_allocated={} late_deferred={}\n",
        stack_depot.config_enabled(),
        ctx.config.stack_depot_always_init(),
        stack_depot.early_init_requested(),
        stack_depot.early_table_allocated(),
        stack_depot.late_init_deferred()
    ));
    SmokeResult::Passed
}
