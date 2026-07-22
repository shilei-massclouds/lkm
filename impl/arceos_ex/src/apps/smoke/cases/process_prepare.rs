use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        state::State,
        vfs::{FileSystemKind, VfsInodeKind},
    },
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::interrupt::process_prepare::is_online() || !phases::boot_init::is_online() {
        printk::write_str("process prepare phase is not online\n");
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

    if ctx.process_prepare_trimmed_paths.state() != State::Prepared
        || !ctx
            .process_prepare_trimmed_paths
            .x86_efi_runtime_switch_trimmed_noop()
        || !ctx
            .process_prepare_trimmed_paths
            .shadow_call_stack_init_trimmed_noop()
        || !ctx
            .process_prepare_trimmed_paths
            .lockdep_init_task_trimmed_noop()
        || !ctx.process_prepare_trimmed_paths.net_namespace_deferred()
        || !ctx.process_prepare_trimmed_paths.pagecache_deferred()
        || !ctx
            .process_prepare_trimmed_paths
            .pagecache_waitqueue_table_deferred()
        || !ctx
            .process_prepare_trimmed_paths
            .signal_core_setup_deferred()
        || !ctx
            .process_prepare_trimmed_paths
            .vfs_pseudo_filesystems_deferred()
        || !ctx
            .process_prepare_trimmed_paths
            .bdev_chrdev_init_deferred()
        || !ctx.process_prepare_trimmed_paths.cpuset_init_trimmed_noop()
        || !ctx.process_prepare_trimmed_paths.cgroup_init_trimmed_noop()
        || !ctx
            .process_prepare_trimmed_paths
            .taskstats_init_trimmed_noop()
        || !ctx
            .process_prepare_trimmed_paths
            .delayacct_init_trimmed_noop()
        || !ctx
            .process_prepare_trimmed_paths
            .acpi_subsystem_init_trimmed_noop()
        || !ctx.process_prepare_trimmed_paths.kcsan_init_trimmed_noop()
        || !ctx
            .process_prepare_trimmed_paths
            .rcu_tasks_generic_out_of_scope()
        || !ctx.process_prepare_trimmed_paths.position_preserved()
    {
        printk::write_str("process prepare trimmed/deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    let Some(root_mount_ref) = ctx.vfs_core.initial_root_mount() else {
        printk::write_str("vfs root mount missing\n");
        return SmokeResult::Failed;
    };
    let Some(root_dentry_ref) = ctx.vfs_core.initial_root_dentry() else {
        printk::write_str("vfs root dentry missing\n");
        return SmokeResult::Failed;
    };
    let Some(root_mount) = ctx.vfs_core.mount(root_mount_ref) else {
        printk::write_str("vfs root mount invalid\n");
        return SmokeResult::Failed;
    };
    let Some(root_superblock) = ctx.vfs_core.superblock(root_mount.superblock_ref()) else {
        printk::write_str("vfs root superblock invalid\n");
        return SmokeResult::Failed;
    };
    let Some(root_inode_ref) = root_superblock.root_inode_ref() else {
        printk::write_str("vfs root inode missing\n");
        return SmokeResult::Failed;
    };
    let Some(root_dentry) = ctx.vfs_core.dentry(root_dentry_ref) else {
        printk::write_str("vfs root dentry invalid\n");
        return SmokeResult::Failed;
    };
    let Some(root_inode) = ctx.vfs_core.inode(root_inode_ref) else {
        printk::write_str("vfs root inode invalid\n");
        return SmokeResult::Failed;
    };
    if ctx.vfs_core.state() != State::Ready
        || ctx.ramfs_type.state() != State::Ready
        || !ctx.vfs_core.ramfs_registered()
        || !ctx.vfs_core.rootfs_mount_created()
        || root_mount.fs_kind() != FileSystemKind::RamFs
        || root_mount.root_dentry_ref() != root_dentry_ref
        || root_superblock.root_dentry_ref() != Some(root_dentry_ref)
        || root_dentry.name() != b"/"
        || root_dentry.inode_ref() != root_inode_ref
        || root_inode.kind() != VfsInodeKind::Directory
    {
        printk::write_str("vfs rootfs facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "process_prepare pid_max={} max_threads={} key_types={} vector={} rootfs=ramfs\n",
        ctx.root_pid_namespace.pid_max(),
        ctx.task_creation_core.max_threads(),
        ctx.keyring_core.builtin_key_type_count(),
        ctx.task_creation_core.vector_context().vector_supported()
    ));
    SmokeResult::Passed
}
