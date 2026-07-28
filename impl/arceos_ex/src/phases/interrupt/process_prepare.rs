use crate::{
    arch::riscv64::csr,
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        process_prepare::TaskCreationSetup,
        state::{EventError, EventErrorCode, EventResult, LifecycleEvent, State, failed_condition},
        vfs::{FileSystemKind, VfsInodeKind},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static PROCESS_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(ctx),
        "arceos_ex process prepare preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::ProcessPreparePhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared_with_check(ctx)),
        "arceos_ex process prepare preset failed\n",
    );
    setup(ctx)
}

fn preset_start(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE);
    if state != State::Base || !preset_dependencies_ready(ctx) {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_dependencies_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::irq_open_prepare::is_online()
        && ctx.boot_cpu_interrupt().state() == State::Online
        && ctx.boot_cpu_interrupt().boot_cpu_local_interrupts_enabled()
        && !ctx.boot_cpu_interrupt().early_boot_irqs_disabled()
        && ctx.boot_cpu_local_interrupt().local_state() == State::Ready
        && ctx.boot_cpu_local_interrupt().enabled()
        && csr::supervisor_interrupts_enabled()
        && !ctx.cpu_group.smp_concurrency_open()
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
        && ctx.slub_subsystem.state() == State::Ready
        && ctx.slub_subsystem.kmalloc_caches().state() == State::Ready
        && ctx.mm_struct_cache.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && ctx.cpu_capabilities.state() == State::Ready
        && ctx.boot_task.state() == State::OnCpu
        && ctx.boot_cpu_exception().state() == State::Ready
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.root_pid_namespace.setup(
        &ctx.cpu_group,
        &ctx.slub_subsystem,
        ctx.slub_subsystem.kmalloc_caches(),
    )?;
    ctx.anon_vma_core
        .setup(&ctx.slub_subsystem, ctx.slub_subsystem.kmalloc_caches())?;
    ctx.task_creation_core.preset(
        &ctx.slub_subsystem,
        ctx.slub_subsystem.kmalloc_caches(),
        &ctx.per_cpu_storage,
    )?;
    ctx.credential_core
        .preset(&ctx.slub_subsystem, ctx.slub_subsystem.kmalloc_caches())?;
    let exception_type = ctx
        .cpu_group
        .boot_cpu_exception()
        .expect("boot CPU exception resource must exist after CPU discovery");
    ctx.task_creation_core.setup(TaskCreationSetup {
        root_pid_namespace: &ctx.root_pid_namespace,
        credential_core: &ctx.credential_core,
        cpu_group: &ctx.cpu_group,
        cpu_capabilities: &ctx.cpu_capabilities,
        slub_subsystem: &ctx.slub_subsystem,
        boot_task: &ctx.boot_task,
        exception_type,
    })?;
    ctx.signal_core
        .preset(&ctx.slub_subsystem, ctx.slub_subsystem.kmalloc_caches())?;
    ctx.task_file_context
        .preset(&ctx.slub_subsystem, ctx.slub_subsystem.kmalloc_caches())?;
    ctx.vma_core.preset(
        &ctx.mm_struct_cache,
        &ctx.anon_vma_core,
        &ctx.slub_subsystem,
        &ctx.per_cpu_storage,
    )?;
    ctx.ns_proxy
        .preset(&ctx.slub_subsystem, ctx.slub_subsystem.kmalloc_caches())?;
    ctx.uts_namespace
        .preset(&ctx.ns_proxy, &ctx.slub_subsystem)?;
    ctx.keyring_core
        .setup(&ctx.credential_core, &ctx.slub_subsystem)?;
    ctx.security_core.setup(
        &ctx.credential_core,
        &ctx.keyring_core,
        &ctx.slub_subsystem,
        &ctx.static_branch,
    )?;
    setup_vfs_rootfs(ctx)?;
    ctx.process_prepare_trimmed_paths.preset(
        &ctx.config,
        &ctx.task_creation_core,
        &ctx.signal_core,
        &ctx.vfs_core,
    )
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
    ctx.fs_struct.setup(&ctx.vfs_core)?;
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

fn adopt_prepared_with_check(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE);
    if state != State::Base || !process_prepare_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    crate::phases::state::mark_checked(
        &PROCESS_PREPARE_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::ProcessPreparePhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(adopt_ready(ctx), "arceos_ex process prepare setup failed\n");
    enable(ctx)
}

fn adopt_ready(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE);
    if state != State::Prepared || !process_prepare_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Setup, state, State::Prepared, State::Ready);
    }

    crate::phases::state::mark_checked(
        &PROCESS_PREPARE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::ProcessPreparePhaseReady,
    )
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        enable_event(ctx),
        "arceos_ex process prepare enable failed\n",
    );
    crate::flows::boot_init_flow::setup_after_process_prepare()
}

fn enable_event(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE);
    if state != State::Ready || !process_prepare_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }

    crate::phases::state::mark_checked(
        &PROCESS_PREPARE_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::ProcessPreparePhaseOnline,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&PROCESS_PREPARE_PHASE_STATE) == State::Online
}

fn process_prepare_phase_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::irq_open_prepare::is_online()
        && ctx.boot_cpu_interrupt().state() == State::Online
        && ctx.boot_cpu_interrupt().boot_cpu_local_interrupts_enabled()
        && !ctx.boot_cpu_interrupt().early_boot_irqs_disabled()
        && ctx.boot_cpu_local_interrupt().local_state() == State::Ready
        && ctx.boot_cpu_local_interrupt().enabled()
        && csr::supervisor_interrupts_enabled()
        && !ctx.cpu_group.smp_concurrency_open()
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
        && ctx.slub_subsystem.state() == State::Ready
        && ctx.slub_subsystem.kmalloc_caches().state() == State::Ready
        && ctx.mm_struct_cache.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && ctx.cpu_capabilities.state() == State::Ready
        && ctx.boot_task.state() == State::OnCpu
        && ctx.boot_cpu_exception().state() == State::Ready
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
        && process_prepare_trimmed_paths_ready(ctx)
        && ctx.vfs_core.state() == State::Ready
        && ctx.vfs_core.fs_type_registry_ready()
        && ctx.vfs_core.mount_table_ready()
        && ctx.vfs_core.dentry_cache_ready()
        && ctx.vfs_core.inode_table_ready()
        && ctx.vfs_core.file_table_ready()
        && ctx.vfs_core.page_cache_deferred()
        && ctx.vfs_core.permissions_deferred()
        && ctx.vfs_core.mount_namespace_deferred()
        && ctx.fs_struct.state() == State::Ready
        && ctx.fs_struct.initial_root_bound()
        && ctx.fs_struct.root_pwd_same()
        && ctx.ramfs_type.state() == State::Ready
        && ctx.ramfs_type.memory_backed()
        && ctx.vfs_core.ramfs_registered()
        && ctx.vfs_core.rootfs_mount_created()
        && rootfs_mount_facts_ready(ctx)
}

fn process_prepare_trimmed_paths_ready(ctx: &Context) -> bool {
    ctx.process_prepare_trimmed_paths.state() == State::Prepared
        && ctx
            .process_prepare_trimmed_paths
            .x86_efi_runtime_switch_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .x86_efi_runtime_switch_trimmed_because_arch_riscv()
        && ctx
            .process_prepare_trimmed_paths
            .shadow_call_stack_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .lockdep_init_task_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .lockdep_init_task_trimmed_because_config_lockdep_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .dbg_late_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .dbg_late_init_trimmed_because_config_kgdb_disabled()
        && ctx.process_prepare_trimmed_paths.net_namespace_deferred()
        && ctx
            .process_prepare_trimmed_paths
            .net_namespace_deferred_even_if_config_net_ns_enabled()
        && ctx.process_prepare_trimmed_paths.pagecache_deferred()
        && ctx
            .process_prepare_trimmed_paths
            .pagecache_waitqueue_table_deferred()
        && ctx
            .process_prepare_trimmed_paths
            .signal_core_setup_deferred()
        && ctx.process_prepare_trimmed_paths.seq_file_core_deferred()
        && ctx.process_prepare_trimmed_paths.procfs_deferred()
        && ctx.process_prepare_trimmed_paths.nsfs_deferred()
        && ctx.process_prepare_trimmed_paths.pidfs_deferred()
        && ctx
            .process_prepare_trimmed_paths
            .vfs_pseudo_filesystems_deferred()
        && ctx
            .process_prepare_trimmed_paths
            .bdev_chrdev_init_deferred()
        && ctx.process_prepare_trimmed_paths.cpuset_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .cpuset_trimmed_because_config_cpusets_disabled()
        && ctx.process_prepare_trimmed_paths.cgroup_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .cgroup_trimmed_because_config_cgroups_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .taskstats_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .taskstats_trimmed_because_config_taskstats_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .delayacct_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .delayacct_trimmed_because_config_task_delay_acct_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .acpi_subsystem_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .acpi_trimmed_because_config_acpi_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .arch_post_acpi_subsys_init_trimmed_noop()
        && ctx.process_prepare_trimmed_paths.kcsan_init_trimmed_noop()
        && ctx
            .process_prepare_trimmed_paths
            .kcsan_trimmed_because_config_kcsan_disabled()
        && ctx
            .process_prepare_trimmed_paths
            .rcu_tasks_generic_out_of_scope()
        && ctx
            .process_prepare_trimmed_paths
            .rcu_tasks_generic_belongs_to_kernel_init_freeable()
        && ctx.process_prepare_trimmed_paths.position_preserved()
}

fn rootfs_mount_facts_ready(ctx: &Context) -> bool {
    let Some(mount_ref) = ctx.vfs_core.initial_root_mount() else {
        return false;
    };
    let Some(root_dentry_ref) = ctx.fs_struct.root_dentry() else {
        return false;
    };
    if ctx.fs_struct.pwd_dentry() != Some(root_dentry_ref) {
        return false;
    }
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
