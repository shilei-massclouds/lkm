/*
 * Entry Prelude Phase Specification
 *
 * This subphase covers the current formal model from kernel entry through
 * the point where EarlyVm is online and the entry-prelude boundary is ready.
 */

/*
 * RootStream 表示内核接管后的第一条常规流对象。它在入口前导期建立最早执行流的受控执行状态。
 */
object RootStream: FlowObject {
    initial_state: State::Base;
    parent: InitTask;

    /*
     * Base 表示根流只进入模型空间，尚未完成入口前导期的执行约束预置。
     */
    state State::Base {
        events {
            /*
             * Preset 禁止内核态使用 FPU 和 VECTOR，建立根流的早期安全执行条件。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.sstatus;
                }

                ensures {
                    kernel_fpu_disabled(Riscv64.sstatus);
                    kernel_vector_disabled(Riscv64.sstatus);
                }
            }
        }
    }

    /*
     * Prepared 表示根流已经完成入口前导期要求的体系结构状态预置。
     */
    state State::Prepared {
        invariant {
            kernel_fpu_disabled(Riscv64.sstatus);
            kernel_vector_disabled(Riscv64.sstatus);
        }
    }
}

/*
 * InitTask 表示入口前导期的根任务对象。它使用 StaticObjects.init_task 作为底层静态存储，并承载根流的任务身份。
 */
object InitTask: TaskObject {
    initial_state: State::Base;

    attrs {
        storage: ObjectRef<StaticObjects.init_task>;
    }

    /*
     * Base 表示根任务对象尚未绑定到当前执行 hart 的任务指针。
     */
    state State::Base {
        events {
            /*
             * Preset 建立物理地址阶段的根任务指针。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    StaticObjects.state == State::Online;
                    valid_task_storage(StaticObjects.init_task);
                }

                may_change {
                    Riscv64.tp;
                }

                ensures {
                    Riscv64.tp == phys_addr(StaticObjects.init_task);
                }
            }
        }
    }

    /*
     * Prepared 表示 tp 已经指向 init_task 的物理地址，可支撑物理地址阶段继续执行。
     */
    state State::Prepared {
        invariant {
            Riscv64.tp == phys_addr(StaticObjects.init_task);
            valid_task_ref(Riscv64.tp);
        }

        events {
            /*
             * Enable 在早期虚拟地址空间可用后，将根任务指针切换为虚拟地址。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                may_change {
                    Riscv64.tp;
                }

                ensures {
                    Riscv64.tp == virt_addr(StaticObjects.init_task, EarlyVm, KernelImageMap);
                }
            }
        }
    }

    /*
     * Online 表示根任务指针已经使用 EarlyVm 中的内核映像虚拟区域地址。
     */
    state State::Online {
        invariant {
            Riscv64.tp == virt_addr(StaticObjects.init_task, EarlyVm, KernelImageMap);
            valid_task_ref(Riscv64.tp);
        }
    }
}

/*
 * InitStack 表示入口前导期根任务使用的静态根栈。它约束 sp 在物理地址阶段和早期虚拟地址阶段的取值。
 */
object InitStack: StackObject {
    initial_state: State::Base;
    parent: InitTask;

    attrs {
        range: Derived<AddrRange, range(Lds.init_stack_start, Lds.init_stack_end)>;
    }

    /*
     * Base 表示根栈范围已可由链接布局描述，但 sp 尚未指向该栈。
     */
    state State::Base {
        events {
            /*
             * Preset 建立物理地址阶段的根栈指针，并预留 pt_regs 区域。
             * 该动作分两步：先让 sp 指向静态根栈高端，再减去 Config.pt_size_on_stack。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                may_change {
                    Riscv64.sp;
                }

                ensures {
                    Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack);
                }
            }
        }
    }

    /*
     * Prepared 表示 sp 已经指向根栈的物理地址阶段可用位置。
     */
    state State::Prepared {
        invariant {
            Lds.init_stack_end - Lds.init_stack_start >= Config.page_size;
            Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack);
            valid_stack_pointer(Riscv64.sp);
            inside(Riscv64.sp, Lds.init_stack_end, Lds.init_stack_start, Lds.init_stack_end);
        }

        events {
            /*
             * Setup 在早期虚拟地址空间可用后，将根栈指针切换为虚拟地址。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Vm.state == State::Ready;
                }

                may_change {
                    Riscv64.sp;
                }

                ensures {
                    Riscv64.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap);
                }
            }
        }
    }

    /*
     * Ready 表示 sp 已经使用 EarlyVm 中的内核映像虚拟区域地址。
     * Online 留给后续栈保护机制建立后的正式服务状态。
     */
    state State::Ready {
        invariant {
            Lds.init_stack_end - Lds.init_stack_start >= Config.page_size;
            Riscv64.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap);
            valid_stack_pointer(Riscv64.sp);
            inside(Riscv64.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_start, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap));
        }

        events {
            /*
             * Enable 在 start_kernel() 早期建立根栈保护状态，例如设置 stack canary。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                ensures {
                    init_stack_canary_ready(InitStack);
                }
            }
        }
    }

    /*
     * Online 表示根栈不仅地址可用，而且已经建立入口后继期要求的栈保护状态。
     */
    state State::Online {
        invariant {
            init_stack_canary_ready(InitStack);
        }
    }
}

/*
 * InterruptStream 表示入口前导期的中断控制对象。它在本阶段先封闭中断进入路径，不开放真实中断处理。
 */
object InterruptStream: FlowObject {
    initial_state: State::Base;

    /*
     * Base 表示中断控制对象尚未完成入口前导期的屏蔽动作。
     */
    state State::Base {
        events {
            /*
             * Preset 清零中断使能和挂起状态，避免入口前导期被异步中断打断。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.sie;
                    Riscv64.sip;
                }

                ensures {
                    Riscv64.sie == 0;
                    Riscv64.sip == 0;
                }
            }
        }
    }

    /*
     * Prepared 表示监管者中断入口路径已经被屏蔽。
     */
    state State::Prepared {
        invariant {
            Riscv64.sie == 0;
            Riscv64.sip == 0;
        }

        events {
            /*
             * Setup 在 start_kernel() 早期再次防御式关闭中断总开关。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.sstatus;
                }

                ensures {
                    Riscv64.sie == 0;
                    Riscv64.sip == 0;
                    supervisor_interrupts_disabled(Riscv64.sstatus);
                }
            }
        }
    }

    /*
     * Ready 表示子开关和总开关都处于关闭状态。
     */
    state State::Ready {
        invariant {
            Riscv64.sie == 0;
            Riscv64.sip == 0;
            supervisor_interrupts_disabled(Riscv64.sstatus);
        }
    }
}

/*
 * EventStream 表示中断流下的事件入口组织对象。它先建立临时陷入入口，再在早期虚拟地址空间可用后切换到正式入口。
 */
object EventStream: FlowObject {
    initial_state: State::Base;
    parent: InterruptStream;

    /*
     * Base 表示事件入口尚未被设置。
     */
    state State::Base {
        events {
            /*
             * Preset 设置物理地址阶段的临时事件入口。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    InterruptStream.state == State::Prepared;
                    StaticObjects.state == State::Online;
                }

                may_change {
                    Riscv64.stvec;
                }

                ensures {
                    Riscv64.stvec == phys_addr(StaticObjects.early_event_entry);
                }
            }
        }
    }

    /*
     * Prepared 表示 stvec 指向物理地址阶段的临时事件入口。
     */
    state State::Prepared {
        invariant {
            Riscv64.stvec == phys_addr(StaticObjects.early_event_entry);
        }

        events {
            /*
             * Enable 设置早期虚拟地址阶段的正式事件入口，并清零 sscratch。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                    StaticObjects.state == State::Online;
                }

                may_change {
                    Riscv64.stvec;
                    Riscv64.sscratch;
                }

                ensures {
                    Riscv64.stvec == virt_addr(StaticObjects.formal_event_entry, EarlyVm, KernelImageMap);
                    Riscv64.sscratch == 0;
                }
            }
        }
    }

    /*
     * Online 表示事件入口已经切换到 EarlyVm 中的正式处理入口。
     */
    state State::Online {
        invariant {
            Riscv64.stvec == virt_addr(StaticObjects.formal_event_entry, EarlyVm, KernelImageMap);
            Riscv64.sscratch == 0;
        }
    }
}

/*
 * ExceptionStream 表示事件入口下的异常流总控对象。入口前导期只建立所有异常的受控兜底，
 * 正式异常分类和分发留给后续对齐 trap_init() 的阶段。
 */
object ExceptionStream: FlowObject {
    initial_state: State::Base;
    parent: EventStream;

    /*
     * Base 表示异常流尚未纳入早期受控处理路径。
     */
    state State::Base {
        events {
            /*
             * Preset 在物理地址阶段事件入口就绪后建立全异常兜底策略。
             * 当前兜底策略是 panic/halt，保证后续建页表和切换地址空间时误入异常仍受控。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    EventStream.state == State::Prepared;
                    InitStack.state == State::Prepared;
                }

                drives {
                    PageFaultException.Event::Preset;
                    SyscallException.Event::Preset;
                    BreakpointException.Event::Preset;
                    UnexpectedException.Event::Preset;
                }

                ensures {
                    exception_stream_fallback_panic_ready(ExceptionStream);
                    all_exceptions_covered_by_fallback(ExceptionStream);
                }
            }
        }
    }

    /*
     * Prepared 表示所有异常都已经有受控兜底；未被子对象机制接管的异常进入 panic/halt。
     */
    state State::Prepared {
        invariant {
            exception_stream_fallback_panic_ready(ExceptionStream);
            all_exceptions_covered_by_fallback(ExceptionStream);
            PageFaultException.state == State::Prepared;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Prepared;
            UnexpectedException.state == State::Prepared;
        }

        events {
            /*
             * Setup 对齐后续 trap_init()：建立正式异常分类、分发和 handler 注册框架。
             * 该事件不属于当前入口前导期。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EventStream.state == State::Online;
                }

                ensures {
                    exception_stream_fallback_panic_ready(ExceptionStream);
                    all_exceptions_covered_by_fallback(ExceptionStream);
                    exception_dispatch_ready(ExceptionStream);
                }
            }
        }
    }

    /*
     * Ready 表示正式异常分类和分发框架已建立；具体异常机制是否可用由子对象 Enable 表达。
     */
    state State::Ready {
        invariant {
            exception_stream_fallback_panic_ready(ExceptionStream);
            all_exceptions_covered_by_fallback(ExceptionStream);
            exception_dispatch_ready(ExceptionStream);
        }

        events {
            /*
             * Enable 表示当前配置要求的异常机制均已可用；例如 syscall 需要等用户态执行路径准备好。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    exception_mechanisms_ready(ExceptionStream);
                }

                ensures {
                    exception_stream_online(ExceptionStream);
                }
            }
        }
    }

    /*
     * Online 表示异常流已经进入当前配置要求的正式服务状态。
     */
    state State::Online {
        invariant {
            exception_stream_online(ExceptionStream);
        }
    }
}

/*
 * PageFaultException 表示 page fault 异常对象。入口前导期只要求它被兜底 panic 覆盖；
 * 后续 VM fault 机制可用后再进入 Enable。
 */
object PageFaultException: FlowObject {
    initial_state: State::Base;
    parent: ExceptionStream;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    ExceptionStream.state == State::Base;
                    EventStream.state == State::Prepared;
                }

                ensures {
                    exception_fallback_panic_ready(PageFaultException);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            exception_fallback_panic_ready(PageFaultException);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Ready;
                }

                ensures {
                    page_fault_exception_handler_ready(PageFaultException);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_fault_exception_handler_ready(PageFaultException);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    vm_fault_recovery_ready();
                }

                ensures {
                    page_fault_exception_mechanism_ready(PageFaultException);
                }
            }
        }
    }

    state State::Online {
        invariant {
            page_fault_exception_mechanism_ready(PageFaultException);
        }
    }
}

/*
 * SyscallException 表示 ecall/syscall 异常对象。入口前导期只纳入兜底；
 * 真正 Enable 依赖用户态上下文、syscall table 和 trap return 路径。
 */
object SyscallException: FlowObject {
    initial_state: State::Base;
    parent: ExceptionStream;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    ExceptionStream.state == State::Base;
                    EventStream.state == State::Prepared;
                }

                ensures {
                    exception_fallback_panic_ready(SyscallException);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            exception_fallback_panic_ready(SyscallException);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Ready;
                }

                ensures {
                    syscall_exception_handler_ready(SyscallException);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            syscall_exception_handler_ready(SyscallException);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    user_trap_return_ready();
                    syscall_table_ready();
                }

                ensures {
                    syscall_exception_mechanism_ready(SyscallException);
                }
            }
        }
    }

    state State::Online {
        invariant {
            syscall_exception_mechanism_ready(SyscallException);
        }
    }
}

/*
 * BreakpointException 表示 breakpoint 调试异常对象。调试机制接入前，其兜底策略仍是 panic/halt。
 */
object BreakpointException: FlowObject {
    initial_state: State::Base;
    parent: ExceptionStream;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    ExceptionStream.state == State::Base;
                    EventStream.state == State::Prepared;
                }

                ensures {
                    exception_fallback_panic_ready(BreakpointException);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            exception_fallback_panic_ready(BreakpointException);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Ready;
                }

                ensures {
                    breakpoint_exception_handler_ready(BreakpointException);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            breakpoint_exception_handler_ready(BreakpointException);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    debug_exception_mechanism_ready();
                }

                ensures {
                    breakpoint_exception_mechanism_ready(BreakpointException);
                }
            }
        }
    }

    state State::Online {
        invariant {
            breakpoint_exception_mechanism_ready(BreakpointException);
        }
    }
}

/*
 * UnexpectedException 表示除机制型和调试型异常之外的意外异常集合，例如非法指令和访存错误。
 * 它的正式策略就是 panic/halt，因此 Setup/Enable 可以在后续阶段保持简单。
 */
object UnexpectedException: FlowObject {
    initial_state: State::Base;
    parent: ExceptionStream;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    ExceptionStream.state == State::Base;
                    EventStream.state == State::Prepared;
                }

                ensures {
                    exception_fallback_panic_ready(UnexpectedException);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            exception_fallback_panic_ready(UnexpectedException);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Ready;
                }

                ensures {
                    unexpected_exception_handler_ready(UnexpectedException);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            unexpected_exception_handler_ready(UnexpectedException);
        }

        events {
            on Event::Enable -> State::Online {
                ensures {
                    unexpected_exception_panic_policy_online(UnexpectedException);
                }
            }
        }
    }

    state State::Online {
        invariant {
            unexpected_exception_panic_policy_online(UnexpectedException);
        }
    }
}

/*
 * KernelImage 表示入口前导期可见的内核映像对象。它跟踪映像边界、BSS 段状态和 gp-relative 寻址状态。
 */
object KernelImage: ImageObject {
    initial_state: State::Base;

    attrs {
        start: Derived<SymbolAddr, Lds.kernel_start>;
        phys_start: PhysAddr<KernelImage>;
        end: Derived<SymbolAddr, Lds.kernel_end>;
        segments: SegmentSet<KernelImageSegment>;
    }

    /*
     * Base 表示内核映像已进入模型空间，但 gp 和 BSS 状态尚未被本阶段处理。
     */
    state State::Base {
        events {
            /*
             * Preset 使用 global_pointer 符号建立物理地址阶段的 gp。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Lds.state == State::Online;
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.gp;
                }

                ensures {
                    phys_start == phys_addr(Lds.kernel_start);
                    Riscv64.gp == phys_addr(Lds.global_pointer);
                }
            }
        }
    }

    /*
     * Prepared 表示 BSS 边界已可识别，且 gp 指向物理地址阶段的 global_pointer。
     */
    state State::Prepared {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == phys_addr(Lds.kernel_start);
            Riscv64.gp == phys_addr(Lds.global_pointer);
        }

        events {
            /*
             * Setup 清零 BSS 段，使内核映像进入早期可运行状态。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Lds.state == State::Online;
                }

                may_change {
                    memory(segments.bss.range);
                }

                ensures {
                    phys_start == phys_addr(Lds.kernel_start);
                    memory_zeroed(segments.bss.range);
                }
            }
        }
    }

    /*
     * Ready 表示 BSS 已清零，但 gp 尚未切换到早期虚拟地址。
     */
    state State::Ready {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == phys_addr(Lds.kernel_start);
            memory_zeroed(segments.bss.range);
        }

        events {
            /*
             * Enable 在 EarlyVm 可用后重置 gp-relative 寻址。
             * 该事件通常由 VM.setup() 内部触发。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    EarlyVm.state == State::Online;
                }

                may_change {
                    Riscv64.gp;
                }

                ensures {
                    phys_start == phys_addr(Lds.kernel_start);
                    Riscv64.gp == virt_addr(Lds.global_pointer, EarlyVm, KernelImageMap);
                    gp_relative_access_ready();
                }
            }
        }
    }

    /*
     * Online 表示内核映像在早期虚拟地址空间中可按 gp-relative 方式访问。
     */
    state State::Online {
        invariant {
            valid_segment_set(segments);
            segments.bss.range == range(Lds.bss_start, Lds.bss_end);
            inside(segments.bss.range.start, segments.bss.range.end, start, end);
            phys_start == phys_addr(Lds.kernel_start);
            Riscv64.gp == virt_addr(Lds.global_pointer, EarlyVm, KernelImageMap);
            gp_relative_access_ready();
        }
    }
}

/*
 * RawDtb 表示启动参数 dtb_pa 指向的原始设备树二进制。
 * 它分层验证原始 dtb 的起始物理地址、头部和完整物理范围。
 * 这是规格前置证明边界：Linux/RISC-V setup_vm() 主要先建立 FDT
 * fixmap 映射，后续 parse_dtb()/early_init_dt_scan() 再验证 header
 * 并扫描内容；本规格在 EarlyVm.Preset 前置收口这些安全前提。
 */
object RawDtb: ResourceObject {
    initial_state: State::Base;

    attrs {
        header: DtbHeader;
        header_range: PhysAddrRange<DtbHeader>;
        range: PhysAddrRange<Dtb>;
    }

    /*
     * Base 表示只知道启动参数中给出了 dtb 物理地址，尚未验证头部。
     */
    state State::Base {
        events {
            /*
             * Preset 读取并验证原始 dtb 头部 magic。
             * 该验证表达规格要求，不表示 Linux setup_vm() 在此处逐项执行。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                    firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
                }

                may_change {
                    RawDtb.header;
                    RawDtb.header_range;
                }

                ensures {
                    header_range.start == BootArgs.dtb_pa;
                    header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
                    firmware_dtb_header_accessible(header_range);
                    valid_dtb_magic(header);
                }
            }
        }
    }

    /*
     * Prepared 表示原始 dtb 头部 magic 有效，但完整范围尚未确定。
     */
    state State::Prepared {
        invariant {
            header_range.start == BootArgs.dtb_pa;
            header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
            firmware_dtb_header_accessible(header_range);
            valid_dtb_magic(header);
        }

        events {
            /*
             * Setup 读取 total_size 并确定原始 dtb 的完整物理范围。
             * 该范围证明用于后续 FDT fixmap 容量检查和映射安全性。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    OpenSbiFirmware.state == State::Online;
                    firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
                }

                may_change {
                    RawDtb.range;
                }

                ensures {
                    valid_dtb_header(header);
                    range.start == BootArgs.dtb_pa;
                    range.end == BootArgs.dtb_pa + header.total_size;
                    firmware_dtb_range_accessible(range);
                }
            }
        }
    }

    /*
     * Ready 表示原始 dtb 的头部和完整物理范围均已验证。
     */
    state State::Ready {
        invariant {
            valid_dtb_header(header);
            range.start == BootArgs.dtb_pa;
            range.end == BootArgs.dtb_pa + header.total_size;
            firmware_dtb_range_accessible(range);
        }
    }
}

/*
 * FixMap 表示入口前导期可用的固定虚拟地址槽位集合。
 * 当前只建模 FDT 槽位，并记录 RawDtb 是否已被安排到该槽位。
 * FDT 槽位容量检查是规格侧的显式前置条件；Linux 实现侧对应
 * FIX_FDT/FIX_FDT_SIZE/MAX_FDT_SIZE 布局和 create_fdt_early_page_table()
 * 的固定映射窗口。
 */
object FixMap: PrepareObject {
    initial_state: State::Base;

    attrs {
        fdt_slot: FixMapSlotRange<Fdt>;
    }

    /*
     * Base 表示 fixmap 槽位布局来自配置，但尚未把 RawDtb 安排到 FDT 槽位。
     */
    state State::Base {
        events {
            /*
             * Preset 检查 FDT 槽位存在且能容纳 RawDtb，并把 RawDtb 安排到该槽位。
             * 该检查把 Linux 隐含在 fixmap 布局常量中的容量前提显式化。
             */
            on Event::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Ready;
                    has_slot(Config.fixmap, FixMapSlot::Fdt);
                    fits_in_fixmap_slot(RawDtb.range, fdt_slot, Config.page_size);
                }

                may_change {
                    FixMap.fdt_slot;
                }

                ensures {
                    attrs_accessible(self);
                    fdt_slot == Config.fixmap.fdt;
                    slot_contains(fdt_slot, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示 FDT 槽位已经承载 RawDtb，后续页表映射可直接引用该槽位。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            fdt_slot == Config.fixmap.fdt;
            slot_contains(fdt_slot, RawDtb);
        }
    }
}

/*
 * LinearMap 表示 PAGE_OFFSET 起始的物理内存线性映射虚拟区域。
 * 入口前导期只预留该区域，完整 RAM banks 映射由后续完整页表阶段建立。
 */
object LinearMap: AddressSpaceObject {
    initial_state: State::Destroyed;
    parent: Vm;

    /*
     * Destroyed 表示线性映射虚拟区域已按布局预留，但尚未建立完整物理内存映射，也不提供当前地址转换服务。
     */
    state State::Destroyed {
        invariant {
            linear_map_area_reserved(self);
            fixmap_adjacent_to_linear_map(FixMap, LinearMap);
        }
    }
}

/*
 * Vm 表示入口前导期正在建立的内核虚拟内存空间抽象。它编排 TrampolineVm 和 EarlyVm，后续阶段再接入 SwapperVm。
 */
object Vm: AddressSpaceObject {
    initial_state: State::Base;

    /*
     * Base 表示页表子对象尚未完成入口前导期所需的准备。
     */
    state State::Base {
        events {
            /*
             * Preset 编排跳板页表和早期页表的建立。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    TrampolineVm.state == State::Base;
                    EarlyVm.state == State::Base;
                }

                drives {
                    TrampolineVm.Event::Setup;
                    EarlyVm.Event::Preset;
                    EarlyVm.Event::Setup;
                }

                may_change {
                    StaticObjects.trampoline_pg_dir;
                    StaticObjects.early_pg_dir;
                }
            }
        }
    }

    /*
     * Prepared 表示 TrampolineVm 与 EarlyVm 都已具备可启用状态。
     */
    state State::Prepared {
        invariant {
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
        }

        events {
            /*
             * Setup 启用跳板页表和早期页表，并完成进入早期虚拟地址阶段的切换。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    TrampolineVm.state == State::Ready;
                    EarlyVm.state == State::Ready;
                }

                drives {
                    TrampolineVm.Event::Enable;
                    EarlyVm.Event::Enable;
                    TrampolineVm.Event::Cleanup;
                    KernelImage.Event::Enable;
                }

                may_change {
                    Riscv64.satp;
                    Riscv64.gp;
                }

                ensures {
                    Riscv64.satp == satp_of(StaticObjects.early_pg_dir, Config.satp_mode);
                }
            }
        }
    }

    /*
     * Ready 表示控制流已经切换到 EarlyVm，入口前导期后续对象可使用早期虚拟地址。
     */
    state State::Ready {
        invariant {
            TrampolineVm.state == State::Destroyed;
            EarlyVm.state == State::Online;
            KernelImage.state == State::Online;
            Riscv64.satp == satp_of(StaticObjects.early_pg_dir, Config.satp_mode);
        }

        events {
            /*
             * Enable 建立完整内核虚拟内存空间；该事件由入口后继期触发。
             */
            on Event::Enable -> State::Online {
                drives {
                    SwapperVm.Event::Setup;
                    SwapperVm.Event::Enable;
                    EarlyVm.Event::Cleanup;
                }

                may_change {
                    Riscv64.satp;
                    StaticObjects.swapper_pg_dir;
                }

                ensures {
                    Riscv64.satp == satp_of(StaticObjects.swapper_pg_dir, Config.satp_mode);
                }
            }
        }
    }

    /*
     * Online 表示完整内核虚拟内存空间已经由 SwapperVm 接管；该状态属于后续阶段。
     */
    state State::Online {
        invariant {
            SwapperVm.state == State::Online;
            EarlyVm.state == State::Destroyed;
            Riscv64.satp == satp_of(StaticObjects.swapper_pg_dir, Config.satp_mode);
        }
    }
}

/*
 * TrampolineVm 表示从物理地址阶段过渡到虚拟地址阶段使用的跳板虚拟内存空间。它只覆盖完成第一次切换所需的最小映射。
 */
object TrampolineVm: AddressSpaceObject {
    initial_state: State::Base;
    parent: Vm;

    /*
     * Base 表示跳板页表尚未建立。
     */
    state State::Base {
        events {
            /*
             * Setup 初始化跳板页表并建立第一次地址空间切换所需映射。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                    Lds.state == State::Online;
                    KernelImage.state == State::Ready;
                    valid_trampoline_map(TrampolineMap);
                }

                may_change {
                    StaticObjects.trampoline_pg_dir;
                }

                ensures {
                    trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, TrampolineMap);
                }
            }
        }
    }

    /*
     * Ready 表示跳板页表已经具备执行第一次地址空间切换的条件。
     */
    state State::Ready {
        invariant {
            trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, TrampolineMap);
        }

        events {
            /*
             * Enable 切换到跳板页表，完成从物理地址阶段进入虚拟地址阶段的第一次过渡。
             * Linux/RISC-V 实现中，在写入 trampoline satp 前执行 sfence.vma，
             * 确保 setup_vm() 刚建立的页表项对新的地址转换可见。
             * 规格层只保留地址转换同步要求，不把 sfence.vma 展开为独立事件。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    KernelImage.state == State::Ready;
                }

                may_change {
                    Riscv64.satp;
                }

                ensures {
                    phys_to_virt_transition_completed(StaticObjects.trampoline_pg_dir, TrampolineMap);
                }
            }
        }
    }

    /*
     * Online 表示第一次物理到虚拟地址过渡已经完成，跳板映射仍处于服务状态。
     */
    state State::Online {
        invariant {
            phys_to_virt_transition_completed(StaticObjects.trampoline_pg_dir, TrampolineMap);
            trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, TrampolineMap);
        }

        events {
            /*
             * Cleanup 在 EarlyVm 接管后让跳板虚拟内存空间退出服务。
             * Destroyed 不表示 StaticObjects.trampoline_pg_dir 这块静态页表存储被释放。
             */
            on Event::Cleanup -> State::Destroyed {
                depends_on {
                    EarlyVm.state == State::Online;
                }
            }
        }
    }

    /*
     * Destroyed 表示跳板虚拟内存空间退出服务，但其静态页表存储仍由 StaticObjects 约束。
     */
    state State::Destroyed {
        invariant {
            StaticObjects.state == State::Online;
            no_service(TrampolineVm);
        }
    }
}

/*
 * EarlyVm 表示入口前导期后半段使用的早期虚拟内存空间。它映射内核映像区域和 FixMap 中承载 RawDtb 的 FDT 槽位，并保留线性映射区域。
 */
object EarlyVm: AddressSpaceObject {
    initial_state: State::Base;
    parent: Vm;

    /*
     * Base 表示早期虚拟内存空间尚未发现 RawDtb，也尚未准备 FDT fixmap 槽位。
     */
    state State::Base {
        events {
            /*
             * Preset 发现并验证原始 dtb，并把 RawDtb 安排到 FDT fixmap 槽位。
             * 这是规格前置证明边界，强于 Linux setup_vm() 的直接实现顺序。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Base;
                    FixMap.state == State::Base;
                }

                drives {
                    RawDtb.Event::Preset;
                    RawDtb.Event::Setup;
                    FixMap.Event::Preset;
                }

                ensures {
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }
            }
        }
    }

    /*
     * Prepared 表示原始 dtb 已验证，且已被安排到 FDT fixmap 槽位。
     */
    state State::Prepared {
        invariant {
            RawDtb.state == State::Ready;
            FixMap.state == State::Ready;
            slot_contains(FixMap.fdt_slot, RawDtb);
        }

        events {
            /*
             * Setup 初始化 early_pg_dir，建立内核映像映射和原始 dtb 的 fixmap 映射。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                    KernelImage.state == State::Ready;
                    RawDtb.state == State::Ready;
                    FixMap.state == State::Ready;
                    fits_in_kernel_image_map(KernelImage, KernelImageMap);
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }

                may_change {
                    StaticObjects.early_pg_dir;
                }

                ensures {
                    kernel_image_mapping_ready(StaticObjects.early_pg_dir, KernelImage, KernelImageMap);
                    fixmap_slot_mapping_ready(StaticObjects.early_pg_dir, FixMap.fdt_slot);
                    kernel_image_mapped_for_plain_data(KernelImage, KernelImageMap);
                }
            }
        }
    }

    /*
     * Ready 表示 early_pg_dir 已建立内核映像映射和 FDT fixmap 槽位映射，并保留 PAGE_OFFSET 起始的线性映射区域。
     */
    state State::Ready {
        invariant {
            kernel_image_mapping_ready(StaticObjects.early_pg_dir, KernelImage, KernelImageMap);
            fixmap_slot_mapping_ready(StaticObjects.early_pg_dir, FixMap.fdt_slot);
            kernel_image_mapped_for_plain_data(KernelImage, KernelImageMap);
            LinearMap.state == State::Destroyed;
        }

        events {
            /*
             * Enable 切换到 early_pg_dir，使早期虚拟地址空间进入服务状态。
             * Linux/RISC-V 实现中，在写入 early/kernel satp 后执行 sfence.vma，
             * 避免继续使用只覆盖首个 superpage 的 trampoline translations。
             * 规格层只保留地址转换同步要求，不把 sfence.vma 展开为独立事件。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    TrampolineVm.state == State::Online;
                }

                may_change {
                    Riscv64.satp;
                }

                ensures {
                    Riscv64.satp == satp_of(StaticObjects.early_pg_dir, Config.satp_mode);
                    kernel_image_accessible(KernelImage, KernelImageMap);
                    fixmap_slot_accessible(FixMap.fdt_slot);
                }
            }
        }
    }

    /*
     * Online 表示 EarlyVm 已启用，内核映像和 FDT fixmap 槽位可以通过早期虚拟地址访问。
     */
    state State::Online {
        invariant {
            Riscv64.satp == satp_of(StaticObjects.early_pg_dir, Config.satp_mode);
            kernel_image_accessible(KernelImage, KernelImageMap);
            fixmap_slot_accessible(FixMap.fdt_slot);
        }

        events {
            /*
             * Cleanup 在 SwapperVm 接管后让早期虚拟内存空间退出服务。
             * Destroyed 不表示 StaticObjects.early_pg_dir 这块静态页表存储被释放。
             */
            on Event::Cleanup -> State::Destroyed {
                depends_on {
                    SwapperVm.state == State::Online;
                }
            }
        }
    }

    /*
     * Destroyed 表示早期虚拟内存空间已被后续完整地址空间接管。
     */
    state State::Destroyed {
        invariant {
            StaticObjects.state == State::Online;
            no_service(EarlyVm);
        }
    }
}

/*
 * SwapperVm 表示后续阶段使用的完整内核虚拟内存空间。
 */
object SwapperVm: AddressSpaceObject {
    initial_state: State::Base;
    parent: Vm;

    /*
     * Base 表示完整内核页表尚未建立。
     */
    state State::Base {
        events {
            /*
             * Setup 建立完整内核页表。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                    MemBlock.state == State::Ready;
                }

                may_change {
                    StaticObjects.swapper_pg_dir;
                }

                ensures {
                    swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap);
                    temporary_fixmap_page_table_slots_clean(SwapperVm);
                }
            }
        }
    }

    /*
     * Ready 表示完整内核页表已准备好等待启用。
     */
    state State::Ready {
        invariant {
            swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap);
            temporary_fixmap_page_table_slots_clean(SwapperVm);
        }

        events {
            /*
             * Enable 切换到完整内核页表。
             */
            on Event::Enable -> State::Online {
                may_change {
                    Riscv64.satp;
                }

                ensures {
                    Riscv64.satp == satp_of(StaticObjects.swapper_pg_dir, Config.satp_mode);
                    swapper_vm_current(SwapperVm);
                }
            }
        }
    }

    /*
     * Online 表示完整内核虚拟内存空间已经启用。
     */
    state State::Online {
        invariant {
            Riscv64.satp == satp_of(StaticObjects.swapper_pg_dir, Config.satp_mode);
            swapper_vm_current(SwapperVm);
        }
    }
}

/*
 * BootCPU 表示当前启动 CPU 本体。后续 secondary CPU 可复用同一类 CPU 对象。
 */
object BootCPU: CPUObject {
    initial_state: State::Base;
    parent: CpuGroup;

    attrs {
        hartid: HartId;
    }

    /*
     * Base 表示启动 CPU 对象尚未记录入口参数中的物理 hartid。
     */
    state State::Base {
        events {
            /*
             * Preset 记录 BootCPU.hartid 来自启动 ABI。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                }

                ensures {
                    boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
                }
            }
        }
    }

    /*
     * Prepared 表示启动 CPU 已识别并记录物理 hartid，等待后继期 boot_cpu_init() 继续推进。
     */
    state State::Prepared {
        invariant {
            boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
        }

        events {
            /*
             * Setup 对应 boot_cpu_init() 中 present/active 边界。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    PlatformCpuInfo.state == State::Online;
                }

                ensures {
                    platform_hart_id_valid(BootArgs.boot_hartid);
                    boot_cpu_present(BootCPU);
                    boot_cpu_active(BootCPU);
                }
            }
        }
    }

    /*
     * Ready 表示启动 CPU 已标记 present/active。
     */
    state State::Ready {
        invariant {
            boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            platform_hart_id_valid(BootArgs.boot_hartid);
            boot_cpu_present(BootCPU);
            boot_cpu_active(BootCPU);
        }

        events {
            /*
             * Enable 对应 boot_cpu_init() 最终 online 边界。
             */
            on Event::Enable -> State::Online {
                ensures {
                    boot_cpu_online(BootCPU);
                }
            }
        }
    }

    /*
     * Online 表示启动 CPU 已进入本阶段需要的 online 边界。
     */
    state State::Online {
        invariant {
            boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            platform_hart_id_valid(BootArgs.boot_hartid);
            boot_cpu_present(BootCPU);
            boot_cpu_active(BootCPU);
            boot_cpu_online(BootCPU);
        }
    }
}

/*
 * CpuGroup 表示 SoC 下的处理器管理对象。入口前导期只建立启动 CPU 子对象的组织边界；
 * 核心准备期再基于正式 DeviceTree 和 CpuIdMap 完成 setup_smp() 对应的拓扑准备。
 */
object CpuGroup: HardwareObject {
    initial_state: State::Base;
    parent: Soc;

    /*
     * Base 表示处理器管理对象尚未组织启动 CPU 子对象。
     */
    state State::Base {
        events {
            /*
             * Preset 驱动 BootCPU 记录入口参数 a0 中的启动 hart 物理标识。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    BootCPU.state == State::Base;
                }

                drives {
                    BootCPU.Event::Preset;
                }

                ensures {
                    boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
                }
            }
        }
    }

    /*
     * Prepared 表示启动 CPU 已识别并挂入处理器管理对象。
     */
    state State::Prepared {
        invariant {
            boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
        }

        events {
            /*
             * Setup 对应 setup_smp()，建立逻辑 CPU 映射和 secondary CPU 候选集合。
             * 它不启动 secondary CPU，也不开放多 hart 并发。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    BootCPU.state == State::Online;
                    DeviceTree.state == State::Ready;
                    CpuIdMap.state == State::Ready;
                    SBI.state == State::Ready;
                }

                ensures {
                    boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
                    cpu_group_topology_ready(CpuGroup, DeviceTree);
                    secondary_cpus_discovered(CpuGroup, DeviceTree);
                    cpu_id_map_boot_cpu_stable(CpuIdMap, BootCPU);
                    cpu_group_concurrency_closed(CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示 CPU 拓扑事实和 secondary CPU 候选集合已经建立，但 secondary CPU 尚未 online。
     */
    state State::Ready {
        invariant {
            boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
            cpu_group_topology_ready(CpuGroup, DeviceTree);
            secondary_cpus_discovered(CpuGroup, DeviceTree);
            cpu_id_map_boot_cpu_stable(CpuIdMap, BootCPU);
            cpu_group_concurrency_closed(CpuGroup);
        }
    }
}

/*
 * Soc 表示片上系统平台对象。入口前导期只建模平台早期预置的边界。
 */
object Soc: HardwareObject {
    initial_state: State::Base;

    /*
     * Base 表示 SoC 平台早期预置尚未执行。
     */
    state State::Base {
        events {
            /*
             * Preset 执行 SoC 平台相关的早期预置。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    CpuGroup.state == State::Prepared;
                }

                may_change {
                    // 待补充：平台相关早期状态点。
                }

                ensures {
                    soc_early_platform_ready();
                }
            }
        }
    }

    /*
     * Prepared 表示 SoC 平台早期预置已完成到入口前导期所需边界。
     */
    state State::Prepared {
        invariant {
            CpuGroup.state == State::Prepared;
            soc_early_platform_ready();
        }

        deferred {
            "具体 SoC 早期平台状态点后续补充；当前入口前导期只要求 CpuGroup 已组织启动 CPU。"
        }
    }
}

/*
 * EntryPreludePhase 表示引导期的入口前导子阶段对象。它编排本子阶段内各对象的状态迁移并定义阶段边界。
 */
object EntryPreludePhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    /*
     * Base 表示入口前导期刚开始，默认上下文为系统独占。
     */
    state State::Base {
        invariant {
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
        }

        events {
            /*
             * Setup 按入口前导期构建时序驱动各对象状态迁移。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                    Lds.state == State::Online;
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                }

                drives {
                    InterruptStream.Event::Preset;
                    KernelImage.Event::Preset;
                    RootStream.Event::Preset;
                    KernelImage.Event::Setup;
                    CpuGroup.Event::Preset;
                    InitTask.Event::Preset;
                    InitStack.Event::Preset;
                    EventStream.Event::Preset;
                    ExceptionStream.Event::Preset;
                    Vm.Event::Preset;
                    Vm.Event::Setup;
                    EventStream.Event::Enable;
                    InitTask.Event::Enable;
                    InitStack.Event::Setup;
                    Soc.Event::Preset;
                }
            }
        }
    }

    /*
     * Ready 表示入口前导期编排的对象状态均已到达本阶段目标。
     */
    state State::Ready {
        invariant {
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
            RootStream.state == State::Prepared;
            InterruptStream.state == State::Prepared;
            EventStream.state == State::Online;
            ExceptionStream.state == State::Prepared;
            PageFaultException.state == State::Prepared;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Prepared;
            UnexpectedException.state == State::Prepared;
            KernelImage.state == State::Online;
            RawDtb.state == State::Ready;
            InitTask.state == State::Online;
            InitStack.state == State::Ready;
            Vm.state == State::Ready;
            TrampolineVm.state == State::Destroyed;
            EarlyVm.state == State::Online;
            BootCPU.state == State::Prepared;
            CpuGroup.state == State::Prepared;
            Soc.state == State::Prepared;
        }

        events {
            /*
             * Cleanup 在后继阶段开始后退出入口前导期对象。
             */
            on Event::Cleanup -> State::Destroyed {
                depends_on {
                    EntrySuccessorPhase.state == State::Base;
                }
            }
        }
    }

    /*
     * Destroyed 表示入口前导期对象已经退出服务，并由后继阶段接续。
     */
    state State::Destroyed {
        invariant {
            no_service(EntryPreludePhase);
        }
    }
}
