use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::interrupt::process_prepare::is_ready() || !phases::interrupt::is_ready() {
        printk::write_str("process prepare phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.root_pid_namespace.state() != State::Ready
        || !ctx.root_pid_namespace.idr_ready()
        || !ctx.root_pid_namespace.compiletime_limit_checked()
        || ctx.root_pid_namespace.pid_max() < ctx.root_pid_namespace.pid_max_min()
    {
        printk::write_str("root pid namespace facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.credential_core.state() != State::Prepared
        || !ctx.credential_core.cred_cache_ready()
        || !ctx.credential_core.init_cred_static_root_not_created_here()
        || !ctx.credential_core.runtime_relations_deferred()
    {
        printk::write_str("credential facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.task_creation_core.state() != State::Ready
        || ctx.task_creation_core.vector_context().state() != State::Prepared
        || ctx.task_creation_core.uprobe_core().state() != State::Ready
        || !ctx.task_creation_core.thread_stack_cache_ready()
        || !ctx.task_creation_core.task_struct_cache_ready()
        || ctx.task_creation_core.max_threads() == 0
        || !ctx.task_creation_core.rest_init_inputs_ready()
        || !ctx.task_creation_core.entry_contract_ready()
        || ctx.task_creation_core.system_scheduling()
    {
        printk::write_str("task creation facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.signal_core.state() != State::Prepared
        || ctx.task_file_context.state() != State::Prepared
        || ctx.vma_core.state() != State::Prepared
        || !ctx.vma_core.vm_area_struct_cache_ready()
        || !ctx.vma_core.per_vma_lock_cache_ready()
        || !ctx.vma_core.vm_committed_as_counter_ready()
        || !ctx.vma_core.runtime_mapping_deferred()
    {
        printk::write_str("process memory/task context facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.ns_proxy.state() != State::Prepared
        || ctx.uts_namespace.state() != State::Prepared
        || ctx.keyring_core.state() != State::Ready
        || ctx.security_core.state() != State::Ready
        || ctx.keyring_core.builtin_key_type_count() == 0
        || ctx.security_core.ordered_lsm_count() == 0
        || ctx.security_core.capability_hook_count() == 0
    {
        printk::write_str("namespace/key/security facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "process_prepare pid_max={} max_threads={} key_types={} vector={}\n",
        ctx.root_pid_namespace.pid_max(),
        ctx.task_creation_core.max_threads(),
        ctx.keyring_core.builtin_key_type_count(),
        ctx.task_creation_core.vector_context().vector_supported()
    ));
    SmokeResult::Passed
}
