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
