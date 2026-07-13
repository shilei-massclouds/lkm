/*
 * BootInitRestInitPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in rest-init.md.
 */

predicate arceos_ex_must_action_lowering_use_context_ref_and_typed_packet() -> bool;
predicate arceos_ex_must_bind_boot_scheduler_locks_to_owned_objects() -> bool;
predicate arceos_ex_must_sched_init_bit_wait_table_expose_bucket_waitqueue_heads() -> bool;
predicate arceos_ex_must_sched_init_bit_wait_table_use_within_context() -> bool;
predicate arceos_ex_must_sched_init_radix_maple_rcu_free_callbacks_deferred() -> bool;
predicate arceos_ex_must_sched_init_workqueue_register_pool_workqueue_cache() -> bool;
predicate arceos_ex_must_sched_init_workqueue_use_within_mutex_contexts() -> bool;
predicate arceos_ex_must_sched_init_workqueue_keep_worker_runtime_deferred() -> bool;
predicate arceos_ex_must_sched_init_softirq_prepare_shell_only() -> bool;
predicate arceos_ex_must_sched_init_rcu_register_rcu_softirq_explicitly() -> bool;
predicate arceos_ex_must_sched_init_rcu_expose_tree_and_tasks_init_facts() -> bool;
predicate arceos_ex_must_sched_init_trace_housekeeping_context_tracking_classify_by_config() -> bool;
predicate arceos_ex_must_scheduler_action_checkpoints_cover_pick_switch_and_schedule_exit() -> bool;
predicate arceos_ex_must_rest_init_model_path_under_up_multitask_phase() -> bool;
predicate arceos_ex_must_not_model_rest_init_phase_wrapper() -> bool;
predicate arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree() -> bool;
predicate arceos_ex_must_rest_init_run_after_process_prepare() -> bool;
predicate arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts() -> bool;
predicate arceos_ex_must_rest_init_create_tasks_with_explicit_entries() -> bool;
predicate arceos_ex_must_kthreadd_entry_model_minimal_schedule_loop() -> bool;
predicate arceos_ex_must_rest_init_publish_system_scheduling() -> bool;
predicate arceos_ex_must_rest_init_complete_kthreadd_ready_gate() -> bool;
predicate arceos_ex_must_rest_init_publish_scheduler_dispatch_facts() -> bool;
predicate arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions() -> bool;
predicate arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle() -> bool;
predicate arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle() -> bool;
predicate arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain() -> bool;
predicate arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations() -> bool;
predicate arceos_ex_must_model_cpu_instances_with_unified_cpu_type() -> bool;
predicate arceos_ex_must_cpu_group_index_cpu_refs_by_logical_id() -> bool;
predicate arceos_ex_must_cpu_group_not_own_cpu_bodies() -> bool;
predicate arceos_ex_must_cpu_state_be_instance_facts_and_group_set_views() -> bool;
predicate arceos_ex_must_not_generate_live_ap_current_cpu_before_entry() -> bool;
predicate arceos_ex_must_cpu_group_lowering_mark_boot_secondary_split_transitional() -> bool;
predicate arceos_ex_must_cpu_group_smoke_cover_index_and_sets() -> bool;
predicate arceos_ex_must_current_task_ref_be_cpu_view_private() -> bool;
predicate arceos_ex_must_current_runqueue_ref_be_cpu_view_private() -> bool;
predicate arceos_ex_must_current_runqueue_ref_api_smoke_use_formal_runqueue_actions() -> bool;
predicate arceos_ex_must_scheduler_schedule_smoke_use_payload_cooperative_switch() -> bool;
predicate arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue() -> bool;
predicate arceos_ex_must_rest_init_pin_kernel_init_as_task_action() -> bool;
predicate arceos_ex_must_rest_init_not_make_pre_smp_depend_on_up_multitask_wrapper() -> bool;
predicate arceos_ex_must_rest_init_handoff_real_boot_idle_to_kernel_init_stack() -> bool;
predicate arceos_ex_must_boot_stack_use_16k_after_kernel_init_handoff() -> bool;
predicate arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred() -> bool;

type ArceosExRestInitCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_rest_init_model_path_under_up_multitask_phase();
        arceos_ex_must_not_model_rest_init_phase_wrapper();

        /* Code path. */
        arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree();

        /* Ordering. */
        arceos_ex_must_rest_init_run_after_process_prepare();

        /* Task creation facts. */
        arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts();

        /* Explicit task entries. */
        arceos_ex_must_rest_init_create_tasks_with_explicit_entries();

        /* Kthreadd entry loop. */
        arceos_ex_must_kthreadd_entry_model_minimal_schedule_loop();

        /* System state. */
        arceos_ex_must_rest_init_publish_system_scheduling();

        /* Completion. */
        arceos_ex_must_rest_init_complete_kthreadd_ready_gate();

        /* Scheduler dispatch facts. */
        arceos_ex_must_rest_init_publish_scheduler_dispatch_facts();

        /* Boot idle runtime actions. */
        arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions();

        /* Representative need_resched idle cycle. */
        arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle();

        /* schedule_idle wrapper. */
        arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle();

        /* Action lowering ABI. */
        arceos_ex_must_action_lowering_use_context_ref_and_typed_packet();

        /* Scheduler action checkpoints. */
        arceos_ex_must_scheduler_action_checkpoints_cover_pick_switch_and_schedule_exit();

        /* BootIdleEntryPhase boot-idle chain. */
        arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain();

        /* Smoke/KUnit coverage. */
        arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations();

        /* CPU instance model. */
        arceos_ex_must_model_cpu_instances_with_unified_cpu_type();

        /* CpuGroup indexing. */
        arceos_ex_must_cpu_group_index_cpu_refs_by_logical_id();

        /* CpuGroup ownership. */
        arceos_ex_must_cpu_group_not_own_cpu_bodies();

        /* CPU state facts. */
        arceos_ex_must_cpu_state_be_instance_facts_and_group_set_views();

        /* AP CurrentCPU boundary. */
        arceos_ex_must_not_generate_live_ap_current_cpu_before_entry();

        /* DefaultSchedRootDomain coverage. */
        arceos_ex_must_default_root_domain_cover_cpu_group_possible_refs();

        /* RunQueue root-domain attach. */
        arceos_ex_must_attach_possible_cpu_runqueues_to_default_root_domain();

        /* sched_init wait-bit/radix/maple synchronization facts. */
        arceos_ex_must_sched_init_bit_wait_table_expose_bucket_waitqueue_heads();
        arceos_ex_must_sched_init_bit_wait_table_use_within_context();
        arceos_ex_must_sched_init_radix_maple_rcu_free_callbacks_deferred();

        /* sched_init workqueue early synchronization facts. */
        arceos_ex_must_sched_init_workqueue_register_pool_workqueue_cache();
        arceos_ex_must_sched_init_workqueue_use_within_mutex_contexts();
        arceos_ex_must_sched_init_workqueue_keep_worker_runtime_deferred();

        /* sched_init softirq/RCU/tracing boundaries. */
        arceos_ex_must_sched_init_softirq_prepare_shell_only();
        arceos_ex_must_sched_init_rcu_register_rcu_softirq_explicitly();
        arceos_ex_must_sched_init_rcu_expose_tree_and_tasks_init_facts();
        arceos_ex_must_sched_init_trace_housekeeping_context_tracking_classify_by_config();

        /* CPU-owned RunQueue/IdleTask. */
        arceos_ex_must_model_runqueue_and_idle_task_as_cpu_owned();

        /* Boot scheduler lock ownership. */
        arceos_ex_must_bind_boot_scheduler_locks_to_owned_objects();

        /* CPU-owned scheduler view lowering. */
        arceos_ex_must_expose_boot_cpu_owned_scheduler_view();
        arceos_ex_must_observe_rest_init_runqueue_facts_through_cpu_view();
        arceos_ex_must_validate_task_creation_through_cpu_owned_scheduler_view();
        arceos_ex_must_observe_smoke_scheduler_facts_through_cpu_view();

        /* Transitional lowering. */
        arceos_ex_must_cpu_group_lowering_mark_boot_secondary_split_transitional();

        /* CPU/CpuGroup coverage. */
        arceos_ex_must_cpu_group_smoke_cover_index_and_sets();

        /* CurrentTaskRef scope. */
        arceos_ex_must_current_task_ref_be_cpu_view_private();

        /* CurrentRunQueueRef scope. */
        arceos_ex_must_current_runqueue_ref_be_cpu_view_private();

        /* CurrentRunQueueRef topology lowering. */
        arceos_ex_must_current_runqueue_ref_carry_resolved_cpu_id();

        /* RunQueueRef / CurrentRunQueueRef type split. */
        arceos_ex_must_split_selected_runqueue_ref_from_current_runqueue_ref();

        /* BootRunQueueRef transitional lowering. */
        arceos_ex_must_treat_boot_runqueue_ref_as_up_transitional_lowering();

        /* CurrentRunQueueRef API smoke. */
        arceos_ex_must_current_runqueue_ref_api_smoke_use_formal_runqueue_actions();

        /* Scheduler.schedule() payload smoke. */
        arceos_ex_must_scheduler_schedule_smoke_use_payload_cooperative_switch();

        /* Wake-up task CPU action. */
        arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue();

        /* KernelInitTask affinity action. */
        arceos_ex_must_rest_init_pin_kernel_init_as_task_action();

        /* Fork dependency. */
        arceos_ex_must_rest_init_not_make_pre_smp_depend_on_up_multitask_wrapper();

        /* Real BootIdle -> KernelInit stack handoff. */
        arceos_ex_must_rest_init_handoff_real_boot_idle_to_kernel_init_stack();

        /* Boot stack size after the owner handoff. */
        arceos_ex_must_boot_stack_use_16k_after_kernel_init_handoff();

        /* Deferred runtime. */
        arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred();
    }
}
