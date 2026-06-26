#[cfg(checkpoint_handler_uart_irq_chain)]
use crate::objects::irq_time::{
    Serial8250ConsoleBurstIrqTxProbe, Serial8250ConsoleIrqTxProbe,
    Serial8250ConsoleLongBurstIrqTxProbe, Serial8250ConsoleLongIrqTxProbe,
    Serial8250ConsoleTxQuiesceProbe, TtyWriteBatchRuntimeTxProbe, TtyWriteRuntimeTxProbe,
};
use crate::objects::state::EventResult;
use crate::objects::{
    block_device::BlockDeviceRegistry,
    boot_param::BootParam,
    cache_block_info::CacheBlockInfo,
    command_line::{CommandLine, SavedCommandLine, StaticCommandLine},
    config::Config,
    cpu::SecondaryCpuStore,
    cpu_capabilities::CpuCapabilities,
    cpu_control::{BootCurrentCpu, CurrentTaskSlot, LocalInterruptControl, RawSpinLock},
    cpu_group::CpuGroup,
    cpu_hotplug::CpuHotplugState,
    devfs::DevFs,
    device_tree::DeviceTree,
    dma_cache_policy::DmaCachePolicy,
    driver::DeviceDriverRef,
    early_dtb::EarlyDtb,
    early_ioremap::EarlyIoremap,
    early_param::EarlyParam,
    event_stream::EventStream,
    exception_stream::{ExceptionStream, SyscallTable},
    exception_table::ExceptionTable,
    ext2::{Ext2Driver, Ext2FileSystem, Ext2Volume},
    files::FilesStruct,
    finalize::{
        AsyncFullSyncDeferred, FinalizeBoundary, InitMemoryCleanupDeferred,
        KernelMappingProtectionDeferred, PtiFinalizeTrimmed, RcuBootEnd, SysctlArgsDeferred,
    },
    fix_map::FixMap,
    hwrng::HwRngCore,
    init_mm::InitMm,
    init_stack::InitStack,
    init_task::InitTask,
    initcall::{
        CpusetSmpTrimmed, CtorTable, DriverCoreBase, DriverCoreDeferred, InitcallBoundary,
        InitcallReturn, InitcallTable, IrqProcViewDeferred, PlatformBus, PlatformBusRootDevice,
    },
    interrupt_stream::InterruptStream,
    ioremap::Ioremap,
    irq_open::{Console, DelayLoop, SchedClock},
    irq_time::{
        BootStackCanary, HrtimerCore, IpiMux, IrqChipInitTable, IrqController, IrqDispatchTree,
        IrqHandlerRegistry, IrqTimeTrimmedPaths, PerfEventCore, Plic, PlicDriver, PlicIrqDomain,
        ProfileCore, RiscvIntc, RiscvIrqStackSet, RiscvTimerProvider, SbiIpi,
        Serial8250RxBatchLoopbackProbe, Serial8250RxLoopbackProbe, SmpCallFunction, SrcuCore, Tick,
        Timekeeper, TimerWheel, TtyXmitFifoProbe, UartExternalIrqEnable, UartInterruptChainProbe,
    },
    kernel_cmdline::KernelCmdline,
    kernel_image::KernelImage,
    lds::Lds,
    maple_tree::MapleTree,
    memblock::MemBlock,
    mm_core::{
        DynamicContainerRuntime, KernelGlobalAllocator, MemoryDebugHardening, MemoryTopology,
        MmCoreTrimmedPaths, MmStructCache, PageAllocator, PageMetadataMap, PageTableCaches,
        SlubSubsystem, StackDepot, Swiotlb, VmallocAllocator,
    },
    mutex::Mutex,
    params::Params,
    payload_param::PayloadParam,
    per_cpu_storage::PerCpuStorage,
    percpu_rw_semaphore::PerCpuRwSemaphore,
    physical_memory::PhysicalMemory,
    platform_cpu_info::PlatformCpuInfo,
    pre_smp_init::{PreSmpInitBoundary, PreSmpInitcallTable, VmstatCore},
    process_prepare::{
        AnonVmaCore, CredentialCore, KeyringCore, NsProxy, RootPidNamespace, SecurityCore,
        SignalCore, TaskCreationCore, TaskFileContext, UtsNamespace, VmaCore,
    },
    radix_tree::RadixTree,
    randomness::Randomness,
    raw_dtb::RawDtb,
    rcu::RcuCore,
    resource_tree::ResourceTree,
    rest_init::{BootIdleRuntime, KernelInitTask, KthreaddReadyGate, KthreaddTask, SystemState},
    root_stream::RootStream,
    rootfs::{
        InitramfsSyncDeferred, IntegrityKeysDeferred, KUnitRuntimeTrimmed, RootFS, RootfsBoundary,
        RootfsConsoleDeferred,
    },
    runtime_core::{AsyncCoreDeferred, PadataCoreDeferred, RuntimeCoreBoundary},
    rwlock::RwLock,
    sbi::Sbi,
    sched_init_boundaries::{SchedInitPreludeTrimmedPaths, SchedInitTraceContextBoundaries},
    scheduler::Scheduler,
    smp_bringup::{
        CpuHotplugSyncSet, CpuStartProvider, SecondaryCpuOnlineAck, SecondaryCpuStartupAck,
        SecondaryIdleTaskSet, SmpBringupBoundary,
    },
    softirq::Softirq,
    static_branch::StaticBranch,
    static_objects::StaticObjects,
    user_boot::{
        ElfObject, UserAddressSpace, UserBootPayload, UserInitProcess, UserStack, UserTrapFrame,
    },
    vfs::{FsStruct, RamFsType, VfsCore},
    virtio::VirtioBus,
    virtio_blk::VirtioBlkRuntime,
    virtio_rng::VirtioRngRuntime,
    vm::Vm,
    workqueue::Workqueue,
    zones::Zones,
};

pub struct Context {
    pub config: Config,
    pub static_objects: StaticObjects,
    pub lds: Lds,

    pub interrupt_stream: InterruptStream,
    pub boot_current_cpu: BootCurrentCpu,
    pub secondary_cpus: SecondaryCpuStore,
    pub boot_cpu_local_interrupt: LocalInterruptControl,
    pub boot_cpu_current_task: CurrentTaskSlot,
    pub kernel_image: KernelImage,
    pub root_stream: RootStream,
    pub cpu_group: CpuGroup,
    pub init_task: InitTask,
    pub init_stack: InitStack,
    pub event_stream: EventStream,
    pub exception_stream: ExceptionStream,
    pub syscall_table: SyscallTable,
    pub vm: Vm,
    pub raw_dtb: RawDtb,
    pub fix_map: FixMap,

    pub early_dtb: EarlyDtb,
    pub platform_cpu_info: PlatformCpuInfo,
    pub physical_memory: PhysicalMemory,
    pub command_line: CommandLine,
    pub kernel_cmdline: KernelCmdline,
    pub init_mm: InitMm,
    pub early_ioremap: EarlyIoremap,
    pub sbi: Sbi,
    pub early_param: EarlyParam,
    pub params: Params,
    pub memblock: MemBlock,

    pub device_tree: DeviceTree,
    pub zones: Zones,
    pub resource_lock: RwLock,
    pub resource_tree: ResourceTree,
    pub cache_block_info: CacheBlockInfo,
    pub cpu_capabilities: CpuCapabilities,
    pub dma_cache_policy: DmaCachePolicy,
    pub jump_label_mutex: Mutex,
    pub cpu_hotplug_lock: PerCpuRwSemaphore,
    pub static_branch: StaticBranch,
    pub saved_command_line: SavedCommandLine,
    pub static_command_line: StaticCommandLine,
    pub per_cpu_storage: PerCpuStorage,
    pub cpu_hotplug_state: CpuHotplugState,
    pub boot_param: BootParam,
    pub payload_param: PayloadParam,
    pub randomness: Randomness,
    pub exception_table: ExceptionTable,

    pub memory_topology: MemoryTopology,
    pub page_metadata_map: PageMetadataMap,
    pub page_allocator: PageAllocator,
    pub memory_debug_hardening: MemoryDebugHardening,
    pub stack_depot: StackDepot,
    pub swiotlb: Swiotlb,
    pub slub_subsystem: SlubSubsystem,
    pub kernel_global_allocator: KernelGlobalAllocator,
    pub dynamic_container_runtime: DynamicContainerRuntime,
    pub page_table_caches: PageTableCaches,
    pub vmalloc_allocator: VmallocAllocator,
    pub ioremap: Ioremap,
    pub mm_struct_cache: MmStructCache,
    pub mm_core_trimmed_paths: MmCoreTrimmedPaths,

    pub scheduler: Scheduler,
    pub radix_tree: RadixTree,
    pub maple_tree: MapleTree,
    pub workqueue: Workqueue,
    pub softirq: Softirq,
    pub rcu_core: RcuCore,
    pub sched_init_prelude_trimmed_paths: SchedInitPreludeTrimmedPaths,
    pub sched_init_trace_context_boundaries: SchedInitTraceContextBoundaries,

    pub irq_controller: IrqController,
    pub riscv_intc: RiscvIntc,
    pub riscv_irq_stack_set: RiscvIrqStackSet,
    pub irqchip_init_table: IrqChipInitTable,
    pub plic_driver: PlicDriver,
    pub irq_dispatch_tree: IrqDispatchTree,
    pub plic: Plic,
    pub plic_irq_domain: PlicIrqDomain,
    pub irq_handler_registry: IrqHandlerRegistry,
    pub uart_external_irq_enable: UartExternalIrqEnable,
    pub uart_interrupt_chain_probe: UartInterruptChainProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub serial8250_console_irq_tx_probe: Serial8250ConsoleIrqTxProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub serial8250_console_burst_irq_tx_probe: Serial8250ConsoleBurstIrqTxProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub serial8250_console_long_irq_tx_probe: Serial8250ConsoleLongIrqTxProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub serial8250_console_long_burst_irq_tx_probe: Serial8250ConsoleLongBurstIrqTxProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub serial8250_console_tx_quiesce_probe: Serial8250ConsoleTxQuiesceProbe,
    pub serial8250_rx_loopback_probe: Serial8250RxLoopbackProbe,
    pub serial8250_rx_batch_loopback_probe: Serial8250RxBatchLoopbackProbe,
    pub tty_xmit_fifo_probe: TtyXmitFifoProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub tty_write_runtime_tx_probe: TtyWriteRuntimeTxProbe,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    pub tty_write_batch_runtime_tx_probe: TtyWriteBatchRuntimeTxProbe,
    pub tick: Tick,
    pub irq_time_trimmed_paths: IrqTimeTrimmedPaths,
    pub timer_wheel: TimerWheel,
    pub srcu_core: SrcuCore,
    pub hrtimer_core: HrtimerCore,
    pub timekeeper: Timekeeper,
    pub riscv_timer_provider: RiscvTimerProvider,
    pub ipi_mux: IpiMux,
    pub sbi_ipi: SbiIpi,
    pub smp_call_function: SmpCallFunction,
    pub boot_stack_canary: BootStackCanary,
    pub perf_event_core: PerfEventCore,
    pub profile_core: ProfileCore,

    pub console: Console,
    pub sched_clock: SchedClock,
    pub delay_loop: DelayLoop,

    pub root_pid_namespace: RootPidNamespace,
    pub anon_vma_core: AnonVmaCore,
    pub task_creation_core: TaskCreationCore,
    pub credential_core: CredentialCore,
    pub signal_core: SignalCore,
    pub task_file_context: TaskFileContext,
    pub vma_core: VmaCore,
    pub ns_proxy: NsProxy,
    pub uts_namespace: UtsNamespace,
    pub keyring_core: KeyringCore,
    pub security_core: SecurityCore,
    pub vfs_core: VfsCore,
    pub fs_struct: FsStruct,
    pub files_struct: FilesStruct,
    pub ramfs_type: RamFsType,

    pub kernel_init_task: KernelInitTask,
    pub kernel_init_task_pi_lock: RawSpinLock,
    pub kthreadd_task: KthreaddTask,
    pub kthreadd_task_pi_lock: RawSpinLock,
    pub system_state: SystemState,
    pub kthreadd_ready_gate: KthreaddReadyGate,
    pub kthreadd_ready_gate_wait_lock: RawSpinLock,
    pub boot_idle_runtime: BootIdleRuntime,
    pub vmstat_core: VmstatCore,
    pub pre_smp_initcalls: PreSmpInitcallTable,
    pub pre_smp_boundary: PreSmpInitBoundary,
    pub secondary_idle_tasks: SecondaryIdleTaskSet,
    pub cpu_hotplug_sync: CpuHotplugSyncSet,
    pub cpu_start_provider: CpuStartProvider,
    pub secondary_cpu_startup_ack: SecondaryCpuStartupAck,
    pub secondary_cpu_online_ack: SecondaryCpuOnlineAck,
    pub smp_bringup_boundary: SmpBringupBoundary,
    pub async_core_deferred: AsyncCoreDeferred,
    pub padata_core_deferred: PadataCoreDeferred,
    pub runtime_core_boundary: RuntimeCoreBoundary,
    pub cpuset_smp_trimmed: CpusetSmpTrimmed,
    pub driver_core_base: DriverCoreBase,
    pub platform_bus_root_device: PlatformBusRootDevice,
    pub platform_bus: PlatformBus,
    pub virtio_bus: VirtioBus,
    pub hwrng_core: HwRngCore,
    pub block_device_registry: BlockDeviceRegistry,
    pub ext2_driver: Ext2Driver,
    pub ext2_volume: Ext2Volume,
    pub ext2_filesystem: Ext2FileSystem,
    pub devfs: DevFs,
    pub virtio_blk_runtime: VirtioBlkRuntime,
    pub virtio_rng_runtime: VirtioRngRuntime,
    pub driver_core_deferred: DriverCoreDeferred,
    pub irq_proc_view_deferred: IrqProcViewDeferred,
    pub ctor_table: CtorTable,
    pub initcall_table: InitcallTable,
    pub initcall_boundary: InitcallBoundary,
    pub kunit_runtime_trimmed: KUnitRuntimeTrimmed,
    pub initramfs_sync_deferred: InitramfsSyncDeferred,
    pub rootfs_console_deferred: RootfsConsoleDeferred,
    pub rootfs: RootFS,
    pub integrity_keys_deferred: IntegrityKeysDeferred,
    pub rootfs_boundary: RootfsBoundary,
    pub async_full_sync_deferred: AsyncFullSyncDeferred,
    pub init_memory_cleanup_deferred: InitMemoryCleanupDeferred,
    pub kernel_mapping_protection_deferred: KernelMappingProtectionDeferred,
    pub pti_finalize_trimmed: PtiFinalizeTrimmed,
    pub rcu_boot_end: RcuBootEnd,
    pub sysctl_args_deferred: SysctlArgsDeferred,
    pub finalize_boundary: FinalizeBoundary,
    pub user_boot_payload: UserBootPayload,
    pub elf_object: ElfObject,
    pub elf_interpreter_object: ElfObject,
    pub user_address_space: UserAddressSpace,
    pub user_stack: UserStack,
    pub user_trap_frame: UserTrapFrame,
    pub user_init_process: UserInitProcess,
}

impl Context {
    pub const fn new() -> Self {
        Self {
            config: Config::new(),
            static_objects: StaticObjects::new(),
            lds: Lds::new(),
            interrupt_stream: InterruptStream::new(),
            boot_current_cpu: BootCurrentCpu::new(),
            secondary_cpus: SecondaryCpuStore::new(),
            boot_cpu_local_interrupt: LocalInterruptControl::new(),
            boot_cpu_current_task: CurrentTaskSlot::new(),
            kernel_image: KernelImage::new(),
            root_stream: RootStream::new(),
            cpu_group: CpuGroup::new(),
            init_task: InitTask::new(),
            init_stack: InitStack::new(),
            event_stream: EventStream::new(),
            exception_stream: ExceptionStream::new(),
            syscall_table: SyscallTable::new(),
            vm: Vm::new(),
            raw_dtb: RawDtb::new(),
            fix_map: FixMap::new(),
            early_dtb: EarlyDtb::new(),
            platform_cpu_info: PlatformCpuInfo::new(),
            physical_memory: PhysicalMemory::new(),
            command_line: CommandLine::new(),
            kernel_cmdline: KernelCmdline::new(),
            init_mm: InitMm::new(),
            early_ioremap: EarlyIoremap::new(),
            sbi: Sbi::new(),
            early_param: EarlyParam::new(),
            params: Params::new(),
            memblock: MemBlock::new(),
            device_tree: DeviceTree::new(),
            zones: Zones::new(),
            resource_lock: RwLock::new_static(),
            resource_tree: ResourceTree::new(),
            cache_block_info: CacheBlockInfo::new(),
            cpu_capabilities: CpuCapabilities::new(),
            dma_cache_policy: DmaCachePolicy::new(),
            jump_label_mutex: Mutex::new_static(),
            cpu_hotplug_lock: PerCpuRwSemaphore::new_static(),
            static_branch: StaticBranch::new(),
            saved_command_line: SavedCommandLine::new(),
            static_command_line: StaticCommandLine::new(),
            per_cpu_storage: PerCpuStorage::new(),
            cpu_hotplug_state: CpuHotplugState::new(),
            boot_param: BootParam::new(),
            payload_param: PayloadParam::new(),
            randomness: Randomness::new(),
            exception_table: ExceptionTable::new(),
            memory_topology: MemoryTopology::new(),
            page_metadata_map: PageMetadataMap::new(),
            page_allocator: PageAllocator::new(),
            memory_debug_hardening: MemoryDebugHardening::new(),
            stack_depot: StackDepot::new(),
            swiotlb: Swiotlb::new(),
            slub_subsystem: SlubSubsystem::new(),
            kernel_global_allocator: KernelGlobalAllocator::new(),
            dynamic_container_runtime: DynamicContainerRuntime::new(),
            page_table_caches: PageTableCaches::new(),
            vmalloc_allocator: VmallocAllocator::new(),
            ioremap: Ioremap::new(),
            mm_struct_cache: MmStructCache::new(),
            mm_core_trimmed_paths: MmCoreTrimmedPaths::new(),
            scheduler: Scheduler::new(),
            radix_tree: RadixTree::new(),
            maple_tree: MapleTree::new(),
            workqueue: Workqueue::new(),
            softirq: Softirq::new(),
            rcu_core: RcuCore::new(),
            sched_init_prelude_trimmed_paths: SchedInitPreludeTrimmedPaths::new(),
            sched_init_trace_context_boundaries: SchedInitTraceContextBoundaries::new(),
            irq_controller: IrqController::new(),
            riscv_intc: RiscvIntc::new(),
            riscv_irq_stack_set: RiscvIrqStackSet::new(),
            irqchip_init_table: IrqChipInitTable::new(),
            plic_driver: PlicDriver::new(),
            irq_dispatch_tree: IrqDispatchTree::new(),
            plic: Plic::new(),
            plic_irq_domain: PlicIrqDomain::new(),
            irq_handler_registry: IrqHandlerRegistry::new(),
            uart_external_irq_enable: UartExternalIrqEnable::new(),
            uart_interrupt_chain_probe: UartInterruptChainProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            serial8250_console_irq_tx_probe: Serial8250ConsoleIrqTxProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            serial8250_console_burst_irq_tx_probe: Serial8250ConsoleBurstIrqTxProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            serial8250_console_long_irq_tx_probe: Serial8250ConsoleLongIrqTxProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            serial8250_console_long_burst_irq_tx_probe: Serial8250ConsoleLongBurstIrqTxProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            serial8250_console_tx_quiesce_probe: Serial8250ConsoleTxQuiesceProbe::new(),
            serial8250_rx_loopback_probe: Serial8250RxLoopbackProbe::new(),
            serial8250_rx_batch_loopback_probe: Serial8250RxBatchLoopbackProbe::new(),
            tty_xmit_fifo_probe: TtyXmitFifoProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            tty_write_runtime_tx_probe: TtyWriteRuntimeTxProbe::new(),
            #[cfg(checkpoint_handler_uart_irq_chain)]
            tty_write_batch_runtime_tx_probe: TtyWriteBatchRuntimeTxProbe::new(),
            tick: Tick::new(),
            irq_time_trimmed_paths: IrqTimeTrimmedPaths::new(),
            timer_wheel: TimerWheel::new(),
            srcu_core: SrcuCore::new(),
            hrtimer_core: HrtimerCore::new(),
            timekeeper: Timekeeper::new(),
            riscv_timer_provider: RiscvTimerProvider::new(),
            ipi_mux: IpiMux::new(),
            sbi_ipi: SbiIpi::new(),
            smp_call_function: SmpCallFunction::new(),
            boot_stack_canary: BootStackCanary::new(),
            perf_event_core: PerfEventCore::new(),
            profile_core: ProfileCore::new(),
            console: Console::new(),
            sched_clock: SchedClock::new(),
            delay_loop: DelayLoop::new(),
            root_pid_namespace: RootPidNamespace::new(),
            anon_vma_core: AnonVmaCore::new(),
            task_creation_core: TaskCreationCore::new(),
            credential_core: CredentialCore::new(),
            signal_core: SignalCore::new(),
            task_file_context: TaskFileContext::new(),
            vma_core: VmaCore::new(),
            ns_proxy: NsProxy::new(),
            uts_namespace: UtsNamespace::new(),
            keyring_core: KeyringCore::new(),
            security_core: SecurityCore::new(),
            vfs_core: VfsCore::new(),
            fs_struct: FsStruct::new(),
            files_struct: FilesStruct::new(),
            ramfs_type: RamFsType::new(),
            kernel_init_task: KernelInitTask::new(),
            kernel_init_task_pi_lock: RawSpinLock::new(),
            kthreadd_task: KthreaddTask::new(),
            kthreadd_task_pi_lock: RawSpinLock::new(),
            system_state: SystemState::new(),
            kthreadd_ready_gate: KthreaddReadyGate::new(),
            kthreadd_ready_gate_wait_lock: RawSpinLock::new(),
            boot_idle_runtime: BootIdleRuntime::new(),
            vmstat_core: VmstatCore::new(),
            pre_smp_initcalls: PreSmpInitcallTable::new(),
            pre_smp_boundary: PreSmpInitBoundary::new(),
            secondary_idle_tasks: SecondaryIdleTaskSet::new(),
            cpu_hotplug_sync: CpuHotplugSyncSet::new(),
            cpu_start_provider: CpuStartProvider::new(),
            secondary_cpu_startup_ack: SecondaryCpuStartupAck::new(),
            secondary_cpu_online_ack: SecondaryCpuOnlineAck::new(),
            smp_bringup_boundary: SmpBringupBoundary::new(),
            async_core_deferred: AsyncCoreDeferred::new(),
            padata_core_deferred: PadataCoreDeferred::new(),
            runtime_core_boundary: RuntimeCoreBoundary::new(),
            cpuset_smp_trimmed: CpusetSmpTrimmed::new(),
            driver_core_base: DriverCoreBase::new(),
            platform_bus_root_device: PlatformBusRootDevice::new(),
            platform_bus: PlatformBus::new(),
            virtio_bus: VirtioBus::new(),
            hwrng_core: HwRngCore::new(),
            block_device_registry: BlockDeviceRegistry::new(),
            ext2_driver: Ext2Driver::new(),
            ext2_volume: Ext2Volume::new(),
            ext2_filesystem: Ext2FileSystem::new(),
            devfs: DevFs::new(),
            virtio_blk_runtime: VirtioBlkRuntime::new(),
            virtio_rng_runtime: VirtioRngRuntime::new(),
            driver_core_deferred: DriverCoreDeferred::new(),
            irq_proc_view_deferred: IrqProcViewDeferred::new(),
            ctor_table: CtorTable::new(),
            initcall_table: InitcallTable::new(),
            initcall_boundary: InitcallBoundary::new(),
            kunit_runtime_trimmed: KUnitRuntimeTrimmed::new(),
            initramfs_sync_deferred: InitramfsSyncDeferred::new(),
            rootfs_console_deferred: RootfsConsoleDeferred::new(),
            rootfs: RootFS::new(),
            integrity_keys_deferred: IntegrityKeysDeferred::new(),
            rootfs_boundary: RootfsBoundary::new(),
            async_full_sync_deferred: AsyncFullSyncDeferred::new(),
            init_memory_cleanup_deferred: InitMemoryCleanupDeferred::new(),
            kernel_mapping_protection_deferred: KernelMappingProtectionDeferred::new(),
            pti_finalize_trimmed: PtiFinalizeTrimmed::new(),
            rcu_boot_end: RcuBootEnd::new(),
            sysctl_args_deferred: SysctlArgsDeferred::new(),
            finalize_boundary: FinalizeBoundary::new(),
            user_boot_payload: UserBootPayload::new(),
            elf_object: ElfObject::new(),
            elf_interpreter_object: ElfObject::new(),
            user_address_space: UserAddressSpace::new(),
            user_stack: UserStack::new(),
            user_trap_frame: UserTrapFrame::new(),
            user_init_process: UserInitProcess::new(),
        }
    }

    pub fn setup_smoke_scheduler_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        self.scheduler.setup_smoke_scheduler_task(entry)
    }

    pub fn enqueue_smoke_scheduler_task(&mut self) -> EventResult {
        self.scheduler.enqueue_smoke_scheduler_task()
    }

    pub fn setup_smoke_mutex_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        self.scheduler.setup_smoke_mutex_task(entry)
    }

    pub fn enqueue_smoke_mutex_task(&mut self) -> EventResult {
        self.scheduler.enqueue_smoke_mutex_task()
    }

    pub fn dequeue_smoke_mutex_task(&mut self) -> EventResult {
        self.scheduler.dequeue_smoke_mutex_task()
    }

    pub fn setup_smoke_rwsem_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        self.scheduler.setup_smoke_rwsem_task(entry)
    }

    pub fn enqueue_smoke_rwsem_task(&mut self) -> EventResult {
        self.scheduler.enqueue_smoke_rwsem_task()
    }

    pub fn dequeue_smoke_rwsem_task(&mut self) -> EventResult {
        self.scheduler.dequeue_smoke_rwsem_task()
    }

    pub fn setup_smoke_rwlock_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        self.scheduler.setup_smoke_rwlock_task(entry)
    }

    pub fn enqueue_smoke_rwlock_task(&mut self) -> EventResult {
        self.scheduler.enqueue_smoke_rwlock_task()
    }

    pub fn dequeue_smoke_rwlock_task(&mut self) -> EventResult {
        self.scheduler.dequeue_smoke_rwlock_task()
    }

    pub fn schedule_current(&mut self) -> EventResult {
        self.scheduler.schedule(
            &self.cpu_group,
            &self.kernel_init_task,
            &self.kthreadd_task,
            &mut self.boot_cpu_local_interrupt,
            &mut self.boot_cpu_current_task,
        )
    }

    pub fn mark_smoke_scheduler_entry_ran(&mut self) -> EventResult {
        self.scheduler.smoke_scheduler_task_mut().mark_entry_ran()
    }

    pub fn mark_smoke_scheduler_yielded_back(&mut self) -> EventResult {
        self.scheduler
            .smoke_scheduler_task_mut()
            .mark_yielded_back()
    }

    pub fn mark_smoke_mutex_entry_ran(&mut self) -> EventResult {
        self.scheduler.smoke_mutex_task_mut().mark_entry_ran()
    }

    pub fn mark_smoke_mutex_yielded_back(&mut self) -> EventResult {
        self.scheduler.smoke_mutex_task_mut().mark_yielded_back()
    }

    pub fn mark_smoke_rwsem_entry_ran(&mut self) -> EventResult {
        self.scheduler.smoke_rwsem_task_mut().mark_entry_ran()
    }

    pub fn mark_smoke_rwsem_yielded_back(&mut self) -> EventResult {
        self.scheduler.smoke_rwsem_task_mut().mark_yielded_back()
    }

    pub fn mark_smoke_rwlock_entry_ran(&mut self) -> EventResult {
        self.scheduler.smoke_rwlock_task_mut().mark_entry_ran()
    }

    pub fn mark_smoke_rwlock_yielded_back(&mut self) -> EventResult {
        self.scheduler.smoke_rwlock_task_mut().mark_yielded_back()
    }

    pub fn platform_driver_register(&mut self, driver: DeviceDriverRef) -> InitcallReturn {
        let mut probe_context = crate::objects::driver::PlatformProbeContext::new(
            &self.device_tree,
            &mut self.vmalloc_allocator,
            &mut self.page_table_caches,
            &mut self.page_allocator,
            &self.page_metadata_map,
            &self.config,
            &mut self.ioremap,
            &mut self.plic_irq_domain,
            &mut self.irq_handler_registry,
            &mut self.virtio_bus,
        );
        self.platform_bus
            .platform_driver_register(driver, &mut probe_context)
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
