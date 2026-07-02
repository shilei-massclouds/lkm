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
    parent: BootInitTask;

    /*
     * Base 表示根流只进入模型空间，尚未完成入口前导期的执行约束预置。
     */
    state State::Base {
        transitions {
            /*
             * Preset 禁止内核态使用 FPU 和 VECTOR，建立根流的早期安全执行条件。
             */
            on Transition::Preset -> State::Prepared {
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
 * BootInitTask 表示入口前导期的根任务对象。它在 Preset 中绑定 Linux 静态 init_task
 * 存储，并承载根流的任务身份。
 */
object BootInitTask: TaskObject {
    initial_state: State::Base;
    source: static::linux_6_12;

    attrs {
        storage: ObjectStorage<BootInitTask>;
    }

    reference linux_6_12 {
        storage = symbol("init_task");
    }

    /*
     * Base 表示根任务对象尚未绑定到当前执行 hart 的任务指针。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立物理地址阶段的根任务指针。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.tp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_object_storage(storage);
                    valid_task_storage(storage);
                    Riscv64.tp == phys_addr(BootInitTask.storage);
                    task_preemption_control_ready(BootInitTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootInitTask);
                    task_preemption_disabled(BootInitTask);
                    task_ref_targets(BootInitTaskRef, BootInitTask);
                    task_ref_ready(BootInitTaskRef);
                }
            }
        }
    }

    /*
     * Prepared 表示 tp 已经指向 init_task 的物理地址，可支撑物理地址阶段继续执行。
     * init_task.thread_info.preempt_count 仍保持 INIT_PREEMPT_COUNT，使调度器运行前
     * 内核抢占关闭。
     */
    state State::Prepared {
        invariant {
            attrs_accessible(self);
            valid_object_storage(storage);
            valid_task_storage(storage);
            Riscv64.tp == phys_addr(BootInitTask.storage);
            valid_task_ref(Riscv64.tp);
            task_preemption_control_ready(BootInitTask);
            task_preempt_count_initialized_to_init_preempt_count(BootInitTask);
            task_preemption_disabled(BootInitTask);
            task_ref_targets(BootInitTaskRef, BootInitTask);
            task_ref_ready(BootInitTaskRef);
        }

        transitions {
            /*
             * Enable 在早期虚拟地址空间可用后，将根任务指针切换为虚拟地址。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                may_change {
                    Riscv64.tp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_object_storage(storage);
                    valid_task_storage(storage);
                    Riscv64.tp == virt_addr(BootInitTask.storage, EarlyVm, KernelImageMap);
                    task_preemption_control_ready(BootInitTask);
                    task_preempt_count_initialized_to_init_preempt_count(BootInitTask);
                    task_preemption_disabled(BootInitTask);
                    task_ref_targets(BootInitTaskRef, BootInitTask);
                    task_ref_ready(BootInitTaskRef);
                }
            }
        }
    }

    /*
     * Online 表示根任务指针已经使用 EarlyVm 中的内核映像虚拟区域地址，且调度器运行前的
     * 初始抢占关闭状态仍被保留。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_object_storage(storage);
            valid_task_storage(storage);
            Riscv64.tp == virt_addr(BootInitTask.storage, EarlyVm, KernelImageMap);
            valid_task_ref(Riscv64.tp);
            task_preemption_control_ready(BootInitTask);
            task_preempt_count_initialized_to_init_preempt_count(BootInitTask);
            task_preemption_disabled(BootInitTask);
            task_ref_targets(BootInitTaskRef, BootInitTask);
            task_ref_ready(BootInitTaskRef);
        }
    }
}

/*
 * BootInitStack 表示入口前导期根任务使用的静态根栈。它约束 sp 在物理地址阶段和早期虚拟地址阶段的取值。
 */
object BootInitStack: StackObject {
    initial_state: State::Base;
    parent: BootInitTask;

    attrs {
        range: Derived<AddrRange, range(Lds.init_stack_start, Lds.init_stack_end)>;
    }

    /*
     * Base 表示根栈范围已可由链接布局描述，但 sp 尚未指向该栈。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立物理地址阶段的根栈指针，并预留 pt_regs 区域。
             * 该动作分两步：先让 sp 指向静态根栈高端，再减去 Config.pt_size_on_stack。
             */
            on Transition::Preset -> State::Prepared {
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

        transitions {
            /*
             * Setup 在早期虚拟地址空间可用后，将根栈指针切换为虚拟地址。
             */
            on Transition::Setup -> State::Ready {
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

        transitions {
            /*
             * Enable 在 start_kernel() 早期建立根栈边界和溢出保护状态。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                ensures {
                    init_stack_guard_ready(BootInitStack);
                }
            }
        }
    }

    /*
     * Online 表示根栈不仅地址可用，而且已经建立入口后继期要求的栈保护状态。
     */
    state State::Online {
        invariant {
            init_stack_guard_ready(BootInitStack);
        }
    }
}

/*
 * EventStream 表示 boot CPU 上异常和中断共享的陷入总入口对象。它先建立物理地址阶段的 early 兜底入口；
 * VM 切换过程允许临时借用 stvec 作为重定位落点；EarlyVm 生效后再切换到 formal 总入口，
 * 由 formal 入口根据 scause 分流到本 EventStream 下的 ExceptionStream 或 InterruptStream。
 * formal 入口只执行第一层分流；具体 cause 到处理策略的绑定由两个子流维护。
 *
 * 当前具名 EventStream 是 boot CPU 的迁移期实例；后续 AP 进入 secondary entry 后，
 * 每个 live CPU 都应建立自己的 EventStream，并通过它关联自己的 InterruptStream/ExceptionStream。
 */
object EventStream: FlowObject {
    initial_state: State::Base;
    source: static::linux_6_12;

    attrs {
        early_event_entry: FunctionSymbol<EventEntryPrototype>;
        formal_event_entry: FunctionSymbol<EventEntryPrototype>;
    }

    reference linux_6_12 {
        early_event_entry = symbol(".Lsecondary_park");
        formal_event_entry = symbol("handle_exception");
    }

    /*
     * Base 表示事件入口尚未被设置。
     */
    state State::Base {
        transitions {
            /*
             * Preset 设置物理地址阶段的 early 总入口。
             * early_event_entry 必须只依赖物理地址阶段可访问的代码和静态只读数据；
             * 不得依赖分页、动态内存、percpu、正式 printk 或已展开设备树。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                }

                may_change {
                    Riscv64.stvec;
                }

                ensures {
                    attrs_accessible(self);
                    valid_function_symbol(early_event_entry);
                    Riscv64.stvec == phys_addr(EventStream.early_event_entry);
                    early_event_entry_phys_safe(EventStream.early_event_entry);
                    event_stream_early_entry_available(EventStream, EventStream.early_event_entry);
                    cpu_event_stream_ready(BootCPU, EventStream);
                }
            }
        }
    }

    /*
     * Prepared 表示 early 总入口能力已经建立。非 VM 切换临界区中，stvec 指向 early_event_entry；
     * Vm.Setup / TrampolineVm.Enable 可以临时接管 stvec 作为物理到虚拟地址过渡的重定位落点。
     */
    state State::Prepared {
        invariant {
            attrs_accessible(self);
            valid_function_symbol(early_event_entry);
            early_event_entry_phys_safe(EventStream.early_event_entry);
            event_stream_early_entry_available(EventStream, EventStream.early_event_entry);
            cpu_event_stream_ready(BootCPU, EventStream);
        }

        transitions {
            /*
             * Setup 设置 EarlyVm 中的 formal 总入口，并清零 sscratch。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Vm.state == State::Ready;
                    ExceptionStream.state == State::Prepared;
                    InterruptStream.state == State::Prepared;
                }

                may_change {
                    Riscv64.stvec;
                    Riscv64.sscratch;
                }

                ensures {
                    attrs_accessible(self);
                    valid_function_symbol(formal_event_entry);
                    Riscv64.stvec == virt_addr(EventStream.formal_event_entry, EarlyVm, KernelImageMap);
                    Riscv64.sscratch == 0;
                    event_stream_dispatch_ready(EventStream, ExceptionStream, InterruptStream);
                    cpu_event_stream_ready(BootCPU, EventStream);
                    cpu_exception_stream_ready(BootCPU, ExceptionStream);
                    cpu_interrupt_stream_ready(BootCPU, InterruptStream);
                }
            }
        }
    }

    /*
     * Ready 表示事件入口已经切换到 EarlyVm 中的 formal 总入口，并具备异常/中断第一层分流能力。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_function_symbol(formal_event_entry);
            Riscv64.stvec == virt_addr(EventStream.formal_event_entry, EarlyVm, KernelImageMap);
            Riscv64.sscratch == 0;
            event_stream_dispatch_ready(EventStream, ExceptionStream, InterruptStream);
            cpu_event_stream_ready(BootCPU, EventStream);
            cpu_exception_stream_ready(BootCPU, ExceptionStream);
            cpu_interrupt_stream_ready(BootCPU, InterruptStream);
        }
    }
}

/*
 * InterruptStream 表示 EventStream 分流后的异步中断流控制对象。它在本阶段先封闭中断进入路径，
 * 不开放真实中断处理。中断流维护 interrupt cause 到 handler policy 的绑定；默认绑定为 panic/halt，
 * 后续具体中断类型启用时再替换对应 cause 的处理策略。
 *
 * 职责分层：InterruptStream 直接管理 RISC-V sie/sip 这类 source enable / pending 分开关；
 * boot CPU 本地中断总开关 sstatus.SIE 由 BootCpuLocalInterrupt 直接管理。InterruptStream
 * 可以在生命周期 transition中驱动 BootCpuLocalInterrupt，但不得绕过它直接改变总开关。
 *
 * 当前具名 InterruptStream 是 boot CPU 的中断流实例，并通过 parent EventStream
 * 间接关联到 BootCPU。
 */
object InterruptStream: FlowObject {
    initial_state: State::Base;
    parent: EventStream;

    /*
     * Base 表示中断控制对象尚未完成入口前导期的屏蔽动作。
     */
    state State::Base {
        transitions {
            /*
             * Preset 清零 sie/sip source enable / pending 分开关，避免入口前导期被异步中断打断。
             * 这不表示直接操作 sstatus.SIE 总开关；总开关由 BootCpuLocalInterrupt 管理。
             */
            on Transition::Preset -> State::Prepared {
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
                    interrupt_fallback_panic_ready(InterruptStream);
                    interrupt_handler_bindings_default_panic(InterruptStream);
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
            interrupt_fallback_panic_ready(InterruptStream);
            interrupt_handler_bindings_default_panic(InterruptStream);
        }

        transitions {
            /*
             * Setup 在 start_kernel() 早期再次防御式关闭中断总开关，但只通过
             * BootCpuLocalInterrupt 完成，不直接改变 sstatus.SIE。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    BootCpuLocalInterrupt.state == State::Ready;
                }

                drives {
                    BootCpuLocalInterrupt.Transition::Disable;
                }

                ensures {
                    Riscv64.sie == 0;
                    Riscv64.sip == 0;
                    cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
                    interrupt_dispatch_ready(InterruptStream);
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
            cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
            interrupt_dispatch_ready(InterruptStream);
        }

        transitions {
            /*
             * Enable 对应 boot CPU 的 local_irq_enable() 边界。当前模型把这一步
             * 收入中断时间准备期末尾，使 IRQ/timer smoke 可以在该阶段之后运行。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    IrqController.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    Softirq.state == State::Ready;
                    EventStream.state == State::Ready;
                }

                drives {
                    BootCpuLocalInterrupt.Transition::Enable;
                }

                ensures {
                    cpu_local_interrupts_enabled(BootCpuLocalInterrupt);
                    boot_cpu_local_interrupts_enabled(BootCPU);
                    interrupt_dispatch_ready(InterruptStream);
                }
            }
        }
    }

    /*
     * Online 表示 boot CPU 本地中断总入口已经开放。各类中断源仍由 sie
     * 子开关、IRQ domain 和设备级 enable 独立控制。
     */
    state State::Online {
        invariant {
            cpu_local_interrupts_enabled(BootCpuLocalInterrupt);
            boot_cpu_local_interrupts_enabled(BootCPU);
            interrupt_dispatch_ready(InterruptStream);
        }
    }
}

/*
 * ExceptionStream 表示事件入口下的异常流总控对象。入口前导期只建立所有异常的受控兜底，
 * 正式异常分类和分发留给后续对齐 trap_init() 的阶段。异常流维护 exception cause 到 handler policy
 * 的绑定；默认绑定为 panic/halt，子异常对象 Setup 时替换它所覆盖的 cause 集合。
 */
object ExceptionStream: FlowObject {
    initial_state: State::Base;
    parent: EventStream;

    /*
     * Base 表示异常流尚未纳入早期受控处理路径。
     */
    state State::Base {
        transitions {
            /*
             * Preset 在物理地址阶段 transition入口就绪后建立全异常兜底策略。
             * 当前兜底策略是 panic/halt，保证后续建页表和切换地址空间时误入异常仍受控。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    EventStream.state == State::Prepared;
                    BootInitStack.state == State::Prepared;
                }

                drives {
                    PageFaultException.Transition::Preset;
                    SyscallException.Transition::Preset;
                    BreakpointException.Transition::Preset;
                    UnexpectedException.Transition::Preset;
                }

                ensures {
                    exception_stream_fallback_panic_ready(ExceptionStream);
                    all_exceptions_covered_by_fallback(ExceptionStream);
                    exception_handler_bindings_default_panic(ExceptionStream);
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
            exception_handler_bindings_default_panic(ExceptionStream);
            PageFaultException.state == State::Prepared;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Prepared;
            UnexpectedException.state == State::Prepared;
        }

        transitions {
            /*
             * Setup 对齐后续 trap_init()：建立正式异常分类、分发和 handler 注册框架。
             * 该事件不属于当前入口前导期。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    EventStream.state == State::Ready;
                }

                drives {
                    PageFaultException.Transition::Setup;
                    BreakpointException.Transition::Setup;
                    UnexpectedException.Transition::Setup;
                }

                ensures {
                    exception_stream_fallback_panic_ready(ExceptionStream);
                    all_exceptions_covered_by_fallback(ExceptionStream);
                    exception_dispatch_ready(ExceptionStream);
                    exception_handler_bindings_ready(ExceptionStream);
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
            exception_handler_bindings_ready(ExceptionStream);
        }

        transitions {
            /*
             * Enable 表示当前配置要求的异常机制均已可用；例如 syscall 需要等用户态执行路径准备好。
             */
            on Transition::Enable -> State::Online {
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
        transitions {
            on Transition::Preset -> State::Prepared {
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

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Prepared;
                }

                ensures {
                    page_fault_exception_handler_ready(PageFaultException);
                    exception_causes_bound_to_handler(
                        ExceptionStream,
                        PageFaultException,
                        {instruction_page_fault, load_page_fault, store_page_fault}
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            page_fault_exception_handler_ready(PageFaultException);
        }

        transitions {
            on Transition::Enable -> State::Online {
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
        transitions {
            on Transition::Preset -> State::Prepared {
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

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Ready;
                }

                ensures {
                    syscall_exception_handler_ready(SyscallException);
                    exception_causes_bound_to_handler(
                        ExceptionStream,
                        SyscallException,
                        {user_ecall, supervisor_ecall}
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            syscall_exception_handler_ready(SyscallException);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    user_trap_return_ready();
                    SyscallTable.state == State::Ready;
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
 * BreakpointException 表示 breakpoint 调试异常对象。Setup 只建立 breakpoint trap 的 hook
 * 分发入口；具体机制通过 hook 接管后才能 resume，未知 #BR 仍进入 fallback panic/halt。
 */
object BreakpointException: FlowObject {
    initial_state: State::Base;
    parent: ExceptionStream;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
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

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Prepared;
                }

                ensures {
                    breakpoint_hook_dispatch_ready(BreakpointException);
                    exception_causes_bound_to_handler(
                        ExceptionStream,
                        BreakpointException,
                        {breakpoint}
                    );
                }
            }
        }
    }

    state State::Ready {
        invariant {
            breakpoint_hook_dispatch_ready(BreakpointException);
        }

        transitions {
            on Transition::Enable -> State::Online {
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
        transitions {
            on Transition::Preset -> State::Prepared {
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

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ExceptionStream.state == State::Prepared;
                }

                ensures {
                    unexpected_exception_handler_ready(UnexpectedException);
                    remaining_exception_causes_bound_to_handler(ExceptionStream, UnexpectedException);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            unexpected_exception_handler_ready(UnexpectedException);
        }

        transitions {
            on Transition::Enable -> State::Online {
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
        transitions {
            /*
             * Preset 使用 global_pointer 符号建立物理地址阶段的 gp。
             */
            on Transition::Preset -> State::Prepared {
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

        transitions {
            /*
             * Setup 清零 BSS 段，使内核映像进入早期可运行状态。
             */
            on Transition::Setup -> State::Ready {
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

        transitions {
            /*
             * Enable 在 EarlyVm 可用后重置 gp-relative 寻址。
             * 该事件通常由 VM.setup() 内部触发。
             */
            on Transition::Enable -> State::Online {
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
        transitions {
            /*
             * Preset 读取并验证原始 dtb 头部 magic。
             * 该验证表达规格要求，不表示 Linux setup_vm() 在此处逐项执行。
             */
            on Transition::Preset -> State::Prepared {
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

        transitions {
            /*
             * Setup 读取 total_size 并确定原始 dtb 的完整物理范围。
             * 该范围证明用于后续 FDT fixmap 容量检查和映射安全性。
             */
            on Transition::Setup -> State::Ready {
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
        transitions {
            /*
             * Preset 检查 FDT 槽位存在且能容纳 RawDtb，并把 RawDtb 安排到该槽位。
             * 该检查把 Linux 隐含在 fixmap 布局常量中的容量前提显式化。
             */
            on Transition::Preset -> State::Ready {
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
        transitions {
            /*
             * Preset 编排跳板页表和早期页表的建立。
             * Linux/RISC-V setup_vm() 在 MMU 关闭期间调用
             * apply_early_boot_alternatives()，当前模型不展开 alternatives/errata
             * text patch 细节，但必须把该同步边界标成显式 deferred。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    TrampolineVm.state == State::Base;
                    EarlyVm.state == State::Base;
                }

                drives {
                    TrampolineVm.Transition::Setup;
                    EarlyVm.Transition::Preset;
                    EarlyVm.Transition::Setup;
                }

                may_change {
                    TrampolineVm.pg_dir;
                    EarlyVm.pg_dir;
                }

                ensures {
                    riscv_early_boot_alternatives_deferred(Vm);
                    riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
                }

                deferred {
                    "apply_early_boot_alternatives() 暂缓：当前 .config 启用 CONFIG_RISCV_ALTERNATIVE_EARLY，Linux 在 setup_vm() 的 MMU-off 区间执行早期 alternatives/errata text patch；本轮只保留该边界，不建 Alternative/Patch 对象，也不把 text patch 同步语义伪装为已实现。"
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
            riscv_early_boot_alternatives_deferred(Vm);
            riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
        }

        transitions {
            /*
             * Setup 启用跳板页表和早期页表，并完成进入早期虚拟地址阶段的切换。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    TrampolineVm.state == State::Ready;
                    EarlyVm.state == State::Ready;
                }

                drives {
                    TrampolineVm.Transition::Enable;
                    EarlyVm.Transition::Enable;
                    TrampolineVm.Transition::Cleanup;
                    KernelImage.Transition::Enable;
                }

                may_change {
                    Riscv64.satp;
                    Riscv64.gp;
                    Riscv64.stvec;
                }

                ensures {
                    Riscv64.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
                    vm_transition_stvec_released_to_event_stream(Riscv64.stvec, EventStream);
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
            Riscv64.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
            early_vm_translation_sync_complete(EarlyVm);
            riscv_early_boot_alternatives_deferred(Vm);
            riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
        }

        transitions {
            /*
             * Enable 建立完整内核虚拟内存空间；该事件由入口后继期触发。
             */
            on Transition::Enable -> State::Online {
                drives {
                    SwapperVm.Transition::Setup;
                    SwapperVm.Transition::Enable;
                    EarlyVm.Transition::Cleanup;
                }

                may_change {
                    Riscv64.satp;
                    SwapperVm.pg_dir;
                }

                ensures {
                    Riscv64.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
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
            Riscv64.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
            swapper_vm_translation_sync_complete(SwapperVm);
        }
    }
}

/*
 * TrampolineVm 表示从物理地址阶段过渡到虚拟地址阶段使用的跳板虚拟内存空间。它只覆盖完成第一次切换所需的最小映射。
 */
object TrampolineVm: AddressSpaceObject {
    initial_state: State::Base;
    parent: Vm;
    source: static::linux_6_12;

    attrs {
        pg_dir: PageTableStorage;
    }

    reference linux_6_12 {
        pg_dir = symbol("trampoline_pg_dir");
    }

    /*
     * Base 表示跳板页表尚未建立。
     */
    state State::Base {
        transitions {
            /*
             * Setup 初始化跳板页表并建立第一次地址空间切换所需映射。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    Lds.state == State::Online;
                    KernelImage.state == State::Ready;
                    valid_trampoline_map(TrampolineMap);
                }

                may_change {
                    TrampolineVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
                }
            }
        }
    }

    /*
     * Ready 表示跳板页表已经具备执行第一次地址空间切换的条件。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
        }

        transitions {
            /*
             * Enable 切换到跳板页表，完成从物理地址阶段进入虚拟地址阶段的第一次过渡。
             * Linux/RISC-V 实现中，在写入 trampoline satp 前执行 sfence.vma，
             * 确保 setup_vm() 刚建立的页表项对新的地址转换可见。
             * 同一临界区会临时把 stvec 设置为虚拟 continuation，借用 trap 入口完成
             * 物理 PC 到虚拟 PC 的重定位；这不是正式异常/中断分发。
             * 规格层只保留地址转换同步要求，不把 sfence.vma 展开为独立事件。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    KernelImage.state == State::Ready;
                }

                may_change {
                    Riscv64.satp;
                    Riscv64.stvec;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    trampoline_vm_translation_sync_ready_before_satp(TrampolineVm);
                    phys_to_virt_transition_completed(TrampolineVm.pg_dir, TrampolineMap);
                    Riscv64.stvec == virt_addr(VmSwitchContinuation, TrampolineVm, TrampolineMap);
                    event_stream_stvec_temporarily_borrowed(EventStream, Vm);
                }
            }
        }
    }

    /*
     * Online 表示第一次物理到虚拟地址过渡已经完成，跳板映射仍处于服务状态。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            trampoline_vm_translation_sync_ready_before_satp(TrampolineVm);
            phys_to_virt_transition_completed(TrampolineVm.pg_dir, TrampolineMap);
            trampoline_mapping_ready(TrampolineVm.pg_dir, TrampolineMap);
        }

        transitions {
            /*
             * Cleanup 在 EarlyVm 接管后让跳板虚拟内存空间退出服务。
             * Destroyed 不表示 TrampolineVm.pg_dir 这块静态页表存储被释放。
             */
            on Transition::Cleanup -> State::Destroyed {
                depends_on {
                    EarlyVm.state == State::Online;
                }
            }
        }
    }

    /*
     * Destroyed 表示跳板虚拟内存空间退出服务，但其静态页表存储仍作为对象绑定保留。
     */
    state State::Destroyed {
        invariant {
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
    source: static::linux_6_12;

    attrs {
        pg_dir: PageTableStorage;
    }

    reference linux_6_12 {
        pg_dir = symbol("early_pg_dir");
    }

    /*
     * Base 表示早期虚拟内存空间尚未发现 RawDtb，也尚未准备 FDT fixmap 槽位。
     */
    state State::Base {
        transitions {
            /*
             * Preset 发现并验证原始 dtb，并把 RawDtb 安排到 FDT fixmap 槽位。
             * 这是规格前置证明边界，强于 Linux setup_vm() 的直接实现顺序。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Base;
                    FixMap.state == State::Base;
                }

                drives {
                    RawDtb.Transition::Preset;
                    RawDtb.Transition::Setup;
                    FixMap.Transition::Preset;
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

        transitions {
            /*
             * Setup 初始化 early_pg_dir，建立内核映像映射和原始 dtb 的 fixmap 映射。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    KernelImage.state == State::Ready;
                    RawDtb.state == State::Ready;
                    FixMap.state == State::Ready;
                    fits_in_kernel_image_map(KernelImage, KernelImageMap);
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }

                may_change {
                    EarlyVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    kernel_image_mapping_ready(EarlyVm.pg_dir, KernelImage, KernelImageMap);
                    fixmap_slot_mapping_ready(EarlyVm.pg_dir, FixMap.fdt_slot);
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
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            kernel_image_mapping_ready(EarlyVm.pg_dir, KernelImage, KernelImageMap);
            fixmap_slot_mapping_ready(EarlyVm.pg_dir, FixMap.fdt_slot);
            kernel_image_mapped_for_plain_data(KernelImage, KernelImageMap);
            LinearMap.state == State::Destroyed;
        }

        transitions {
            /*
             * Enable 切换到 early_pg_dir，使早期虚拟地址空间进入服务状态。
             * Linux/RISC-V 实现中，在写入 early/kernel satp 后执行 sfence.vma，
             * 避免继续使用只覆盖首个 superpage 的 trampoline translations。
             * 规格层只保留地址转换同步要求，不把 sfence.vma 展开为独立事件。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    TrampolineVm.state == State::Online;
                }

                may_change {
                    Riscv64.satp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    Riscv64.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
                    early_vm_translation_sync_complete(EarlyVm);
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
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            Riscv64.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
            early_vm_translation_sync_complete(EarlyVm);
            kernel_image_accessible(KernelImage, KernelImageMap);
            fixmap_slot_accessible(FixMap.fdt_slot);
        }

        transitions {
            /*
             * Cleanup 在 SwapperVm 接管后让早期虚拟内存空间退出服务。
             * Destroyed 不表示 EarlyVm.pg_dir 这块静态页表存储被释放。
             */
            on Transition::Cleanup -> State::Destroyed {
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
    source: static::linux_6_12;

    attrs {
        pg_dir: PageTableStorage;
    }

    reference linux_6_12 {
        pg_dir = symbol("swapper_pg_dir");
    }

    /*
     * Base 表示完整内核页表尚未建立。
     */
    state State::Base {
        transitions {
            /*
             * Setup 建立完整内核页表。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    MemBlock.state == State::Ready;
                }

                may_change {
                    SwapperVm.pg_dir;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap);
                    temporary_fixmap_page_table_slots_clean(SwapperVm);
                    swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm);
                    swapper_vm_final_permissions_not_split_yet(SwapperVm);
                }

                deferred {
                    "CONFIG_STRICT_KERNEL_RWX 下的最终 text/rodata/data RW/RO/NX 权限细分暂缓：setup_vm_final() 的完整线性映射和 SATP/TLB 同步已建模，最终 mark_rodata_ro()/细粒度权限域留给后续 mapping-protection 对象。"
                }
            }
        }
    }

    /*
     * Ready 表示完整内核页表已准备好等待启用。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            swapper_vm_mappings_ready(SwapperVm, MemBlock, KernelImage, LinearMap, FixMap);
            temporary_fixmap_page_table_slots_clean(SwapperVm);
            swapper_vm_strict_kernel_rwx_boundary_deferred(SwapperVm);
            swapper_vm_final_permissions_not_split_yet(SwapperVm);
        }

        transitions {
            /*
             * Enable 切换到完整内核页表。Linux/RISC-V 的 setup_vm_final()
             * 在写入 swapper_pg_dir 对应 SATP 后执行 local_flush_tlb_all()；
             * 规格层把该边界收敛为完整内核地址空间切换后的翻译同步事实。
             */
            on Transition::Enable -> State::Online {
                may_change {
                    Riscv64.satp;
                }

                ensures {
                    attrs_accessible(self);
                    valid_page_table_storage(pg_dir);
                    Riscv64.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
                    swapper_vm_current(SwapperVm);
                    swapper_vm_translation_sync_complete(SwapperVm);
                }
            }
        }
    }

    /*
     * Online 表示完整内核虚拟内存空间已经启用。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_page_table_storage(pg_dir);
            Riscv64.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
            swapper_vm_current(SwapperVm);
            swapper_vm_translation_sync_complete(SwapperVm);
        }
    }
}

/*
 * BootCurrentCPU 表示 BP 启动时天然存在的当前 CPU 自我身份入口。它独立于
 * CpuGroup，拥有当前启动 CPU 对象；过渡期内该 CPU 对象仍由 BootCPU 承载，
 * BootCPU 作为 CurrentCPU.cpu 的 bootstrap role/alias 保留。
 */
object BootCurrentCPU: CurrentCPU {
    initial_state: State::Base;

    state State::Base {
        transitions {
            /*
             * Preset 记录 BP 的自我身份入口和入口 hartid 来源。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                    BootCPU.state == State::Base;
                }

                drives {
                    BootCPU.Transition::Preset;
                    BootCpuLocalInterrupt.Transition::Setup;
                    BootCpuCurrentTask.Transition::Setup;
                }

                ensures {
                    current_cpu_self_identity_ready(BootCurrentCPU);
                    current_cpu_hartid_ready(BootCurrentCPU, BootArgs.boot_hartid);
                    current_cpu_owns_cpu(BootCurrentCPU, BootCPU);
                    current_cpu_bootstrap_role_ready(BootCurrentCPU, BootCPU);
                    cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
                    cpu_bootstrap_role(BootCPU);
                    cpu_local_interrupt_control_ready(BootCpuLocalInterrupt, BootCPU);
                    current_task_slot_ready(BootCpuCurrentTask, BootCPU);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            current_cpu_self_identity_ready(BootCurrentCPU);
            current_cpu_hartid_ready(BootCurrentCPU, BootArgs.boot_hartid);
            current_cpu_owns_cpu(BootCurrentCPU, BootCPU);
            current_cpu_bootstrap_role_ready(BootCurrentCPU, BootCPU);
            cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            cpu_bootstrap_role(BootCPU);
        }

        transitions {
            /*
             * Setup 确认 BP 的 logical CPU id。当前启动闭环固定为 0。
             */
            on Transition::Setup -> State::Ready {
                ensures {
                    current_cpu_logical_id_ready(BootCurrentCPU, 0);
                    current_cpu_owns_cpu(BootCurrentCPU, BootCPU);
                    cpu_logical_id_ready(BootCPU, 0);
                    cpu_bootstrap_role(BootCPU);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            current_cpu_hartid_ready(BootCurrentCPU, BootArgs.boot_hartid);
            current_cpu_logical_id_ready(BootCurrentCPU, 0);
            current_cpu_owns_cpu(BootCurrentCPU, BootCPU);
            current_cpu_bootstrap_role_ready(BootCurrentCPU, BootCPU);
            cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            cpu_logical_id_ready(BootCPU, 0);
            cpu_bootstrap_role(BootCPU);
        }

        transitions {
            /*
             * Enable 在 CpuGroup 建立后，把 CurrentCPU 拥有的 CPU 对象注册到
             * CpuGroup 的引用集合。CpuGroup 不拥有 CPU 本体。
             */
            on Transition::Enable -> State::Online {
                depends_on {
                    CpuGroup.state == State::Prepared;
                }

                ensures {
                    current_cpu_registered_in_cpu_group(BootCurrentCPU, CpuGroup);
                    boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
                    cpu_group_uses_logical_id_index(CpuGroup);
                    cpu_group_boot_cpu_index_zero(CpuGroup, BootCPU);
                }
            }
        }
    }

    state State::Online {
        invariant {
            current_cpu_hartid_ready(BootCurrentCPU, BootArgs.boot_hartid);
            current_cpu_logical_id_ready(BootCurrentCPU, 0);
            current_cpu_owns_cpu(BootCurrentCPU, BootCPU);
            current_cpu_registered_in_cpu_group(BootCurrentCPU, CpuGroup);
            current_cpu_bootstrap_role_ready(BootCurrentCPU, BootCPU);
            cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            cpu_logical_id_ready(BootCPU, 0);
            cpu_bootstrap_role(BootCPU);
        }
    }
}

/*
 * BootCPU 表示 BootCurrentCPU 拥有的启动 CPU 对象。后续它会退化为
 * BootCurrentCPU.cpu 的 bootstrap role/alias；当前迁移期保留具名对象。
 */
object BootCPU: CPUObject {
    initial_state: State::Base;
    parent: BootCurrentCPU;

    attrs {
        hartid: HartId;
    }

    /*
     * Base 表示启动 CPU 对象尚未记录入口参数中的物理 hartid。
     */
    state State::Base {
        transitions {
            /*
             * Preset 记录 BootCPU.hartid 来自启动 ABI。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                }

                ensures {
                    boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
                    cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
                    cpu_bootstrap_role(BootCPU);
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
            cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            cpu_bootstrap_role(BootCPU);
        }

        transitions {
            /*
             * Setup 对应 boot_cpu_init() 中 present/active 边界。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    PlatformCpuInfo.state == State::Online;
                }

                ensures {
                    platform_hart_id_valid(BootArgs.boot_hartid);
                    cpu_possible(BootCPU);
                    cpu_present(BootCPU);
                    cpu_active(BootCPU);
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
            cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            cpu_bootstrap_role(BootCPU);
            platform_hart_id_valid(BootArgs.boot_hartid);
            cpu_possible(BootCPU);
            cpu_present(BootCPU);
            cpu_active(BootCPU);
            boot_cpu_present(BootCPU);
            boot_cpu_active(BootCPU);
        }

        transitions {
            /*
             * Enable 对应 boot_cpu_init() 最终 online 边界。
             */
            on Transition::Enable -> State::Online {
                ensures {
                    cpu_online(BootCPU);
                    boot_cpu_online(BootCPU);
                    cpu_ref_targets(BootCPURef, BootCPU);
                    cpu_ref_ready(BootCPURef);
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
            cpu_hartid_ready(BootCPU, BootArgs.boot_hartid);
            cpu_bootstrap_role(BootCPU);
            platform_hart_id_valid(BootArgs.boot_hartid);
            cpu_possible(BootCPU);
            cpu_present(BootCPU);
            cpu_active(BootCPU);
            cpu_online(BootCPU);
            boot_cpu_present(BootCPU);
            boot_cpu_active(BootCPU);
            boot_cpu_online(BootCPU);
            cpu_ref_targets(BootCPURef, BootCPU);
            cpu_ref_ready(BootCPURef);
        }
    }
}

/*
 * BootCpuLocalInterrupt 是 BootCPU 的本地中断总开关控制对象，对应 RISC-V
 * sstatus.SIE。接管后只有它可以直接表示和改变 local interrupt 总开关状态；
 * InterruptStream 可以驱动它，但不能绕过它直接改变总开关。
 */
object BootCpuLocalInterrupt: LocalInterruptControl {
    initial_state: State::Base;
    parent: BootCPU;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    cpu_local_interrupt_control_ready(BootCpuLocalInterrupt, BootCPU);
                    cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpu_local_interrupt_control_ready(BootCpuLocalInterrupt, BootCPU);
            cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
        }
    }
}

/*
 * BootCpuCurrentTask 是 BootCPU 的 current task 引用槽。它不拥有任务，只保存
 * 当前 CPU 正在执行的 task 引用。
 */
object BootCpuCurrentTask: CurrentTaskSlot {
    initial_state: State::Base;
    parent: BootCPU;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    current_task_slot_ready(BootCpuCurrentTask, BootCPU);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            current_task_slot_ready(BootCpuCurrentTask, BootCPU);
        }
    }
}

/*
 * CpuGroup 表示 SoC 下的处理器管理对象。它维护 CPU 对象引用、索引和拓扑
 * 组织关系，不拥有 CPU 本体；启动 CPU 本体由 BootCurrentCPU 拥有。
 * 核心准备期再基于正式 DeviceTree 完成 setup_smp() 对应的拓扑准备。
 */
object CpuGroup: HardwareObject {
    initial_state: State::Base;
    parent: Soc;

    /*
     * Base 表示处理器管理对象尚未记录启动 CPU 引用。
     */
    state State::Base {
        transitions {
            /*
             * Preset 注册 BootCurrentCPU 拥有的 BootCPU 引用。CpuGroup 不拥有
             * BootCPU，只维护该引用和后续 topology/index 关系。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootCurrentCPU.state == State::Ready;
                    BootCPU.state == State::Prepared;
                }

                ensures {
                    current_cpu_registered_in_cpu_group(BootCurrentCPU, CpuGroup);
                    boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
                    cpu_group_uses_logical_id_index(CpuGroup);
                    cpu_group_boot_cpu_index_zero(CpuGroup, BootCPU);
                }
            }
        }
    }

    /*
     * Prepared 表示启动 CPU 已识别并挂入处理器管理对象。
     */
    state State::Prepared {
        invariant {
            current_cpu_registered_in_cpu_group(BootCurrentCPU, CpuGroup);
            boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
            cpu_group_uses_logical_id_index(CpuGroup);
            cpu_group_boot_cpu_index_zero(CpuGroup, BootCPU);
        }

        transitions {
            /*
             * Setup 对应 setup_smp()，建立 CPU 拓扑事实和 secondary CPU 候选集合。
             * 它不启动 secondary CPU，也不开放多 hart 并发。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    BootCPU.state == State::Online;
                    DeviceTree.state == State::Ready;
                    SBI.state == State::Ready;
                }

                ensures {
                    boot_cpu_managed_by_cpu_group(CpuGroup, BootCPU);
                    current_cpu_registered_in_cpu_group(BootCurrentCPU, CpuGroup);
                    cpu_group_uses_logical_id_index(CpuGroup);
                    cpu_group_boot_cpu_index_zero(CpuGroup, BootCPU);
                    cpu_group_cpu_ref_at(CpuGroup, 0, BootCPURef);
                    cpu_group_cpu_ref_targets(CpuGroup, BootCPURef, BootCPU);
                    cpu_group_possible_set_ready(CpuGroup);
                    cpu_group_present_set_ready(CpuGroup);
                    cpu_group_online_set_ready(CpuGroup);
                    cpu_group_possible_contains(CpuGroup, BootCPURef);
                    cpu_group_present_contains(CpuGroup, BootCPURef);
                    cpu_group_online_contains(CpuGroup, BootCPURef);
                    cpu_group_topology_ready(CpuGroup, DeviceTree);
                    cpu_group_boot_cpu_present(CpuGroup, BootCPU);
                    secondary_cpus_discovered(CpuGroup, DeviceTree);
                    secondary_cpus_have_unique_hartids(CpuGroup);
                    secondary_cpus_exclude_boot_cpu(CpuGroup, BootCPU);
                    secondary_cpus_possible(CpuGroup);
                    secondary_cpus_present(CpuGroup);
                    secondary_cpus_not_online(CpuGroup);
                    cpu_group_secondary_cpu_entries_ready(CpuGroup);
                    cpu_group_cpu_refs_have_unique_logical_ids(CpuGroup);
                    cpu_group_cpu_refs_have_unique_hartids(CpuGroup);
                    cpu_group_possible_cpu_boundary_ready(CpuGroup);
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
            current_cpu_registered_in_cpu_group(BootCurrentCPU, CpuGroup);
            cpu_group_uses_logical_id_index(CpuGroup);
            cpu_group_boot_cpu_index_zero(CpuGroup, BootCPU);
            cpu_group_cpu_ref_at(CpuGroup, 0, BootCPURef);
            cpu_group_cpu_ref_targets(CpuGroup, BootCPURef, BootCPU);
            cpu_group_possible_set_ready(CpuGroup);
            cpu_group_present_set_ready(CpuGroup);
            cpu_group_online_set_ready(CpuGroup);
            cpu_group_possible_contains(CpuGroup, BootCPURef);
            cpu_group_present_contains(CpuGroup, BootCPURef);
            cpu_group_online_contains(CpuGroup, BootCPURef);
            cpu_group_topology_ready(CpuGroup, DeviceTree);
            cpu_group_boot_cpu_present(CpuGroup, BootCPU);
            secondary_cpus_discovered(CpuGroup, DeviceTree);
            secondary_cpus_have_unique_hartids(CpuGroup);
            secondary_cpus_exclude_boot_cpu(CpuGroup, BootCPU);
            secondary_cpus_possible(CpuGroup);
            secondary_cpus_present(CpuGroup);
            secondary_cpus_not_online(CpuGroup);
            cpu_group_secondary_cpu_entries_ready(CpuGroup);
            cpu_group_cpu_refs_have_unique_logical_ids(CpuGroup);
            cpu_group_cpu_refs_have_unique_hartids(CpuGroup);
            cpu_group_possible_cpu_boundary_ready(CpuGroup);
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
        transitions {
            /*
             * Preset 执行 SoC 平台相关的早期预置。
             */
            on Transition::Preset -> State::Prepared {
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

        transitions {
            /*
             * Setup 按入口前导期构建时序驱动各对象状态迁移。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                drives {
                    InterruptStream.Transition::Preset;
                    KernelImage.Transition::Preset;
                    RootStream.Transition::Preset;
                    KernelImage.Transition::Setup;
                    BootCurrentCPU.Transition::Preset;
                    BootCurrentCPU.Transition::Setup;
                    CpuGroup.Transition::Preset;
                    BootCurrentCPU.Transition::Enable;
                    BootInitTask.Transition::Preset;
                    BootInitStack.Transition::Preset;
                    EventStream.Transition::Preset;
                    ExceptionStream.Transition::Preset;
                    Vm.Transition::Preset;
                    Vm.Transition::Setup;
                    EventStream.Transition::Setup;
                    BootInitTask.Transition::Enable;
                    BootInitStack.Transition::Setup;
                    Soc.Transition::Preset;
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
            EventStream.state == State::Ready;
            ExceptionStream.state == State::Prepared;
            PageFaultException.state == State::Prepared;
            SyscallException.state == State::Prepared;
            BreakpointException.state == State::Prepared;
            UnexpectedException.state == State::Prepared;
            KernelImage.state == State::Online;
            RawDtb.state == State::Ready;
            BootInitTask.state == State::Online;
            BootInitStack.state == State::Ready;
            Vm.state == State::Ready;
            TrampolineVm.state == State::Destroyed;
            EarlyVm.state == State::Online;
            BootCurrentCPU.state == State::Online;
            BootCPU.state == State::Prepared;
            CpuGroup.state == State::Prepared;
            Soc.state == State::Prepared;
        }

        transitions {
            /*
             * Cleanup 在后继阶段开始后退出入口前导期对象。
             */
            on Transition::Cleanup -> State::Destroyed {
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
