/*
 * Entry Prelude Object Model Specification
 *
 * This file is extracted from spec/charter.md.
 * It is intended to be parsed by verifier/modeling tools.
 * Rust/C style comments are for human readers and should be ignored by tools.
 *
 * Global rule:
 * - entries in a drives block form an ordered derivation queue and must be
 *   processed in declaration order.
 */

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

enum TaskRuntimeState {
    New,
    Running,
}

enum RunQueueRuntimeState {
    None,
    Some,
}

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
predicate selected_payload_ready() -> bool;
predicate selected_payload_no_return_handoff() -> bool;
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
predicate cpu_ref_targets<T, U>(cpu_ref: T, cpu: U) -> bool;
predicate cpu_ref_ready<T>(cpu_ref: T) -> bool;
predicate cpu_event_stream_ready<T, U>(cpu: T, event_stream: U) -> bool;
predicate cpu_exception_stream_ready<T, U>(cpu: T, exception_stream: U) -> bool;
predicate cpu_interrupt_stream_ready<T, U>(cpu: T, interrupt_stream: U) -> bool;
predicate cpu_local_interrupt_control_ready<T, U>(control: T, cpu: U) -> bool;
predicate cpu_local_interrupts_disabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_enabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_saved_and_disabled<T>(control: T) -> bool;
predicate cpu_local_interrupts_restored<T>(control: T) -> bool;
predicate current_task_slot_ready<T, U>(slot: T, cpu: U) -> bool;
predicate current_task_slot_current<T, U>(slot: T, task: U) -> bool;
predicate task_preemption_control_ready<T>(task: T) -> bool;
predicate task_preemption_disabled<T>(task: T) -> bool;
predicate task_preemption_enabled<T>(task: T) -> bool;
predicate task_preemption_enabled_no_resched<T>(task: T) -> bool;
predicate task_ref_targets<T, U>(task_ref: T, task: U) -> bool;
predicate task_ref_ready<T>(task_ref: T) -> bool;
predicate task_state_new<T>(task: T) -> bool;
predicate task_state_running<T>(task: T) -> bool;
predicate task_not_enqueued<T>(task: T) -> bool;
predicate task_enqueued_on_runqueue<T, U>(task: T, runqueue: U) -> bool;
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
predicate scheduler_select_runqueue_returns<T, U, V>(scheduler: T, task_ref: U, runqueue_ref: V) -> bool;
predicate scheduler_schedule_event_available<T>(scheduler: T) -> bool;
predicate scheduler_schedule_smoke_ready<T>(scheduler: T) -> bool;
predicate scheduler_schedule_local_interrupts_closed<T, U>(scheduler: T, local_interrupt: U) -> bool;
predicate scheduler_runqueue_lock_held_for_schedule<T, U>(scheduler: T, runqueue: U) -> bool;
predicate scheduler_pick_next_task_identity<T, U, V>(scheduler: T, runqueue: U, task: V) -> bool;
predicate scheduler_no_task_switch_on_single_task_path<T, U>(scheduler: T, task: U) -> bool;
predicate scheduler_switch_to_prepared<T, U, V, W>(scheduler: T, runqueue: U, prev_ref: V, next_ref: W) -> bool;
predicate scheduler_switch_to_committed<T, U, V>(scheduler: T, prev_ref: U, next_ref: V) -> bool;
predicate scheduler_switch_to_identity_path<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_switch_to_core_context_saved<T, U>(scheduler: T, task_ref: U) -> bool;
predicate scheduler_switch_to_core_context_restored<T, U>(scheduler: T, task_ref: U) -> bool;
predicate task_runqueue_selected<T, U, V>(scheduler: T, task: U, runqueue: V) -> bool;
predicate runqueue_runtime_state_is<T>(runqueue: T, state: RunQueueRuntimeState) -> bool;
predicate runqueue_task_refs_empty<T>(runqueue: T) -> bool;
predicate runqueue_task_refs_some<T>(runqueue: T) -> bool;
predicate runqueue_contains_task<T, U>(runqueue: T, task_ref: U) -> bool;
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

type TimelineObject {
}

type TaskObject {
}

type TaskRef {
}

type RunQueueRef {
}

type CpuRef {
}

type TaskRefSet {
}

type RegisterValue {
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

    owned {
        thread_context: TaskThreadContext;
    }

    processes {
        Event::SetRuntimeState(state: TaskRuntimeState) {
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
 * SchedulerObject is the reusable scheduler service type. SelectRunQueue is
 * a pure selection action: it consumes a TaskRef and returns a RunQueueRef.
 * The current UP rest_init path fixes that result to the boot CPU runqueue;
 * full select_task_rq policy is deferred.
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
                            current_task_slot_current(BootCpuCurrentTask, BootIdleTask);
                            boot_idle_task_ready(BootIdleTask, BootInitTask, BootRunQueue);
                            task_ref_targets(BootIdleTaskRef, BootIdleTask);
                            task_ref_ready(BootIdleTaskRef);
                        }

                        drives {
                            self.Action::SwitchTo(
                                prev_ref: BootIdleTaskRef,
                                next_ref: BootIdleTaskRef
                            );
                        }

                        ensures {
                            scheduler_schedule_local_interrupts_closed(self, BootCpuLocalInterrupt);
                            scheduler_runqueue_lock_held_for_schedule(self, BootRunQueue);
                            scheduler_pick_next_task_identity(self, BootRunQueue, BootIdleTask);
                            scheduler_no_task_switch_on_single_task_path(self, BootIdleTask);
                            scheduler_switch_to_committed(self, BootIdleTaskRef, BootIdleTaskRef);
                            scheduler_switch_to_identity_path(self, BootIdleTaskRef);
                            scheduler_switch_to_core_context_saved(self, BootIdleTaskRef);
                            scheduler_switch_to_core_context_restored(self, BootIdleTaskRef);
                            scheduler_first_schedule_committed(self);
                        }
                    }
                }
            }
            ensures {
                scheduler_schedule_local_interrupts_closed(self, BootCpuLocalInterrupt);
                scheduler_runqueue_lock_held_for_schedule(self, BootRunQueue);
                scheduler_pick_next_task_identity(self, BootRunQueue, BootIdleTask);
                scheduler_no_task_switch_on_single_task_path(self, BootIdleTask);
                scheduler_switch_to_committed(self, BootIdleTaskRef, BootIdleTaskRef);
                scheduler_switch_to_identity_path(self, BootIdleTaskRef);
                scheduler_switch_to_core_context_saved(self, BootIdleTaskRef);
                scheduler_switch_to_core_context_restored(self, BootIdleTaskRef);
                scheduler_first_schedule_committed(self);
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
                BootIdleTask.Action::SaveCoreContext;
                BootIdleTask.Action::RestoreCoreContext;
            }
            ensures {
                scheduler_switch_to_committed(self, prev_ref, next_ref);
                scheduler_switch_to_core_context_saved(self, prev_ref);
                scheduler_switch_to_core_context_restored(self, next_ref);
            }
            deferred {
                "当前 SwitchTo 只建立 RISC-V __switch_to 核心寄存器保存/恢复框架；真实栈切换、last 返回值、FPU/vector、MM 切换、finish_task_switch 钩子和 prev != next 路径后续展开。";
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
            }
            deferred {
                "当前 SelectRunQueue 固定返回 Boot CPU runqueue；完整 select_task_rq 策略后续展开。";
            }
        }
    }
}

/*
 * RunQueue is a top-level scheduler runqueue abstraction. The current model
 * stores task_refs as a temporary aggregate view; future CFS/RT/DL scheduler
 * class queues should own concrete membership, with RunQueue.task_refs derived
 * from those queues. EnqueueTask commits the local membership fact
 * runqueue_contains_task(self, task_ref); phase-level sequencing may derive
 * task_enqueued_on_runqueue(task_ref, runqueue_ref) after selection and enqueue
 * both succeed.
 */
type RunQueue: TaskObject {
    ext_state: RunQueueRuntimeState;
    task_refs: TaskRefSet;

    processes {
        Event::EnqueueTask(task_ref: TaskRef) {
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
    }
}

type CurrentCPU {
    owned {
        cpu: CPUObject;
    }

    lifecycle {
        Event::Preset {
            state_effect: StateEffect::Always;
            ensures {
                current_cpu_self_identity_ready(self);
            }
        }

        Event::Setup {
            state_effect: StateEffect::Always;
            ensures {
                current_cpu_self_identity_ready(self);
            }
        }

        Event::Enable {
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
        Event::Disable {
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

        Event::Enable {
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

        Event::SaveAndDisable {
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

        Event::Restore {
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
        Event::Disable {
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

        Event::Enable {
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

        Event::EnableNoResched {
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
        Event::Setup {
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

        Event::LockIrqSave {
            state_effect: StateEffect::Conditional;
            drives {
                current_cpu.cpu.LocalInterruptControl.Event::SaveAndDisable(out flags);
                current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Event::Disable;
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

        Event::UnlockIrqRestore {
            state_effect: StateEffect::Conditional;
            drives {
                self.Action::Release;
                current_cpu.cpu.LocalInterruptControl.Event::Restore(flags);
                current_cpu.cpu.CurrentTaskSlot.current_task.PreemptionControl.Event::Enable;
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

type CompletionTokenCount {
}

type SimpleWaitQueue {
    lifecycle {
        Event::Setup {
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
        Event::Preset {
            state_effect: StateEffect::Always;
            ensures {
                completion_storage_bound(self);
                completion_owns_wait_queue(self);
            }
        }

        Event::Setup {
            state_effect: StateEffect::Always;
            drives {
                self.wait_queue.Event::Setup;
            }
            ensures {
                completion_ready(self);
                completion_pending(self);
                completion_done_count_is_zero(self);
                completion_wait_queue_ready(self);
            }
        }

        Event::Enable {
            state_effect: StateEffect::Always;
            ensures {
                completion_online(self);
                completion_handle_published(self);
            }
        }

        Event::Disable {
            state_effect: StateEffect::Conditional;
            ensures {
                completion_handle_revoked(self);
            }
        }
    }

    processes {
        Event::Complete {
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
            }
            result {
                Pending: Success(token_produced);
                Completed: Success(token_produced);
                CompletedAll: Success(no_change);
            }
        }

        Event::CompleteAll {
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

        Event::Wait {
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

        Event::TryWait {
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

        Event::Reinit {
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
