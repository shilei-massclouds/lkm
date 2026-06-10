/*
 * Generic allocator model.
 *
 * PageAllocatorType is the reusable boundary for Linux-like buddy page
 * allocation. Its lifecycle is still provided by the concrete PageAllocator
 * instance in mm_core_init(); the runtime actions here are only valid after
 * that instance reaches Ready.
 */

type PageOrder {
}

type GfpFlags {
}

type PageRef {
}

predicate page_allocator_alloc_pages_api_ready<T>(allocator: T) -> bool;
predicate page_allocator_free_pages_api_ready<T>(allocator: T) -> bool;
predicate page_allocator_can_allocate_order<T, O>(allocator: T, order: O) -> bool;
predicate page_allocator_gfp_allowed<T, G>(allocator: T, gfp: G) -> bool;
predicate page_allocator_alloc_pages_called<T, O, G>(allocator: T, order: O, gfp: G) -> bool;
predicate page_allocator_alloc_pages_returns<T, R, O, G>(
    allocator: T,
    page_ref: R,
    order: O,
    gfp: G
) -> bool;
predicate page_allocator_free_pages_called<T, R, O>(allocator: T, page_ref: R, order: O) -> bool;
predicate page_allocator_free_pages_committed<T, R, O>(allocator: T, page_ref: R, order: O) -> bool;
predicate page_ref_ready<T>(page_ref: T) -> bool;
predicate page_ref_targets_buddy_pages<T, A>(page_ref: T, allocator: A) -> bool;
predicate page_ref_order_bound<T, O>(page_ref: T, order: O) -> bool;
predicate page_ref_linear_mapped<T>(page_ref: T) -> bool;
predicate page_ref_exclusively_owned_by_caller<T>(page_ref: T) -> bool;
predicate page_ref_released_to_allocator<T, A>(page_ref: T, allocator: A) -> bool;

type PageAllocatorType: MemoryObject {
    processes {
        /*
         * AllocPages corresponds to Linux alloc_pages(gfp, order), expressed
         * with the model's preferred argument order (order, gfp). The returned
         * PageRef is a caller-owned reference to 2^order contiguous buddy pages.
         */
        Action::AllocPages(order: PageOrder, gfp: GfpFlags) -> PageRef {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_alloc_pages_api_ready(self);
                buddy_free_page_sets_populated(self, Zones);
                page_allocator_can_allocate_order(self, order);
                page_allocator_gfp_allowed(self, gfp);
            }
            ensures {
                page_allocator_alloc_pages_called(self, order, gfp);
            }
            result {
                Available: Success(page_ref_returned);
                NoMemory: Failed(no_buddy_pages_available);
            }
            deferred {
                "当前 AllocPages 规格只展开成功返回 PageRef 的使用路径；GFP reclaim/compaction/oom 和 NULL/ERR 失败传播后续随完整内存压力模型展开。";
            }
        }

        /*
         * FreePages corresponds to Linux free_pages()/__free_pages(). The
         * order must match the order used when the PageRef was allocated.
         */
        Action::FreePages(page_ref: PageRef, order: PageOrder) {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Ready;
                page_allocator_free_pages_api_ready(self);
                page_ref_ready(page_ref);
                page_ref_targets_buddy_pages(page_ref, self);
                page_ref_order_bound(page_ref, order);
                page_ref_exclusively_owned_by_caller(page_ref);
            }
            ensures {
                page_allocator_free_pages_called(self, page_ref, order);
                page_allocator_free_pages_committed(self, page_ref, order);
                page_ref_released_to_allocator(page_ref, self);
            }
        }
    }
}
