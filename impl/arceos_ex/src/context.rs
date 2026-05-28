use crate::objects::{
    boot_cpu_hotplug::BootCpuHotplugState, boot_param::BootParam,
    cache_block_info::CacheBlockInfo,
    command_line::{SavedCommandLine, StaticCommandLine},
    config::Config,
    cpu_group::CpuGroup,
    cpu_id_map::CpuIdMap,
    device_tree::DeviceTree,
    early_dtb::EarlyDtb,
    early_ioremap::EarlyIoremap,
    early_param::EarlyParam,
    event_stream::EventStream,
    exception_stream::ExceptionStream,
    exception_table::ExceptionTable,
    fix_map::FixMap,
    init_mm::InitMm,
    init_stack::InitStack,
    init_task::InitTask,
    interrupt_stream::InterruptStream,
    kernel_cmdline::KernelCmdline,
    kernel_image::KernelImage,
    lds::Lds,
    memblock::MemBlock,
    page_allocator_prepare::PageAllocatorPrepare,
    payload_param::PayloadParam,
    per_cpu_storage::PerCpuStorage,
    physical_memory::PhysicalMemory,
    platform_cpu_info::PlatformCpuInfo,
    randomness::Randomness,
    raw_dtb::RawDtb,
    resource_tree::ResourceTree,
    riscv_hwcap::RiscvHwCap,
    root_stream::RootStream,
    sbi::Sbi,
    static_objects::StaticObjects,
    vm::Vm,
    zones::Zones,
};

pub struct Context {
    pub config: Config,
    pub static_objects: StaticObjects,
    pub lds: Lds,

    pub interrupt_stream: InterruptStream,
    pub kernel_image: KernelImage,
    pub root_stream: RootStream,
    pub cpu_group: CpuGroup,
    pub init_task: InitTask,
    pub init_stack: InitStack,
    pub event_stream: EventStream,
    pub exception_stream: ExceptionStream,
    pub vm: Vm,
    pub raw_dtb: RawDtb,
    pub fix_map: FixMap,

    pub early_dtb: EarlyDtb,
    pub platform_cpu_info: PlatformCpuInfo,
    pub physical_memory: PhysicalMemory,
    pub cpu_id_map: CpuIdMap,
    pub kernel_cmdline: KernelCmdline,
    pub init_mm: InitMm,
    pub early_ioremap: EarlyIoremap,
    pub sbi: Sbi,
    pub early_param: EarlyParam,
    pub memblock: MemBlock,

    pub device_tree: DeviceTree,
    pub zones: Zones,
    pub page_allocator_prepare: PageAllocatorPrepare,
    pub resource_tree: ResourceTree,
    pub cache_block_info: CacheBlockInfo,
    pub riscv_hwcap: RiscvHwCap,
    pub saved_command_line: SavedCommandLine,
    pub static_command_line: StaticCommandLine,
    pub per_cpu_storage: PerCpuStorage,
    pub boot_cpu_hotplug_state: BootCpuHotplugState,
    pub boot_param: BootParam,
    pub payload_param: PayloadParam,
    pub randomness: Randomness,
    pub exception_table: ExceptionTable,
}

impl Context {
    pub const fn new() -> Self {
        Self {
            config: Config::new(),
            static_objects: StaticObjects::new(),
            lds: Lds::new(),
            interrupt_stream: InterruptStream::new(),
            kernel_image: KernelImage::new(),
            root_stream: RootStream::new(),
            cpu_group: CpuGroup::new(),
            init_task: InitTask::new(),
            init_stack: InitStack::new(),
            event_stream: EventStream::new(),
            exception_stream: ExceptionStream::new(),
            vm: Vm::new(),
            raw_dtb: RawDtb::new(),
            fix_map: FixMap::new(),
            early_dtb: EarlyDtb::new(),
            platform_cpu_info: PlatformCpuInfo::new(),
            physical_memory: PhysicalMemory::new(),
            cpu_id_map: CpuIdMap::new(),
            kernel_cmdline: KernelCmdline::new(),
            init_mm: InitMm::new(),
            early_ioremap: EarlyIoremap::new(),
            sbi: Sbi::new(),
            early_param: EarlyParam::new(),
            memblock: MemBlock::new(),
            device_tree: DeviceTree::new(),
            zones: Zones::new(),
            page_allocator_prepare: PageAllocatorPrepare::new(),
            resource_tree: ResourceTree::new(),
            cache_block_info: CacheBlockInfo::new(),
            riscv_hwcap: RiscvHwCap::new(),
            saved_command_line: SavedCommandLine::new(),
            static_command_line: StaticCommandLine::new(),
            per_cpu_storage: PerCpuStorage::new(),
            boot_cpu_hotplug_state: BootCpuHotplugState::new(),
            boot_param: BootParam::new(),
            payload_param: PayloadParam::new(),
            randomness: Randomness::new(),
            exception_table: ExceptionTable::new(),
        }
    }
}

static mut CONTEXT: Context = Context::new();

pub fn context() -> &'static mut Context {
    // SAFETY: the current boot path is single-hart and system-exclusive. The
    // context owns long-lived resource objects that must survive address-space
    // switches during startup.
    unsafe { &mut *core::ptr::addr_of_mut!(CONTEXT) }
}

pub fn context_ref() -> &'static Context {
    // SAFETY: read-only access is used for boundary checks before the mutable
    // phase path starts mutating the context.
    unsafe { &*core::ptr::addr_of!(CONTEXT) }
}
