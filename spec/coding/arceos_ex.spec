/*
 * arceos_ex Object Coding Specification
 *
 * This file records target-specific object-coding constraints for the current
 * arceos_ex prototype. It does not redefine model semantics; it constrains how
 * code must realize selected model events in this implementation target.
 */

predicate arceos_ex_must_device_tree_setup_allocates_from_memblock() -> bool;
predicate arceos_ex_must_device_tree_unflatten_uses_two_passes() -> bool;
predicate arceos_ex_must_device_tree_storage_uses_established_linear_mapping() -> bool;
predicate arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage() -> bool;
predicate arceos_ex_must_device_tree_checkpoint_after_validation() -> bool;
predicate arceos_ex_should_encapsulate_device_tree_unflatten_unsafe() -> bool;
predicate arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks() -> bool;
predicate arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy() -> bool;
predicate arceos_ex_must_memblock_disable_reaches_offline_not_destroyed() -> bool;
predicate arceos_ex_must_swiotlb_setup_before_memblock_disable() -> bool;
predicate arceos_ex_must_memory_debug_hardening_use_static_branch_registry() -> bool;
predicate arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready() -> bool;
predicate arceos_ex_must_page_table_lock_cache_named_page_ptl() -> bool;
predicate arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_not_page_tables() -> bool;
predicate arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects() -> bool;
predicate arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache() -> bool;
predicate arceos_ex_should_keep_mm_core_init_checkpoints_observable() -> bool;
predicate arceos_ex_must_startup_drive_boot_then_interrupt_then_payload() -> bool;
predicate arceos_ex_must_payload_require_interrupt_phase_ready() -> bool;
predicate arceos_ex_must_payload_follow_smp_runtime_not_nested_under_it() -> bool;
predicate arceos_ex_must_payload_require_finalize_ready() -> bool;
predicate arceos_ex_must_kernel_init_execution_line_reach_payload() -> bool;
predicate arceos_ex_must_irq_time_init_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_irq_time_init_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_irq_time_init_local_irq_enable_is_terminal_action() -> bool;
predicate arceos_ex_must_irq_time_init_keep_task_and_smp_concurrency_closed() -> bool;
predicate arceos_ex_must_irq_time_init_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_irq_time_init_expose_time_and_clockevent_smoke_actions() -> bool;
predicate arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_irq_open_prepare_run_after_irq_time_init() -> bool;
predicate arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online() -> bool;
predicate arceos_ex_must_irq_open_prepare_console_prepared_only() -> bool;
predicate arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions() -> bool;
predicate arceos_ex_must_process_prepare_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_process_prepare_run_after_irq_open_prepare() -> bool;
predicate arceos_ex_must_process_prepare_not_create_rest_init_tasks() -> bool;
predicate arceos_ex_must_process_prepare_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects() -> bool;
predicate arceos_ex_must_process_prepare_keep_deferred_paths_explicit() -> bool;
predicate arceos_ex_must_completion_map_to_reusable_object() -> bool;
predicate arceos_ex_must_completion_own_simple_wait_queue() -> bool;
predicate arceos_ex_must_completion_processes_keep_state_effects() -> bool;
predicate arceos_ex_must_completion_instances_drive_type_processes() -> bool;
predicate arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow() -> bool;
predicate arceos_ex_must_rest_init_model_path_under_up_multitask_phase() -> bool;
predicate arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree() -> bool;
predicate arceos_ex_must_rest_init_run_after_process_prepare() -> bool;
predicate arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts() -> bool;
predicate arceos_ex_must_rest_init_publish_system_scheduling() -> bool;
predicate arceos_ex_must_rest_init_complete_kthreadd_ready_gate() -> bool;
predicate arceos_ex_must_rest_init_publish_scheduler_dispatch_facts() -> bool;
predicate arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions() -> bool;
predicate arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle() -> bool;
predicate arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle() -> bool;
predicate arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain() -> bool;
predicate arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations() -> bool;
predicate arceos_ex_must_current_task_ref_be_cpu_view_private() -> bool;
predicate arceos_ex_must_current_runqueue_ref_be_cpu_view_private() -> bool;
predicate arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue() -> bool;
predicate arceos_ex_must_rest_init_pin_kernel_init_as_task_action() -> bool;
predicate arceos_ex_must_rest_init_not_make_pre_smp_depend_on_rest_init_ready() -> bool;
predicate arceos_ex_must_rest_init_keep_true_task_switching_deferred() -> bool;
predicate arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred() -> bool;
predicate arceos_ex_must_pre_smp_init_model_path_under_up_multitask_phase() -> bool;
predicate arceos_ex_must_pre_smp_init_code_path_follow_up_multitask_phase_tree() -> bool;
predicate arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts() -> bool;
predicate arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology() -> bool;
predicate arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls() -> bool;
predicate arceos_ex_must_pre_smp_init_stop_before_smp_init() -> bool;
predicate arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase() -> bool;
predicate arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree() -> bool;
predicate arceos_ex_must_smp_bringup_focus_bp_side_flow() -> bool;
predicate arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit() -> bool;
predicate arceos_ex_must_smp_bringup_make_secondary_cpus_online() -> bool;
predicate arceos_ex_must_smp_bringup_keep_ap_internals_deferred() -> bool;
predicate arceos_ex_must_smp_runtime_keep_later_subphases_deferred() -> bool;

type ArceosExDeviceTreeCodingMust {
    invariant {
        /*
         * MemBlock allocation:
         *
         * DeviceTree.setup() must allocate the expanded tree storage through
         * MemBlock early allocation. Its dependency on MemBlock.Online is an
         * allocation capability, not merely a read-only state check.
         */
        arceos_ex_must_device_tree_setup_allocates_from_memblock();

        /*
         * Two-pass unflatten:
         *
         * DeviceTree.setup() must first traverse RawDtb to validate and compute
         * the required expanded storage, then allocate storage, then traverse
         * RawDtb again to populate DeviceNode and property relations.
         */
        arceos_ex_must_device_tree_unflatten_uses_two_passes();

        /*
         * Established mapping:
         *
         * MemBlock returns physical storage. DeviceTree.setup() may write the
         * expanded tree only after resolving that storage through the already
         * established kernel linear mapping provided by SwapperVm/Config.
         */
        arceos_ex_must_device_tree_storage_uses_established_linear_mapping();

        /*
         * No heap or fixed static substitute:
         *
         * DeviceTree.setup() must not use the ordinary heap, Vec/Box, or a
         * fixed static array as the final expanded tree storage. A bounded
         * temporary stack/global workspace is allowed only for traversal state,
         * not as the DeviceTree storage itself.
         */
        arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage();

        /*
         * Checkpoint after validation:
         *
         * The DeviceTree Ready checkpoint may be emitted only after the second
         * pass has populated the tree and the model-level root, parent/child,
         * path lookup and property query facts have been checked.
         */
        arceos_ex_must_device_tree_checkpoint_after_validation();
    }
}

type ArceosExDeviceTreeCodingShould {
    invariant {
        /*
         * Unsafe encapsulation:
         *
         * Raw pointer writes into MemBlock-backed storage should be kept behind
         * a narrow internal boundary. The public DeviceTree event surface
         * should expose safe state/query operations.
         */
        arceos_ex_should_encapsulate_device_tree_unflatten_unsafe();
    }
}

type ArceosExMmCoreInitCodingMust {
    invariant {
        /*
         * PageAllocator preset/setup split:
         *
         * PageAllocator.preset() must only establish zonelist topology and
         * page allocator CPU hotplug step registration. It must not release
         * MemBlock pages to buddy/free page sets.
         */
        arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks();

        /*
         * MemBlock handoff:
         *
         * PageAllocator.setup() must perform the memblock_free_all() handoff:
         * account managed pages, populate buddy free page sets, and advance
         * MemBlock through Disable to Offline.
         */
        arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy();

        /*
         * MemBlock remains Offline:
         *
         * mm_core_init() must not destroy or discard MemBlock metadata. That
         * later discard belongs to page_alloc_init_late()/memblock_discard().
         */
        arceos_ex_must_memblock_disable_reaches_offline_not_destroyed();

        /*
         * SWIOTLB ordering:
         *
         * Swiotlb.setup() must complete before MemBlock.Disable so any early
         * default pool or resolved no-pool fact is established while MemBlock
         * allocation/reservation is still available.
         */
        arceos_ex_must_swiotlb_setup_before_memblock_disable();

        /*
         * Static branch policy:
         *
         * MemoryDebugHardening.setup() must consume the existing StaticBranch
         * registry and early parameter facts; it must not initialize a private
         * static-key mechanism.
         */
        arceos_ex_must_memory_debug_hardening_use_static_branch_registry();

        /*
         * SLUB bootstrap:
         *
         * SlubAllocator.setup() must bootstrap kmem_cache/kmem_cache_node
         * before KmallocCaches is considered Ready.
         */
        arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready();

        /*
         * Page table lock cache:
         *
         * PageTableLockCache.setup() must create the split page-table lock
         * cache corresponding to Linux "page->ptl".
         */
        arceos_ex_must_page_table_lock_cache_named_page_ptl();

        /*
         * Vmalloc boundary:
         *
         * VmallocAllocator.setup() must manage vmalloc/vmap virtual address
         * resources and metadata. It must not be used as the page-table
         * mapping implementation; page table range preparation belongs to
         * PageTableCaches/SwapperVm facts.
         */
        arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_not_page_tables();

        /*
         * Vmap subobjects:
         *
         * VmallocAllocator.setup() must establish VmapAreaCache,
         * VmapAddressSpace, VmapNodeSet, VmapBlockQueues, and VfreeDeferredSet
         * before VmallocAllocator.Ready is emitted.
         */
        arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects();

        /*
         * mm_struct only:
         *
         * MmStructCache.setup() must only establish the "mm_struct" cache.
         * vm_area_struct, vma lock cache, and mmap_init() remain later
         * proc_caches_init()/process-memory work.
         */
        arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache();
    }
}

type ArceosExMmCoreInitCodingShould {
    invariant {
        /*
         * Checkpoints:
         *
         * mm_core_init() implementation should keep compile-time, report and
         * trimmed-path checkpoints observable in trace/logging without turning
         * them into lifecycle states.
         */
        arceos_ex_should_keep_mm_core_init_checkpoints_observable();
    }
}

type ArceosExStartupPhaseCodingMust {
    invariant {
        /*
         * Startup phase order:
         *
         * The arceos_ex startup chain must follow the formal top-level model
         * order PreparePhase -> BootPhase -> InterruptPhase ->
         * UpMultitaskPhase -> SmpRuntimePhase -> PayloadPhase.
         * PayloadPhase setup/enable must not run directly after BootPhase
         * without first completing the intervening phase chain.
         */
        arceos_ex_must_startup_drive_boot_then_interrupt_then_payload();

        /*
         * Payload dependency:
         *
         * PayloadPhase setup and enable must require InterruptPhase.Ready in
         * addition to BootPhase.Ready. A selected payload may rely on the
         * boot CPU interrupt gate and IRQ/time acceptance facts established by
         * InterruptPhase.
         */
        arceos_ex_must_payload_require_interrupt_phase_ready();

        /*
         * Payload placement:
         *
         * PayloadPhase is a StartupTimeline child that follows
         * SmpRuntimePhase. It must not be implemented as the last subphase
         * nested under SmpRuntimePhase.
         */
        arceos_ex_must_payload_follow_smp_runtime_not_nested_under_it();

        /*
         * Finalize handoff:
         *
         * PayloadPhase setup/enable must require FinalizePhase.Ready and the
         * FinalizeBoundary next-boundary fact. This makes the direct handoff
         * from kernel_init() finalization to payload explicit.
         */
        arceos_ex_must_payload_require_finalize_ready();

        /*
         * KernelInitTask execution line:
         *
         * KernelInitTask execution starts at the PreSmpInitPhase entry after
         * rest_init() dispatch facts, then proceeds through the ordered
         * phase chain to FinalizePhase and PayloadPhase. Payload ownership is
         * derived from that continuous execution line instead of from an
         * isolated payload-only fact.
         */
        arceos_ex_must_kernel_init_execution_line_reach_payload();
    }
}

type ArceosExIrqTimeInitCodingMust {
    invariant {
        /*
         * Model path:
         *
         * IrqTimeInitPhase is InterruptPhase subphase 1. Its formal model
         * path is spec/model/interrupt/irq-time-init/, not
         * spec/model/boot/irq-time-init/.
         */
        arceos_ex_must_irq_time_init_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/irq_time_init.rs,
         * rather than the boot phase subtree.
         */
        arceos_ex_must_irq_time_init_code_path_follow_interrupt_phase_tree();

        /*
         * Interrupt-open boundary:
         *
         * local_irq_enable() is the terminal action of IrqTimeInitPhase. It
         * must be represented as InterruptStream.enable() on the boot CPU and
         * must not be moved back to the beginning of a later IRQ-open phase.
         */
        arceos_ex_must_irq_time_init_local_irq_enable_is_terminal_action();

        /*
         * Concurrency scope:
         *
         * IrqTimeInitPhase opens only the boot CPU local interrupt gate.
         * Task concurrency and SMP concurrency remain closed when this phase
         * reaches Ready.
         */
        arceos_ex_must_irq_time_init_keep_task_and_smp_concurrency_closed();

        /*
         * Runtime services:
         *
         * Opening the boot CPU interrupt gate must not implicitly advance
         * periodic tick service, full softirq execution, IPI enable, workqueue
         * workers, RCU GP kthreads or secondary CPU execution to Online.
         */
        arceos_ex_must_irq_time_init_keep_runtime_services_deferred();

        /*
         * Smoke actions:
         *
         * RiscvTimerProvider.setup() must expose enough action surface for two
         * smoke checks after InterruptStream.enable(): a monotonic time read
         * check and a one-shot clockevent callback check through the timer IRQ
         * route. These checks do not imply full periodic tick service.
         */
        arceos_ex_must_irq_time_init_expose_time_and_clockevent_smoke_actions();
    }
}

type ArceosExIrqOpenPrepareCodingMust {
    invariant {
        /*
         * Model path:
         *
         * IrqOpenPreparePhase is InterruptPhase subphase 2. Its formal model
         * path is spec/model/interrupt/irq-open-prepare/.
         */
        arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs.
         */
        arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree();

        /*
         * Ordering:
         *
         * IrqOpenPreparePhase must run after IrqTimeInitPhase.Ready, with the
         * boot CPU local interrupt gate already open. It must not move
         * local_irq_enable() out of IrqTimeInitPhase.
         */
        arceos_ex_must_irq_open_prepare_run_after_irq_time_init();

        /*
         * Runtime services:
         *
         * This phase must keep task concurrency and SMP concurrency closed and
         * must not implicitly start periodic tick service, IPI enable,
         * workqueue workers, RCU GP kthreads or full softirq execution.
         */
        arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred();

        /*
         * SLUB late boundary:
         *
         * kmem_cache_init_late() must be represented as the internal
         * SlubAllocator flush workqueue fact. It must not advance
         * SlubAllocator to Online/FULL; that belongs to later slab_sysfs_init()
         * style work.
         */
        arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online();

        /*
         * Console boundary:
         *
         * console_init() must prepare the formal Console object, line
         * discipline registry and early console driver set only. Real device
         * probe, boot console unregister and full handoff remain conditional
         * or deferred facts.
         */
        arceos_ex_must_irq_open_prepare_console_prepared_only();

        /*
         * Smoke actions:
         *
         * SchedClock.setup() and DelayLoop.setup() must expose enough action
         * surface for smoke checks after IrqOpenPreparePhase.Ready: a sched
         * clock read that advances and a bounded busy-wait delay action.
         */
        arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions();
    }
}

type ArceosExProcessPrepareCodingMust {
    invariant {
        /*
         * Model path:
         *
         * ProcessPreparePhase is InterruptPhase subphase 3. Its formal model
         * path is spec/model/interrupt/process-prepare/.
         */
        arceos_ex_must_process_prepare_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/process_prepare.rs.
         */
        arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree();

        /*
         * Ordering:
         *
         * ProcessPreparePhase must run after IrqOpenPreparePhase.Ready, with
         * Console.Prepared, SchedClock.Ready, DelayLoop.Ready and the boot CPU
         * local interrupt gate already established.
         */
        arceos_ex_must_process_prepare_run_after_irq_open_prepare();

        /*
         * rest_init boundary:
         *
         * This phase prepares the inputs to rest_init(). It must not create
         * kernel_init, kthreadd or any PID 1 task, and must not advance the
         * system into the scheduling-running state.
         */
        arceos_ex_must_process_prepare_not_create_rest_init_tasks();

        /*
         * Runtime services:
         *
         * This phase must keep task concurrency and SMP concurrency closed and
         * must not implicitly start workqueue workers, RCU GP kthreads, full
         * softirq execution, network namespace runtime or VFS/proc visible
         * services.
         */
        arceos_ex_must_process_prepare_keep_runtime_services_deferred();

        /*
         * Object coverage:
         *
         * The implementation must provide explicit object carriers for the
         * formal PID namespace, anonymous VMA, task creation, credential,
         * vector context, uprobe, signal, task file context, VMA, namespace,
         * keyring and security readiness/preparedness facts.
         */
        arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects();

        /*
         * Deferred paths:
         *
         * Trimmed and deferred Linux start_kernel() calls in this interval
         * must remain visible as checkpoints or deferred facts rather than
         * silently disappearing from the implementation boundary.
         */
        arceos_ex_must_process_prepare_keep_deferred_paths_explicit();
    }
}

type ArceosExCompletionCodingMust {
    invariant {
        /*
         * Reusable object:
         *
         * The formal Completion Type must map to a reusable Rust resource
         * object, not to ad-hoc boolean fields on each user. Its implementation
         * target is impl/arceos_ex/src/objects/completion.rs.
         */
        arceos_ex_must_completion_map_to_reusable_object();

        /*
         * Owned wait queue:
         *
         * Completion must own a SimpleWaitQueue field. The field is an owned
         * child resource corresponding to Linux swait_queue_head, not an
         * external wait-queue reference supplied by the caller.
         */
        arceos_ex_must_completion_own_simple_wait_queue();

        /*
         * Event/action boundary:
         *
         * Completion.setup()/enable() advance the ordinary lifecycle state.
         * complete(), complete_all(), wait(), try_wait(), reinit() operate on
         * CompletionExtState and token count while preserving the main
         * lifecycle state, and done() is a read-only action.
         */
        arceos_ex_must_completion_processes_keep_state_effects();

        /*
         * Instance inheritance:
         *
         * A model object declared as an instance of Completion, such as
         * KthreaddReadyGate, must drive the reusable Completion implementation
         * instead of copying completion-specific pending/completed bookkeeping
         * into a private object-local state machine.
         */
        arceos_ex_must_completion_instances_drive_type_processes();

        /*
         * Smoke coverage:
         *
         * Smoke tests must cover Completion.setup(), Completion.enable(),
         * complete(), token observation and token consumption, and must also
         * verify the live kthreadd_done/KthreaddReadyGate instance.
         */
        arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow();
    }
}

type ArceosExRestInitCodingMust {
    invariant {
        /*
         * Model path:
         *
         * RestInitPhase is UpMultitaskPhase subphase 1. Its formal model path
         * is spec/model/up-multitask/rest-init/.
         */
        arceos_ex_must_rest_init_model_path_under_up_multitask_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the up-multitask phase
         * subtree, for example impl/arceos_ex/src/phases/up_multitask/rest_init.rs.
         */
        arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree();

        /*
         * Ordering:
         *
         * RestInitPhase must run after ProcessPreparePhase.Ready and before
         * PayloadPhase. It consumes the prepared PID/task/cred/scheduler facts
         * and opens the single-CPU multitask boundary.
         */
        arceos_ex_must_rest_init_run_after_process_prepare();

        /*
         * Task creation facts:
         *
         * The implementation must publish explicit object facts for PID 1
         * KernelInitTask and KthreaddTask creation, scheduling eligibility and
         * kthreadd provider binding.
         */
        arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts();

        /*
         * System state:
         *
         * rest_init() must publish SystemState.value == SYSTEM_SCHEDULING and
         * the opening of task-concurrency semantics, without implying SMP.
         */
        arceos_ex_must_rest_init_publish_system_scheduling();

        /*
         * Completion:
         *
         * complete(&kthreadd_done) must drive the reusable Completion object
         * carried by KthreaddReadyGate, publish the visible gate fact, and
         * release KernelInitTask for the next PreSmpInitPhase.
         */
        arceos_ex_must_rest_init_complete_kthreadd_ready_gate();

        /*
         * Scheduler dispatch facts:
         *
         * schedule_preempt_disabled() must be expanded into preemption guard
         * exit, Scheduler.schedule(), and post-schedule boot idle context
         * entry. It must not be implemented as a single Scheduler action and
         * must not introduce a KernelInitDispatchGate lifecycle object; the
         * branch point is the combination of Scheduler first-schedule and
         * KernelInitTask dispatch facts while BootInitTask continues the
         * cpu_startup_entry() tail. Scheduler.schedule() must derive the
         * current task reference from the current CPU current-task view,
         * pick next from CurrentRunQueueRef, then switch through TaskRef-based core
         * context save/restore and publish the updated CPU-local current task
         * fact. The implementation boundary must pass through the current
         * CPU's CurrentTaskSlot; it must not infer or publish the current task
         * only from Scheduler counters or BootRunQueue.curr.
         */
        arceos_ex_must_rest_init_publish_scheduler_dispatch_facts();

        /*
         * Boot idle runtime actions:
         *
         * BootIdleRuntime.setup() must only establish the Ready object shell
         * after scheduler dispatch facts exist. It must not collapse
         * PrepareIdleEntry, RunIdleLoop and DoIdleCycle into one setup-time
         * fact update. The phase code must explicitly drive
         * BootIdleRuntime.prepare_idle_entry(), then
         * BootIdleRuntime.run_idle_loop(), with run_idle_loop() committing one
         * representative do_idle_cycle() boundary. This step still keeps the
         * real idle loop and need_resched loop deferred; it only aligns the
         * code shape with the model action boundary so later AI-generated code
         * has named hooks to extend.
         */
        arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions();

        /*
         * Representative need_resched idle cycle:
         *
         * BootIdleRuntime.do_idle_cycle() must expose the three model action
         * hooks WaitWhileNoNeedResched, ObserveNeedResched and
         * ScheduleIfNeedResched as named implementation boundaries. The first
         * boundary records that the boot idle task enters an abstract
         * no-need-resched wait state with polling/nohz details deferred; the
         * second records that the CPU-visible environment sets need_resched and
         * the idle task leaves the wait state; the third records a
         * schedule_idle request/return and drains the need_resched fact. This
         * remains an object-level representative cycle: it must not add a true
         * infinite loop, real timer/IRQ wakeup source, or cpuidle/WFI path.
         */
        arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle();

        /*
         * schedule_idle wrapper:
         *
         * BootIdleRuntime.schedule_if_need_resched() must now drive a concrete
         * Scheduler.schedule_idle() implementation boundary. The wrapper must
         * require the current CPU's CurrentTaskSlot to still target BootIdleTask
         * and the need_resched observation to have been recorded by
         * BootIdleRuntime. It may reuse the existing Scheduler.schedule() switch
         * skeleton for the current single-task path, but it must publish
         * idle-specific counters/facts separately from ordinary schedule()
         * calls so smoke/KUnit coverage can distinguish the idle path. It must
         * not model the full Linux do { __schedule(SM_IDLE); } while
         * (need_resched()) loop, sched_submit_work() skip details, or a real
         * prev != next task switch yet.
         */
        arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle();

        /*
         * RestInitPhase.Setup boot-idle chain:
         *
         * RestInitPhase.setup() must present the boot-idle tail chain directly
         * in phase order: BootIdleRuntime.setup(), then
         * BootIdleRuntime.prepare_idle_entry(), then
         * BootIdleRuntime.run_idle_loop(), then the RestInitPhase.Ready
         * checkpoint. It may use one small helper for each named action, but it
         * must not hide the whole chain behind a single setup_boot_idle_tail()
         * helper or collapse the model action order into one opaque phase call.
         */
        arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain();

        /*
         * Smoke/KUnit coverage:
         *
         * The rest_init smoke case and the checkpoint KUnit smoke reuse must
         * validate the boot idle schedule relation, not only non-zero facts:
         * the representative idle cycle records exactly one
         * Scheduler.schedule_idle() pass in the current BP implementation;
         * idle schedule request, return and identity counters match each
         * other; ordinary schedule/switch/current-task switch counters include
         * that idle pass; and the scheduler smoke case proves that an ordinary
         * Scheduler.schedule() call does not increment idle-specific counters.
         */
        arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations();

        /*
         * CurrentTaskRef scope:
         *
         * CurrentTaskRef must be realized as a private object in the current
         * CPU view. The BP path owns the BootCurrentCPU CurrentTaskRef and must
         * not introduce a descriptive CurrentTask object or a global current
         * task singleton. On task switch, next must become the target of this
         * CPU-local CurrentTaskRef. RISC-V64 code should follow the Linux-style
         * tp register implementation reference through the object-level
         * CurrentTaskSlot boundary, but per-cpu storage remains an
         * implementation term, not the model definition.
         */
        arceos_ex_must_current_task_ref_be_cpu_view_private();

        /*
         * CurrentRunQueueRef scope:
         *
         * CurrentRunQueueRef must be realized as a private reference in the
         * current CPU view. It must not be implemented as a descriptive
         * current-runqueue object or as a global current-runqueue singleton. Code
         * should follow the Linux-style path: derive the current task through
         * CurrentTaskRef, read the task's recorded CPU id, then resolve that
         * CPU's runqueue through CPUGroup/runqueue topology. The current BP
         * implementation may collapse this to the boot runqueue while marking
         * that binding as a temporary UP specialization.
         */
        arceos_ex_must_current_runqueue_ref_be_cpu_view_private();

        /*
         * Wake-up task CPU action:
         *
         * KernelInitTask and KthreaddTask wake-up paths must follow the
         * Linux ordering: select the target runqueue, update the task's
         * recorded CPU through a Task-level set_task_cpu boundary, then
         * enqueue the task on that runqueue. The current BP implementation may
         * bind the selected runqueue CPU to BootCPU/BootCPURef, but this is a
         * temporary specialization; future SMP code must resolve cpu_of from
         * the selected RunQueueRef.
         */
        arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue();

        /*
         * KernelInitTask affinity action:
         *
         * PID 1 boot CPU pinning must be implemented as a KernelInitTask
         * action that sets the PF_NO_SETAFFINITY-equivalent flag and cpumask
         * facts. It must not be used as the wake-up set_task_cpu action and
         * must not be represented by an independent
         * KernelInitAffinity lifecycle object. The RCU read-side boundary
         * around the Linux pid lookup remains a deferred context-modeling
         * question, not a completed resource-exclusive context.
         */
        arceos_ex_must_rest_init_pin_kernel_init_as_task_action();

        /*
         * Fork dependency:
         *
         * PreSmpInitPhase must depend on the KernelInitTask release/dispatch
         * facts and Scheduler first-schedule fact, not on RestInitPhase.Ready.
         * RestInitPhase.Ready still records the boot idle tail completion.
         */
        arceos_ex_must_rest_init_not_make_pre_smp_depend_on_rest_init_ready();

        /*
         * No real task switch:
         *
         * The current object-level implementation must not pretend to perform
         * a real task-stack switch, preemptive scheduler context switch or
         * idle loop. It may only publish the rest_init boundary facts.
         */
        arceos_ex_must_rest_init_keep_true_task_switching_deferred();

        /*
         * Deferred runtime:
         *
         * Secondary CPU bringup, workqueue workers, Tasks RCU GP kthreads,
         * KernelInitTask.kernel_init_freeable() and kthreadd request
         * consumption remain later-phase work.
         */
        arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred();
    }
}

type ArceosExPreSmpInitCodingMust {
    invariant {
        /*
         * Model path:
         *
         * PreSmpInitPhase is UpMultitaskPhase subphase 2. Its formal model
         * path is spec/model/up-multitask/pre-smp-init/.
         */
        arceos_ex_must_pre_smp_init_model_path_under_up_multitask_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/up_multitask/.
         */
        arceos_ex_must_pre_smp_init_code_path_follow_up_multitask_phase_tree();

        /*
         * Entry facts:
         *
         * This phase must run from the KernelInitTask release/dispatch facts
         * and Scheduler first-schedule fact, not from RestInitPhase.Ready.
         */
        arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts();

        /*
         * Allocation and CPU topology:
         *
         * This phase must open PageAllocator full GFP mask and record pre-SMP
         * CPU topology/present facts without making secondary CPUs online.
         */
        arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology();

        /*
         * Runtime support setup:
         *
         * This phase must setup Workqueue, VmstatCore, TasksRcu and early
         * pre-SMP initcall boundary facts.
         */
        arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls();

        /*
         * Stop before SMP:
         *
         * smp_init() is the next top-level phase boundary and must not be
         * executed or modeled as complete here.
         */
        arceos_ex_must_pre_smp_init_stop_before_smp_init();
    }
}

type ArceosExSmpBringupCodingMust {
    invariant {
        /*
         * Model path:
         *
         * SmpBringupPhase is SMP Runtime Phase subphase 1. Its formal model
         * path is spec/model/smp-runtime/smp-bringup/.
         */
        arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree();

        /*
         * BP-side focus:
         *
         * The current implementation should follow the boot processor side of
         * smp_init(), not expand AP entry internals.
         */
        arceos_ex_must_smp_bringup_focus_bp_side_flow();

        /*
         * Synchronization:
         *
         * BP/AP synchronization facts must stay explicit: cpu_running,
         * done_up and done_down placement must be represented even when AP
         * internals are summarized.
         */
        arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit();

        /*
         * Online boundary:
         *
         * This phase must move secondary CPUs from present/not-online to
         * online and publish the opening of SMP concurrency.
         */
        arceos_ex_must_smp_bringup_make_secondary_cpus_online();

        /*
         * AP internals:
         *
         * secondary_start_sbi, smp_callin(), AP local IRQ enable, AP idle and
         * AP hotplug callback details remain deferred summary paths.
         */
        arceos_ex_must_smp_bringup_keep_ap_internals_deferred();

        /*
         * Later runtime:
         *
         * SmpBringupPhase must hand off to RuntimeCorePhase. Later
         * subphases are expanded through their own formal model and code
         * steps rather than being silently assumed complete.
         */
        arceos_ex_must_smp_bringup_handoff_to_runtime_core();
        arceos_ex_must_smp_runtime_expand_later_subphases_explicitly();
    }
}

type ArceosExRuntimeCoreCodingMust {
    invariant {
        /*
         * Model path:
         *
         * RuntimeCorePhase is SMP Runtime Phase subphase 2. Its formal model
         * path is spec/model/smp-runtime/runtime-core/.
         */
        arceos_ex_must_runtime_core_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_runtime_core_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * RuntimeCorePhase must run after SmpBringupPhase.Ready, with
         * secondary CPUs online and SMP concurrency open.
         */
        arceos_ex_must_runtime_core_run_after_smp_bringup();

        /*
         * Scheduler SMP action:
         *
         * Scheduler.enable_smp() must publish SMP scheduler domains, release
         * PID 1 boot CPU affinity, clear PF_NO_SETAFFINITY, refresh
         * granularity and initialize RT/DL SMP post state without re-running
         * Scheduler lifecycle enable.
         */
        arceos_ex_must_runtime_core_enable_scheduler_smp_action();

        /*
         * Workqueue topology:
         *
         * RuntimeCorePhase must publish workqueue topology facts for CPU/SMT,
         * cache and NUMA pod types and rebind unbound pools while keeping the
         * current object-level Workqueue.Ready historical state stable.
         */
        arceos_ex_must_runtime_core_setup_workqueue_topology_action();

        /*
         * Deferred runtime cores:
         *
         * async_init() and padata_init() must remain explicit deferred
         * boundaries in this step.
         */
        arceos_ex_must_runtime_core_keep_async_and_padata_deferred();

        /*
         * Page allocator late:
         *
         * RuntimeCorePhase must publish page_alloc_init_late() facts,
         * including memory stats, buffer init, memblock private discard, zone
         * contiguous, sysctl and current-config trimmed late paths.
         */
        arceos_ex_must_runtime_core_setup_page_allocator_late_action();
    }
}

type ArceosExInitcallCodingMust {
    invariant {
        /*
         * Model path:
         *
         * InitcallPhase is SMP Runtime Phase subphase 3. Its formal model
         * path is spec/model/smp-runtime/initcall/.
         */
        arceos_ex_must_initcall_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_initcall_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * InitcallPhase must run after RuntimeCorePhase.Ready and preserve
         * the do_basic_setup() entry boundary.
         */
        arceos_ex_must_initcall_run_after_runtime_core();

        /*
         * Deferred heavy subsystems:
         *
         * DriverCore and IrqProcView must remain explicit deferred
         * boundaries in this step.
         */
        arceos_ex_must_initcall_keep_driver_core_and_irq_proc_deferred();

        /*
         * Constructors:
         *
         * CtorTable must preserve the do_ctors() table position and record
         * the current trimmed/empty constructor table status.
         */
        arceos_ex_must_initcall_record_ctor_table_boundary();

        /*
         * Initcall table:
         *
         * InitcallTable.run_all_levels() must represent do_initcalls() as a
         * static linker table action. It must record level count, all-level
         * execution, command-line scratch reuse, parameter parsing, filtering
         * and run-context checks without promoting every entry to a top-level
         * object.
         */
        arceos_ex_must_initcall_run_static_initcall_table_summary();
    }
}

type ArceosExRootfsCodingMust {
    invariant {
        /*
         * Model path:
         *
         * RootfsPhase is SMP Runtime Phase subphase 4. Its formal model path
         * is spec/model/smp-runtime/rootfs/.
         */
        arceos_ex_must_rootfs_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_rootfs_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * RootfsPhase must run after InitcallPhase.Ready and preserve the
         * kunit_run_all_tests() entry position inside RootfsPhase.
         */
        arceos_ex_must_rootfs_run_after_initcall();

        /*
         * KUnit runtime:
         *
         * CONFIG_KUNIT=n in the current Linux-like configuration, so
         * kunit_run_all_tests() must be encoded as a trimmed/no-op object
         * inside RootfsPhase, not as a standalone KUnitPhase.
         */
        arceos_ex_must_rootfs_keep_kunit_trimmed_inside_rootfs_phase();

        /*
         * Deferred initramfs and console details:
         *
         * wait_for_initramfs() and console_on_rootfs() must preserve their
         * Linux order but remain deferred in this round.
         */
        arceos_ex_must_rootfs_keep_initramfs_and_console_deferred();

        /*
         * Required branch checkpoint:
         *
         * init_eaccess(ramdisk_execute_command) must force the supported
         * Linux-like path toward prepare_namespace().
         */
        arceos_ex_must_rootfs_require_prepare_namespace_branch();

        /*
         * RootFS enable:
         *
         * prepare_namespace() must be represented by RootFsEnableDeferred in
         * this round. Root device probing, filesystem selection, devtmpfs
         * mount, MS_MOVE and chroot(".") must remain unimplemented details.
         */
        arceos_ex_must_rootfs_keep_rootfs_enable_deferred_only();

        /*
         * Integrity keys:
         *
         * integrity_load_keys() must preserve CONFIG_INTEGRITY=y timing but
         * keep IMA/EVM keyring and certificate loading details deferred.
         */
        arceos_ex_must_rootfs_keep_integrity_keys_deferred_only();

        /*
         * Boundary:
         *
         * RootfsBoundary must mark the next boundary as FinalizePhase.
         */
        arceos_ex_must_rootfs_boundary_handoff_to_finalize();
    }
}

type ArceosExFinalizeCodingMust {
    invariant {
        /*
         * Model path:
         *
         * FinalizePhase is SMP Runtime Phase subphase 5. Its formal model
         * path is spec/model/smp-runtime/finalize/.
         */
        arceos_ex_must_finalize_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_finalize_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * FinalizePhase must run after RootfsPhase.Ready and preserve the
         * kernel_init_freeable() return boundary before PayloadPhase.
         */
        arceos_ex_must_finalize_run_after_rootfs_phase();

        /*
         * Deferred cleanup details:
         *
         * async_synchronize_full(), ftrace/free_initmem, mark_readonly() and
         * do_sysctl_args() must preserve Linux order but remain deferred in
         * this round.
         */
        arceos_ex_must_finalize_keep_cleanup_details_deferred();

        /*
         * Trimmed current-config paths:
         *
         * kprobe_free_init_mem(), kgdb_free_init_mem(), exit_boot_config(),
         * pti_finalize() and numa_default_policy() must be recorded as
         * trimmed/no-op under the current RISC-V/default configuration.
         */
        arceos_ex_must_finalize_record_trimmed_config_paths();

        /*
         * System state:
         *
         * SystemState.enable() must publish the SYSTEM_FREEING_INITMEM window
         * and end with SystemState.state == Online and value == SYSTEM_RUNNING.
         */
        arceos_ex_must_finalize_publish_system_running();

        /*
         * RCU boot end:
         *
         * RcuCore.end_inkernel_boot() must record rcu_boot_ended == true
         * without claiming full runtime RCU GP service implementation.
         */
        arceos_ex_must_finalize_end_rcu_inkernel_boot();

        /*
         * Boundary:
         *
         * FinalizeBoundary must mark the next boundary as PayloadPhase.
         */
        arceos_ex_must_finalize_boundary_handoff_to_payload();
    }
}
