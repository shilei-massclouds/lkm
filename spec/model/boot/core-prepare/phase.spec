/*
 * Core Prepare Phase Specification
 *
 * This subphase starts after paging_init() has completed and ends at the
 * trap_init() boundary. It prepares core kernel mechanisms that require the
 * full swapper virtual address space but precede the final selected payload.
 */

/*
 * PageAllocatorPrepare 表示正式页分配器进入 mm_core_init() 之前的准备状态。
 * 本阶段只建立页分配核心的早期准备条件，不宣称完整内存分配机制已经完成。
 */
object PageAllocatorPrepare: MemoryObject {
    initial_state: State::Base;

    /*
     * Base 表示页分配器准备对象尚未承接 paging_init() 后的内存状态。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 paging_init() 之后、mm_core_init() 之前的页分配准备工作。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                }

                ensures {
                    page_allocator_prepare_ready(PageAllocatorPrepare, MemBlock, SwapperVm);
                }
            }
        }
    }

    /*
     * Ready 表示正式页分配器的前置准备已完成，但 mm_core_init() 尚未完成。
     */
    state State::Ready {
        invariant {
            page_allocator_prepare_ready(PageAllocatorPrepare, MemBlock, SwapperVm);
        }
    }
}

/*
 * ExceptionTable 表示内核异常表准备对象。它为后续异常恢复和故障定位提供排序后的表结构。
 */
object ExceptionTable: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示异常表尚未完成启动期整理。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 sort_main_extable()，整理内核主异常表。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelImage.state == State::Online;
                    Vm.state == State::Online;
                }

                ensures {
                    exception_table_ready(ExceptionTable, KernelImage);
                }
            }
        }
    }

    /*
     * Ready 表示异常表已可供后续异常处理路径查询。
     */
    state State::Ready {
        invariant {
            exception_table_ready(ExceptionTable, KernelImage);
        }
    }
}

/*
 * CorePreparePhase 表示从 paging_init() 完成后到 trap_init() 完成的核心准备子阶段。
 * 它补齐正式异常分发之前的核心机制准备，并以 ExceptionStream.Setup 作为阶段末尾边界。
 */
object CorePreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    /*
     * Base 表示核心准备期尚未开始，仍处于系统独占上下文。
     */
    state State::Base {
        events {
            /*
             * Setup 按 paging_init() 后到 trap_init() 的最小核心路径编排对象推进。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EntrySuccessorPhase.state == State::Ready;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                    MemBlock.state == State::Online;
                    ExceptionStream.state == State::Prepared;
                }

                drives {
                    PageAllocatorPrepare.Event::Setup;
                    ExceptionTable.Event::Setup;
                    ExceptionStream.Event::Setup;
                    PageFaultException.Event::Setup;
                    BreakpointException.Event::Setup;
                    UnexpectedException.Event::Setup;
                }

                ensures {
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                }

                deferred {
                    "jump_label_init() 在该位置再次出现；当前暂不区分它与入口后继期中同名调用的关系。"
                }
            }
        }
    }

    /*
     * Ready 表示 trap_init() 边界已完成，正式异常分类和分发框架已建立。
     */
    state State::Ready {
        invariant {
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
            EntrySuccessorPhase.state == State::Ready;
            PageAllocatorPrepare.state == State::Ready;
            ExceptionTable.state == State::Ready;
            ExceptionStream.state == State::Ready;
            PageFaultException.state == State::Ready;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Ready;
            UnexpectedException.state == State::Ready;
        }
    }
}
