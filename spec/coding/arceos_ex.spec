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
predicate arceos_ex_must_rest_init_model_path_under_up_multitask_phase() -> bool;
predicate arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree() -> bool;
predicate arceos_ex_must_rest_init_run_after_process_prepare() -> bool;
predicate arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts() -> bool;
predicate arceos_ex_must_rest_init_publish_system_scheduling() -> bool;
predicate arceos_ex_must_rest_init_complete_kthreadd_ready_gate() -> bool;
predicate arceos_ex_must_rest_init_publish_kernel_init_dispatch_gate() -> bool;
predicate arceos_ex_must_rest_init_not_make_pre_smp_depend_on_rest_init_ready() -> bool;
predicate arceos_ex_must_rest_init_keep_true_task_switching_deferred() -> bool;
predicate arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred() -> bool;
predicate arceos_ex_must_pre_smp_init_model_path_under_up_multitask_phase() -> bool;
predicate arceos_ex_must_pre_smp_init_code_path_follow_up_multitask_phase_tree() -> bool;
predicate arceos_ex_must_pre_smp_init_run_from_kernel_init_dispatch_gate() -> bool;
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
         * order PreparePhase -> BootPhase -> InterruptPhase -> PayloadPhase.
         * PayloadPhase setup/enable must not run directly after BootPhase
         * without first completing InterruptPhase.
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
         * complete(&kthreadd_done) must be represented as a visible
         * KthreaddReadyGate fact that releases KernelInitTask for the next
         * PreSmpInitPhase.
         */
        arceos_ex_must_rest_init_complete_kthreadd_ready_gate();

        /*
         * Dispatch gate:
         *
         * schedule_preempt_disabled() must publish a visible
         * KernelInitDispatchGate fact. This gate is the branch point where
         * KernelInitTask may enter PreSmpInitPhase while BootInitTask
         * continues the cpu_startup_entry() tail.
         */
        arceos_ex_must_rest_init_publish_kernel_init_dispatch_gate();

        /*
         * Fork dependency:
         *
         * PreSmpInitPhase must depend on KernelInitDispatchGate.Ready, not on
         * RestInitPhase.Ready. RestInitPhase.Ready still records the boot idle
         * tail completion.
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
         * Entry gate:
         *
         * This phase must run from KernelInitDispatchGate.Ready, not from
         * RestInitPhase.Ready.
         */
        arceos_ex_must_pre_smp_init_run_from_kernel_init_dispatch_gate();

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
         * RuntimeCorePhase, InitcallPhase, RootfsPhase and FinalizePhase are
         * not implemented in this step; they must remain explicit deferred
         * boundaries rather than being silently assumed complete.
         */
        arceos_ex_must_smp_runtime_keep_later_subphases_deferred();
    }
}
