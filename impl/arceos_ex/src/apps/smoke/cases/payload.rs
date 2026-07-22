use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        binary_format_registry::BinaryFormatError,
        config::SelectedPayloadKind,
        elf_object::ElfObject,
        exec_transaction::{ExecArguments, ExecError},
        files::FILE_PATH_MAX,
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let boundaries = &ctx.exec_sync_boundaries;
    let registry = &ctx.binary_format_registry;
    let transaction = &ctx.exec_transaction;
    let mut rejected_elf = ElfObject::new();
    let four = [
        b"/a".as_slice(),
        b"b".as_slice(),
        b"c".as_slice(),
        b"d".as_slice(),
    ];
    let five = [
        b"/a".as_slice(),
        b"b".as_slice(),
        b"c".as_slice(),
        b"d".as_slice(),
        b"e".as_slice(),
    ];
    let six = [
        b"/a".as_slice(),
        b"b".as_slice(),
        b"c".as_slice(),
        b"d".as_slice(),
        b"e".as_slice(),
        b"f".as_slice(),
    ];
    let max_string = [b'x'; FILE_PATH_MAX];
    let mut absolute_max_string = max_string;
    absolute_max_string[0] = b'/';
    let mut too_long_string = [b'x'; FILE_PATH_MAX + 1];
    too_long_string[0] = b'/';
    let limits = ctx.config.exec_argument_limits();

    let payload_prepare_online = crate::phases::payload::prepare::is_online();
    let payload_handoff_prepare_online = crate::phases::payload::handoff_prepare::is_online();
    let kernel_init_flow_online = ctx.kernel_init_flow.state() == State::Online;
    let kernel_init_flow_active = ctx.kernel_init_flow.active();
    let payload_handoff_committed = ctx.kernel_init_flow.payload_handoff_committed();
    let task_flow_binding_active = ctx.kernel_init_task.kernel_init_flow_active();
    if !payload_prepare_online
        || !payload_handoff_prepare_online
        || !kernel_init_flow_online
        || !kernel_init_flow_active
        || !payload_handoff_committed
        || !task_flow_binding_active
    {
        printk::write_fmt(format_args!(
            "payload topology invalid prepare_online={} handoff_prepare_online={} flow_online={} flow_active={} handoff_committed={} task_flow_active={}\n",
            payload_prepare_online,
            payload_handoff_prepare_online,
            kernel_init_flow_online,
            kernel_init_flow_active,
            payload_handoff_committed,
            task_flow_binding_active,
        ));
        return SmokeResult::Failed;
    }

    if ctx.config.selected_payload_kind() != SelectedPayloadKind::Smoke
        || ctx.selected_payload_handoff.state() != State::Online
        || ctx.selected_payload_handoff.kind() != SelectedPayloadKind::Smoke
        || !ctx.selected_payload_handoff.kind_bound()
        || !ctx.selected_payload_handoff.variant_setup_ready()
        || !ctx.selected_payload_handoff.variant_prepare_ready()
        || !ctx.selected_payload_handoff.no_return_entry_bound()
        || ctx.user_clone_deferred_boundaries.state() != State::Ready
        || !ctx
            .user_clone_deferred_boundaries
            .plain_fork_first_slice_bound()
        || !ctx
            .user_clone_deferred_boundaries
            .vfork_vm_first_slice_bound()
        || !ctx
            .user_clone_deferred_boundaries
            .vfork_pidfd_first_slice_bound()
        || boundaries.state() != State::Ready
        || !boundaries.kernel_execve_linux_window_bound()
        || !boundaries.binfmt_lock_deferred()
        || !boundaries.cred_guard_mutex_deferred()
        || !boundaries.exec_update_lock_deferred()
        || !boundaries.exec_mmap_local_irq_deferred()
        || !boundaries.exec_task_siglock_deferred()
        || !boundaries.exec_tasklist_lock_deferred()
        || !boundaries.exec_fs_lock_rcu_deferred()
        || !boundaries.exec_mmap_lock_deferred()
        || !boundaries.exec_membarrier_deferred()
        || !boundaries.bprm_mm_init_task_lock_deferred()
        || !boundaries.exec_mmap_task_lock_deferred()
        || !boundaries.exec_sched_mm_cid_deferred()
        || !boundaries.exec_files_unshare_cloexec_deferred()
        || !boundaries.exec_io_uring_cancel_deferred()
        || !boundaries.exec_posix_timer_siglock_deferred()
        || !boundaries.exec_namespace_switch_deferred()
        || !boundaries.exec_success_accounting_hooks_deferred()
        || !boundaries.exec_full_binfmt_deferred()
        || !boundaries.binfmt_module_retry_trimmed_noop()
        || !boundaries.binfmt_module_retry_trimmed_because_modules_disabled()
        || !boundaries.ramdisk_init_branch_trimmed_noop()
        || !boundaries.ramdisk_init_trimmed_because_config_initrd_disabled()
        || !boundaries.default_init_branch_trimmed_noop()
        || !boundaries.default_init_trimmed_because_config_default_init_empty()
        || !boundaries.binfmt_script_deferred()
        || !boundaries.exec_panic_terminal_bound()
        || !boundaries.single_active_transaction()
        || !boundaries.point_of_no_return_bound()
        || !boundaries.cloexec_precheck_bound()
        || !boundaries.current_staging_mm_handoff_bound()
        || !boundaries.retired_mm_release_bound()
        || !boundaries.boot_runtime_owner_handoff_bound()
        || registry.state() != State::Ready
        || registry.handler_count() != 1
        || !registry.only_elf_handler()
        || !registry.script_deferred()
        || !registry.misc_deferred()
        || !registry.dynamic_registration_deferred()
        || !registry.module_retry_trimmed()
        || !matches!(
            registry.prepare_main(b"not-an-elf", &mut rejected_elf),
            Err(BinaryFormatError::NoExecutableFormat)
        )
        || rejected_elf.state() != State::Base
        || transaction.state() != State::Ready
        || transaction.active()
        || transaction.point_of_no_return()
        || limits.max_arg_strings() != 0x7fff_ffff
        || limits.max_arg_strlen() != 32 * 4096
        || limits.arg_max_floor() != 128 * 1024
        || limits.stack_rlimit() != 8 * 1024 * 1024
        || limits.stk_lim() != 8 * 1024 * 1024
        || limits.argument_bytes() != 2 * 1024 * 1024
        || !limits.accepts(
            1,
            0,
            limits.argument_bytes() - core::mem::size_of::<usize>(),
        )
        || limits.accepts(
            1,
            0,
            limits.argument_bytes() - core::mem::size_of::<usize>() + 1,
        )
        || ExecArguments::for_boot(b"/bin/sh", limits).is_err()
        || ExecArguments::from_slices(&absolute_max_string, &four, &four, limits).is_err()
        || !matches!(
            ExecArguments::from_slices(b"relative", &four, &four, limits),
            Err(ExecError::InvalidPath)
        )
        || ExecArguments::from_slices(b"/bin/sh", &five, &six, limits).is_err()
        || !matches!(
            ExecArguments::from_slices(&too_long_string, &four, &four, limits),
            Err(ExecError::ArgumentsTooBig)
        )
    {
        printk::write_str("exec object lifecycle, registry, or argument bounds invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_str("exec object lifecycle and argument bounds valid\n");
    SmokeResult::Passed
}
