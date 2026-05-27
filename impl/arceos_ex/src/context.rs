use crate::objects::{
    config::Config,
    cpu_id_map::CpuIdMap,
    early_dtb::EarlyDtb,
    early_ioremap::EarlyIoremap,
    entry_prelude::{
        CpuGroup, EventStream, InitStack, InitTask, InterruptStream, KernelImage, Lds, RootStream,
    },
    fix_map::FixMap,
    init_mm::InitMm,
    kernel_cmdline::KernelCmdline,
    kernel_param::KernelParam,
    memblock::MemBlock,
    physical_memory::PhysicalMemory,
    platform_cpu_info::PlatformCpuInfo,
    raw_dtb::RawDtb,
    sbi::Sbi,
    static_objects::StaticObjects,
    vm::Vm,
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
    pub kernel_param: KernelParam,
    pub memblock: MemBlock,
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
            kernel_param: KernelParam::new(),
            memblock: MemBlock::new(),
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
