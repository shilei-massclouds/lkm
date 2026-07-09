/*
 * RootfsPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in rootfs.md.
 */

predicate arceos_ex_must_rootfs_move_ext2_mount_and_chroot_dot_as_separate_actions() -> bool;
predicate arceos_ex_must_rootfs_classify_prepare_namespace_paths() -> bool;

type ArceosExRootfsCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_rootfs_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_rootfs_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_rootfs_run_after_initcall();

        /* KUnit runtime. */
        arceos_ex_must_rootfs_keep_kunit_trimmed_inside_rootfs_phase();

        /* Deferred initramfs and console details. */
        arceos_ex_must_rootfs_keep_initramfs_and_console_deferred();

        /* Required branch checkpoint. */
        arceos_ex_must_rootfs_require_prepare_namespace_branch();

        /* RootFS enable. */
        arceos_ex_must_rootfs_prepare_namespace_inputs_use_existing_devfs_and_block_registry();
        arceos_ex_must_rootfs_mount_ext2_at_linux_root_staging_point();

        /* prepare_namespace() path classification. */
        arceos_ex_must_rootfs_classify_prepare_namespace_paths();

        /* Root switch. */
        arceos_ex_must_rootfs_move_ext2_mount_and_chroot_dot_as_separate_actions();

        /* Integrity keys. */
        arceos_ex_must_rootfs_keep_integrity_keys_deferred_only();

        /* Boundary. */
        arceos_ex_must_rootfs_boundary_handoff_to_finalize();
    }
}
