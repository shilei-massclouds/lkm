/*
 * Entry Prelude Object Model Specification
 *
 * This file is extracted from spec/charter/main.md.
 * It is intended to be parsed by verifier/modeling tools.
 * Rust/C style comments are for human readers and should be ignored by tools.
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

enum TaskRuntimeState {
    New,
    Running,
}

enum TaskEntry {
    None,
    KernelInit,
    Kthreadd,
}

enum RunQueueRuntimeState {
    None,
    Some,
}

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
predicate primary_hart_sie_clear_at_kernel_entry() -> bool;
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
predicate static_branch_cpu_hotplug_read_guard_used<T>(static_branch: T) -> bool;
predicate static_branch_jump_label_mutex_guard_used<T>(static_branch: T) -> bool;
predicate static_branch_text_patch_sync_deferred<T>(static_branch: T) -> bool;
predicate riscv_early_boot_alternatives_deferred<T>(vm: T) -> bool;
predicate riscv_early_boot_alternatives_mmu_off_boundary_preserved<T>(vm: T) -> bool;
predicate vmlinux_build_id_deferred<T>(phase: T) -> bool;
predicate page_address_init_deferred<T>(phase: T) -> bool;
predicate entry_successor_start_kernel_position_preserved<T>(phase: T) -> bool;
predicate memblock_phys_ram_base_ready<T>(memblock: T) -> bool;
predicate memblock_kernel_va_pa_offset_ready<T, U>(memblock: T, vm: U) -> bool;
predicate memblock_dma32_limit_ready<T>(memblock: T) -> bool;
predicate memblock_dma32_zone_input_ready<T>(memblock: T) -> bool;
predicate memblock_hugetlb_early_reserve_deferred<T>(memblock: T) -> bool;
predicate swapper_vm_strict_kernel_rwx_boundary_deferred<T>(swapper_vm: T) -> bool;
predicate swapper_vm_final_permissions_not_split_yet<T>(swapper_vm: T) -> bool;
predicate cpu_hotplug_ap_sync_state_online<T, U>(hotplug_state: T, cpu: U) -> bool;
predicate cpu_hotplug_read_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate cpu_hotplug_write_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate cpu_add_remove_mutex_guard_used<T, U>(subject: T, mutex: U) -> bool;
predicate smpboot_threads_mutex_guard_used<T, U>(subject: T, mutex: U) -> bool;
predicate cpu_running_wait_lock_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate done_up_wait_lock_guard_used<T, U>(subject: T, lock: U) -> bool;
predicate sbi_boot_data_publish_barriers_observed<T>(provider: T) -> bool;
predicate ap_local_irq_enable_summary_deferred<T>(ack: T) -> bool;
predicate ap_cache_tlb_flush_summary_observed<T>(ack: T) -> bool;
predicate ap_ipi_enable_observed<T>(ack: T) -> bool;
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
predicate current_cpu_self_identity_ready<T>(current_cpu: T) -> bool;
predicate current_cpu_hartid_ready<T>(current_cpu: T, hartid: HartId) -> bool;
predicate current_cpu_logical_id_ready<T, U>(current_cpu: T, logical_id: U) -> bool;
predicate current_cpu_owns_cpu<T, U>(current_cpu: T, cpu: U) -> bool;
predicate current_cpu_registered_in_cpu_group<T, U>(current_cpu: T, cpu_group: U) -> bool;
predicate current_cpu_bootstrap_role_ready<T, U>(current_cpu: T, cpu: U) -> bool;
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
predicate cpu_event_stream_ready<T, U>(cpu: T, event_stream: U) -> bool;
predicate cpu_exception_stream_ready<T, U>(cpu: T, exception_stream: U) -> bool;
predicate cpu_interrupt_stream_ready<T, U>(cpu: T, interrupt_stream: U) -> bool;
predicate cpu_local_interrupt_control_ready<T, U>(control: T, cpu: U) -> bool;
predicate cpu_local_interrupts_disabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_enabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_saved_and_disabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_restored<T>(control: T) -> bool;
predicate zonelist_update_seq_write_irqsave_entered<T, I>(update_seq: T, local_interrupt: I) -> bool;
predicate zonelist_update_seq_write_irqrestore_exited<T, I>(update_seq: T, local_interrupt: I) -> bool;
predicate printk_deferred_section_entered<T>(section: T) -> bool;
predicate printk_deferred_section_exited<T>(section: T) -> bool;
predicate current_task_slot_ready<T, U>(slot: T, cpu: U) -> bool;
predicate current_task_slot_current<T, U>(slot: T, task: U) -> bool;
predicate current_task_ref_private_to_cpu<T, U>(task_ref: T, current_cpu: U) -> bool;
predicate current_task_ref_targets_cpu_task<T, U, V>(task_ref: T, current_cpu: U, task: V) -> bool;
predicate current_task_ref_from_cpu_view<T, U, V>(task_ref: T, current_cpu: U, task: V) -> bool;
predicate task_ref_loaded_into_current_cpu<T, U>(task_ref: T, current_cpu: U) -> bool;
predicate current_task_ref_updated_by_switch<T, U, V>(current_cpu: T, prev_ref: U, next_ref: V) -> bool;
predicate task_creation_entry_contract_ready<T>(core: T) -> bool;
predicate task_clone_args_ready<T>(task: T) -> bool;
predicate task_creation_copy_process_committed<T, U, V>(core: T, src_task: U, dst_task: V) -> bool;
predicate task_creation_used_clone_args<T, U>(core: T, task: U) -> bool;
predicate task_creation_bound_entry<T, U>(core: T, task: U, entry: TaskEntry) -> bool;
predicate task_creation_copy_process_sighand_siglock_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_tasklist_lock_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_pidmap_lock_deferred<T>(core: T) -> bool;
predicate task_creation_copy_process_sched_fork_locks_deferred<T>(core: T) -> bool;
predicate task_struct_allocated<T>(task: T) -> bool;
predicate task_duplicated_from<T, U>(dst_task: T, src_task: U) -> bool;
predicate task_pid_allocated<T, U>(task: T, pid_ns: U) -> bool;
predicate task_creds_copied<T, U>(task: T, creds: U) -> bool;
predicate task_file_context_copied_or_shared<T, U>(task: T, files: U) -> bool;
predicate task_signal_context_ready<T, U>(task: T, signal: U) -> bool;
predicate task_security_context_allocated<T, U>(task: T, security: U) -> bool;
predicate task_thread_context_ready<T>(task: T) -> bool;
predicate task_sched_entity_initialized<T, U>(task: T, scheduler: U) -> bool;
predicate task_entry_bound<T>(task: T, entry: TaskEntry) -> bool;
predicate task_entry_first_phase<T, U>(task: T, phase: U) -> bool;
predicate kernel_init_entry_reaches_smp_runtime<T, U>(task: T, phase: U) -> bool;
predicate kernel_init_still_waiting_for_kthreadd_done<T>(task: T) -> bool;
predicate kernel_init_kthreadd_done_wait_ready<T, U, V>(wait: T, task: U, gate: V) -> bool;
predicate kernel_init_kthreadd_done_wait_released<T, U, V>(wait: T, task: U, gate: V) -> bool;
predicate kernel_init_observed_kthreadd_done_release<T, U>(task: T, gate: U) -> bool;
predicate kernel_init_released_for_pre_smp_init<T>(task: T) -> bool;
predicate kthreadd_entry_reaches_schedule_loop<T, U>(task: T, scheduler: U) -> bool;
predicate kthreadd_schedule_loop_ready<T, U>(task: T, scheduler: U) -> bool;
predicate kthreadd_schedule_loop_schedule_boundary_deferred<T, U>(task: T, scheduler: U) -> bool;
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
predicate task_enqueued_on_runqueue<T, U>(task: T, runqueue: U) -> bool;
predicate task_cpu_ref_is<T, U>(task: T, cpu_ref: U) -> bool;
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
predicate runqueue_ref_targets<T, U>(runqueue_ref: T, runqueue: U) -> bool;
predicate runqueue_ref_ready<T>(runqueue_ref: T) -> bool;
predicate runqueue_ref_cpu_is<T, U>(runqueue_ref: T, cpu_ref: U) -> bool;
predicate current_runqueue_ref_private_to_cpu<T, U>(runqueue_ref: T, current_cpu: U) -> bool;
predicate current_runqueue_ref_from_current_task<T, U, V, W, X>(runqueue_ref: T, current_cpu: U, current_task_ref: V, task: W, cpu_ref: X) -> bool;
predicate runqueue_pick_next_task_returns<T, U, V>(runqueue_ref: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_select_runqueue_returns<T, U, V>(scheduler: T, task_ref: U, runqueue_ref: V) -> bool;
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
predicate task_runqueue_selected<T, U, V>(scheduler: T, task: U, runqueue: V) -> bool;
predicate task_wakeup_new_rq_clock_updated<T, U>(task: T, runqueue: U) -> bool;
predicate task_wakeup_new_initial_util_avg_posted<T, U>(task: T, runqueue: U) -> bool;
predicate task_wakeup_new_trace_emitted<T>(task: T) -> bool;
predicate task_wakeup_new_preempt_check_done<T, U>(task: T, runqueue: U) -> bool;
predicate task_wakeup_new_task_woken_hook_deferred<T>(task: T) -> bool;
predicate scheduler_payload_cooperative_switch_ready<T>(scheduler: T) -> bool;
predicate scheduler_payload_schedule_from_kernel_init<T, U>(scheduler: T, current_ref: U) -> bool;
predicate scheduler_payload_smoke_task_enqueued<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_payload_smoke_task_entry_executed<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_payload_smoke_task_yielded_back<T, U>(scheduler: T, task_ref: U) -> bool;
predicate runqueue_runtime_state_is<T>(runqueue: T, state: RunQueueRuntimeState) -> bool;
predicate runqueue_task_refs_empty<T>(runqueue: T) -> bool;
predicate runqueue_task_refs_some<T>(runqueue: T) -> bool;
predicate runqueue_contains_task<T, U>(runqueue: T, task_ref: U) -> bool;
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
predicate boot_idle_task_identity_entered<T, U>(boot_init_task: T, boot_idle_task: U) -> bool;
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

predicate linear_map_area_reserved<T: Object>(obj: T) -> bool {
    obj.state == State::Destroyed
}

predicate fixmap_adjacent_to_linear_map<T: Object, U: Object>(fixmap: T, linear_map: U) -> bool {
    adjacent(fixmap, linear_map)
}

predicate fits_in_kernel_image_map<T: Object, U: VirtualAddressArea>(image: T, map: U) -> bool {
    contains(map, image)
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

predicate swapper_vm_translation_sync_complete<T>(swapper_vm: T) -> bool;
predicate trampoline_vm_translation_sync_ready_before_satp<T>(trampoline_vm: T) -> bool;
predicate early_vm_translation_sync_complete<T>(early_vm: T) -> bool;

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

type KernelImageMap: VirtualAddressArea {
    range: Derived<VirtAddrRange<KernelImage>, range(Config.kernel_link_addr, Config.kernel_link_addr + Config.kernel_image_va_window_size)>;
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

type TaskObject {
}

type TaskRef {
    processes {
        Action::SetCurrent(task: Task) {
            state_effect: StateEffect::None;
            ensures {
                task_ref_targets(self, task);
            }
        }
    }
}

type RunQueueRef {
}

type CpuRef {
}

type TaskRefSet {
}

type RegisterValue {
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

/*
 * TaskThreadContext is the architecture-specific core switch context owned by
 * every Task. On RISC-V this corresponds to Linux task_struct.thread fields
 * saved/restored by arch/riscv/kernel/entry.S::__switch_to:
 * ra, sp and callee-saved s0..s11. Floating-point/vector state, prev_cpu,
 * icache flush policy and memory-context switching are intentionally outside
 * this minimum core context and are modeled later as separate task subobjects
 * or scheduler hooks.
 */
type TaskThreadContext {
    ra: RegisterValue;
    sp: RegisterValue;
    s0: RegisterValue;
    s1: RegisterValue;
    s2: RegisterValue;
    s3: RegisterValue;
    s4: RegisterValue;
    s5: RegisterValue;
    s6: RegisterValue;
    s7: RegisterValue;
    s8: RegisterValue;
    s9: RegisterValue;
    s10: RegisterValue;
    s11: RegisterValue;
}

/*
 * Task is a reusable runtime task type. TaskRuntimeState is an extended
 * runtime state rather than an object lifecycle state. The current formal
 * model keeps SetRuntimeState as a simple operational event; later state
 * machine support will add transition guards and enter-state consistency
 * checks for concrete runtime states.
 */
type Task: TaskObject {
    ext_state: TaskRuntimeState;
    cpu_ref: CpuRef;

    owned {
        thread_context: TaskThreadContext;
    }

    processes {
        Transition::SetRuntimeState(state: TaskRuntimeState) {
            state_effect: StateEffect::Conditional;
            depends_on {
                task_runtime_state_transition_allowed(self, state);
            }
            transitions {
                TaskRuntimeState::New -> TaskRuntimeState::Running;
                TaskRuntimeState::Running -> TaskRuntimeState::Running;
            }
            ensures {
                task_runtime_state_is(self, state);
            }
            result {
                Allowed: Success(runtime_state_set);
                Disallowed: Failed(invalid_runtime_state_transition);
            }
        }

        Action::PinToBootCpu(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                cpu_ref_ready(cpu_ref);
            }
            ensures {
                task_flag_no_setaffinity(self);
                task_cpumask_is(self, cpu_ref);
            }
        }

        Action::SetTaskCpu(cpu_ref: CpuRef) {
            state_effect: StateEffect::None;
            depends_on {
                cpu_ref_ready(cpu_ref);
            }
            ensures {
                task_cpu_ref_is(self, cpu_ref);
            }
        }

        Action::SaveCoreContext {
            state_effect: StateEffect::None;
            depends_on {
                task_thread_context_owned(self, self.thread_context);
                task_thread_context_core_register_set(self.thread_context);
            }
            ensures {
                task_thread_context_core_saved(self.thread_context);
            }
        }

        Action::RestoreCoreContext {
            state_effect: StateEffect::None;
            depends_on {
                task_thread_context_owned(self, self.thread_context);
                task_thread_context_core_register_set(self.thread_context);
            }
            ensures {
                task_thread_context_core_restored(self.thread_context);
            }
        }
    }
}

/*
 * SchedulerObject is the reusable scheduler service type. Schedule models the
 * minimal schedule()/__schedule() path: derive the prev task ref from the
 * current CPU current-task view, read that task's recorded CPU ownership, resolve
 * that CPU view's CurrentRunQueueRef through CpuGroup.Cpu[id].RunQueue, ask the
 * current runqueue to pick next, then switch from prev to next.
 * ScheduleIdle models Linux schedule_idle(): it is only reachable from the
 * CPU-local idle loop after this CPU's idle task observes need_resched, and it
 * returns to that same idle-loop point after the scheduler drains the resched
 * request. The current model reuses Schedule for the shared switch skeleton and
 * records the idle-specific facts separately.
 * Payload smoke may use the same Schedule boundary after startup from a
 * non-idle current task. That path is modeled as a minimal cooperative switch
 * loop: KernelInitTask calls Schedule after enqueueing a smoke scheduler task,
 * switch_to enters that task's entry on a real task stack, the smoke task
 * records execution and calls Schedule/Yield, and the CPU returns to the
 * KernelInitTask continuation. This does not weaken the rest_init first
 * schedule facts below; it is an additional payload-phase schedule use.
 * CurrentTaskRef and CurrentRunQueueRef are private to the current CPU view;
 * the model does not introduce descriptive current-task/current-runqueue
 * objects or global current-task/current-runqueue singletons. SelectRunQueue is a pure
 * wake-up selection action: it consumes a TaskRef and returns a RunQueueRef.
 * Linux updates the task's recorded CPU after select_task_rq() and before
 * enqueue; callers therefore drive the target Task.Action::SetTaskCpu(...)
 * between SelectRunQueue and EnqueueTask. The current UP rest_init path proves
 * selected_rq's CPU fact as BootCPURef, but schedule's CurrentRunQueueRef must be
 * derived from CurrentTaskRef -> task_cpu_ref_is(...) -> CpuGroup/runqueue
 * topology, not from CpuGroup.boot_cpu() as a primary source. RunQueueRef is
 * the selected/wakeup runqueue reference type; CurrentRunQueueRef remains a
 * private current-CPU reference and must not be reused as SelectRunQueue's
 * result or as the generic EnqueueTask receiver.
 */
type SchedulerObject: TaskObject {
    processes {
        Action::Schedule {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_schedule_event_available(self);
            }
            within SchedulePreemptionContext {
                within ScheduleLocalInterruptContext {
                    within ScheduleRunQueueContext {
                        depends_on {
                            task_ref_targets(CurrentTaskRef, BootIdleTask);
                            boot_idle_task_ready(BootIdleTask, BootInitTask, BootRunQueue);
                            task_ref_targets(CurrentTaskRef, BootIdleTask);
                            task_ref_ready(CurrentTaskRef);
                            current_task_ref_private_to_cpu(CurrentTaskRef, BootCurrentCPU);
                            current_task_ref_targets_cpu_task(CurrentTaskRef, BootCurrentCPU, BootIdleTask);
                            current_task_ref_from_cpu_view(CurrentTaskRef, BootCurrentCPU, BootIdleTask);
                            task_cpu_ref_is(BootIdleTask, BootCPURef);
                            runqueue_ref_ready(CurrentRunQueueRef);
                            runqueue_ref_targets(CurrentRunQueueRef, BootRunQueue);
                            runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
                            current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
                            current_runqueue_ref_from_current_task(CurrentRunQueueRef, BootCurrentCPU, CurrentTaskRef, BootIdleTask, BootCPURef);
                        }

                        drives {
                            let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef);
                            self.Action::SwitchTo(CurrentTaskRef, next);
                        }

                        ensures {
                            scheduler_schedule_local_interrupts_closed(self, BootCpuLocalInterrupt);
                            scheduler_runqueue_lock_held_for_schedule(self, BootRunQueue);
                            scheduler_rcu_context_switch_noted(self, CurrentTaskRef, KernelInitTaskRef);
                            scheduler_rq_lock_mb_after_spinlock(self, BootRunQueue);
                            scheduler_rq_clock_updated_for_schedule(self, BootRunQueue);
                            scheduler_pick_next_task_selects_runnable(self, BootRunQueue, KernelInitTaskRef);
                            runqueue_pick_next_task_returns(CurrentRunQueueRef, CurrentTaskRef, KernelInitTaskRef);
                            scheduler_need_resched_cleared(self, CurrentTaskRef);
                            scheduler_rq_curr_published_rcu(self, BootRunQueue, KernelInitTaskRef);
                            scheduler_trace_sched_switch_emitted(self, CurrentTaskRef, KernelInitTaskRef);
                            scheduler_switch_to_committed(self, CurrentTaskRef, KernelInitTaskRef);
                            scheduler_switch_to_core_context_saved(self, CurrentTaskRef);
                            scheduler_switch_to_core_context_restored(self, KernelInitTaskRef);
                            scheduler_prepare_task_switch_done(
                                self,
                                BootRunQueue,
                                CurrentTaskRef,
                                KernelInitTaskRef
                            );
                            scheduler_finish_task_switch_done(
                                self,
                                BootRunQueue,
                                CurrentTaskRef
                            );
                            scheduler_finish_task_switch_releases_rq_lock(self, BootRunQueue);
                            scheduler_finish_task_switch_restores_preempt_count(
                                self,
                                KernelInitTaskRef
                            );
                            task_ref_targets(KernelInitTaskRef, KernelInitTask);
                            task_ref_loaded_into_current_cpu(KernelInitTaskRef, BootCurrentCPU);
                            current_task_ref_updated_by_switch(BootCurrentCPU, CurrentTaskRef, KernelInitTaskRef);
                            scheduler_first_schedule_committed(self);
                        }
                    }
                }
            }
            ensures {
                scheduler_schedule_local_interrupts_closed(self, BootCpuLocalInterrupt);
                scheduler_runqueue_lock_held_for_schedule(self, BootRunQueue);
                scheduler_schedule_exit_restores_local_interrupts(self, BootCpuLocalInterrupt);
                scheduler_rcu_context_switch_noted(self, CurrentTaskRef, KernelInitTaskRef);
                scheduler_rq_lock_mb_after_spinlock(self, BootRunQueue);
                scheduler_rq_clock_updated_for_schedule(self, BootRunQueue);
                scheduler_pick_next_task_selects_runnable(self, BootRunQueue, KernelInitTaskRef);
                runqueue_pick_next_task_returns(CurrentRunQueueRef, CurrentTaskRef, KernelInitTaskRef);
                scheduler_need_resched_cleared(self, CurrentTaskRef);
                scheduler_rq_curr_published_rcu(self, BootRunQueue, KernelInitTaskRef);
                scheduler_trace_sched_switch_emitted(self, CurrentTaskRef, KernelInitTaskRef);
                scheduler_switch_to_committed(self, CurrentTaskRef, KernelInitTaskRef);
                scheduler_switch_to_core_context_saved(self, CurrentTaskRef);
                scheduler_switch_to_core_context_restored(self, KernelInitTaskRef);
                scheduler_prepare_task_switch_done(
                    self,
                    BootRunQueue,
                    CurrentTaskRef,
                    KernelInitTaskRef
                );
                scheduler_finish_task_switch_done(self, BootRunQueue, CurrentTaskRef);
                scheduler_finish_task_switch_releases_rq_lock(self, BootRunQueue);
                scheduler_finish_task_switch_restores_preempt_count(self, KernelInitTaskRef);
                scheduler_switch_mm_or_lazy_tlb_deferred(self);
                scheduler_membarrier_switch_barrier_deferred(self);
                task_ref_targets(KernelInitTaskRef, KernelInitTask);
                task_ref_loaded_into_current_cpu(KernelInitTaskRef, BootCurrentCPU);
                current_task_ref_updated_by_switch(BootCurrentCPU, CurrentTaskRef, KernelInitTaskRef);
                scheduler_first_schedule_committed(self);
            }
            deferred {
                "Payload-phase Scheduler.schedule() cooperative switch branch: from KernelInitTask current, pick a smoke scheduler task, enter its stack/entry, let that task schedule/yield back, and return to KernelInitTask continuation. This branch is required for the app-smoke API test and is distinct from rest_init first-schedule checkpoints; full preemptive scheduling remains deferred.";
            }
        }

        Action::ScheduleIdle {
            state_effect: StateEffect::None;
            depends_on {
                scheduler_schedule_event_available(self);
                task_ref_ready(CurrentTaskRef);
                task_ref_targets(CurrentTaskRef, BootIdleTask);
                boot_idle_need_resched_set_for_schedule(BootIdleTask);
            }
            drives {
                self.Action::Schedule;
            }
            ensures {
                scheduler_idle_mode_used(self);
                scheduler_idle_schedule_committed(self, CurrentTaskRef);
                scheduler_idle_schedule_committed_to_runnable(self, KernelInitTaskRef);
                boot_idle_schedule_idle_loop_until_resched_clear(self);
                boot_idle_need_resched_drained_after_schedule(BootIdleTask);
                task_ref_loaded_into_current_cpu(KernelInitTaskRef, BootCurrentCPU);
            }
            deferred {
                "ScheduleIdle 当前复用 Scheduler.Schedule 的对象级 pick-next/switch-to 框架，不能手工提交 BootIdleTask -> BootIdleTask identity switch；Linux schedule_idle() 的 SM_IDLE 模式、跳过 sched_submit_work()、do { __schedule(SM_IDLE); } while (need_resched()) 循环，以及未来某刻返回 idle-loop continuation 的完整控制流后续展开。";
            }
        }

        Action::SwitchTo(prev_ref: TaskRef, next_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(prev_ref);
                task_ref_ready(next_ref);
                scheduler_switch_to_prepared(self, BootRunQueue, prev_ref, next_ref);
            }
            drives {
                prev_ref.Action::SaveCoreContext;
                next_ref.Action::RestoreCoreContext;
                CurrentTaskRef.Action::SetCurrent(task: next_ref);
            }
            ensures {
                scheduler_prepare_task_switch_done(self, BootRunQueue, prev_ref, next_ref);
                scheduler_switch_to_committed(self, prev_ref, next_ref);
                scheduler_switch_to_core_context_saved(self, prev_ref);
                scheduler_switch_to_core_context_restored(self, next_ref);
                scheduler_finish_task_switch_done(self, BootRunQueue, prev_ref);
                scheduler_finish_task_switch_releases_rq_lock(self, BootRunQueue);
                scheduler_finish_task_switch_restores_preempt_count(self, next_ref);
                scheduler_switch_mm_or_lazy_tlb_deferred(self);
                scheduler_membarrier_switch_barrier_deferred(self);
                task_ref_loaded_into_current_cpu(next_ref, BootCurrentCPU);
                current_task_ref_updated_by_switch(BootCurrentCPU, prev_ref, next_ref);
            }
            deferred {
                "当前 SwitchTo 只建立 RISC-V __switch_to 核心寄存器保存/恢复框架、prepare_task_switch/finish_task_switch 边界和 CurrentTaskRef commit；真实栈切换、next task 上下文恢复、last 返回值、FPU/vector、MM/lazy-TLB/membarrier 细节、finish_task_switch 具体钩子和长期任务 continuation 后续展开。";
            }
        }

        Action::SelectRunQueue(task_ref: TaskRef) -> RunQueueRef {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(task_ref);
            }
            ensures {
                scheduler_select_runqueue_returns(self, task_ref, BootRunQueueRef);
                runqueue_ref_targets(BootRunQueueRef, BootRunQueue);
                runqueue_ref_cpu_is(BootRunQueueRef, BootCPURef);
            }
            deferred {
                "当前 SelectRunQueue 固定返回 Boot CPU runqueue，并通过 runqueue_ref_cpu_is(selected_rq, BootCPURef) 证明调用方可执行 Task.SetTaskCpu(BootCPURef)；未来 SMP 选择策略仍应由 selected RunQueueRef 的 CPU 事实驱动 Task.SetTaskCpu，而不是由调用方硬编码 boot CPU。完整 select_task_rq 策略后续展开。";
            }
        }
    }
}

/*
 * BootIdleRuntimeObject is the boot CPU idle runtime process type. It models
 * the Linux cpu_startup_entry(CPUHP_ONLINE) tail without treating the wrapper
 * function itself as a standalone object: PrepareIdleEntry covers
 * current->flags |= PF_IDLE, arch_cpu_idle_prepare(), and
 * cpuhp_online_idle(CPUHP_ONLINE); RunIdleLoop covers while (1) do_idle();
 * DoIdleCycle covers one representative do_idle() pass and its conditional
 * schedule_idle() boundary.
 */
type BootIdleRuntimeObject: TaskObject {
    processes {
        Action::PrepareIdleEntry {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                scheduler_first_schedule_committed(Scheduler);
                task_ref_ready(CurrentTaskRef);
                task_ref_targets(CurrentTaskRef, BootIdleTask);
            }
            ensures {
                boot_idle_entry_prepared(self, BootIdleTask);
                boot_idle_task_identity_entered(BootInitTask, BootIdleTask);
                boot_idle_task_pf_idle(BootIdleTask);
                boot_idle_arch_cpu_idle_prepare_done(self, BootCPU);
                boot_idle_cpuhp_online_state_confirmed(self, BootCPU);
                boot_cpu_hotplug_state_online(BootCPU);
                boot_idle_need_resched_clear_before_wait(BootIdleTask);
            }
        }

        Action::RunIdleLoop {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_entry_prepared(self, BootIdleTask);
            }
            drives {
                self.Action::DoIdleCycle;
            }
            ensures {
                boot_idle_runtime_loop_entered(self, BootIdleTask);
                boot_idle_loop_continues(self);
            }
        }

        Action::DoIdleCycle {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_entry_prepared(self, BootIdleTask);
            }
            drives {
                self.Action::WaitWhileNoNeedResched;
                self.Action::ObserveNeedResched;
                self.Action::ScheduleIfNeedResched;
            }
            ensures {
                boot_idle_runtime_cycle_started(self, BootIdleTask);
                boot_idle_nohz_run_idle_balance_done(self, BootCPU);
                boot_idle_loop_cycle_committed(self);
                boot_idle_loop_continues(self);
            }
            deferred {
                "DoIdleCycle 当前只展开一轮代表性 do_idle() 主线；真实 while (1) do_idle() 会重复执行，后续可用循环/运行期 trace 语义表达多轮。";
            }
        }

        Action::WaitWhileNoNeedResched {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_entry_prepared(self, BootIdleTask);
            }
            within BootIdleWaitLocalInterruptContext {
                ensures {
                    boot_idle_local_irq_disabled_for_sleep(self, BootCpuLocalInterrupt);
                    boot_idle_arch_cpu_idle_enter_done(self, BootCPU);
                    boot_idle_rcu_nocb_deferred_wakeup_flushed(self);
                    boot_idle_cpu_offline_dead_path_not_taken(self, BootCPU);
                    boot_idle_poll_or_cpuidle_path_deferred(self);
                    boot_idle_arch_cpu_idle_exit_done(self, BootCPU);
                }
            }
            ensures {
                boot_idle_runtime_cycle_started(self, BootIdleTask);
                boot_idle_need_resched_clear_before_wait(BootIdleTask);
                boot_idle_runtime_observed_no_need_resched(self, BootIdleTask);
                boot_idle_polling_set(BootIdleTask);
                boot_idle_polling_rmb_before_sleep_check(BootIdleTask);
                boot_idle_nohz_entered(self);
                boot_idle_local_irq_disabled_for_sleep(self, BootCpuLocalInterrupt);
                boot_idle_arch_cpu_idle_enter_done(self, BootCPU);
                boot_idle_rcu_nocb_deferred_wakeup_flushed(self);
                boot_idle_cpu_offline_dead_path_not_taken(self, BootCPU);
                boot_idle_poll_or_cpuidle_path_deferred(self);
                boot_idle_arch_cpu_idle_exit_done(self, BootCPU);
                boot_idle_runtime_waiting(self, BootIdleTask);
                boot_idle_wait_path_deferred(self);
            }
            deferred {
                "WaitWhileNoNeedResched 抽象 Linux do_idle() 中 while (!need_resched()) 的 idle wait 段；当前显式记录 polling rmb、本地中断关闭、arch_cpu_idle_enter/exit、RCU nocb deferred wakeup flush、offline-dead path 未进入和 poll/cpuidle 分支 deferred。cpu_idle_poll、cpuidle_idle_call、WFI、tick broadcast 和完整 context-tracking 细节后续展开。";
            }
        }

        Action::ObserveNeedResched {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_runtime_waiting(self, BootIdleTask);
            }
            ensures {
                boot_idle_need_resched_set_for_schedule(BootIdleTask);
                boot_idle_runtime_observed_need_resched(self, BootIdleTask);
                boot_idle_polling_cleared(BootIdleTask);
                boot_idle_preempt_need_resched_set(BootIdleTask);
                boot_idle_nohz_exited(self);
                boot_idle_polling_clear_mb_before_flush(BootIdleTask);
                boot_idle_smp_call_function_queue_flushed(self);
            }
            deferred {
                "need_resched 由本 CPU 可观察环境设置，通常来自唤醒、定时器或跨 CPU 调度请求；当前模型只把该环境结果作为 idle loop 的条件分界事实。";
            }
        }

        Action::ScheduleIfNeedResched {
            state_effect: StateEffect::None;
            depends_on {
                boot_idle_need_resched_set_for_schedule(BootIdleTask);
                task_ref_ready(CurrentTaskRef);
                task_ref_targets(CurrentTaskRef, BootIdleTask);
            }
            drives {
                Scheduler.Action::ScheduleIdle;
            }
            ensures {
                boot_idle_schedule_requested(self, Scheduler);
                boot_idle_schedule_returned(self, Scheduler);
                scheduler_idle_schedule_returned_to_idle(Scheduler, CurrentTaskRef);
                boot_idle_need_resched_drained_after_schedule(BootIdleTask);
                boot_idle_loop_continues(self);
                boot_idle_livepatch_state_update_deferred(self);
                task_ref_targets(CurrentTaskRef, BootIdleTask);
            }
        }
    }
}

/*
 * RunQueue is the CPU-owned scheduler runqueue abstraction. A concrete
 * instance is reached through CpuGroup.Cpu[id].RunQueue, while Scheduler
 * orchestrates setup and selection policy rather than owning every runqueue.
 * The current model stores task_refs as a temporary aggregate view; future
 * CFS/RT/DL scheduler class queues should own concrete membership, with
 * RunQueue.task_refs derived from those queues. EnqueueTask commits the local membership fact
 * runqueue_contains_task(self, task_ref); phase-level sequencing may derive
 * task_enqueued_on_runqueue(task_ref, runqueue_ref) after selection and enqueue
 * both succeed. PickNextTask is a pure selection action corresponding to the
 * current minimal pick_next_task(rq, prev, &rf) boundary.
 */
type RunQueue: TaskObject {
    ext_state: RunQueueRuntimeState;
    task_refs: TaskRefSet;

    processes {
        Transition::EnqueueTask(task_ref: TaskRef) {
            state_effect: StateEffect::Conditional;
            depends_on {
                task_ref_ready(task_ref);
                task_not_enqueued(task_ref);
            }
            transitions {
                RunQueueRuntimeState::None -> RunQueueRuntimeState::Some;
                RunQueueRuntimeState::Some -> RunQueueRuntimeState::Some;
            }
            ensures {
                runqueue_runtime_state_is(self, RunQueueRuntimeState::Some);
                runqueue_task_refs_some(self);
                runqueue_contains_task(self, task_ref);
            }
            result {
                None: Success(first_task_enqueued);
                Some: Success(additional_task_enqueued);
                AlreadyQueued: Failed(duplicate_enqueue);
            }
            deferred {
                "当前 RunQueue.task_refs 是调度类队列尚未展开前的汇总视图；未来引入 CFS/RT/DL 等调度类子队列后，task_refs 应改为由具体队列派生。";
            }
        }

        Action::PickNextTask(prev_ref: TaskRef) -> TaskRef {
            state_effect: StateEffect::None;
            depends_on {
                runqueue_ref_targets(CurrentRunQueueRef, self);
                runqueue_ref_ready(CurrentRunQueueRef);
                runqueue_ref_cpu_is(CurrentRunQueueRef, BootCPURef);
                current_runqueue_ref_private_to_cpu(CurrentRunQueueRef, BootCurrentCPU);
                task_ref_ready(prev_ref);
            }
            ensures {
                runqueue_pick_next_task_returns(CurrentRunQueueRef, prev_ref, CurrentTaskRef);
                task_ref_targets(CurrentTaskRef, KernelInitTask);
                task_ref_ready(CurrentTaskRef);
                scheduler_pick_next_task_selects_runnable(Scheduler, self, CurrentTaskRef);
            }
            deferred {
                "当前 UP/rest_init schedule 路径在 BootRunQueue 中优先选择已入队的 KernelInitTask；未来可扩展为 KernelInitTask/KthreaddTask 以及完整 scheduler class pick_next_task 策略。";
            }
        }
    }
}

type CurrentCPU {
    owned {
        cpu: CPUObject;
    }

    lifecycle {
        Transition::Preset {
            state_effect: StateEffect::Always;
            ensures {
                current_cpu_self_identity_ready(self);
            }
        }

        Transition::Setup {
            state_effect: StateEffect::Always;
            ensures {
                current_cpu_self_identity_ready(self);
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Always;
            ensures {
                current_cpu_registered_in_cpu_group(self, CpuGroup);
            }
        }
    }
}

type LocalInterruptControl {
    ext_state: LocalInterruptExtState;

    processes {
        Transition::Disable {
            state_effect: StateEffect::Conditional;
            transitions {
                LocalInterruptExtState::Enabled -> LocalInterruptExtState::Disabled;
                LocalInterruptExtState::Disabled -> LocalInterruptExtState::Disabled;
            }
            ensures {
                cpu_local_interrupts_disabled(self);
            }
            result {
                Enabled: Success(disabled);
                Disabled: Success(no_change);
            }
        }

        Transition::Enable {
            state_effect: StateEffect::Conditional;
            transitions {
                LocalInterruptExtState::Disabled -> LocalInterruptExtState::Enabled;
                LocalInterruptExtState::Enabled -> LocalInterruptExtState::Enabled;
            }
            ensures {
                cpu_local_interrupts_enabled(self);
            }
            result {
                Disabled: Success(enabled);
                Enabled: Success(no_change);
            }
        }

        Transition::SaveAndDisable {
            state_effect: StateEffect::Conditional;
            transitions {
                LocalInterruptExtState::Enabled -> LocalInterruptExtState::Disabled;
                LocalInterruptExtState::Disabled -> LocalInterruptExtState::Disabled;
            }
            ensures {
                cpu_local_interrupts_saved_and_disabled(self);
                cpu_local_interrupts_disabled(self);
            }
            result {
                Enabled: Success(saved_enabled_then_disabled);
                Disabled: Success(saved_disabled);
            }
        }

        Transition::Restore {
            state_effect: StateEffect::Conditional;
            transitions {
                SavedInterruptState::Enabled -> LocalInterruptExtState::Enabled;
                SavedInterruptState::Disabled -> LocalInterruptExtState::Disabled;
            }
            ensures {
                cpu_local_interrupts_restored(self);
            }
            result {
                SavedEnabled: Success(restored_enabled);
                SavedDisabled: Success(restored_disabled);
            }
        }
    }
}

type CurrentTaskSlot {
    processes {
        Action::SetCurrent {
            state_effect: StateEffect::None;
            ensures {
                current_task_slot_current(self, task);
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
                current_cpu.cpu.LocalInterruptControl.Transition::SaveAndDisable(out flags);
                current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Transition::Disable;
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
                current_cpu.cpu.LocalInterruptControl.Transition::Restore(flags);
                current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Transition::Enable;
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
