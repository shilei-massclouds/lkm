/*
 * Entry Prelude Object Model Specification
 *
 * This file is extracted from spec/charter/main.md.
 * It is intended to be parsed by verifier/modeling tools.
 * Rust/C style comments do not add formal predicates or executable behavior.
 * A comment immediately preceding a Transition or Action may additionally be
 * carried as that handler's human-readable animation description.
 *
 * Global rule:
 * - entries in a drives block form an ordered derivation queue and must be
 *   processed in declaration order.
 */

include "object_kinds.spec";

enum TransitionResult {
    Moved,
    Rejected,
    Blocked,
    Failed,
    NoChange,
}

enum ProcessResult {
    Success,
    Blocked,
    Failed,
}

enum StateEffect {
    Always,
    Conditional,
    None,
}

enum CompletionExtState {
    Pending,
    Completed,
    CompletedAll,
}

enum LocalInterruptExtState {
    Disabled,
    Enabled,
}

enum SavedInterruptState {
    Disabled,
    Enabled,
}

enum PreemptionExtState {
    Enabled,
    Disabled,
}

enum RawSpinLockExtState {
    Unlocked,
    Locked,
}

enum RcuReadSideExtState {
    Quiescent,
    ReadHeld,
}

enum MutexExtState {
    Unlocked,
    Locked,
}

enum MutexInitKind {
    StaticInitializer,
    RuntimeInit,
}

enum RwLockExtState {
    Unlocked,
    ReadHeld,
    WriteHeld,
}

enum RwLockInitKind {
    StaticInitializer,
    RuntimeInit,
}

enum RcuSyncExtState {
    Idle,
    WriterActive,
}

enum PerCpuRwSemaphoreExtState {
    ReadersFast,
    WriterBlocked,
    WriterActive,
}

enum PerCpuRwSemaphoreInitKind {
    StaticInitializer,
    RuntimeInit,
}

enum TranslationControllerKind {
    PhysicalDirect,
    TrampolineVm,
    EarlyVm,
    SwapperVm,
}

enum TranslationActivationKind {
    InitialActivation,
    Handoff,
}

include "scheduler.spec";
include "cpu.spec";
include "cpu_group.spec";
include "kernel_image.spec";
include "raw_dtb.spec";
include "fix_map.spec";
include "linear_map.spec";
include "user_space_reserve.spec";
include "kernel_addr_space.spec";
include "physical_direct.spec";
include "trampoline_vm.spec";
include "early_vm.spec";
include "swapper_vm.spec";
include "vm.spec";
include "trap_type.spec";
include "interrupt_type.spec";
include "exception_type.spec";
include "page_fault_exception_type.spec";
include "syscall_exception_type.spec";
include "breakpoint_exception_type.spec";
include "unexpected_exception_type.spec";
include "trap_flow_type.spec";
include "interrupt_flow_type.spec";
include "exception_flow_type.spec";
include "page_fault_exception_flow_type.spec";
include "syscall_exception_flow_type.spec";
include "breakpoint_exception_flow_type.spec";
include "unexpected_exception_flow_type.spec";

/* Build-time selected payload. Exactly one value is bound by Config. */
enum SelectedPayloadKind {
    Hello,
    Smoke,
    UserBoot,
}

include "task.spec";
include "task_flow.spec";
include "device.spec";
include "initcall.spec";
include "vmalloc.spec";
include "ioremap.spec";
include "bus_type.spec";
include "console.spec";
include "ns16550a_driver.spec";
include "virtio.spec";
include "virtio_mmio.spec";
include "virtio_ring.spec";
include "block_device.spec";
include "bio.spec";
include "virtio_blk.spec";
include "ext2.spec";
include "hwrng.spec";
include "virtio_rng.spec";
include "vfs.spec";
include "files.spec";
include "devfs.spec";
include "allocator.spec";
include "binary_format_registry.spec";
include "exec_sync_boundaries.spec";
include "exec_transaction.spec";
include "elf_object.spec";
include "user_stack.spec";
include "user_boot.spec";

function addr_of<T>(value: T) -> AddrIdentity<T>;
function phys_addr<T>(value: T) -> PhysAddr<T>;
function virt_addr<T, S: VirtualAddressSpace, A: VirtualAddressArea>(value: T, space: S, area: A) -> VirtAddr<T>;
function page_cover_count<T>(range: PhysAddrRange<T>, page_size: Size) -> PageCount;
function slot_page_count<T>(slot: FixMapSlotRange<T>) -> PageCount;

predicate exists<T>(value: T) -> bool;
predicate readable<T>(value: T) -> bool;
predicate stable<T>(value: T) -> bool;
predicate static_allocated<T>(value: T) -> bool;
predicate size_of<T>(value: T) -> Size;
predicate size_of<T>() -> Size;
predicate align_of<T>() -> Size;
predicate aligned(addr: Addr, align: Size) -> bool;
predicate inside<T>(inner_start: T, inner_end: T, outer_start: T, outer_end: T) -> bool;
predicate non_empty<T>(value: T) -> bool;
predicate well_formed<T>(value: T) -> bool;
predicate contains<T, U>(container: T, value: U) -> bool;
predicate sbi_hsm_available() -> bool;
predicate ordered_booting_enabled() -> bool;
predicate primary_hart_only_at_kernel_entry() -> bool;
predicate firmware_dtb_blob_in_ram_at_kernel_entry<T>(dtb_pa: PhysAddr<T>) -> bool;
predicate firmware_dtb_blob_complete_at_kernel_entry<T>(dtb_pa: PhysAddr<T>) -> bool;
predicate firmware_dtb_blob_accessible_at_kernel_entry<T>(dtb_pa: PhysAddr<T>) -> bool;
predicate firmware_dtb_header_accessible<T>(range: PhysAddrRange<T>) -> bool;
predicate firmware_dtb_range_accessible<T>(range: PhysAddrRange<T>) -> bool;
predicate platform_hart_id_valid(hartid: HartId) -> bool;
predicate physical_memory_ranges_ready<T, U>(memory: T, dtb: U) -> bool;
predicate physical_memory_ranges_published<T>(memory: T) -> bool;
predicate platform_cpu_info_ready<T, U>(cpu_info: T, dtb: U) -> bool;
predicate platform_cpu_info_published<T>(cpu_info: T) -> bool;
predicate early_dtb_platform_facts_ready<T, U>(early_dtb: T, raw_dtb: U) -> bool;
predicate fdt_reserved_memory_ranges_ready<T, U>(early_dtb: T, raw_dtb: U) -> bool;
predicate memblock_fdt_reserved_ranges_applied<T, U>(memblock: T, early_dtb: U) -> bool;
predicate interrupt_concurrency_closed() -> bool;
predicate task_concurrency_closed() -> bool;
predicate smp_concurrency_closed() -> bool;
predicate smp_concurrency_open<T>(cpu_group: T) -> bool;
predicate early_boot_irqs_disabled_true() -> bool;
predicate early_boot_irqs_disabled_false() -> bool;
predicate early_boot_irqs_disabled_cleared_before_local_irq_enable<T>(phase: T) -> bool;
predicate local_irq_enable_phase_ready<T>(phase: T) -> bool;
predicate local_irq_enable_phase_has_no_within_context<T>(phase: T) -> bool;
predicate interrupt_concurrency_open_for_boot_cpu() -> bool;
predicate boot_cpu_local_irq_enabled() -> bool;
predicate selected_payload_ready() -> bool;
predicate selected_payload_no_return_handoff() -> bool;
predicate selected_payload_handoff_kind_bound<T, C>(handoff: T, config: C) -> bool;
predicate selected_payload_variant_setup_ready<T, C, U>(handoff: T, config: C, user_boot: U) -> bool;
predicate selected_payload_variant_prepare_ready<T, C, U>(handoff: T, config: C, user_boot: U) -> bool;
predicate selected_payload_no_return_entry_bound<T>(handoff: T) -> bool;
predicate static_branch_cpu_hotplug_read_guard_used<T>(static_branch: T) -> bool;
predicate static_branch_jump_label_mutex_guard_used<T>(static_branch: T) -> bool;
predicate static_branch_text_patch_sync_deferred<T>(static_branch: T) -> bool;
predicate riscv_early_boot_alternatives_deferred<T>(vm: T) -> bool;
predicate riscv_early_boot_alternatives_mmu_off_boundary_preserved<T>(vm: T) -> bool;
predicate vmlinux_build_id_deferred<T>(phase: T) -> bool;
predicate page_address_init_deferred<T>(phase: T) -> bool;
predicate efi_boot_init_deferred<T>(phase: T) -> bool;
predicate entry_successor_start_kernel_position_preserved<T>(phase: T) -> bool;
predicate memblock_phys_ram_base_ready<T>(memblock: T) -> bool;
predicate memblock_kernel_va_pa_offset_ready<T, U>(memblock: T, vm: U) -> bool;
predicate memblock_dma32_limit_ready<T>(memblock: T) -> bool;
predicate memblock_dma32_zone_input_ready<T>(memblock: T) -> bool;
predicate memblock_hugetlb_early_reserve_deferred<T>(memblock: T) -> bool;
predicate core_prepare_acpi_boot_tables_trimmed<T>(phase: T) -> bool;
predicate core_prepare_early_memtest_input_trimmed<T>(phase: T) -> bool;
predicate core_prepare_sparse_init_trimmed<T>(phase: T) -> bool;
predicate core_prepare_vmemmap_tlb_flush_trimmed<T>(phase: T) -> bool;
predicate core_prepare_crashkernel_trimmed<T>(phase: T) -> bool;
predicate core_prepare_kasan_trimmed<T>(phase: T) -> bool;
predicate core_prepare_acpi_rintc_trimmed<T>(phase: T) -> bool;
predicate core_prepare_acpi_cpu_numa_trimmed<T>(phase: T) -> bool;
predicate core_prepare_cbop_block_size_deferred<T>(phase: T) -> bool;
predicate core_prepare_boot_alternatives_deferred<T>(phase: T) -> bool;
predicate core_prepare_rt_signal_env_deferred<T>(phase: T) -> bool;
predicate core_prepare_user_isa_deferred<T>(phase: T) -> bool;
predicate core_prepare_static_call_trimmed<T>(phase: T) -> bool;
predicate core_prepare_early_security_deferred<T>(phase: T) -> bool;
predicate core_prepare_boot_config_trimmed<T>(phase: T) -> bool;
predicate core_prepare_boot_cpu_hook_trimmed<T>(phase: T) -> bool;
predicate core_prepare_extra_init_args_trimmed<T>(phase: T) -> bool;
predicate core_prepare_vfs_caches_early_deferred<T>(phase: T) -> bool;
predicate sched_init_housekeeping_deferred<T>(phase: T) -> bool;
predicate sched_init_workqueue_workers_deferred<T>(phase: T) -> bool;
predicate sched_init_context_tracking_runtime_deferred<T>(phase: T) -> bool;
predicate irq_time_full_irq_desc_allocator_deferred<T>(phase: T) -> bool;
predicate irq_time_pmu_runtime_lifecycle_deferred<T>(phase: T) -> bool;
predicate irq_time_late_time_hook_trimmed<T>(phase: T) -> bool;
predicate irq_time_periodic_timer_service_deferred<T>(phase: T) -> bool;
predicate irq_time_uart_rx_deferred<T>(phase: T) -> bool;
predicate irq_time_tty_runtime_deferred<T>(phase: T) -> bool;
predicate irq_time_uart_fifo_concurrency_deferred<T>(phase: T) -> bool;
predicate irq_open_real_console_handoff_deferred<T>(phase: T) -> bool;
predicate irq_open_slub_full_enable_deferred<T>(phase: T) -> bool;
predicate process_prepare_root_pid_runtime_deferred<T>(owner: T) -> bool;
predicate process_prepare_vm_stack_hotplug_callbacks_deferred<T>(owner: T) -> bool;
predicate process_prepare_key_runtime_sync_deferred<T>(owner: T) -> bool;
predicate process_prepare_lsm_runtime_sync_deferred<T>(owner: T) -> bool;
predicate process_prepare_pseudo_fs_runtime_locks_deferred<T>(owner: T) -> bool;
predicate scheduler_fpu_vector_switch_deferred<T>(scheduler: T) -> bool;
predicate scheduler_generic_task_return_deferred<T>(scheduler: T) -> bool;
predicate boot_idle_full_tick_runtime_deferred<T>(runtime: T) -> bool;
predicate boot_idle_full_rcu_runtime_deferred<T>(runtime: T) -> bool;
predicate boot_idle_full_irq_idle_deferred<T>(runtime: T) -> bool;
predicate pre_smp_cad_pid_deferred<T>(phase: T) -> bool;
predicate pre_smp_proc_vmstat_deferred<T>(phase: T) -> bool;
predicate pre_smp_lockup_detector_deferred<T>(phase: T) -> bool;
predicate smp_bringup_full_ap_cpu_local_chain_deferred<T>(phase: T) -> bool;
predicate runtime_async_domains_deferred<T>(phase: T) -> bool;
predicate runtime_async_cookies_deferred<T>(phase: T) -> bool;
predicate runtime_async_pending_list_deferred<T>(phase: T) -> bool;
predicate runtime_async_waitqueue_deferred<T>(phase: T) -> bool;
predicate runtime_async_workers_deferred<T>(phase: T) -> bool;
predicate runtime_padata_instances_deferred<T>(phase: T) -> bool;
predicate initcall_same_level_order_independence_proof_deferred<T>(table: T) -> bool;
predicate swapper_vm_strict_kernel_rwx_boundary_deferred<T>(swapper_vm: T) -> bool;
predicate swapper_vm_final_permissions_not_split_yet<T>(swapper_vm: T) -> bool;
predicate cpu_hotplug_ap_sync_state_online<T, U>(hotplug_state: T, cpu: U) -> bool;
predicate cpu_hotplug_read_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate cpu_hotplug_write_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate cpu_add_remove_mutex_guard_used<T, U>(subject: T, mutex: U) -> bool;
predicate smpboot_threads_mutex_guard_used<T, U>(subject: T, mutex: U) -> bool;
predicate scheduler_domains_mutex_guard_used<T, U>(scheduler: T, mutex: U) -> bool;
predicate scheduler_smp_cpu_masks_stable<T, U>(scheduler: T, cpu_group: U) -> bool;
predicate cpu_running_wait_lock_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate done_up_wait_lock_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate sbi_boot_data_publish_barriers_observed<T>(provider: T) -> bool;
predicate sbi_hsm_extension_available<T>(sbi: T) -> bool;
predicate bp_selects_secondary_start_sbi_entry<T>(provider: T) -> bool;
predicate sbi_hsm_hart_start_requests_issued<T, U>(provider: T, cpu_group: U) -> bool;
predicate sbi_hsm_startup_signal_keyed_by_logical_id<T, F: TaskFlow>(provider: T, flow: F) -> bool;
predicate sbi_hsm_startup_targets_task_fixed_flow<T, K: Task, F: TaskFlow>(
    provider: T,
    task: K,
    flow: F
) -> bool;
predicate secondary_idle_tasks_on_cpu_reserved<T>(cpu_group: T) -> bool;
predicate secondary_idle_task_breakpoints_invalid<T>(cpu_group: T) -> bool;
predicate secondary_idle_flows_base<T>(cpu_group: T) -> bool;
predicate sbi_hsm_hart_start_return_observed<T, U>(provider: T, cpu_group: U) -> bool;
predicate sbi_hart_boot_data_per_secondary_cpu<T, U>(provider: T, cpu_group: U) -> bool;
predicate sbi_hart_boot_data_task_ptr_is_secondary_idle_task<T, U>(provider: T, idle_tasks: U) -> bool;
predicate sbi_hart_boot_data_stack_ptr_is_secondary_pt_regs_stack<T, U>(provider: T, idle_tasks: U) -> bool;
predicate ap_secondary_start_sbi_entry_reached<T>(cpu_group: T) -> bool;
predicate ap_entry_uses_logical_secondary_cpu<T>(cpu_group: T) -> bool;
predicate ap_entry_does_not_create_boot_current_cpu<T>(cpu_group: T) -> bool;
predicate ap_entry_consumes_sbi_hart_boot_data<T, U>(provider: T, cpu_group: U) -> bool;
predicate ap_current_task_is_secondary_idle_task<T, U>(cpu_group: T, idle_tasks: U) -> bool;
predicate ap_stack_is_secondary_idle_task_stack<T, U>(cpu_group: T, idle_tasks: U) -> bool;
predicate ap_pt_regs_pointer_established<T, U>(cpu_group: T, idle_tasks: U) -> bool;
predicate ap_kernel_fpu_vector_disabled<T>(cpu_group: T) -> bool;
predicate ap_interrupts_masked_on_entry<T>(cpu_group: T) -> bool;
predicate ap_switches_to_swapper_vm<T>(vm: T) -> bool;
predicate ap_formal_trap_entry_installed<T, U>(trap: T, exception: U) -> bool;
predicate ap_smp_callin_reached<T>(cpu_group: T) -> bool;
predicate ap_current_active_mm_is_init_mm<T, U>(cpu_group: T, init_mm: U) -> bool;
predicate ap_topology_recorded<T>(cpu_group: T) -> bool;
predicate ap_notify_cpu_starting_observed<T>(cpu_group: T) -> bool;
predicate ap_local_irq_enable_summary_deferred<T>(ack: T) -> bool;
predicate ap_cache_tlb_flush_summary_observed<T>(ack: T) -> bool;
predicate ap_ipi_enable_observed<T>(ack: T) -> bool;
predicate ap_cpu_online_fact_published<T>(cpu_group: T) -> bool;
predicate ap_cpu_running_completion_produced<T>(sync_set: T) -> bool;
predicate ap_local_irq_enable_observed<T>(phase: T) -> bool;
predicate ap_cpu_startup_entry_reached<T>(phase: T) -> bool;
predicate ap_cpuhp_online_idle_reached<T>(phase: T) -> bool;
predicate ap_done_up_completion_produced<T>(sync_set: T) -> bool;
predicate ap_idle_or_park_loop_entered<T>(cpu_group: T) -> bool;
predicate ap_does_not_run_bp_payload_or_syscalls<T>(cpu_group: T) -> bool;
predicate ap_smp_callin_ack_matches_secondary_cpu<T>(cpu_group: T) -> bool;
predicate secondary_cpus_online_after_ap_ack<T>(cpu_group: T) -> bool;
predicate ap_hotplug_thread_memory_barrier_pair_deferred<T>(ack: T) -> bool;
predicate resource_tree_write_lock_guard_used<T>(resource_tree: T) -> bool;
predicate printk_buffer_setup_local_irq_save_restore_used<T>(buffer: T) -> bool;
predicate printk_buffer_setup_local_irq_guard_used<T, U>(buffer: T, control: U) -> bool;
predicate sched_clock_setup_local_irq_disable_enable_used<T>(clock: T) -> bool;
predicate sched_clock_setup_local_irq_guard_used<T, U>(clock: T, control: U) -> bool;
predicate printk_buffer_setup_prepared_dynamic_buffer<T>(buffer: T) -> bool;
predicate printk_buffer_setup_switched_active_buffer<T>(buffer: T) -> bool;
predicate printk_buffer_setup_copied_remaining_records<T>(buffer: T) -> bool;
predicate jump_label_mutex_static_initializer<T>(mutex: T) -> bool;
predicate jump_label_mutex_storage_bound<T>(mutex: T) -> bool;
predicate jump_label_mutex_init_kind_static<T>(mutex: T) -> bool;
predicate jump_label_mutex_ready<T>(mutex: T) -> bool;
predicate jump_label_mutex_unlocked<T>(mutex: T) -> bool;
predicate jump_label_mutex_wait_queue_ready<T>(mutex: T) -> bool;
predicate jump_label_mutex_wait_lock_internal_deferred<T>(mutex: T) -> bool;
predicate mutex_storage_bound<T>(mutex: T) -> bool;
predicate mutex_init_kind_recorded<T>(mutex: T) -> bool;
predicate mutex_preset_respects_init_kind<T>(mutex: T) -> bool;
predicate mutex_owns_wait_queue<T>(mutex: T) -> bool;
predicate mutex_initialized<T>(mutex: T) -> bool;
predicate mutex_ready<T>(mutex: T) -> bool;
predicate mutex_unlocked<T>(mutex: T) -> bool;
predicate mutex_locked<T>(mutex: T) -> bool;
predicate mutex_wait_queue_ready<T>(mutex: T) -> bool;
predicate mutex_wait_lock_internal_deferred<T>(mutex: T) -> bool;
predicate mutex_lock_success_sets_owner<T, U>(mutex: T, task_ref: U) -> bool;
predicate mutex_owner_matches_unlocker<T, U>(mutex: T, task_ref: U) -> bool;
predicate mutex_lock_acquired<T, U>(mutex: T, task_ref: U) -> bool;
predicate mutex_unlock_released<T, U>(mutex: T, task_ref: U) -> bool;
predicate mutex_waiter_enqueued<T, U>(mutex: T, task_ref: U) -> bool;
predicate mutex_waiter_finished<T, U>(mutex: T, task_ref: U) -> bool;
predicate mutex_wakes_one_waiter<T>(mutex: T) -> bool;
predicate mutex_contention_may_sleep<T>(mutex: T) -> bool;
predicate mutex_recursive_locking_forbidden<T>(mutex: T) -> bool;
predicate mutex_unlock_requires_owner<T>(mutex: T) -> bool;
predicate completion_storage_bound<T>(completion: T) -> bool;
predicate completion_ready<T>(completion: T) -> bool;
predicate completion_pending<T>(completion: T) -> bool;
predicate completion_done_count_is_zero<T>(completion: T) -> bool;
predicate completion_wait_queue_ready<T>(completion: T) -> bool;
predicate completion_owns_wait_queue<T>(completion: T) -> bool;
predicate completion_online<T>(completion: T) -> bool;
predicate completion_handle_published<T>(completion: T) -> bool;
predicate completion_handle_revoked<T>(completion: T) -> bool;
predicate completion_complete_committed<T>(completion: T) -> bool;
predicate completion_token_available<T>(completion: T) -> bool;
predicate completion_wakes_one_waiter<T>(completion: T) -> bool;
predicate completion_complete_all_committed<T>(completion: T) -> bool;
predicate completion_all_waiters_released<T>(completion: T) -> bool;
predicate completion_waiter_enqueued<T>(completion: T) -> bool;
predicate completion_waiter_finished<T>(completion: T) -> bool;
predicate completion_wait_queue_preserved<T>(completion: T) -> bool;
predicate completion_done_observed<T>(completion: T) -> bool;
predicate completion_wait_lock_irqsave_entered<T, U>(completion: T, current_cpu: U) -> bool;
predicate completion_wait_lock_irqrestore_exited<T, U>(completion: T, current_cpu: U) -> bool;
predicate completion_done_increment_guarded_by_wait_lock<T>(completion: T) -> bool;
predicate completion_wake_guarded_by_wait_lock<T>(completion: T) -> bool;
predicate completion_wait_lock_guard_used<T, U>(completion: T, lock: U) -> bool;
predicate wait_queue_ready<T>(queue: T) -> bool;
predicate wait_queue_wake_one_committed<T>(queue: T) -> bool;
predicate wait_queue_wake_all_committed<T>(queue: T) -> bool;
predicate wait_queue_waiter_enqueued<T>(queue: T) -> bool;
predicate wait_queue_waiter_finished<T>(queue: T) -> bool;
predicate current_cpu_resolved_target_is<F, R, C>(flow: F, cpu_ref: R, cpu: C) -> bool;
predicate current_cpu_inherited_by_sync_drives<F, C>(flow: F, child: C) -> bool;
predicate current_cpu_not_inherited_by_async_emits<F, C>(flow: F, child: C) -> bool;
predicate cpu_hartid_ready<T>(cpu: T, hartid: HartId) -> bool;
predicate cpu_logical_id_ready<T, U>(cpu: T, logical_id: U) -> bool;
predicate cpu_bootstrap_role<T>(cpu: T) -> bool;
predicate cpu_possible<T>(cpu: T) -> bool;
predicate cpu_present<T>(cpu: T) -> bool;
predicate cpu_active<T>(cpu: T) -> bool;
predicate cpu_online<T>(cpu: T) -> bool;
predicate cpu_ref_targets<T, U>(cpu_ref: T, cpu: U) -> bool;
predicate cpu_ref_ready<T>(cpu_ref: T) -> bool;
predicate cpu_group_uses_logical_id_index<T>(cpu_group: T) -> bool;
predicate cpu_group_boot_cpu_index_zero<T, U>(cpu_group: T, cpu: U) -> bool;
predicate cpu_group_cpu_ref_at<T, U, V>(cpu_group: T, logical_id: U, cpu_ref: V) -> bool;
predicate cpu_group_cpu_ref_targets<T, U, V>(cpu_group: T, cpu_ref: U, cpu: V) -> bool;
predicate cpu_group_possible_set_ready<T>(cpu_group: T) -> bool;
predicate cpu_group_present_set_ready<T>(cpu_group: T) -> bool;
predicate cpu_group_online_set_ready<T>(cpu_group: T) -> bool;
predicate cpu_group_possible_contains<T, U>(cpu_group: T, cpu_ref: U) -> bool;
predicate cpu_group_present_contains<T, U>(cpu_group: T, cpu_ref: U) -> bool;
predicate cpu_group_online_contains<T, U>(cpu_group: T, cpu_ref: U) -> bool;
predicate cpu_trap_resource_ready<T, U>(cpu: T, trap: U) -> bool;
predicate cpu_exception_resource_ready<T, U>(cpu: T, exception: U) -> bool;
predicate cpu_interrupt_resource_ready<T, U>(cpu: T, interrupt: U) -> bool;
predicate cpu_local_interrupt_resource_ready<T, U>(interrupt: T, cpu: U) -> bool;
predicate cpu_local_interrupts_disabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_enabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_saved_and_disabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_restored<T>(control: T) -> bool;
predicate zonelist_update_seq_write_irqsave_entered<T, I>(update_seq: T, local_interrupt: I) -> bool;
predicate zonelist_update_seq_write_irqrestore_exited<T, I>(update_seq: T, local_interrupt: I) -> bool;
predicate printk_deferred_section_entered<T>(section: T) -> bool;
predicate printk_deferred_section_exited<T>(section: T) -> bool;
predicate current_task_resolved_target_is<F, R, T>(flow: F, task_ref: R, task: T) -> bool;
predicate current_task_ref_derived_from_selector<R, T>(task_ref: R, task: T) -> bool;
predicate current_task_selector_validates_execution<F, T>(flow: F, task: T) -> bool;
predicate current_task_bind_scheduler_commit_boundary_valid<F: TaskFlow, T: Task, D: TaskFlow>(flow: F, task: T, dispatch_flow: D) -> bool;
predicate boot_task_bind_task_stack_boundary_valid<F: TaskFlow, T: Task, S: Stack>(flow: F, task: T, stack: S) -> bool;
predicate boot_task_refresh_task_stack_boundary_valid<F: TaskFlow, T: Task, S: Stack>(flow: F, task: T, stack: S) -> bool;
predicate boot_task_stack_argument_matches_task<T: Task, S: Stack>(task: T, stack: S) -> bool;
predicate current_task_bind_task_has_unique_valid_ref<T: Task>(task: T) -> bool;
predicate current_task_binding_committed<C: CPU, T: Task, F: TaskFlow>(cpu: C, task: T, flow: F) -> bool;
predicate current_task_binding_ref_is<C: CPU, R: TaskRef>(cpu: C, task_ref: R) -> bool;
predicate current_task_binding_address_view_is<C: CPU, T: Task, V>(cpu: C, task: T, view: V) -> bool;
predicate current_task_binding_revision_is<C: CPU>(cpu: C, revision: usize) -> bool;
predicate current_task_binding_address_refreshed<C: CPU, T: Task>(cpu: C, task: T) -> bool;
predicate current_task_binding_identity_preserved_for_same_task<C: CPU, T: Task>(cpu: C, task: T) -> bool;
predicate current_task_binding_is_cpu_local<C: CPU>(cpu: C) -> bool;
predicate current_task_binding_replaced_at_scheduler_commit<C: CPU, T: Task>(cpu: C, task: T) -> bool;
predicate current_task_bind_preserves_preemption_state<T: Task>(task: T) -> bool;
predicate current_task_stack_attribute_valid<T: Task, S: Stack>(task: T, stack: S) -> bool;
predicate current_task_bind_stack_pointer_valid_for_boundary<F: TaskFlow, T: Task, S: Stack>(flow: F, task: T, stack: S) -> bool;
predicate current_task_stack_pair_unbound<C: CPU>(cpu: C) -> bool;
predicate current_stack_binding_committed<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_stack_pointer_matches_active_controller<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_stack_binding_address_refreshed<C: CPU, S: Stack>(cpu: C, stack: S) -> bool;
predicate current_stack_binding_identity_preserved_for_same_stack<C: CPU, S: Stack>(cpu: C, stack: S) -> bool;
predicate current_stack_binding_is_cpu_local<C: CPU>(cpu: C) -> bool;
predicate current_stack_binding_matches_task<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_task_stack_binding_pair_consistent<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate current_stack_bind_preserves_preemption_state<T: Task>(task: T) -> bool;
predicate boot_task_bind_task_stack_atomic<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate boot_task_refresh_task_stack_atomic<C: CPU, T: Task, S: Stack>(cpu: C, task: T, stack: S) -> bool;
predicate task_creation_flow_contract_ready<T>(core: T) -> bool;
predicate task_clone_args_ready<T>(task: T) -> bool;
predicate task_creation_copy_process_committed<T, U, V>(core: T, src_task: U, dst_task: V) -> bool;
predicate task_creation_copy_process_used_current_source<T, U, R>(
    core: T,
    src_task: U,
    src_task_ref: R
) -> bool;
predicate task_creation_used_clone_args<T, U>(core: T, task: U) -> bool;
predicate task_creation_copy_process_sighand_siglock_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_tasklist_lock_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_pidmap_lock_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_sched_fork_locks_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_reference_sync_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_failure_rollback_deferred<T>(core: T) -> bool;
predicate task_struct_allocated<T>(task: T) -> bool;
predicate task_duplicated_from<T, U>(dst_task: T, src_task: U) -> bool;
predicate task_pid_allocated<T, U>(task: T, pid_ns: U) -> bool;
predicate task_creds_copied<T, U>(task: T, creds: U) -> bool;
predicate task_file_context_copied_or_shared<T, U>(task: T, files: U) -> bool;
predicate task_signal_context_ready<T, U>(task: T, signal: U) -> bool;
predicate task_security_context_allocated<T, U>(task: T, security: U) -> bool;
predicate task_thread_context_ready<T>(task: T) -> bool;
predicate task_sched_entity_initialized<T, U>(task: T, scheduler: U) -> bool;
predicate kernel_init_task_stack_switch_committed<T, U, V>(scheduler: T, prev_task: U, next_task: V) -> bool;
predicate payload_execution_owned_by_kernel_init_task<T, U>(payload_phase: T, task: U) -> bool;
predicate kernel_init_still_waiting_for_kthreadd_done<T>(task: T) -> bool;
predicate kernel_init_kthreadd_done_wait_ready<T, U, V>(wait: T, task: U, gate: V) -> bool;
predicate kernel_init_kthreadd_done_wait_released<T, U, V>(wait: T, task: U, gate: V) -> bool;
predicate kernel_init_observed_kthreadd_done_release<T, U>(task: T, gate: U) -> bool;
predicate kernel_init_released_for_pre_smp_init<T>(task: T) -> bool;
predicate cpuset_smp_trimmed_noop() -> bool;
predicate cpuset_smp_trimmed_because_config_cpusets_disabled() -> bool;
predicate cpuset_smp_trimmed_because_config_cgroups_disabled() -> bool;
predicate driver_core_device_registry_ready<T>(core: T) -> bool;
predicate driver_core_bus_registry_ready<T>(core: T) -> bool;
predicate driver_core_class_registry_ready<T>(core: T) -> bool;
predicate driver_core_firmware_kobject_ready<T>(core: T) -> bool;
predicate driver_core_backing_dev_info_deferred<T>(core: T) -> bool;
predicate driver_core_device_link_workqueue_deferred<T>(core: T) -> bool;
predicate driver_core_devtmpfs_init_deferred<T>(core: T) -> bool;
predicate driver_core_devtmpfs_sync_primitives_deferred<T>(core: T) -> bool;
predicate driver_core_of_core_init_deferred<T>(core: T) -> bool;
predicate driver_core_of_core_mutex_guard_deferred<T>(core: T) -> bool;
predicate driver_core_hypervisor_trimmed_noop<T>(core: T) -> bool;
predicate driver_core_hypervisor_trimmed_because_config_sys_hypervisor_disabled<T>(core: T) -> bool;
predicate driver_core_pre_platform_deferred() -> bool;
predicate driver_core_pre_platform_order_preserved() -> bool;
predicate driver_core_auxiliary_bus_deferred<T>(core: T) -> bool;
predicate driver_core_memory_dev_deferred<T>(core: T) -> bool;
predicate driver_core_node_dev_deferred<T>(core: T) -> bool;
predicate driver_core_cpu_dev_deferred<T>(core: T) -> bool;
predicate driver_core_container_dev_deferred<T>(core: T) -> bool;
predicate driver_core_post_platform_deferred() -> bool;
predicate driver_model_entry_position_preserved() -> bool;
predicate irq_proc_view_procfs_config_enabled<T>(view: T) -> bool;
predicate irq_proc_view_default_smp_affinity_deferred<T>(view: T) -> bool;
predicate irq_proc_view_existing_irq_desc_exports_deferred<T>(view: T) -> bool;
predicate irq_proc_view_effective_affinity_exports_deferred<T>(view: T) -> bool;
predicate irq_proc_view_setup_deferred() -> bool;
predicate proc_irq_export_deferred() -> bool;
predicate ctor_table_position_preserved<T, L>(table: T, lds: L) -> bool;
predicate constructors_trimmed_or_empty<T>(table: T) -> bool;
predicate constructors_trimmed_because_config_constructors_disabled<T>(table: T) -> bool;
predicate kthreadd_entry_reaches_schedule_loop<T, U>(task: T, scheduler: U) -> bool;
predicate kthreadd_schedule_loop_ready<T, U>(task: T, scheduler: U) -> bool;
predicate kthreadd_schedule_loop_active<T, U>(task: T, scheduler: U) -> bool;
predicate task_preemption_control_ready<T>(task: T) -> bool;
predicate task_preempt_count_initialized_to_init_preempt_count<T>(task: T) -> bool;
predicate task_preemption_disabled<T>(task: T) -> bool;
predicate task_preemption_enabled<T>(task: T) -> bool;
predicate task_preemption_enabled_no_resched<T>(task: T) -> bool;
predicate task_ref_targets<T, U>(task_ref: T, task: U) -> bool;
predicate task_ref_ready<T>(task_ref: T) -> bool;
predicate task_state_new<T>(task: T) -> bool;
predicate task_state_running<T>(task: T) -> bool;
predicate task_not_enqueued<T>(task: T) -> bool;
predicate task_enqueued_on_scheduler<T, U>(task: T, runqueue: U) -> bool;
predicate task_pid_lookup_under_rcu_read<T, U, V>(task: T, pid_ns: U, read_side: V) -> bool;
predicate task_pid_lookup_rcu_guard_used<T, U>(task: T, read_side: U) -> bool;
predicate task_runtime_state_transition_allowed<T>(task: T, state: TaskRuntimeState) -> bool;
predicate task_runtime_state_is<T>(task: T, state: TaskRuntimeState) -> bool;
predicate task_thread_context_owned<T, U>(task: T, context: U) -> bool;
predicate task_thread_context_core_register_set<T>(context: T) -> bool;
predicate task_thread_context_core_saved<T>(context: T) -> bool;
predicate task_thread_context_core_restored<T>(context: T) -> bool;
predicate task_flag_no_setaffinity<T>(task: T) -> bool;
predicate task_cpumask_is<T, U>(task: T, cpu_ref: U) -> bool;
predicate kthreadd_provider_ref_targets<T, U>(task_ref: T, task: U) -> bool;
predicate kthreadd_provider_ready<T>(task: T) -> bool;
predicate runqueue_pick_next_task_returns<T, U, V>(runqueue_ref: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_select_scheduler_returns<T, U, V>(scheduler: T, task_ref: U, runqueue_ref: V) -> bool;
predicate scheduler_schedule_event_available<T>(scheduler: T) -> bool;
predicate scheduler_schedule_smoke_ready<T>(scheduler: T) -> bool;
predicate scheduler_preset_ready<T>(scheduler: T) -> bool;
predicate sched_init_prelude_trimmed_paths_ready<T>(paths: T) -> bool;
predicate sched_init_poking_init_trimmed_noop<T>(paths: T) -> bool;
predicate sched_init_ftrace_init_trimmed_noop<T>(paths: T) -> bool;
predicate sched_init_ftrace_trimmed_because_mcount_record_disabled<T>(paths: T) -> bool;
predicate sched_init_prelude_trimmed_paths_position_preserved<T>(paths: T) -> bool;
predicate sched_init_trace_context_boundaries_ready<T>(paths: T) -> bool;
predicate sched_init_early_trace_init_deferred<T>(paths: T) -> bool;
predicate sched_init_trace_init_deferred<T>(paths: T) -> bool;
predicate sched_init_context_tracking_init_trimmed_noop<T>(paths: T) -> bool;
predicate sched_init_context_tracking_trimmed_because_user_force_disabled<T>(paths: T) -> bool;
predicate sched_init_trace_context_boundaries_position_preserved<T>(paths: T) -> bool;
predicate scheduler_default_root_domain_ready<T, U>(scheduler: T, root_domain: U) -> bool;
predicate sched_class_skeletons_deferred<T>(scheduler: T) -> bool;
predicate scheduler_schedule_local_interrupts_closed<T, U>(scheduler: T, local_interrupt: U) -> bool;
predicate scheduler_schedule_exit_restores_local_interrupts<T, U>(scheduler: T, local_interrupt: U) -> bool;
predicate scheduler_runqueue_lock_held_for_schedule<T, U>(scheduler: T, runqueue: U) -> bool;
predicate scheduler_rcu_context_switch_noted<T, U, V>(scheduler: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_rq_lock_mb_after_spinlock<T, U>(scheduler: T, runqueue: U) -> bool;
predicate scheduler_rq_clock_updated_for_schedule<T, U>(scheduler: T, runqueue: U) -> bool;
predicate scheduler_need_resched_cleared<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_rq_curr_published_rcu<T, U, V>(scheduler: T, runqueue: U, next_ref: V) -> bool;
predicate scheduler_trace_sched_switch_emitted<T, U, V>(scheduler: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_possible_cpu_runqueues_ready<T, U>(scheduler: T, cpu_group: U) -> bool;
predicate scheduler_possible_cpu_runqueues_attached_to_default_root_domain<T, U, V>(
    scheduler: T,
    cpu_group: U,
    root_domain: V
) -> bool;
predicate scheduler_pick_next_task_identity<T, U, V>(scheduler: T, runqueue: U, task: V) -> bool;
predicate scheduler_pick_next_task_selects_runnable<T, U, V>(scheduler: T, runqueue: U, task_ref: V) -> bool;
predicate scheduler_no_task_switch_on_single_task_path<T, U>(scheduler: T, task: U) -> bool;
predicate scheduler_switch_to_prepared<T, U, V, W>(scheduler: T, runqueue: U, prev_ref: V, next_ref: W) -> bool;
predicate scheduler_switch_to_committed<T, U, V>(scheduler: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_switch_to_identity_path<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_switch_to_core_context_saved<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_switch_to_core_context_restored<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_switch_prepare_validates_prev_live_fixed_flow<T, U>(scheduler: T, prev_ref: U) -> bool;
predicate scheduler_switch_prepare_validates_next_breakpoint_flow_ref<T, U>(scheduler: T, next_ref: U) -> bool;
predicate scheduler_switch_finish_atomic<T, U, V>(scheduler: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_identity_switch_emits_no_task_or_context_event<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_terminal_switch_keeps_prev_breakpoint_invalid<T, U>(scheduler: T, prev_ref: U) -> bool;
predicate scheduler_terminal_cleanup_runs_on_next_stack<T, U, V>(scheduler: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_prepare_task_switch_done<T, U, V, W>(scheduler: T, runqueue: U, prev_ref: V, next_ref: W) -> bool;
predicate scheduler_finish_task_switch_done<T, U, V>(scheduler: T, runqueue: U, prev_ref: V) -> bool;
predicate scheduler_finish_task_switch_releases_rq_lock<T, U>(scheduler: T, runqueue: U) -> bool;
predicate scheduler_finish_task_switch_restores_preempt_count<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_switch_mm_or_lazy_tlb_deferred<T>(scheduler: T) -> bool;
predicate scheduler_membarrier_switch_barrier_deferred<T>(scheduler: T) -> bool;
predicate scheduler_idle_mode_used<T>(scheduler: T) -> bool;
predicate scheduler_idle_schedule_committed<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_idle_schedule_returned_to_idle<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_idle_schedule_committed_to_runnable<T, U>(scheduler: T, task_ref: U) -> bool;
predicate boot_idle_schedule_idle_loop_until_resched_clear<T>(scheduler: T) -> bool;
predicate task_scheduler_selected<T, U, V>(scheduler: T, task: U, runqueue: V) -> bool;
predicate task_wakeup_new_rq_clock_updated<T, U>(task: T, runqueue: U) -> bool;
predicate task_wakeup_new_initial_util_avg_posted<T, U>(task: T, runqueue: U) -> bool;
predicate task_wakeup_new_trace_emitted<T>(task: T) -> bool;
predicate task_wakeup_new_preempt_check_done<T, U>(task: T, runqueue: U) -> bool;
predicate task_wakeup_new_task_woken_hook_deferred<T>(task: T) -> bool;
predicate scheduler_payload_cooperative_switch_ready<T>(scheduler: T) -> bool;
predicate scheduler_payload_schedule_from_kernel_init<T, U>(scheduler: T, current_ref: U) -> bool;
predicate scheduler_payload_smoke_task_enqueued<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_payload_smoke_task_yielded_back<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_queue_runtime_state_is<T>(runqueue: T, state: SchedulerQueueRuntimeState) -> bool;
predicate scheduler_queue_task_refs_empty<T>(runqueue: T) -> bool;
predicate scheduler_queue_task_refs_some<T>(runqueue: T) -> bool;
predicate scheduler_contains_task<T, U>(runqueue: T, task_ref: U) -> bool;
predicate boot_runqueue_lock_ready<T, U>(runqueue: T, lock: U) -> bool;
predicate boot_runqueue_ready<T, U>(runqueue: T, cpu: U) -> bool;
predicate boot_runqueue_possible_cpu_set_covered_by_cpu_group<T, U>(runqueue: T, cpu_group: U) -> bool;
predicate boot_runqueue_cpu_ref_covered_by_root_domain<T, U, V>(
    runqueue: T,
    root_domain: U,
    cpu_ref: V
) -> bool;
predicate boot_runqueue_attached_to_root_domain<T, U>(runqueue: T, root_domain: U) -> bool;
predicate boot_runqueue_root_attach_held_runqueue_lock<T, U>(
    runqueue: T,
    lock: U
) -> bool;
predicate boot_runqueue_class_queues_ready<T>(runqueue: T) -> bool;
predicate boot_runqueue_balance_push_disabled<T>(runqueue: T) -> bool;
predicate cpu_owns_runqueue<T, U>(cpu: T, runqueue: U) -> bool;
predicate cpu_owns_idle_task<T, U>(cpu: T, task: U) -> bool;
predicate cpu_runqueue_idle_is_cpu_idle_task<T, U, V>(
    cpu: T,
    runqueue: U,
    task: V
) -> bool;
predicate scheduler_orchestrates_cpu_owned_runqueues<T, U>(
    scheduler: T,
    cpu_group: U
) -> bool;
predicate bit_wait_queue_table_ready<T>(table: T) -> bool;
predicate bit_wait_queue_table_bucket_count_matches_wait_table_size<T>(table: T) -> bool;
predicate bit_wait_queue_table_bucket_waitqueues_ready<T>(table: T) -> bool;
predicate bit_wait_queue_table_bucket_locks_ready<T>(table: T) -> bool;
predicate bit_wait_queue_table_bucket_lists_empty<T>(table: T) -> bool;
predicate workqueue_early_framework_ready<T, U>(workqueue: T, cpu_group: U) -> bool;
predicate workqueue_system_queues_ready<T>(workqueue: T) -> bool;
predicate workqueue_system_queue_count_matches_linux_early<T>(workqueue: T) -> bool;
predicate workqueue_worker_pools_prepared<T>(workqueue: T) -> bool;
predicate workqueue_cpu_worker_pools_ready<T>(workqueue: T) -> bool;
predicate workqueue_bh_pools_ready<T>(workqueue: T) -> bool;
predicate workqueue_pool_workqueue_cache_ready<T>(workqueue: T) -> bool;
predicate workqueue_unbound_cpumask_ready<T>(workqueue: T) -> bool;
predicate workqueue_attrs_ready<T>(workqueue: T) -> bool;
predicate workqueue_system_affinity_pods_ready<T>(workqueue: T) -> bool;
predicate workqueue_pool_mutex_ready<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_struct_mutex_ready<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_pool_mutex_guard_used<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_struct_mutex_guard_used<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_init_pool_mutex_guard_used<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_topology_pool_mutex_guard_used<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_topology_struct_mutex_guard_used<T, U>(workqueue: T, mutex: U) -> bool;
predicate workqueue_pool_attach_mutex_deferred<T>(workqueue: T) -> bool;
predicate workqueue_mayday_lock_deferred<T>(workqueue: T) -> bool;
predicate workqueue_manager_wait_deferred<T>(workqueue: T) -> bool;
predicate workqueue_workers_not_running<T>(workqueue: T) -> bool;
predicate workqueue_ready_before_smp<T>(workqueue: T) -> bool;
predicate workqueue_rescuers_ready<T, U>(workqueue: T, task: U) -> bool;
predicate workqueue_initial_workers_created<T, U>(workqueue: T, cpu_group: U) -> bool;
predicate workqueue_worker_creation_open<T>(workqueue: T) -> bool;
predicate workqueue_watchdog_ready<T>(workqueue: T) -> bool;
predicate workqueue_smp_topology_deferred<T>(workqueue: T) -> bool;
predicate async_min_active_update_deferred() -> bool;
predicate padata_hotplug_online_state_deferred() -> bool;
predicate padata_hotplug_dead_state_deferred() -> bool;
predicate padata_free_work_list_deferred() -> bool;
predicate deferred_struct_page_init_trimmed_because_config_disabled() -> bool;
predicate deferred_struct_page_completion_trimmed() -> bool;
predicate deferred_pages_static_key_disable_trimmed() -> bool;
predicate page_extension_late_trimmed_because_config_disabled() -> bool;
predicate shuffle_page_allocator_late_trimmed_because_config_disabled() -> bool;
predicate default_sched_root_domain_ready<T, U>(root_domain: T, cpu_group: U) -> bool;
predicate default_sched_root_domain_covers_cpu_group_possible<T, U>(
    root_domain: T,
    cpu_group: U
) -> bool;
predicate default_sched_root_domain_covered_cpus_are_cpu_refs<T, U>(
    root_domain: T,
    cpu_group: U
) -> bool;
predicate default_sched_root_domain_does_not_own_cpu_bodies<T>(root_domain: T) -> bool;
predicate default_sched_root_domain_covers_cpu_ref<T, U>(
    root_domain: T,
    cpu_ref: U
) -> bool;
predicate sched_smp_topology_deferred<T>(root_domain: T) -> bool;
predicate boot_idle_pi_lock_ready<T, U>(task: T, lock: U) -> bool;
predicate boot_idle_init_held_pi_lock<T, U>(task: T, lock: U) -> bool;
predicate boot_idle_init_held_runqueue_lock<T, U>(runqueue: T, lock: U) -> bool;
predicate boot_idle_task_cpu_set_under_rcu_read<T, U>(task: T, cpu_ref: U) -> bool;
predicate boot_runqueue_current_published_with_rcu<T, U>(runqueue: T, task: U) -> bool;
predicate softirq_action_table_ready<T>(softirq: T) -> bool;
predicate softirq_slots_ready<T>(softirq: T) -> bool;
predicate softirq_pending_set_ready<T, U>(softirq: T, per_cpu_storage: U) -> bool;
predicate softirq_execution_closed<T>(softirq: T) -> bool;
predicate softirq_rcu_action_registered<T, U>(softirq: T, rcu_core: U) -> bool;
predicate softirq_tasklet_queues_ready<T, U>(softirq: T, per_cpu_storage: U) -> bool;
predicate softirq_tasklet_actions_registered<T>(softirq: T) -> bool;
predicate softirq_timer_actions_registered<T, U, V>(
    softirq: T,
    timer_wheel: U,
    hrtimer_core: V
) -> bool;
predicate rcu_core_ready<T, U>(rcu_core: T, cpu_group: U) -> bool;
predicate rcu_boot_cpu_online_ready<T, U>(rcu_core: T, boot_cpu: U) -> bool;
predicate rcu_softirq_registered<T, U>(rcu_core: T, softirq: U) -> bool;
predicate rcu_workqueues_ready<T, U>(rcu_core: T, workqueue: U) -> bool;
predicate rcu_node_tree_ready<T, U>(rcu_core: T, cpu_group: U) -> bool;
predicate rcu_node_locks_ready<T>(rcu_core: T) -> bool;
predicate rcu_node_waitqueues_ready<T>(rcu_core: T) -> bool;
predicate rcu_node_poll_work_ready<T>(rcu_core: T) -> bool;
predicate rcu_percpu_data_ready<T, U>(rcu_core: T, per_cpu_storage: U) -> bool;
predicate rcu_kfree_batch_ready<T, U>(rcu_core: T, workqueue: U) -> bool;
predicate rcu_kfree_shrinker_registered<T>(rcu_core: T) -> bool;
predicate rcu_pm_notifier_registered<T>(rcu_core: T) -> bool;
predicate rcu_gp_threads_deferred<T>(rcu_core: T) -> bool;
predicate rcu_runtime_read_side_full_semantics_deferred<T>(rcu_core: T) -> bool;
predicate rcu_scheduler_starting_ready<T>(rcu_core: T) -> bool;
predicate rcu_scheduler_active_level_init<T>(rcu_core: T) -> bool;
predicate rcu_single_online_cpu_at_scheduler_start<T, U>(rcu_core: T, cpu_group: U) -> bool;
predicate rcu_gp_seq_baseline_synced<T>(rcu_core: T) -> bool;
predicate rcu_scheduler_starting_local_irq_guard_used<T, U>(rcu_core: T, local_interrupt: U) -> bool;
predicate rcu_scheduler_starting_gp_seq_update_guarded<T>(rcu_core: T) -> bool;
predicate tasks_rcu_prepared<T>(tasks_rcu: T) -> bool;
predicate tasks_rcu_callback_lists_ready<T, U>(tasks_rcu: T, per_cpu_storage: U) -> bool;
predicate tasks_rcu_enabled_flavors_recorded<T>(tasks_rcu: T) -> bool;
predicate tasks_rcu_percpu_arrays_ready<T, U>(tasks_rcu: T, per_cpu_storage: U) -> bool;
predicate tasks_rcu_percpu_locks_ready<T>(tasks_rcu: T) -> bool;
predicate tasks_rcu_percpu_work_ready<T, U>(tasks_rcu: T, workqueue: U) -> bool;
predicate tasks_rcu_barrier_heads_ready<T>(tasks_rcu: T) -> bool;
predicate tasks_rcu_gp_threads_deferred<T>(tasks_rcu: T) -> bool;
predicate tasks_rcu_ready<T>(tasks_rcu: T) -> bool;
predicate tasks_rcu_gp_threads_created<T, U>(tasks_rcu: T, kthreadd_task: U) -> bool;
predicate rcu_read_side_ready<T>(read_side: T) -> bool;
predicate rcu_read_side_entered<T, U>(read_side: T, current_cpu: U) -> bool;
predicate rcu_read_side_exited<T, U>(read_side: T, current_cpu: U) -> bool;
predicate rcu_read_side_incomplete_first_slice<T>(read_side: T) -> bool;
predicate rcu_read_side_full_semantics_deferred<T>(read_side: T) -> bool;
predicate boot_idle_entry_prepared<T, U>(runtime: T, task: U) -> bool;
predicate boot_idle_arch_cpu_idle_prepare_done<T, U>(runtime: T, cpu: U) -> bool;
predicate boot_idle_cpuhp_online_state_confirmed<T, U>(runtime: T, cpu: U) -> bool;
predicate boot_idle_task_pf_idle<T>(task: T) -> bool;
predicate boot_idle_runtime_loop_entered<T, U>(runtime: T, task: U) -> bool;
predicate boot_idle_runtime_cycle_started<T, U>(runtime: T, task: U) -> bool;
predicate boot_idle_runtime_waiting<T, U>(runtime: T, task: U) -> bool;
predicate boot_idle_runtime_observed_no_need_resched<T, U>(runtime: T, task: U) -> bool;
predicate boot_idle_runtime_observed_need_resched<T, U>(runtime: T, task: U) -> bool;
predicate boot_idle_need_resched_clear_before_wait<T>(task: T) -> bool;
predicate boot_idle_need_resched_set_for_schedule<T>(task: T) -> bool;
predicate boot_idle_need_resched_drained_after_schedule<T>(task: T) -> bool;
predicate boot_idle_polling_set<T>(task: T) -> bool;
predicate boot_idle_polling_cleared<T>(task: T) -> bool;
predicate boot_idle_polling_rmb_before_sleep_check<T>(task: T) -> bool;
predicate boot_idle_polling_clear_mb_before_flush<T>(task: T) -> bool;
predicate boot_idle_nohz_entered<T>(runtime: T) -> bool;
predicate boot_idle_nohz_exited<T>(runtime: T) -> bool;
predicate boot_idle_nohz_run_idle_balance_done<T, U>(runtime: T, cpu: U) -> bool;
predicate boot_idle_local_irq_disabled_for_sleep<T, U>(
    runtime: T,
    local_interrupt: U
) -> bool;
predicate boot_idle_arch_cpu_idle_enter_done<T, U>(runtime: T, cpu: U) -> bool;
predicate boot_idle_arch_cpu_idle_exit_done<T, U>(runtime: T, cpu: U) -> bool;
predicate boot_idle_rcu_nocb_deferred_wakeup_flushed<T>(runtime: T) -> bool;
predicate boot_idle_cpu_offline_dead_path_not_taken<T, U>(runtime: T, cpu: U) -> bool;
predicate boot_idle_poll_or_cpuidle_path_deferred<T>(runtime: T) -> bool;
predicate boot_idle_preempt_need_resched_set<T>(task: T) -> bool;
predicate boot_idle_smp_call_function_queue_flushed<T>(runtime: T) -> bool;
predicate boot_idle_livepatch_state_update_deferred<T>(runtime: T) -> bool;
predicate boot_idle_wait_path_deferred<T>(runtime: T) -> bool;
predicate boot_idle_schedule_requested<T, U>(runtime: T, scheduler: U) -> bool;
predicate boot_idle_schedule_returned<T, U>(runtime: T, scheduler: U) -> bool;
predicate boot_idle_loop_continues<T>(runtime: T) -> bool;
predicate boot_idle_loop_cycle_committed<T>(runtime: T) -> bool;
predicate raw_spinlock_storage_bound<T>(lock: T) -> bool;
predicate raw_spinlock_initialized<T>(lock: T) -> bool;
predicate raw_spinlock_unlocked<T>(lock: T) -> bool;
predicate raw_spinlock_ready<T>(lock: T) -> bool;
predicate raw_spinlock_acquired<T>(lock: T) -> bool;
predicate raw_spinlock_released<T>(lock: T) -> bool;
predicate raw_spinlock_held<T>(lock: T) -> bool;
predicate raw_spinlock_irqsave_entered<T, U>(lock: T, current_cpu: U) -> bool;
predicate raw_spinlock_irqrestore_exited<T, U>(lock: T, current_cpu: U) -> bool;

predicate attrs_accessible<T: Object>(obj: T) -> bool {
    forall attr in obj.attrs {
        exists(attr);
        readable(attr);
    }
}

predicate page_size_min() -> Size;

predicate page_aligned(addr: Addr) -> bool {
    aligned(addr, page_size_min())
}

predicate valid_object_storage<T>(storage: ObjectStorage<T>) -> bool {
    exists(storage);
    stable(addr_of(storage));
    static_allocated(storage);
    size_of(storage) >= size_of::<T>();
    aligned(addr_of(storage), align_of::<T>());
}

predicate valid_page_table_storage(storage: PageTableStorage) -> bool {
    exists(storage);
    stable(addr_of(storage));
    static_allocated(storage);
    page_aligned(addr_of(storage));
    size_of(storage) >= page_size_min();
}

predicate valid_function_symbol<P>(func: FunctionSymbol<P>) -> bool {
    exists(func);
    stable(addr_of(func));
}

predicate valid_segment_set<T>(segments: SegmentSet<T>) -> bool {
    exists(segments);
    non_empty(segments);
    well_formed(segments);
}

predicate valid_dtb_magic(header: DtbHeader) -> bool {
    exists(header);
    header.magic == dtb::magic;
}

predicate valid_dtb_header(header: DtbHeader) -> bool {
    valid_dtb_magic(header);
    header.total_size >= size_of::<DtbHeader>();
}

predicate valid_fixmap_config(config: FixMapConfig) -> bool {
    exists(config);
    has_slot(config, FixMapSlot::Fdt);
    valid_fixmap_slot(config, FixMapSlot::Fdt);
}

predicate readonly<T: Object>(obj: T) -> bool {
    obj.access == Access::ReadOnly
}

predicate no_service<T: Object>(obj: T) -> bool {
    obj.state == State::Destroyed
}

predicate valid_phys_range_set<T>(ranges: PhysRangeSet<T>) -> bool {
    exists(ranges);
    non_empty(ranges);
    well_formed(ranges);
}

predicate disjoint<T, U>(left: PhysRangeSet<T>, right: PhysRangeSet<U>) -> bool {
    forall l in left, r in right {
        l.end <= r.start || r.end <= l.start;
    }
}

predicate fits_in_fixmap_slot<T, U>(range: PhysAddrRange<T>, slot: FixMapSlotRange<U>, page_size: Size) -> bool {
    page_cover_count(range, page_size) <= slot_page_count(slot)
}

predicate slot_contains<T, U>(slot: FixMapSlotRange<T>, obj: U) -> bool {
    contains(slot, obj)
}

predicate linear_map_area_reserved<T: Object>(obj: T) -> bool;

predicate fixmap_adjacent_to_linear_map<T: Object, U: Object>(fixmap: T, linear_map: U) -> bool {
    adjacent(fixmap, linear_map)
}

predicate fits_in_kernel_image_range<T: Object, U: VirtualAddressArea>(image: T, range: U) -> bool {
    contains(range, image)
}

predicate entry_head_text_layout_ready<T>(lds: T) -> bool {
    exists(lds.head_text_range);
    lds.head_text_range.start == lds.kernel_start;
    inside(lds.head_text_range.start, lds.head_text_range.end, lds.kernel_start, lds.kernel_end);
}

predicate pre_mmu_access_discipline_ready<T>(lds: T) -> bool {
    exists(lds.pre_mmu_text_range);
    contains(lds.head_text_range, lds.pre_mmu_text_range);
}

predicate trampoline_access_discipline_ready<T>(lds: T) -> bool {
    exists(lds.trampoline_safe_text_range);
    contains(lds.head_text_range, lds.trampoline_safe_text_range);
}

predicate per_cpu_static_image_layout_ready<T>(lds: T) -> bool {
    exists(lds.per_cpu_start);
    exists(lds.per_cpu_end);
    exists(lds.per_cpu_load);
    lds.per_cpu_end > lds.per_cpu_start;
    aligned(lds.per_cpu_start, page_size_min());
    inside(lds.per_cpu_start, lds.per_cpu_end, lds.kernel_start, lds.kernel_end);
    inside(lds.per_cpu_load, lds.per_cpu_load + (lds.per_cpu_end - lds.per_cpu_start), lds.kernel_start, lds.kernel_end);
}

predicate kernel_image_mapped_for_plain_data<T, U>(image: T, map: U) -> bool {
    contains(map, image)
}

predicate valid_trampoline_map<T: VirtualAddressArea>(map: T) -> bool {
    exists(map);
    aligned(map.phys_start, map.size);
    page_aligned(map.virt_start);
    map.size >= page_size_min();
}

predicate swapper_vm_translation_sync_complete<T, C>(swapper_vm: T, cpu: C) -> bool;
predicate trampoline_vm_translation_sync_complete<T, R>(trampoline_vm: T, cpu_ref: R) -> bool;
predicate early_vm_translation_sync_complete<T, R>(early_vm: T, cpu_ref: R) -> bool;

type ObjectStorage<T> {
    invariant {
        valid_object_storage(self);
    }
}

type PageTableStorage {
    invariant {
        valid_page_table_storage(self);
    }
}

type FunctionSymbol<P> {
    invariant {
        valid_function_symbol(self);
    }
}

type SegmentSet<T> {
    bss: T;

    invariant {
        valid_segment_set(self);
    }
}

type KernelImageSegment {
    range: AddrRange;
}

type TrampolineMap: VirtualAddressArea {
    phys_start: Derived<PhysAddr<KernelImage>, KernelImage.phys_start>;
    virt_start: Derived<VirtAddr<KernelImage>, Config.kernel_link_addr>;
    size: Derived<Size, Config.pmd_size>;

    invariant {
        valid_trampoline_map(self);
    }
}

type DtbHeader {
    magic: u32;
    total_size: Size;
}

type FixMapConfig {
    slots {
        fdt: FixMapSlotRange<Fdt>;
    }

    invariant {
        valid_fixmap_config(self);
    }
}

type CpuRef {
}

type TranslationControllerRef {
}

type BufferObject {
    processes {
        Action::PrepareDynamicLogBuffer {
            state_effect: StateEffect::None;
            ensures {
                printk_buffer_setup_prepared_dynamic_buffer(self);
            }
        }

        Action::SwitchActiveBufferAndCopyExistingRecords {
            state_effect: StateEffect::None;
            ensures {
                printk_buffer_setup_switched_active_buffer(self);
                printk_buffer_records_preserved(self);
                printk_buffer_setup_local_irq_save_restore_used(self);
            }
        }

        Action::CopyRemainingRecords {
            state_effect: StateEffect::None;
            ensures {
                printk_buffer_setup_copied_remaining_records(self);
            }
        }
    }
}

type PreemptionControl {
    ext_state: PreemptionExtState;

    processes {
        Transition::Disable {
            state_effect: StateEffect::Conditional;
            transitions {
                PreemptionExtState::Enabled -> PreemptionExtState::Disabled;
                PreemptionExtState::Disabled -> PreemptionExtState::Disabled;
            }
            ensures {
                task_preemption_disabled(self);
            }
            result {
                Enabled: Success(disabled);
                Disabled: Success(nested_disable);
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Conditional;
            transitions {
                PreemptionExtState::Disabled -> PreemptionExtState::Enabled;
                PreemptionExtState::Enabled -> PreemptionExtState::Enabled;
            }
            ensures {
                task_preemption_enabled(self);
            }
            result {
                Disabled: Success(enabled_or_nested_count_decremented);
                Enabled: Success(no_change);
            }
        }

        Transition::EnableNoResched {
            state_effect: StateEffect::Conditional;
            transitions {
                PreemptionExtState::Disabled -> PreemptionExtState::Enabled;
                PreemptionExtState::Enabled -> PreemptionExtState::Enabled;
            }
            ensures {
                task_preemption_enabled_no_resched(self);
                task_preemption_enabled(self);
            }
            result {
                Disabled: Success(enabled_no_resched);
                Enabled: Success(no_change);
            }
        }
    }
}

type RawSpinLock {
    ext_state: RawSpinLockExtState;

    lifecycle {
        Transition::Setup {
            state_effect: StateEffect::Always;
            ensures {
                raw_spinlock_storage_bound(self);
                raw_spinlock_initialized(self);
                raw_spinlock_unlocked(self);
                raw_spinlock_ready(self);
            }
        }
    }

    processes {
        Action::Acquire {
            state_effect: StateEffect::None;
            ensures {
                raw_spinlock_acquired(self);
                raw_spinlock_held(self);
            }
        }

        Action::Release {
            state_effect: StateEffect::None;
            ensures {
                raw_spinlock_released(self);
                raw_spinlock_unlocked(self);
            }
        }

        Transition::LockIrqSave {
            state_effect: StateEffect::Conditional;
            drives {
                CurrentCPU.trap.interrupt.Action::SaveAndDisable(out flags);
                CurrentTask.PreemptionControl.Transition::Disable;
                self.Action::Acquire;
            }
            transitions {
                RawSpinLockExtState::Unlocked -> RawSpinLockExtState::Locked;
                RawSpinLockExtState::Locked -> RawSpinLockExtState::Locked;
            }
            ensures {
                raw_spinlock_irqsave_entered(self, current_cpu);
                raw_spinlock_acquired(self);
            }
            result {
                Unlocked: Success(acquired);
                Locked: Blocked(contended);
            }
        }

        Transition::UnlockIrqRestore {
            state_effect: StateEffect::Conditional;
            drives {
                self.Action::Release;
                CurrentCPU.trap.interrupt.Action::Restore(flags);
                CurrentTask.PreemptionControl.Transition::Enable;
            }
            transitions {
                RawSpinLockExtState::Locked -> RawSpinLockExtState::Unlocked;
            }
            ensures {
                raw_spinlock_irqrestore_exited(self, current_cpu);
                raw_spinlock_released(self);
            }
        }
    }
}

/*
 * RcuReadSide models a CPU-local RCU read-side critical section. It is a
 * synchronization/context primitive, not a mutual-exclusion lock. The first
 * model slice records balanced enter/exit and the fact that protected pointer
 * publication or lookup happened under RCU read-side coverage; grace-period
 * accounting and quiescent-state machinery remain in RcuCore and later phases.
 */
type RcuReadSide {
    ext_state: RcuReadSideExtState;

    processes {
        Transition::ReadLock {
            state_effect: StateEffect::Conditional;
            transitions {
                RcuReadSideExtState::Quiescent -> RcuReadSideExtState::ReadHeld;
                RcuReadSideExtState::ReadHeld -> RcuReadSideExtState::ReadHeld;
            }
            ensures {
                rcu_read_side_entered(self, current_cpu);
            }
            result {
                Quiescent: Success(read_held);
                ReadHeld: Success(nested_read_held);
            }
        }

        Transition::ReadUnlock {
            state_effect: StateEffect::Conditional;
            transitions {
                RcuReadSideExtState::ReadHeld -> RcuReadSideExtState::Quiescent;
            }
            ensures {
                rcu_read_side_exited(self, current_cpu);
            }
        }
    }
}

/*
 * Mutex corresponds to Linux struct mutex. It is a blocking mutual-exclusion
 * primitive: one task owns it while locked, contending tasks may sleep on the
 * internal wait queue, recursive locking is forbidden, and unlock must be done
 * by the owning task. Linux also carries wait_lock, optional optimistic
 * spinning, handoff flags, ww_mutex and lockdep/debug fields; the first model
 * slice keeps those as internal/deferred details unless an instance later
 * needs them explicitly.
 */
type Mutex {
    init_kind: MutexInitKind;
    ext_state: MutexExtState;

    owned {
        wait_queue: SimpleWaitQueue;
    }

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            ensures {
                mutex_storage_bound(self);
                mutex_init_kind_recorded(self);
                mutex_preset_respects_init_kind(self);
                mutex_owns_wait_queue(self);
                mutex_wait_lock_internal_deferred(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            drives {
                self.wait_queue.Transition::Setup;
            }
            ensures {
                mutex_initialized(self);
                mutex_ready(self);
                mutex_unlocked(self);
                mutex_wait_queue_ready(self);
                mutex_recursive_locking_forbidden(self);
                mutex_unlock_requires_owner(self);
            }
        }
    }

    processes {
        Transition::Lock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                MutexExtState::Unlocked -> MutexExtState::Locked;
                MutexExtState::Locked -> MutexExtState::Locked;
            }
            ensures {
                mutex_lock_success_sets_owner(self, current_task);
                mutex_lock_acquired(self, current_task);
            }
            result {
                Unlocked: Success(acquired);
                Locked: Blocked(contended_or_sleeping);
            }
        }

        Transition::Unlock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
                mutex_owner_matches_unlocker(self, current_task);
            }
            drives {
                self.wait_queue.Action::WakeOne;
            }
            transitions {
                MutexExtState::Locked -> MutexExtState::Unlocked;
            }
            ensures {
                mutex_unlock_released(self, current_task);
                mutex_unlocked(self);
                mutex_wakes_one_waiter(self);
            }
        }

        Transition::Wait(current_task: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            drives {
                self.wait_queue.Action::PrepareWait;
                self.wait_queue.Action::FinishWait;
            }
            ensures {
                mutex_waiter_enqueued(self, current_task);
                mutex_waiter_finished(self, current_task);
                mutex_contention_may_sleep(self);
            }
        }
    }
}

/*
 * RwLock corresponds to Linux rwlock_t: a spin-based reader/writer lock.
 * Readers may share the lock with other readers, writers exclude both readers
 * and writers, and ordinary read_lock()/write_lock() do not save/restore IRQ
 * flags. Linux also has lockdep/debug owner state, PREEMPT_RT rwbase_rt
 * variants, irqsave/bh/nested APIs and architecture-specific raw_lock details;
 * this first model slice keeps those as internal/deferred unless an instance
 * needs a specific variant.
 */
type RwLock {
    init_kind: RwLockInitKind;
    ext_state: RwLockExtState;

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            ensures {
                rwlock_storage_bound(self);
                rwlock_init_kind_recorded(self);
                rwlock_preset_respects_init_kind(self);
                rwlock_arch_raw_lock_internal(self);
                rwlock_debug_lockdep_internal_deferred(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            ensures {
                rwlock_initialized(self);
                rwlock_ready(self);
                rwlock_unlocked(self);
                rwlock_no_readers(self);
                rwlock_no_writer(self);
                rwlock_readers_may_share(self);
                rwlock_write_side_exclusive(self);
                rwlock_ordinary_lock_does_not_save_irq_flags(self);
            }
        }
    }

    processes {
        Transition::ReadLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                RwLockExtState::Unlocked -> RwLockExtState::ReadHeld;
                RwLockExtState::ReadHeld -> RwLockExtState::ReadHeld;
                RwLockExtState::WriteHeld -> RwLockExtState::WriteHeld;
            }
            ensures {
                rwlock_read_lock_entered(self, current_task);
                rwlock_reader_acquired(self, current_task);
                rwlock_read_acquire_barrier_observed(self);
            }
            result {
                Unlocked: Success(read_acquired);
                ReadHeld: Success(read_shared);
                WriteHeld: Blocked(contended_on_writer);
            }
        }

        Transition::ReadUnlock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
                rwlock_reader_held(self, current_task);
            }
            transitions {
                RwLockExtState::ReadHeld -> RwLockExtState::Unlocked;
            }
            ensures {
                rwlock_read_unlock_exited(self, current_task);
                rwlock_reader_released(self, current_task);
                rwlock_read_release_barrier_observed(self);
            }
        }

        Transition::WriteLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                RwLockExtState::Unlocked -> RwLockExtState::WriteHeld;
                RwLockExtState::ReadHeld -> RwLockExtState::ReadHeld;
                RwLockExtState::WriteHeld -> RwLockExtState::WriteHeld;
            }
            ensures {
                rwlock_write_lock_entered(self, current_task);
                rwlock_writer_acquired(self, current_task);
                rwlock_write_acquire_barrier_observed(self);
            }
            result {
                Unlocked: Success(write_acquired);
                ReadHeld: Blocked(contended_on_readers);
                WriteHeld: Blocked(contended_on_writer);
            }
        }

        Transition::WriteUnlock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
                rwlock_writer_held(self, current_task);
            }
            transitions {
                RwLockExtState::WriteHeld -> RwLockExtState::Unlocked;
            }
            ensures {
                rwlock_write_unlock_exited(self, current_task);
                rwlock_writer_released(self, current_task);
                rwlock_unlocked(self);
                rwlock_write_release_barrier_observed(self);
            }
        }

        Transition::ReadTryLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                RwLockExtState::Unlocked -> RwLockExtState::ReadHeld;
                RwLockExtState::ReadHeld -> RwLockExtState::ReadHeld;
                RwLockExtState::WriteHeld -> RwLockExtState::WriteHeld;
            }
            ensures {
                rwlock_read_trylock_attempted(self, current_task);
            }
            result {
                Unlocked: Success(read_acquired);
                ReadHeld: Success(read_shared);
                WriteHeld: Blocked(read_trylock_failed);
            }
        }

        Transition::WriteTryLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                RwLockExtState::Unlocked -> RwLockExtState::WriteHeld;
                RwLockExtState::ReadHeld -> RwLockExtState::ReadHeld;
                RwLockExtState::WriteHeld -> RwLockExtState::WriteHeld;
            }
            ensures {
                rwlock_write_trylock_attempted(self, current_task);
            }
            result {
                Unlocked: Success(write_acquired);
                ReadHeld: Blocked(write_trylock_failed);
                WriteHeld: Blocked(write_trylock_failed);
            }
        }
    }
}

/*
 * RcuSync models Linux struct rcu_sync as a local support object. It is not
 * the global RcuCore; it only controls whether a containing primitive may use
 * a reader fast path or must route readers through the synchronized slow path.
 */
type RcuSync {
    ext_state: RcuSyncExtState;

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            ensures {
                rcu_sync_static_or_runtime_initialized(self);
                rcu_sync_idle(self);
                rcu_sync_reader_fast_path_available(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            ensures {
                rcu_sync_ready(self);
                rcu_sync_idle(self);
                rcu_sync_reader_fast_path_available(self);
            }
        }
    }

    processes {
        Transition::Enter {
            state_effect: StateEffect::Conditional;
            transitions {
                RcuSyncExtState::Idle -> RcuSyncExtState::WriterActive;
            }
            ensures {
                rcu_sync_entered(self);
                rcu_sync_reader_fast_path_blocked(self);
            }
        }

        Transition::Exit {
            state_effect: StateEffect::Conditional;
            transitions {
                RcuSyncExtState::WriterActive -> RcuSyncExtState::Idle;
            }
            ensures {
                rcu_sync_exited(self);
                rcu_sync_grace_period_completed(self);
                rcu_sync_reader_fast_path_available(self);
            }
        }
    }
}

/*
 * PerCpuRwSemaphore corresponds to Linux struct percpu_rw_semaphore. It uses
 * per-CPU reader counters for cheap read-side critical sections, an atomic
 * writer block flag to stop new readers, RcuSync to force readers through a
 * synchronized slow path while a writer is active, rcuwait for writer drain,
 * and a wait queue for contended readers/writers. Lockdep, tracing and exact
 * scheduler wait mechanics are internal/deferred; the observable read/write
 * protocol and counter/drain facts are part of this type.
 */
type PerCpuRwSemaphore {
    init_kind: PerCpuRwSemaphoreInitKind;
    ext_state: PerCpuRwSemaphoreExtState;

    owned {
        rcu_sync: RcuSync;
        wait_queue: SimpleWaitQueue;
    }

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            drives {
                self.rcu_sync.Transition::Preset;
            }
            ensures {
                percpu_rwsem_storage_bound(self);
                percpu_rwsem_init_kind_recorded(self);
                percpu_rwsem_percpu_read_counter_bound(self);
                percpu_rwsem_block_flag_clear(self);
                percpu_rwsem_writer_wait_ready(self);
                percpu_rwsem_wait_queue_storage_bound(self);
                percpu_rwsem_rcu_sync_bound(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            drives {
                self.rcu_sync.Transition::Setup;
                self.wait_queue.Transition::Setup;
            }
            ensures {
                percpu_rwsem_ready(self);
                percpu_rwsem_readers_fast(self);
                percpu_rwsem_percpu_read_count_zero(self);
                percpu_rwsem_wait_queue_ready(self);
                percpu_rwsem_writer_inactive(self);
                percpu_rwsem_reader_fast_path_available(self);
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Always;
            ensures {
                percpu_rwsem_online(self);
                percpu_rwsem_all_possible_cpu_counters_addressable(self);
            }
        }
    }

    processes {
        Transition::ReadLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                PerCpuRwSemaphoreExtState::ReadersFast -> PerCpuRwSemaphoreExtState::ReadersFast;
                PerCpuRwSemaphoreExtState::WriterBlocked -> PerCpuRwSemaphoreExtState::WriterBlocked;
                PerCpuRwSemaphoreExtState::WriterActive -> PerCpuRwSemaphoreExtState::WriterActive;
            }
            ensures {
                percpu_rwsem_read_lock_entered(self, current_task);
                percpu_rwsem_preemption_disabled_for_counter_update(self);
                percpu_rwsem_current_cpu_read_count_incremented(self);
                percpu_rwsem_read_acquire_barrier_observed(self);
            }
            result {
                ReadersFast: Success(read_acquired_fast);
                WriterBlocked: Blocked(read_waiting_for_writer);
                WriterActive: Blocked(read_waiting_for_writer);
            }
        }

        Transition::ReadTryLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            transitions {
                PerCpuRwSemaphoreExtState::ReadersFast -> PerCpuRwSemaphoreExtState::ReadersFast;
                PerCpuRwSemaphoreExtState::WriterBlocked -> PerCpuRwSemaphoreExtState::WriterBlocked;
                PerCpuRwSemaphoreExtState::WriterActive -> PerCpuRwSemaphoreExtState::WriterActive;
            }
            ensures {
                percpu_rwsem_read_trylock_attempted(self, current_task);
                percpu_rwsem_read_trylock_has_unconditional_barrier(self);
            }
            result {
                ReadersFast: Success(read_acquired_fast);
                WriterBlocked: Blocked(read_trylock_failed);
                WriterActive: Blocked(read_trylock_failed);
            }
        }

        Transition::ReadUnlock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
                percpu_rwsem_reader_held_on_current_cpu(self, current_task);
            }
            transitions {
                PerCpuRwSemaphoreExtState::ReadersFast -> PerCpuRwSemaphoreExtState::ReadersFast;
                PerCpuRwSemaphoreExtState::WriterBlocked -> PerCpuRwSemaphoreExtState::WriterBlocked;
                PerCpuRwSemaphoreExtState::WriterActive -> PerCpuRwSemaphoreExtState::WriterActive;
            }
            ensures {
                percpu_rwsem_read_unlock_exited(self, current_task);
                percpu_rwsem_current_cpu_read_count_decremented(self);
                percpu_rwsem_read_release_barrier_observed(self);
                percpu_rwsem_writer_wake_may_be_signaled(self);
            }
        }

        Transition::WriteLock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
            }
            drives {
                self.rcu_sync.Transition::Enter;
            }
            transitions {
                PerCpuRwSemaphoreExtState::ReadersFast -> PerCpuRwSemaphoreExtState::WriterActive;
                PerCpuRwSemaphoreExtState::WriterBlocked -> PerCpuRwSemaphoreExtState::WriterBlocked;
                PerCpuRwSemaphoreExtState::WriterActive -> PerCpuRwSemaphoreExtState::WriterActive;
            }
            ensures {
                percpu_rwsem_write_lock_entered(self, current_task);
                percpu_rwsem_block_flag_set(self);
                percpu_rwsem_new_readers_blocked(self);
                percpu_rwsem_existing_readers_drained(self);
                percpu_rwsem_write_acquire_barrier_observed(self);
            }
            result {
                ReadersFast: Success(write_acquired);
                WriterBlocked: Blocked(write_waiting);
                WriterActive: Blocked(write_waiting);
            }
        }

        Transition::WriteUnlock(current_task: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Ready;
                task_ref_ready(current_task);
                percpu_rwsem_writer_held(self, current_task);
            }
            drives {
                self.wait_queue.Action::WakeOne;
                self.rcu_sync.Transition::Exit;
            }
            transitions {
                PerCpuRwSemaphoreExtState::WriterActive -> PerCpuRwSemaphoreExtState::ReadersFast;
            }
            ensures {
                percpu_rwsem_write_unlock_exited(self, current_task);
                percpu_rwsem_block_flag_clear(self);
                percpu_rwsem_waiters_woken(self);
                percpu_rwsem_reader_fast_path_available(self);
                percpu_rwsem_write_release_barrier_observed(self);
            }
        }
    }
}

type CompletionTokenCount {
}

type SimpleWaitQueue {
    lifecycle {
        Transition::Setup {
            state_effect: StateEffect::Always;
            ensures {
                wait_queue_ready(self);
            }
        }
    }

    processes {
        Action::WakeOne {
            state_effect: StateEffect::None;
            depends_on {
                wait_queue_ready(self);
            }
            ensures {
                wait_queue_wake_one_committed(self);
            }
        }

        Action::WakeAll {
            state_effect: StateEffect::None;
            depends_on {
                wait_queue_ready(self);
            }
            ensures {
                wait_queue_wake_all_committed(self);
            }
        }

        Action::PrepareWait {
            state_effect: StateEffect::None;
            depends_on {
                wait_queue_ready(self);
            }
            ensures {
                wait_queue_waiter_enqueued(self);
            }
        }

        Action::FinishWait {
            state_effect: StateEffect::None;
            depends_on {
                wait_queue_ready(self);
            }
            ensures {
                wait_queue_waiter_finished(self);
            }
        }
    }
}

/*
 * Completion corresponds to Linux struct completion. It is a reusable Type
 * with ordinary lifecycle state plus an extended completion state. The
 * extended state is not a lifecycle state: complete/wait/reinit style
 * processes operate while the instance remains Online.
 */
type Completion {
    ext_state: CompletionExtState;
    done: CompletionTokenCount;

    owned {
        wait_queue: SimpleWaitQueue;
    }

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            ensures {
                completion_storage_bound(self);
                completion_owns_wait_queue(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            drives {
                self.wait_queue.Transition::Setup;
            }
            ensures {
                completion_ready(self);
                completion_pending(self);
                completion_done_count_is_zero(self);
                completion_wait_queue_ready(self);
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Always;
            ensures {
                completion_online(self);
                completion_handle_published(self);
            }
        }

        Transition::Disable {
            state_effect: StateEffect::Conditional;
            ensures {
                completion_handle_revoked(self);
            }
        }
    }

    processes {
        Transition::Complete {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Online;
            }
            drives {
                self.wait_queue.Action::WakeOne;
            }
            transitions {
                CompletionExtState::Pending -> CompletionExtState::Completed;
                CompletionExtState::Completed -> CompletionExtState::Completed;
                CompletionExtState::CompletedAll -> CompletionExtState::CompletedAll;
            }
            ensures {
                completion_complete_committed(self);
                completion_token_available(self);
                completion_wakes_one_waiter(self);
                completion_done_increment_guarded_by_wait_lock(self);
                completion_wake_guarded_by_wait_lock(self);
            }
            result {
                Pending: Success(token_produced);
                Completed: Success(token_produced);
                CompletedAll: Success(no_change);
            }
        }

        Transition::CompleteAll {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Online;
            }
            drives {
                self.wait_queue.Action::WakeAll;
            }
            transitions {
                CompletionExtState::Pending -> CompletionExtState::CompletedAll;
                CompletionExtState::Completed -> CompletionExtState::CompletedAll;
                CompletionExtState::CompletedAll -> CompletionExtState::CompletedAll;
            }
            ensures {
                completion_complete_all_committed(self);
                completion_all_waiters_released(self);
            }
            result {
                Pending: Success(all_tokens_published);
                Completed: Success(all_tokens_published);
                CompletedAll: Success(no_change);
            }
        }

        Transition::Wait {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Online;
            }
            drives {
                self.wait_queue.Action::PrepareWait;
                self.wait_queue.Action::FinishWait;
            }
            transitions {
                CompletionExtState::Pending -> CompletionExtState::Pending;
                CompletionExtState::Completed -> CompletionExtState::Pending;
                CompletionExtState::CompletedAll -> CompletionExtState::CompletedAll;
            }
            ensures {
                completion_waiter_enqueued(self);
                completion_waiter_finished(self);
            }
            result {
                Pending: Blocked(waiting_or_timeout_or_signal);
                Completed: Success(token_consumed);
                CompletedAll: Success(no_token_consumed);
            }
        }

        Transition::TryWait {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Online;
            }
            transitions {
                CompletionExtState::Pending -> CompletionExtState::Pending;
                CompletionExtState::Completed -> CompletionExtState::Pending;
                CompletionExtState::CompletedAll -> CompletionExtState::CompletedAll;
            }
            result {
                Pending: Failed(no_token);
                Completed: Success(token_consumed);
                CompletedAll: Success(no_token_consumed);
            }
        }

        Transition::Reinit {
            state_effect: StateEffect::Conditional;
            depends_on {
                self.state == State::Online;
            }
            transitions {
                CompletionExtState::Pending -> CompletionExtState::Pending;
                CompletionExtState::Completed -> CompletionExtState::Pending;
                CompletionExtState::CompletedAll -> CompletionExtState::Pending;
            }
            ensures {
                completion_pending(self);
                completion_done_count_is_zero(self);
                completion_wait_queue_preserved(self);
            }
            result {
                Pending: Success(no_change);
                Completed: Success(reset_to_pending);
                CompletedAll: Success(reset_to_pending);
            }
        }

        Action::Done {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
            }
            ensures {
                completion_done_observed(self);
            }
        }
    }
}
