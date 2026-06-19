use crate::{
    arch::riscv64::csr,
    context::Context,
    objects::{
        process_prepare::TaskCreationSetup,
        state::{failed_condition, EventError, EventErrorCode, EventResult, LifecycleEvent, State},
        vfs::{FileSystemKind, VfsInodeKind},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static PROCESS_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::ProcessPreparePhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex process prepare event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.root_pid_namespace.setup(
        &ctx.cpu_group,
        &ctx.slub_allocator,
        ctx.slub_allocator.kmalloc_caches(),
    )?;
    ctx.anon_vma_core
        .setup(&ctx.slub_allocator, ctx.slub_allocator.kmalloc_caches())?;
    checkpoint_x86_efi_runtime_switch_trimmed()?;
    ctx.task_creation_core.preset(
        &ctx.slub_allocator,
        ctx.slub_allocator.kmalloc_caches(),
        &ctx.per_cpu_storage,
    )?;
    ctx.credential_core
        .preset(&ctx.slub_allocator, ctx.slub_allocator.kmalloc_caches())?;
    ctx.task_creation_core.setup(TaskCreationSetup {
        root_pid_namespace: &ctx.root_pid_namespace,
        credential_core: &ctx.credential_core,
        cpu_group: &ctx.cpu_group,
        cpu_capabilities: &ctx.cpu_capabilities,
        slub_allocator: &ctx.slub_allocator,
        init_task: &ctx.init_task,
        exception_stream: &ctx.exception_stream,
    })?;
    checkpoint_shadow_call_stack_noop()?;
    checkpoint_lockdep_init_task_noop()?;
    ctx.signal_core
        .preset(&ctx.slub_allocator, ctx.slub_allocator.kmalloc_caches())?;
    ctx.task_file_context
        .preset(&ctx.slub_allocator, ctx.slub_allocator.kmalloc_caches())?;
    ctx.vma_core.preset(
        &ctx.mm_struct_cache,
        &ctx.anon_vma_core,
        &ctx.slub_allocator,
        &ctx.per_cpu_storage,
    )?;
    ctx.ns_proxy
        .preset(&ctx.slub_allocator, ctx.slub_allocator.kmalloc_caches())?;
    ctx.uts_namespace
        .preset(&ctx.ns_proxy, &ctx.slub_allocator)?;
    ctx.keyring_core
        .setup(&ctx.credential_core, &ctx.slub_allocator)?;
    ctx.security_core.setup(
        &ctx.credential_core,
        &ctx.keyring_core,
        &ctx.slub_allocator,
        &ctx.static_branch,
    )?;
    setup_vfs_rootfs(ctx)?;
    checkpoint_dbg_late_init_noop()?;
    checkpoint_net_namespace_deferred()?;
    checkpoint_page_cache_deferred()?;
    checkpoint_signal_core_setup_deferred()?;
    checkpoint_seq_file_core_deferred()?;
    checkpoint_procfs_deferred()?;
    checkpoint_nsfs_deferred()?;
    checkpoint_pidfs_deferred()?;
    checkpoint_cpuset_noop()?;
    checkpoint_cgroup_noop()?;
    checkpoint_taskstats_noop()?;
    checkpoint_delay_accounting_noop()?;
    checkpoint_acpi_subsystem_noop()?;
    checkpoint_arch_post_acpi_noop()?;
    checkpoint_kcsan_noop()
}

fn handoff() -> ! {
    crate::phases::interrupt::setup_after_children()
}

fn setup_vfs_rootfs(ctx: &mut Context) -> EventResult {
    ctx.vfs_core.setup()?;
    ctx.ramfs_type.setup()?;
    ctx.vfs_core
        .register_ramfs_type(&ctx.ramfs_type)
        .map_err(|_| vfs_setup_error(ctx))?;
    ctx.vfs_core
        .mount_initial_ramfs_root(&ctx.ramfs_type)
        .map_err(|_| vfs_setup_error(ctx))?;
    Ok(())
}

fn vfs_setup_error(ctx: &Context) -> EventError {
    EventError::failed(
        EventErrorCode::ConditionFailed,
        LifecycleEvent::Setup,
        ctx.vfs_core.state(),
        State::Ready,
        State::Ready,
    )
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !process_prepare_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &PROCESS_PREPARE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::ProcessPreparePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE) == State::Ready
}

fn process_prepare_phase_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::irq_open_prepare::is_ready()
        && ctx.interrupt_stream.state() == State::Online
        && ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && ctx.boot_cpu_local_interrupt.state() == State::Ready
        && ctx.boot_cpu_local_interrupt.enabled()
        && csr::supervisor_interrupts_enabled()
        && ctx.console.state() == State::Prepared
        && ctx.sched_clock.state() == State::Ready
        && ctx.delay_loop.state() == State::Ready
        && ctx.scheduler.state() == State::Online
        && ctx.scheduler.scheduler_running()
        && ctx.workqueue.state() == State::Prepared
        && !ctx.workqueue.workers_running()
        && ctx.softirq.state() == State::Ready
        && !ctx.softirq.execution_open()
        && ctx.rcu_core.state() == State::Ready
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.slub_allocator.state() == State::Ready
        && ctx.slub_allocator.kmalloc_caches().state() == State::Ready
        && ctx.mm_struct_cache.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && ctx.cpu_capabilities.state() == State::Ready
        && ctx.init_task.state() == State::Online
        && ctx.exception_stream.state() == State::Ready
        && ctx.root_pid_namespace.state() == State::Ready
        && ctx.root_pid_namespace.idr_ready()
        && ctx.root_pid_namespace.compiletime_limit_checked()
        && ctx.root_pid_namespace.pid_cache_level() == 0
        && ctx.anon_vma_core.state() == State::Ready
        && ctx.anon_vma_core.anon_vma_cache_ready()
        && ctx.anon_vma_core.anon_vma_chain_cache_ready()
        && ctx.anon_vma_core.runtime_graph_deferred()
        && ctx.credential_core.state() == State::Prepared
        && ctx.credential_core.cred_cache_ready()
        && ctx.task_creation_core.state() == State::Ready
        && ctx.task_creation_core.vector_context().state() == State::Prepared
        && (!ctx.task_creation_core.vector_context().vector_supported()
            || (ctx.task_creation_core.vector_context().user_cache_ready()
                && ctx.task_creation_core.vector_context().kernel_cache_ready()))
        && ctx.task_creation_core.uprobe_core().state() == State::Ready
        && ctx.task_creation_core.uprobe_core().hash_mutex_ready()
        && ctx
            .task_creation_core
            .uprobe_core()
            .die_notifier_registered()
        && ctx.task_creation_core.thread_stack_cache_ready()
        && ctx.task_creation_core.thread_stack_cache_bytes() != 0
        && ctx.task_creation_core.vmap_stack_selected()
        && ctx.task_creation_core.task_struct_cache_ready()
        && ctx.task_creation_core.task_struct_cache_bytes() != 0
        && ctx.task_creation_core.max_threads() != 0
        && ctx.task_creation_core.init_task_rlimits_ready()
        && ctx.task_creation_core.init_user_namespace_ucounts_ready()
        && ctx.task_creation_core.fork_vm_stack_cpuhp_registered()
        && ctx.task_creation_core.rest_init_inputs_ready()
        && !ctx.task_creation_core.kernel_init_created()
        && !ctx.task_creation_core.kthreadd_created()
        && !ctx.task_creation_core.system_scheduling()
        && ctx.signal_core.state() == State::Prepared
        && ctx.signal_core.sighand_cache_ready()
        && ctx.signal_core.signal_struct_cache_ready()
        && ctx.signal_core.sigqueue_cache_deferred()
        && ctx.task_file_context.state() == State::Prepared
        && ctx.task_file_context.files_struct_cache_ready()
        && ctx.task_file_context.fs_struct_cache_ready()
        && ctx.task_file_context.vfs_runtime_deferred()
        && ctx.vma_core.state() == State::Prepared
        && ctx.vma_core.vm_area_struct_cache_ready()
        && ctx.vma_core.per_vma_lock_cache_ready()
        && ctx.vma_core.vm_committed_as_counter_ready()
        && ctx.vma_core.runtime_mapping_deferred()
        && ctx.ns_proxy.state() == State::Prepared
        && ctx.ns_proxy.nsproxy_cache_ready()
        && ctx.ns_proxy.runtime_refs_deferred()
        && ctx.uts_namespace.state() == State::Prepared
        && ctx.uts_namespace.cache_ready()
        && ctx.uts_namespace.static_root_not_created_here()
        && ctx.uts_namespace.runtime_ops_deferred()
        && ctx.keyring_core.state() == State::Ready
        && ctx.keyring_core.key_cache_ready()
        && ctx.keyring_core.builtin_key_type_count() != 0
        && ctx.keyring_core.root_key_user_tracking_ready()
        && ctx.keyring_core.persistent_keyrings_trimmed()
        && ctx.security_core.state() == State::Ready
        && ctx.security_core.ordered_lsm_count() != 0
        && ctx.security_core.blob_layout_ready()
        && ctx.security_core.hook_dispatcher_ready()
        && ctx.security_core.capability_hook_count() != 0
        && ctx.security_core.optional_lsms_deferred()
        && ctx.vfs_core.state() == State::Ready
        && ctx.vfs_core.fs_type_registry_ready()
        && ctx.vfs_core.mount_table_ready()
        && ctx.vfs_core.dentry_cache_ready()
        && ctx.vfs_core.inode_table_ready()
        && ctx.vfs_core.file_table_ready()
        && ctx.vfs_core.page_cache_deferred()
        && ctx.vfs_core.permissions_deferred()
        && ctx.vfs_core.mount_namespace_deferred()
        && ctx.ramfs_type.state() == State::Ready
        && ctx.ramfs_type.memory_backed()
        && ctx.vfs_core.ramfs_registered()
        && ctx.vfs_core.rootfs_mount_created()
        && rootfs_mount_facts_ready(ctx)
}

fn rootfs_mount_facts_ready(ctx: &Context) -> bool {
    let Some(mount_ref) = ctx.vfs_core.current_root_mount() else {
        return false;
    };
    let Some(root_dentry_ref) = ctx.vfs_core.current_root_dentry() else {
        return false;
    };
    let Some(mount) = ctx.vfs_core.mount(mount_ref) else {
        return false;
    };
    if !mount.mounted()
        || mount.fs_kind() != FileSystemKind::RamFs
        || mount.root_dentry_ref() != root_dentry_ref
    {
        return false;
    }

    let Some(superblock) = ctx.vfs_core.superblock(mount.superblock_ref()) else {
        return false;
    };
    let Some(root_inode_ref) = superblock.root_inode_ref() else {
        return false;
    };
    if superblock.fs_kind() != FileSystemKind::RamFs
        || !superblock.ramfs_private_bound()
        || superblock.root_dentry_ref() != Some(root_dentry_ref)
    {
        return false;
    }

    let Some(root_dentry) = ctx.vfs_core.dentry(root_dentry_ref) else {
        return false;
    };
    let Some(root_inode) = ctx.vfs_core.inode(root_inode_ref) else {
        return false;
    };
    root_dentry.parent().is_none()
        && root_dentry.positive()
        && !root_dentry.removed()
        && root_dentry.name() == b"/"
        && root_dentry.inode_ref() == root_inode_ref
        && root_inode.superblock_ref() == mount.superblock_ref()
        && root_inode.kind() == VfsInodeKind::Directory
        && !root_inode.removed()
}

fn checkpoint_x86_efi_runtime_switch_trimmed() -> EventResult {
    crate::trace::checkpoint(Checkpoint::X86EfiRuntimeSwitchTrimmed);
    Ok(())
}

fn checkpoint_shadow_call_stack_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::ShadowCallStackInitNoop);
    Ok(())
}

fn checkpoint_lockdep_init_task_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::LockdepInitTaskNoop);
    Ok(())
}

fn checkpoint_dbg_late_init_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::DbgLateInitNoop);
    Ok(())
}

fn checkpoint_net_namespace_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::NetNamespaceDeferred);
    Ok(())
}

fn checkpoint_page_cache_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::PageCacheDeferred);
    Ok(())
}

fn checkpoint_signal_core_setup_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::SignalCoreSetupDeferred);
    Ok(())
}

fn checkpoint_seq_file_core_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::SeqFileCoreDeferred);
    Ok(())
}

fn checkpoint_procfs_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::ProcfsDeferred);
    Ok(())
}

fn checkpoint_nsfs_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::NsfsDeferred);
    Ok(())
}

fn checkpoint_pidfs_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::PidfsDeferred);
    Ok(())
}

fn checkpoint_cpuset_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::CpusetNoop);
    Ok(())
}

fn checkpoint_cgroup_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::CgroupNoop);
    Ok(())
}

fn checkpoint_taskstats_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::TaskstatsNoop);
    Ok(())
}

fn checkpoint_delay_accounting_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::DelayAccountingNoop);
    Ok(())
}

fn checkpoint_acpi_subsystem_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::AcpiSubsystemNoop);
    Ok(())
}

fn checkpoint_arch_post_acpi_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::ArchPostAcpiNoop);
    Ok(())
}

fn checkpoint_kcsan_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::KcsanNoop);
    Ok(())
}
