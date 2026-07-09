/*
 * CorePreparePhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in core-prepare.md.
 */

predicate arceos_ex_must_core_prepare_preserve_early_irq_and_smp_closed_facts() -> bool;
predicate arceos_ex_must_static_branch_setup_record_jump_label_guards() -> bool;
predicate arceos_ex_must_jump_label_mutex_be_independent_context_object() -> bool;
predicate arceos_ex_must_static_branch_setup_drive_jump_label_mutex_guard() -> bool;
predicate arceos_ex_must_static_branch_text_patch_sync_remain_deferred() -> bool;
predicate arceos_ex_must_resource_tree_setup_record_resource_lock_write_guard() -> bool;
predicate arceos_ex_must_resource_tree_setup_drive_resource_lock_write_guard() -> bool;
predicate arceos_ex_must_rwlock_expose_reader_writer_protocol() -> bool;
predicate arceos_ex_must_printk_buffer_setup_record_local_irq_save_restore() -> bool;
predicate arceos_ex_must_printk_buffer_setup_bind_local_irq_guard_to_boot_cpu_control() -> bool;
predicate arceos_ex_must_randomness_preset_not_unconditionally_lock_input_pool() -> bool;
predicate arceos_ex_must_randomness_conditional_reseed_use_base_crng_irqsave_lock() -> bool;

type ArceosExCorePrepareCodingMust {
    invariant {
        /* CorePrepare concurrency boundary. */
        arceos_ex_must_core_prepare_preserve_early_irq_and_smp_closed_facts();

        /* StaticBranch.setup guard facts. */
        arceos_ex_must_static_branch_setup_record_jump_label_guards();

        /* JumpLabelMutex object mapping. */
        arceos_ex_must_jump_label_mutex_be_independent_context_object();

        /* CpuHotplugLock object mapping. */
        arceos_ex_must_cpu_hotplug_lock_be_independent_percpu_rwsem_object();

        /* StaticBranch jump-label mutex guard lowering. */
        arceos_ex_must_static_branch_setup_drive_jump_label_mutex_guard();

        /* StaticBranch CPU hotplug read guard lowering. */
        arceos_ex_must_static_branch_setup_drive_cpu_hotplug_read_guard();

        /* PerCpuRwSemaphore observable behavior. */
        arceos_ex_must_percpu_rwsem_expose_reader_writer_protocol();

        /* Text patch synchronization boundary. */
        arceos_ex_must_static_branch_text_patch_sync_remain_deferred();

        /* ResourceTree setup guard. */
        arceos_ex_must_resource_tree_setup_record_resource_lock_write_guard();

        /* ResourceTree resource_lock guard lowering. */
        arceos_ex_must_resource_tree_setup_drive_resource_lock_write_guard();

        /* RwLock observable behavior. */
        arceos_ex_must_rwlock_expose_reader_writer_protocol();

        /* PrintkBuffer setup IRQ guard. */
        arceos_ex_must_printk_buffer_setup_record_local_irq_save_restore();
        arceos_ex_must_printk_buffer_setup_bind_local_irq_guard_to_boot_cpu_control();

        /* Randomness preset conditional lock boundary. */
        arceos_ex_must_randomness_preset_not_unconditionally_lock_input_pool();

        /* Randomness conditional reseed/credit lock boundary. */
        arceos_ex_must_randomness_conditional_reseed_use_base_crng_irqsave_lock();
    }
}
