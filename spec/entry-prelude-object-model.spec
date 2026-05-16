/*
 * Entry Prelude Object Model Specification
 *
 * This file is extracted from spec/组件化内核规格.md.
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
    obj.state == State::Reserved
}

predicate fixmap_adjacent_to_linear_map<T: Object, U: Object>(fixmap: T, linear_map: U) -> bool {
    adjacent(fixmap, linear_map)
}

predicate fits_in_kernel_image_map<T: Object, U: VirtualAddressArea>(image: T, map: U) -> bool {
    contains(map, image)
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

/*
 * Riscv64 表示当前模型引用的 RISC-V 64 位体系结构对象。
 * 它提供入口前导期需要读取或改写的通用寄存器和监管者 CSR。
 */
object Riscv64: IsaObject {
    initial_state: State::Online;
    source: external_spec::riscv_isa;

    attrs {
        a0: Gpr<HartId>;
        a1: Gpr<PhysAddr<Dtb>>;
        sp: Gpr<Addr>;
        tp: Gpr<Addr>;
        gp: Gpr<Addr>;

        sstatus: Csr<Sstatus>;
        sie: Csr<Sie>;
        sip: Csr<Sip>;
        stvec: Csr<TrapVector>;
        sscratch: Csr<usize>;
        satp: Csr<Satp>;
    }

    /*
     * Online 表示体系结构对象在推导起点已经可用。
     * 本状态只要求当前模型声明的体系结构属性都可读取。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
        }
    }
}

/*
 * BootArgs 表示启动 ABI 对入口寄存器的语义解释。
 * RISC-V64 下 a0 是启动 hartid，a1 是原始 dtb 物理地址。
 */
object BootArgs: PrepareObject {
    initial_state: State::Online;

    attrs {
        boot_hartid: HartId;
        dtb_pa: PhysAddr<Dtb>;
    }

    /*
     * Online 表示启动参数在入口前导期开始前已经由启动 ABI 给出。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            boot_hartid == Riscv64.a0;
            dtb_pa == Riscv64.a1;
        }
    }
}

/*
 * Lds 表示链接脚本形成的内核映像布局对象。
 * 它提供符号地址、BSS 边界、根栈边界和内核映像边界。
 */
object Lds: PrepareObject {
    initial_state: State::Online;

    attrs {
        global_pointer: SymbolAddr;
        bss_start: SymbolAddr;
        bss_end: SymbolAddr;
        init_stack_start: SymbolAddr;
        init_stack_end: SymbolAddr;
        kernel_start: SymbolAddr;
        kernel_end: SymbolAddr;
    }

    /*
     * Online 表示链接布局在入口前导期开始前已经确定。
     * 本状态约束关键符号存在、范围有序且栈边界页对齐。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            global_pointer != 0;
            kernel_start != 0;
            kernel_end > kernel_start;
            bss_start != 0;
            bss_end > bss_start;
            inside(bss_start, bss_end, kernel_start, kernel_end);
            init_stack_start != 0;
            init_stack_end > init_stack_start;
            page_aligned(init_stack_start);
            page_aligned(init_stack_end);
        }
    }

    reference linux_6_12_37 {
        global_pointer = symbol("__global_pointer$");
        kernel_start = symbol("_start");
        kernel_end = symbol("_end");
        init_stack_start = symbol("init_thread_union");
        init_stack_end = expr("init_thread_union + THREAD_SIZE");
    }
}

/*
 * StaticObjects 表示构建和静态初始化阶段已经分配好的静态对象集合。
 * 入口前导期只引用这些底层存储或函数符号，不动态创建它们。
 */
object StaticObjects: PrepareObject {
    initial_state: State::Online;

    attrs {
        init_task: ObjectStorage<InitTask>;
        early_event_entry: FunctionSymbol<EventEntryPrototype>;
        formal_event_entry: FunctionSymbol<EventEntryPrototype>;
        trampoline_pg_dir: PageTableStorage;
        early_pg_dir: PageTableStorage;
        swapper_pg_dir: PageTableStorage;
    }

    /*
     * Online 表示静态对象集合在推导起点已经可引用。
     * 本状态检查静态任务存储、事件入口符号和页表存储的基本有效性。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_object_storage(init_task);
            valid_function_symbol(early_event_entry);
            valid_function_symbol(formal_event_entry);
            valid_page_table_storage(trampoline_pg_dir);
            valid_page_table_storage(early_pg_dir);
            valid_page_table_storage(swapper_pg_dir);
        }
    }

    reference linux_6_12_37 {
        init_task = symbol("init_task");
        early_event_entry = symbol(".Lsecondary_park");
        formal_event_entry = symbol("handle_exception");
        trampoline_pg_dir = symbol("trampoline_pg_dir");
        early_pg_dir = symbol("early_pg_dir");
        swapper_pg_dir = symbol("swapper_pg_dir");
    }
}

/*
 * Config 表示入口前导期可见的构建配置和静态参数。
 * 它约束页大小、内核虚拟区域、地址转换模式和 fixmap 布局。
 */
object Config: PrepareObject {
    initial_state: State::Online;

    attrs {
        page_size: Size;
        pt_size_on_stack: Size;
        pmd_size: Size;
        kernel_link_addr: VirtAddr<KernelImage>;
        kernel_image_va_window_size: Size;
        satp_mode: SatpMode;
        fixmap: FixMapConfig;
    }

    /*
     * Online 表示配置对象在入口前导期开始前已经确定。
     * 本状态保证当前推导依赖的配置项存在并满足基本边界约束。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            page_size > 0;
            pmd_size >= page_size;
            aligned(pmd_size, page_size);
            pt_size_on_stack > 0;
            pt_size_on_stack < page_size;
            kernel_link_addr != 0;
            page_aligned(kernel_link_addr);
            kernel_image_va_window_size > 0;
            kernel_image_va_window_size >= pmd_size;
            valid_satp_mode(satp_mode);
            valid_fixmap_config(fixmap);
        }
    }

    reference linux_6_12_37 {
        kernel_link_addr = symbol("KERNEL_LINK_ADDR");
        kernel_image_va_window_size = symbol("SZ_2G");
    }
}

/*
 * PhysicalMemory 表示平台提供的物理 RAM 和设备 I/O 地址布局。
 * 它是只读准备期输入，入口前导期只能读取和验证其范围约束。
 */
object PhysicalMemory: PrepareObject {
    initial_state: State::Online;
    access: Access::ReadOnly;

    attrs {
        ram: PhysRangeSet<Ram>;
        iomap: PhysRangeSet<Io>;
    }

    /*
     * Online 表示物理资源布局在推导起点已经可读取。
     * 本状态要求 RAM 和 I/O 范围结构良好、非空且互不重叠。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            readonly(self);
            valid_phys_range_set(ram);
            valid_phys_range_set(iomap);
            disjoint(ram, iomap);
        }
    }
}

/*
 * StartupTimeline 表示当前模型的内核启动时间轴对象。
 * 它临时编排准备期和当前已经展开的引导期阶段，并在内核启动完成后退出。
 */
object StartupTimeline: TimelineObject {
    initial_state: State::Base;

    /*
     * Base 表示顶层启动阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        events {
            /*
             * Setup 先推进准备期边界，再推进当前已经展开的引导期阶段。
             */
            on Event::Setup -> State::Ready {
                drives {
                    PreparePhase.Event::Setup;
                    PreparePhase.Event::Enable;
                    BootPhase.Event::Setup;
                }
            }
        }
    }

    /*
     * Ready 表示准备期边界已经生效，且当前已经展开的引导期阶段已经完成。
     */
    state State::Ready {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
        }
    }
}

/*
 * PreparePhase 表示准备期阶段对象。
 * 当前模型不展开准备期内部过程，只验证入口前导期依赖的准备期输入对象均已在线且满足各自不变量。
 */
object PreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    /*
     * Base 表示准备期阶段对象已经进入模型空间，但尚未形成当前规格所需的准备期完成边界。
     */
    state State::Base {
        events {
            /*
             * Setup 汇总并验证入口前导期所需的准备期输入对象，形成准备期完成边界。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    Lds.state == State::Online;
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                    PhysicalMemory.state == State::Online;
                }
            }
        }
    }

    /*
     * Ready 表示入口前导期所需的准备期输入对象均已通过阶段边界验证。
     */
    state State::Ready {
        invariant {
            Riscv64.state == State::Online;
            Lds.state == State::Online;
            StaticObjects.state == State::Online;
            Config.state == State::Online;
            PhysicalMemory.state == State::Online;
        }

        events {
            /*
             * Enable 将准备期完成边界发布为后续引导期可依赖的输入边界。
             * 当前不执行额外动作，只保留阶段生命周期中的显式边界。
             */
            on Event::Enable -> State::Online {
            }
        }
    }

    /*
     * Online 表示准备期输入边界已经对后续引导期生效。
     */
    state State::Online {
        invariant {
            Riscv64.state == State::Online;
            Lds.state == State::Online;
            StaticObjects.state == State::Online;
            Config.state == State::Online;
            PhysicalMemory.state == State::Online;
        }
    }
}

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
             * Enable 在早期虚拟地址空间可用后，将根栈指针切换为虚拟地址。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    Vm.state == State::Ready;
                }

                may_change {
                    Riscv64.sp;
                }
            }
        }
    }

    /*
     * Online 表示 sp 已经使用 EarlyVm 中的内核映像虚拟区域地址。
     */
    state State::Online {
        invariant {
            Lds.init_stack_end - Lds.init_stack_start >= Config.page_size;
            Riscv64.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap);
            valid_stack_pointer(Riscv64.sp);
            inside(Riscv64.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_start, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap));
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
                    valid_function_symbol(StaticObjects.early_event_entry);
                }

                may_change {
                    Riscv64.stvec;
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
                    valid_function_symbol(StaticObjects.formal_event_entry);
                }

                may_change {
                    Riscv64.stvec;
                    Riscv64.sscratch;
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
 * KernelImage 表示入口前导期可见的内核映像对象。它跟踪映像边界、BSS 段状态和 gp-relative 寻址状态。
 */
object KernelImage: ImageObject {
    initial_state: State::Base;

    attrs {
        start: Derived<SymbolAddr, Lds.kernel_start>;
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
            Riscv64.gp == virt_addr(Lds.global_pointer, EarlyVm, KernelImageMap);
            gp_relative_access_ready();
        }
    }
}

/*
 * RawDtb 表示启动参数 dtb_pa 指向的原始设备树二进制。
 * 它分层验证原始 dtb 的起始物理地址、头部和完整物理范围。
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
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    BootArgs.state == State::Online;
                    PhysicalMemory.state == State::Online;
                    header_range.start == BootArgs.dtb_pa;
                    header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>();
                    contains(PhysicalMemory.ram, header_range);
                }
            }
        }
    }

    /*
     * Prepared 表示原始 dtb 头部 magic 有效，但完整范围尚未确定。
     */
    state State::Prepared {
        invariant {
            valid_dtb_magic(header);
        }

        events {
            /*
             * Setup 读取 total_size 并确定原始 dtb 的完整物理范围。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    PhysicalMemory.state == State::Online;
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
            contains(PhysicalMemory.ram, range);
        }
    }
}

/*
 * FixMap 表示入口前导期可用的固定虚拟地址槽位集合。
 * 当前只建模 FDT 槽位，并记录 RawDtb 是否已被安排到该槽位。
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
             */
            on Event::Preset -> State::Ready {
                depends_on {
                    Config.state == State::Online;
                    RawDtb.state == State::Ready;
                    has_slot(Config.fixmap, FixMapSlot::Fdt);
                    fdt_slot == Config.fixmap.fdt;
                    fits_in_fixmap_slot(RawDtb.range, fdt_slot, Config.page_size);
                }

                may_change {
                    FixMap.fdt_slot;
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
    initial_state: State::Reserved;
    parent: Vm;

    /*
     * Reserved 表示线性映射虚拟区域已按布局预留，但尚未建立完整物理内存映射。
     */
    state State::Reserved {
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
             * Enable 建立完整内核虚拟内存空间；入口前导期不触发该事件。
             */
            on Event::Enable -> State::Online {
                deferred {
                    "当前入口前导期不会触发该事件；它属于后续阶段，用于建立完整虚拟内存空间。"
                }

                drives {
                    SwapperVm.Event::Setup;
                    SwapperVm.Event::Enable;
                    EarlyVm.Event::Cleanup;
                }

                may_change {
                    Riscv64.satp;
                    StaticObjects.swapper_pg_dir;
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
                    page_aligned(StaticObjects.trampoline_pg_dir);
                    Lds.state == State::Online;
                    aligned(Lds.kernel_start, Config.pmd_size);
                    valid_virt_addr(Config.kernel_link_addr);
                    valid_satp_mode(Config.satp_mode);
                }

                may_change {
                    StaticObjects.trampoline_pg_dir;
                }
            }
        }
    }

    /*
     * Ready 表示跳板页表已经具备执行第一次地址空间切换的条件。
     */
    state State::Ready {
        invariant {
            trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, Lds.kernel_start, Config.kernel_link_addr);
        }

        events {
            /*
             * Enable 切换到跳板页表，完成从物理地址阶段进入虚拟地址阶段的第一次过渡。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    KernelImage.state == State::Ready;
                }

                may_change {
                    Riscv64.satp;
                }
            }
        }
    }

    /*
     * Online 表示第一次物理到虚拟地址过渡已经完成，跳板映射仍处于服务状态。
     */
    state State::Online {
        invariant {
            phys_to_virt_transition_completed();
            trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, Lds.kernel_start, Config.kernel_link_addr);
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
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                    PhysicalMemory.state == State::Online;
                    RawDtb.state == State::Base;
                    FixMap.state == State::Base;
                }

                drives {
                    RawDtb.Event::Preset;
                    RawDtb.Event::Setup;
                    FixMap.Event::Preset;
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
                    page_aligned(StaticObjects.early_pg_dir);
                    fits_in_kernel_image_map(KernelImage, KernelImageMap);
                    slot_contains(FixMap.fdt_slot, RawDtb);
                }

                may_change {
                    StaticObjects.early_pg_dir;
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
            LinearMap.state == State::Reserved;
        }

        events {
            /*
             * Enable 切换到 early_pg_dir，使早期虚拟地址空间进入服务状态。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    TrampolineVm.state == State::Online;
                }

                may_change {
                    Riscv64.satp;
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
 * SwapperVm 表示后续阶段使用的完整内核虚拟内存空间。入口前导期只保留其状态机占位。
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
             * Setup 建立完整内核页表；当前入口前导期只保留占位。
             */
            on Event::Setup -> State::Ready {
                deferred {
                    "SwapperVm.Setup 属于后续阶段；当前只保留状态机占位。"
                }

                depends_on {
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                    page_aligned(StaticObjects.swapper_pg_dir);
                }

                may_change {
                    StaticObjects.swapper_pg_dir;
                }
            }
        }
    }

    /*
     * Ready 表示完整内核页表已准备好等待启用。
     */
    state State::Ready {
        events {
            /*
             * Enable 切换到完整内核页表；当前入口前导期只保留占位。
             */
            on Event::Enable -> State::Online {
                deferred {
                    "SwapperVm.Enable 属于后续阶段；当前只保留状态机占位。"
                }

                may_change {
                    Riscv64.satp;
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
        }

        deferred {
            "完整虚拟内存空间的页表内容、权限和覆盖范围约束在后续阶段补充。"
        }
    }
}

/*
 * CpuGroup 表示 SoC 下的处理器管理对象。入口前导期只记录启动 hart 的物理标识。
 */
object CpuGroup: HardwareObject {
    initial_state: State::Base;
    parent: Soc;

    attrs {
        boot_cpu_hartid: HartId;
    }

    /*
     * Base 表示处理器管理对象尚未记录启动 hart。
     */
    state State::Base {
        events {
            /*
             * Preset 记录入口参数 a0 中的启动 hart 物理标识。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    Riscv64.state == State::Online;
                    valid_hart_id(Riscv64.a0);
                }

                may_change {
                    CpuGroup.boot_cpu_hartid;
                }
            }
        }
    }

    /*
     * Prepared 表示启动 hart 的物理标识已经记录并通过有效性检查。
     */
    state State::Prepared {
        invariant {
            boot_cpu_hartid == Riscv64.a0;
            valid_hart_id(boot_cpu_hartid);
        }

        deferred {
            "CpuGroup.Online 以及 CPU 拓扑、物理 ID 与逻辑 ID 映射属于后续阶段；入口前导期只建立 boot_cpu_hartid。"
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
            "具体 SoC 早期平台状态点后续补充；当前入口前导期只要求 CpuGroup 已记录启动处理器 hartid。"
        }
    }
}

/*
 * BootPhase 表示引导期阶段对象。它负责推进当前模型已经展开的引导期子阶段。
 */
object BootPhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    /*
     * Base 表示引导期阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        events {
            /*
             * Setup 推进入口前导期子阶段。
             * BootPhase 不直接依赖 PreparePhase；二者作为平级阶段由上级阶段对象编排衔接。
             */
            on Event::Setup -> State::Ready {
                drives {
                    EntryPreludePhase.Event::Setup;
                }
            }
        }
    }

    /*
     * Ready 表示当前模型已经展开的引导期子阶段均已完成。
     */
    state State::Ready {
        invariant {
            EntryPreludePhase.state == State::Ready;
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
            context_is(SystemExclusive);
        }

        events {
            /*
             * Setup 按入口前导期构建时序驱动各对象状态迁移。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    Lds.state == State::Online;
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                    PhysicalMemory.state == State::Online;
                }

                drives {
                    RootStream.Event::Preset;
                    InterruptStream.Event::Preset;
                    EventStream.Event::Preset;
                    KernelImage.Event::Preset;
                    KernelImage.Event::Setup;
                    InitTask.Event::Preset;
                    InitStack.Event::Preset;
                    CpuGroup.Event::Preset;
                    Soc.Event::Preset;
                    Vm.Event::Preset;
                    Vm.Event::Setup;
                    InitTask.Event::Enable;
                    InitStack.Event::Enable;
                    EventStream.Event::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示入口前导期编排的对象状态均已到达本阶段目标。
     */
    state State::Ready {
        invariant {
            context_is(SystemExclusive);
            RootStream.state == State::Prepared;
            InterruptStream.state == State::Prepared;
            EventStream.state == State::Online;
            KernelImage.state == State::Online;
            RawDtb.state == State::Ready;
            InitTask.state == State::Online;
            InitStack.state == State::Online;
            Vm.state == State::Ready;
            TrampolineVm.state == State::Destroyed;
            EarlyVm.state == State::Online;
            CpuGroup.state == State::Prepared;
            Soc.state == State::Prepared;
        }

        events {
            /*
             * Cleanup 在后继阶段开始后退出入口前导期对象。
             */
            on Event::Cleanup -> State::Destroyed {
                deferred {
                    "next_phase_started() 是后继阶段边界谓词，后续建立入口后继期对象时展开。"
                }

                depends_on {
                    next_phase_started();
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
