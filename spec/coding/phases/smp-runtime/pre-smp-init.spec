/*
 * PreSmpInitPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in pre-smp-init.md.
 */

predicate arceos_ex_must_kernel_init_wait_observe_kthreadd_ready_gate() -> bool;
predicate arceos_ex_must_pre_smp_init_model_path_under_smp_runtime_phase() -> bool;
predicate arceos_ex_must_pre_smp_init_code_path_follow_smp_runtime_phase_tree() -> bool;
predicate arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts() -> bool;
predicate arceos_ex_must_pre_smp_init_consume_kernel_init_entry_contract() -> bool;
predicate arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology() -> bool;
predicate arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls() -> bool;
predicate arceos_ex_must_pre_smp_init_workqueue_init_use_pool_mutex_context() -> bool;
predicate arceos_ex_must_pre_smp_init_stop_before_smp_init() -> bool;

type ArceosExPreSmpInitCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_pre_smp_init_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_pre_smp_init_code_path_follow_smp_runtime_phase_tree();

        /* Entry facts. */
        arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts();

        /* kthreadd_done wait side. */
        arceos_ex_must_kernel_init_wait_observe_kthreadd_ready_gate();

        /* KernelInitTask entry. */
        arceos_ex_must_pre_smp_init_consume_kernel_init_entry_contract();

        /* Allocation and CPU topology. */
        arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology();

        /* Runtime support setup. */
        arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls();

        /* workqueue_init() synchronization. */
        arceos_ex_must_pre_smp_init_workqueue_init_use_pool_mutex_context();

        /* Stop before SMP. */
        arceos_ex_must_pre_smp_init_stop_before_smp_init();
    }
}
