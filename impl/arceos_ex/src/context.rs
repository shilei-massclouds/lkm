use crate::flows::{boot_idle_flow::BootIdleFlow, boot_init_flow::BootInitFlow};
#[cfg(checkpoint_handler_uart_irq_chain)]
use crate::objects::irq_time::{
    Serial8250ConsoleBurstIrqTxProbe, Serial8250ConsoleIrqTxProbe,
    Serial8250ConsoleLongBurstIrqTxProbe, Serial8250ConsoleLongIrqTxProbe,
    Serial8250ConsoleTxQuiesceProbe, TtyWriteBatchRuntimeTxProbe, TtyWriteRuntimeTxProbe,
};
use crate::objects::scheduler::{SchedulerTestStacks, SchedulerTestTasks};
use crate::objects::scheduler_shared::SchedulerShared;
use crate::objects::scheduler_task_access::SchedulerTaskAccess;
use crate::objects::state::{
    EventError, EventErrorCode, EventResult, LifecycleEvent, State, failed_condition,
};
use crate::objects::{
    binary_format_registry::BinaryFormatRegistry,
    block_device::BlockDeviceRegistry,
    boot_param::BootParam,
    boot_task::BootTask,
    cache_block_info::CacheBlockInfo,
    command_line::{CommandLine, SavedCommandLine, StaticCommandLine},
    config::Config,
    cpu_capabilities::CpuCapabilities,
    cpu_control::RawSpinLock,
    cpu_group::CpuGroup,
    cpu_hotplug::CpuHotplugState,
    current_task::{
        CurrentTask, CurrentTaskCandidate, CurrentTaskDiagnostic, CurrentTaskError,
        CurrentTaskErrorCode, validate_candidate,
    },
    devfs::DevFs,
    device_tree::DeviceTree,
    dma_cache_policy::DmaCachePolicy,
    driver::DeviceDriverRef,
    early_dtb::EarlyDtb,
    early_ioremap::EarlyIoremap,
    early_param::EarlyParam,
    exception_table::ExceptionTable,
    exception_type::SyscallTable,
    exec_sync_boundaries::ExecSyncBoundaries,
    exec_transaction::ExecTransaction,
    ext2::{Ext2Driver, Ext2FileSystem, Ext2Volume},
    files::FilesStruct,
    finalize::{
        AsyncFullSyncDeferred, FinalizeBoundary, InitMemoryCleanupDeferred,
        KernelMappingProtectionDeferred, NumaDefaultPolicyTrimmed, PtiFinalizeTrimmed, RcuBootEnd,
        SysctlArgsDeferred,
    },
    fix_map::FixMap,
    hwrng::HwRngCore,
    init_mm::InitMm,
    init_stack::InitStack,
    initcall::{
        CpusetSmpTrimmed, CtorTable, DriverCoreBase, DriverCoreDeferred, InitcallBoundary,
        InitcallReturn, InitcallTable, IrqProcViewDeferred, PlatformBus, PlatformBusRootDevice,
    },
    interrupt_type::InterruptType,
    ioremap::Ioremap,
    irq_open::{Console, DelayLoop, IrqOpenPrepareTrimmedPaths, SchedClock},
    irq_time::{
        BootStackCanary, HrtimerCore, IpiMux, IrqChipInitTable, IrqController, IrqDispatchTree,
        IrqHandlerRegistry, IrqTimeTrimmedPaths, PerfEventCore, Plic, PlicDriver, PlicIrqDomain,
        ProfileCore, RiscvIntc, RiscvIrqStackSet, RiscvTimerProvider, SbiIpi,
        Serial8250RxBatchLoopbackProbe, Serial8250RxLoopbackProbe, SmpCallFunction, SrcuCore, Tick,
        Timekeeper, TimerWheel, TtyXmitFifoProbe, UartExternalIrqEnable, UartInterruptChainProbe,
    },
    kernel_addr_space::KernelAddrSpace,
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
        AnonVmaCore, CredentialCore, KeyringCore, NsProxy, ProcessPrepareTrimmedPaths,
        RootPidNamespace, SecurityCore, SignalCore, TaskCreationCore, TaskFileContext,
        UtsNamespace, VmaCore,
    },
    radix_tree::RadixTree,
    randomness::Randomness,
    raw_dtb::RawDtb,
    rcu::RcuCore,
    resource_tree::ResourceTree,
    rest_init::{
        KernelInitFlow, KernelInitTask, KthreaddFlow, KthreaddReadyGate, KthreaddTask, SystemState,
    },
    rootfs::{
        InitramfsSyncDeferred, IntegrityKeysDeferred, KUnitRuntimeTrimmed, RootFS, RootfsBoundary,
        RootfsConsoleDeferred, RootfsPrepareNamespacePaths,
    },
    runtime_core::{AsyncCoreDeferred, PadataCoreDeferred, RuntimeCoreBoundary},
    rwlock::RwLock,
    sbi::Sbi,
    sched_init_boundaries::{SchedInitPreludeTrimmedPaths, SchedInitTraceContextBoundaries},
    selected_payload::SelectedPayloadHandoff,
    smp_bringup::{
        CpuHotplugSyncSet, CpuStartProvider, SecondaryCpuOnlineAck, SecondaryCpuStartupAck,
        SecondaryIdleTaskSet, SmpBringupBoundary,
    },
    softirq::Softirq,
    static_branch::StaticBranch,
    static_objects::StaticObjects,
    task::{Task, TaskRef},
    task_flow::{TaskFlow, TaskFlowRef},
    user_boot::{
        ElfObject, KernelInitTaskUserState, UserAddressSpace, UserAppFlow, UserBootPayload,
        UserCloneDeferredBoundaries, UserTaskSet, UserTrapFrame,
    },
    user_stack::UserStack,
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

    pub kernel_image: KernelImage,
    pub kernel_addr_space: KernelAddrSpace,
    pub cpu_group: CpuGroup,
    pub boot_task: BootTask,
    pub boot_init_flow: BootInitFlow,
    pub init_stack: InitStack,
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

    pub radix_tree: RadixTree,
    pub maple_tree: MapleTree,
    pub workqueue: Workqueue,
    pub softirq: Softirq,
    pub rcu_core: RcuCore,
    pub sched_init_prelude_trimmed_paths: SchedInitPreludeTrimmedPaths,
    pub sched_init_trace_context_boundaries: SchedInitTraceContextBoundaries,
    pub scheduler_shared: SchedulerShared,
    pub(crate) scheduler_test_tasks: SchedulerTestTasks,
    scheduler_test_stacks: SchedulerTestStacks,

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
    pub irq_open_prepare_trimmed_paths: IrqOpenPrepareTrimmedPaths,
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
    pub process_prepare_trimmed_paths: ProcessPrepareTrimmedPaths,
    pub vfs_core: VfsCore,
    pub fs_struct: FsStruct,
    pub files_struct: FilesStruct,
    pub ramfs_type: RamFsType,

    pub kernel_init_task: KernelInitTask,
    #[cfg_attr(app_hello, allow(dead_code))]
    pub kernel_init_flow: KernelInitFlow,
    pub kernel_init_task_pi_lock: RawSpinLock,
    pub kthreadd_task: KthreaddTask,
    pub kthreadd_flow: KthreaddFlow,
    pub kthreadd_task_pi_lock: RawSpinLock,
    pub system_state: SystemState,
    pub kthreadd_ready_gate: KthreaddReadyGate,
    pub kthreadd_ready_gate_wait_lock: RawSpinLock,
    pub boot_idle_flow: BootIdleFlow,
    pub vmstat_core: VmstatCore,
    pub pre_smp_initcalls: PreSmpInitcallTable,
    pub pre_smp_boundary: PreSmpInitBoundary,
    pub secondary_idle_tasks: SecondaryIdleTaskSet,
    pub smpboot_threads_lock: Mutex,
    pub cpu_hotplug_sync: CpuHotplugSyncSet,
    pub cpu_add_remove_lock: Mutex,
    pub cpu_running_wait_lock: RawSpinLock,
    pub done_up_wait_lock: RawSpinLock,
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
    pub binary_format_registry: BinaryFormatRegistry,
    pub initcall_boundary: InitcallBoundary,
    pub kunit_runtime_trimmed: KUnitRuntimeTrimmed,
    pub initramfs_sync_deferred: InitramfsSyncDeferred,
    pub rootfs_console_deferred: RootfsConsoleDeferred,
    pub rootfs_prepare_namespace_paths: RootfsPrepareNamespacePaths,
    pub rootfs: RootFS,
    pub integrity_keys_deferred: IntegrityKeysDeferred,
    pub rootfs_boundary: RootfsBoundary,
    pub async_full_sync_deferred: AsyncFullSyncDeferred,
    pub init_memory_cleanup_deferred: InitMemoryCleanupDeferred,
    pub kernel_mapping_protection_deferred: KernelMappingProtectionDeferred,
    pub pti_finalize_trimmed: PtiFinalizeTrimmed,
    pub numa_default_policy_trimmed: NumaDefaultPolicyTrimmed,
    pub rcu_boot_end: RcuBootEnd,
    pub sysctl_args_deferred: SysctlArgsDeferred,
    pub finalize_boundary: FinalizeBoundary,
    pub exec_sync_boundaries: ExecSyncBoundaries,
    pub exec_transaction: ExecTransaction,
    pub user_clone_deferred_boundaries: UserCloneDeferredBoundaries,
    pub selected_payload_handoff: SelectedPayloadHandoff,
    // User payload carriers are live only in the user-boot configuration.
    #[cfg_attr(app_hello, allow(dead_code))]
    pub user_boot_payload: UserBootPayload,
    #[cfg_attr(app_hello, allow(dead_code))]
    pub elf_object: ElfObject,
    #[cfg_attr(app_hello, allow(dead_code))]
    pub elf_interpreter_object: ElfObject,
    pub user_address_space: UserAddressSpace,
    #[cfg_attr(app_hello, allow(dead_code))]
    pub user_stack: UserStack,
    pub user_trap_frame: UserTrapFrame,
    pub user_task_set: UserTaskSet,
    pub kernel_init_user_state: KernelInitTaskUserState,
    #[cfg_attr(app_hello, allow(dead_code))]
    pub user_app_flow: UserAppFlow,
}

impl Context {
    pub fn scheduler(&self) -> &crate::objects::scheduler::Scheduler {
        self.cpu_group
            .boot_scheduler()
            .expect("published Context must contain CPU0 Scheduler")
    }

    pub fn scheduler_mut(&mut self) -> &mut crate::objects::scheduler::Scheduler {
        self.cpu_group
            .boot_scheduler_mut()
            .expect("published Context must contain CPU0 Scheduler")
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn smoke_scheduler_task(&self) -> &crate::objects::scheduler::SmokeSchedulerTask {
        self.scheduler_test_tasks.smoke_scheduler_task()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn smoke_mutex_task(&self) -> &crate::objects::scheduler::SmokeSchedulerTask {
        self.scheduler_test_tasks.smoke_mutex_task()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn smoke_rwsem_task(&self) -> &crate::objects::scheduler::SmokeSchedulerTask {
        self.scheduler_test_tasks.smoke_rwsem_task()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn smoke_rwlock_task(&self) -> &crate::objects::scheduler::SmokeSchedulerTask {
        self.scheduler_test_tasks.smoke_rwlock_task()
    }

    pub const fn new() -> Self {
        Self {
            config: Config::new(),
            static_objects: StaticObjects::new(),
            lds: Lds::new(),
            kernel_image: KernelImage::new(),
            kernel_addr_space: KernelAddrSpace::new(),
            cpu_group: CpuGroup::new(),
            boot_task: BootTask::new(),
            boot_init_flow: BootInitFlow::new(),
            init_stack: InitStack::new(),
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
            radix_tree: RadixTree::new(),
            maple_tree: MapleTree::new(),
            workqueue: Workqueue::new(),
            softirq: Softirq::new(),
            rcu_core: RcuCore::new(),
            sched_init_prelude_trimmed_paths: SchedInitPreludeTrimmedPaths::new(),
            sched_init_trace_context_boundaries: SchedInitTraceContextBoundaries::new(),
            scheduler_shared: SchedulerShared::new(),
            scheduler_test_tasks: SchedulerTestTasks::new(),
            scheduler_test_stacks: SchedulerTestStacks::new(),
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
            irq_open_prepare_trimmed_paths: IrqOpenPrepareTrimmedPaths::new(),
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
            process_prepare_trimmed_paths: ProcessPrepareTrimmedPaths::new(),
            vfs_core: VfsCore::new(),
            fs_struct: FsStruct::new(),
            files_struct: FilesStruct::new(),
            ramfs_type: RamFsType::new(),
            kernel_init_task: KernelInitTask::new(),
            kernel_init_flow: KernelInitFlow::new(),
            kernel_init_task_pi_lock: RawSpinLock::new(),
            kthreadd_task: KthreaddTask::new(),
            kthreadd_flow: KthreaddFlow::new(),
            kthreadd_task_pi_lock: RawSpinLock::new(),
            system_state: SystemState::new(),
            kthreadd_ready_gate: KthreaddReadyGate::new(),
            kthreadd_ready_gate_wait_lock: RawSpinLock::new(),
            boot_idle_flow: BootIdleFlow::new(),
            vmstat_core: VmstatCore::new(),
            pre_smp_initcalls: PreSmpInitcallTable::new(),
            pre_smp_boundary: PreSmpInitBoundary::new(),
            secondary_idle_tasks: SecondaryIdleTaskSet::new(),
            smpboot_threads_lock: Mutex::new_static(),
            cpu_hotplug_sync: CpuHotplugSyncSet::new(),
            cpu_add_remove_lock: Mutex::new_static(),
            cpu_running_wait_lock: RawSpinLock::new(),
            done_up_wait_lock: RawSpinLock::new(),
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
            binary_format_registry: BinaryFormatRegistry::new(),
            initcall_boundary: InitcallBoundary::new(),
            kunit_runtime_trimmed: KUnitRuntimeTrimmed::new(),
            initramfs_sync_deferred: InitramfsSyncDeferred::new(),
            rootfs_console_deferred: RootfsConsoleDeferred::new(),
            rootfs_prepare_namespace_paths: RootfsPrepareNamespacePaths::new(),
            rootfs: RootFS::new(),
            integrity_keys_deferred: IntegrityKeysDeferred::new(),
            rootfs_boundary: RootfsBoundary::new(),
            async_full_sync_deferred: AsyncFullSyncDeferred::new(),
            init_memory_cleanup_deferred: InitMemoryCleanupDeferred::new(),
            kernel_mapping_protection_deferred: KernelMappingProtectionDeferred::new(),
            pti_finalize_trimmed: PtiFinalizeTrimmed::new(),
            numa_default_policy_trimmed: NumaDefaultPolicyTrimmed::new(),
            rcu_boot_end: RcuBootEnd::new(),
            sysctl_args_deferred: SysctlArgsDeferred::new(),
            finalize_boundary: FinalizeBoundary::new(),
            exec_sync_boundaries: ExecSyncBoundaries::new(),
            exec_transaction: ExecTransaction::new(),
            user_clone_deferred_boundaries: UserCloneDeferredBoundaries::new(),
            selected_payload_handoff: SelectedPayloadHandoff::new(),
            user_boot_payload: UserBootPayload::new(),
            elf_object: ElfObject::new(),
            elf_interpreter_object: ElfObject::new(),
            user_address_space: UserAddressSpace::new(),
            user_stack: UserStack::new(),
            user_trap_frame: UserTrapFrame::new(),
            user_task_set: UserTaskSet::new(),
            kernel_init_user_state: KernelInitTaskUserState::new(),
            user_app_flow: UserAppFlow::new(),
        }
    }

    pub fn boot_cpu_local_interrupt(&self) -> &InterruptType {
        self.cpu_group
            .boot_cpu_local_interrupt()
            .expect("CpuGroup.cpus[0] must exist before local interrupt access")
    }

    pub fn boot_cpu_interrupt(&self) -> &InterruptType {
        self.cpu_group
            .boot_cpu_interrupt()
            .expect("CpuGroup.cpus[0].trap.interrupt must exist before access")
    }

    pub fn boot_cpu_exception(&self) -> &crate::objects::exception_type::ExceptionType {
        self.cpu_group
            .boot_cpu_exception()
            .expect("CpuGroup.cpus[0].trap.exception must exist before access")
    }

    pub fn boot_cpu_trap(&self) -> &crate::objects::trap_type::TrapType {
        self.cpu_group
            .boot_cpu_trap()
            .expect("CpuGroup.cpus[0].trap must exist before access")
    }

    pub fn current_task(&self) -> Result<CurrentTask, CurrentTaskError> {
        let tp = crate::arch::riscv64::csr::read_tp();
        let Some(task_ref) = self.task_ref_from_identity(tp) else {
            return Err(CurrentTaskError::new(
                CurrentTaskErrorCode::UnknownIdentity,
                CurrentTaskDiagnostic::new(
                    tp,
                    TaskRef::NONE,
                    TaskFlowRef::NONE,
                    crate::objects::cpu::CpuRef::invalid(),
                ),
            ));
        };
        self.resolve_current_task_identity(tp, task_ref)
    }

    pub fn current_task_ref(&self) -> Result<TaskRef, CurrentTaskError> {
        self.current_task().map(CurrentTask::task_ref)
    }

    pub fn current_task_flow_ref(&self) -> Result<TaskFlowRef, CurrentTaskError> {
        let current = self.current_task()?;
        let tp = crate::arch::riscv64::csr::read_tp();
        let candidate = if current.task_ref().same_identity(TaskRef::BOOT) {
            self.boot_current_task_candidate()
        } else {
            self.current_task_candidate(current.task_ref())
        }
        .ok_or_else(|| {
            self.current_task_error(
                CurrentTaskErrorCode::MissingActiveFlow,
                tp,
                current.task_ref(),
            )
        })?;
        Ok(candidate.flow.flow_ref())
    }

    pub(crate) fn committed_exec_flow_handoff_matches(
        &self,
        task_ref: TaskRef,
        predecessor: TaskFlowRef,
        successor: TaskFlowRef,
        entry_commit_count: usize,
    ) -> bool {
        if !task_ref.is_valid()
            || !predecessor.is_valid()
            || !successor.is_valid()
            || predecessor.same_identity(successor)
            || self.exec_transaction.active()
            || self.exec_transaction.point_of_no_return()
            || entry_commit_count.checked_add(1) != Some(self.exec_transaction.commit_count())
        {
            return false;
        }
        let Some(candidate) = self.current_task_candidate(task_ref) else {
            return false;
        };
        candidate.task.task_ref().same_identity(task_ref)
            && candidate.task.active_flow().same_identity(successor)
            && candidate.flow.flow_ref().same_identity(successor)
            && candidate.flow.predecessor().same_identity(predecessor)
    }

    pub(crate) fn bind_task_root_trap_flow(
        &mut self,
        task_ref: TaskRef,
        root_ref: crate::objects::trap_flow_type::TrapFlowRef,
    ) -> Result<bool, &'static str> {
        let candidate = self
            .current_task_candidate(task_ref)
            .ok_or("current_task_candidate_missing")?;
        let effective_flow_ref = candidate.flow.flow_ref();
        let cpu_ref = candidate
            .flow
            .cpu_ref()
            .ok_or("current_task_flow_cpu_missing")?;
        let task_identity = crate::arch::riscv64::csr::read_tp();
        let installed = self
            .task_mut_for_ref(task_ref)
            .ok_or("mutable_task_ref_missing")?
            .bind_root_trap_flow(effective_flow_ref, root_ref)?;
        if installed
            && !self
                .cpu_group
                .cpu_mut(cpu_ref.logical_id())
                .ok_or("current_cpu_missing")?
                .trap_mut()
                .refresh_entry_task_root(task_identity, root_ref)
        {
            return Err("trap_entry_task_root_refresh_failed");
        }
        Ok(installed)
    }

    pub(crate) fn task_root_trap_flow_resolves(&mut self, task_ref: TaskRef) -> Option<bool> {
        self.task_mut_for_ref(task_ref)
            .map(|task| task.root_trap_flow_resolves())
    }

    pub(crate) fn task_root_trap_flow_ref(
        &mut self,
        task_ref: TaskRef,
    ) -> crate::objects::trap_flow_type::TrapFlowRef {
        self.task_mut_for_ref(task_ref)
            .map(|task| task.root_trap_flow_ref())
            .unwrap_or(crate::objects::trap_flow_type::TrapFlowRef::NONE)
    }

    pub(crate) fn task_root_trap_flow_ref_raw(
        &self,
        task_ref: TaskRef,
    ) -> crate::objects::trap_flow_type::TrapFlowRef {
        self.current_task_candidate(task_ref)
            .map(|candidate| candidate.task.root_trap_flow_ref())
            .unwrap_or(crate::objects::trap_flow_type::TrapFlowRef::NONE)
    }

    pub(crate) fn clear_task_root_trap_flow(
        &mut self,
        task_ref: TaskRef,
        root_ref: crate::objects::trap_flow_type::TrapFlowRef,
    ) -> Option<bool> {
        let cpu_ref = self.current_task_candidate(task_ref)?.flow.cpu_ref()?;
        let task_identity = crate::arch::riscv64::csr::read_tp();
        let cleared = self
            .task_mut_for_ref(task_ref)
            .map(|task| task.clear_root_trap_flow(root_ref))?;
        if cleared
            && !self
                .cpu_group
                .cpu_mut(cpu_ref.logical_id())?
                .trap_mut()
                .refresh_entry_task_root(
                    task_identity,
                    crate::objects::trap_flow_type::TrapFlowRef::NONE,
                )
        {
            return None;
        }
        Some(cleared)
    }

    pub fn current_cpu(&self) -> Result<crate::objects::cpu_group::CurrentCpu, CurrentTaskError> {
        let current = self.current_task()?;
        let tp = crate::arch::riscv64::csr::read_tp();
        let candidate = if current.task_ref().same_identity(TaskRef::BOOT) {
            self.boot_current_task_candidate()
        } else {
            self.current_task_candidate(current.task_ref())
        }
        .ok_or_else(|| {
            self.current_task_error(
                CurrentTaskErrorCode::MissingActiveFlow,
                tp,
                current.task_ref(),
            )
        })?;
        let cpu_ref = candidate.flow.cpu_ref().ok_or_else(|| {
            self.current_task_error(CurrentTaskErrorCode::MissingCpu, tp, current.task_ref())
        })?;
        self.cpu_group.current_cpu(candidate.flow).ok_or_else(|| {
            CurrentTaskError::new(
                CurrentTaskErrorCode::MissingCpu,
                CurrentTaskDiagnostic::new(
                    tp,
                    current.task_ref(),
                    candidate.flow.flow_ref(),
                    cpu_ref,
                ),
            )
        })
    }

    fn task_ref_from_identity(&self, identity: usize) -> Option<TaskRef> {
        let boot_address = self.boot_task.carrier_address();
        let boot_physical = self.kernel_image.runtime_to_phys(boot_address);
        let boot_virtual = self.kernel_image.runtime_to_link(boot_address);
        if identity == boot_address
            || boot_physical == Some(identity)
            || boot_virtual == Some(identity)
        {
            return Some(TaskRef::BOOT);
        }
        if identity == self.kernel_init_task.task() as *const Task as usize {
            return Some(self.kernel_init_task.task_ref());
        }
        if identity == self.kthreadd_task.task() as *const Task as usize {
            return Some(self.kthreadd_task.task_ref());
        }
        if let Some(candidate) = self
            .scheduler_test_tasks
            .current_task_candidate_by_identity(identity)
        {
            return Some(candidate.task.task_ref());
        }
        if let Some(candidate) = self
            .user_task_set
            .current_task_candidate_by_identity(identity)
        {
            return Some(candidate.task.task_ref());
        }
        crate::objects::smp_bringup::ap_current_task_candidate_by_identity(identity)
            .map(|candidate| candidate.task.task_ref())
    }

    fn task_mut_for_ref(&mut self, task_ref: TaskRef) -> Option<&mut Task> {
        match task_ref {
            TaskRef::BOOT => Some(self.boot_task.task_mut()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.task_mut()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.task_mut()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.scheduler_test_tasks
                    .smoke_scheduler_task_mut()
                    .task_mut(),
            ),
            TaskRef::SMOKE_MUTEX => {
                Some(self.scheduler_test_tasks.smoke_mutex_task_mut().task_mut())
            }
            TaskRef::SMOKE_RWSEM => {
                Some(self.scheduler_test_tasks.smoke_rwsem_task_mut().task_mut())
            }
            TaskRef::SMOKE_RWLOCK => {
                Some(self.scheduler_test_tasks.smoke_rwlock_task_mut().task_mut())
            }
            _ if task_ref.is_user() => self.user_task_set.task_mut_by_ref(task_ref),
            _ => crate::objects::smp_bringup::ap_task_mut_by_ref(task_ref),
        }
    }

    fn current_task_candidate(&self, task_ref: TaskRef) -> Option<CurrentTaskCandidate<'_>> {
        let candidate = if task_ref.same_identity(TaskRef::BOOT) {
            self.boot_current_task_candidate()
        } else if task_ref.same_identity(TaskRef::KERNEL_INIT) {
            let task = self.kernel_init_task.task();
            let effective_flow = if task.active_flow().is_valid() {
                task.active_flow()
            } else {
                task.initial_flow()
            };
            let flow = self.flow_for_kernel_init_task(effective_flow)?;
            Some(CurrentTaskCandidate { task, flow })
        } else if task_ref.same_identity(TaskRef::KTHREADD) {
            let task = self.kthreadd_task.task();
            let effective_flow = if task.active_flow().is_valid() {
                task.active_flow()
            } else {
                task.initial_flow()
            };
            if !effective_flow.same_identity(self.kthreadd_flow.flow_ref()) {
                return None;
            }
            Some(CurrentTaskCandidate {
                task,
                flow: self.kthreadd_flow.core(),
            })
        } else if let Some(candidate) = self
            .scheduler_test_tasks
            .current_task_candidate_by_ref(task_ref)
        {
            Some(candidate)
        } else if let Some(candidate) = self.user_task_set.current_task_candidate_by_ref(task_ref) {
            Some(candidate)
        } else {
            crate::objects::smp_bringup::ap_current_task_candidate_by_ref(task_ref)
        }?;
        Some(candidate)
    }

    /// Early boot executes through the physical alias with `satp=0`. Keep the
    /// BootTask selector path out of the general multi-task dispatch so the
    /// compiler cannot lower it through a virtual-address jump table.
    #[inline(never)]
    fn boot_current_task_candidate(&self) -> Option<CurrentTaskCandidate<'_>> {
        let task = self.boot_task.task();
        let active_flow = task.active_flow();
        if active_flow.same_identity(TaskFlowRef::BOOT_INIT) {
            Some(CurrentTaskCandidate {
                task,
                flow: self.boot_init_flow.core(),
            })
        } else if active_flow.same_identity(TaskFlowRef::BOOT_IDLE) {
            Some(CurrentTaskCandidate {
                task,
                flow: self.boot_idle_flow.core(),
            })
        } else {
            None
        }
    }

    fn flow_for_kernel_init_task(&self, flow_ref: TaskFlowRef) -> Option<&TaskFlow> {
        if flow_ref.same_identity(self.kernel_init_flow.flow_ref()) {
            Some(self.kernel_init_flow.core())
        } else if flow_ref.same_identity(self.user_app_flow.flow_ref()) {
            Some(self.user_app_flow.current_core())
        } else {
            None
        }
    }

    fn resolve_current_task_identity(
        &self,
        tp: usize,
        requested_ref: TaskRef,
    ) -> Result<CurrentTask, CurrentTaskError> {
        let Some(identity_ref) = self.task_ref_from_identity(tp) else {
            return Err(self.current_task_error(
                CurrentTaskErrorCode::UnknownIdentity,
                tp,
                requested_ref,
            ));
        };
        if identity_ref.same_identity(TaskRef::BOOT) {
            let Some(candidate) = self.boot_current_task_candidate() else {
                return Err(self.current_task_error(
                    CurrentTaskErrorCode::MissingActiveFlow,
                    tp,
                    identity_ref,
                ));
            };
            return validate_candidate(tp, requested_ref, candidate);
        }
        let Some(candidate) = self.current_task_candidate(identity_ref) else {
            return Err(self.current_task_error(
                CurrentTaskErrorCode::MissingActiveFlow,
                tp,
                identity_ref,
            ));
        };
        validate_candidate(tp, requested_ref, candidate)
    }

    fn current_task_error(
        &self,
        code: CurrentTaskErrorCode,
        tp: usize,
        task_ref: TaskRef,
    ) -> CurrentTaskError {
        let candidate = if task_ref.same_identity(TaskRef::BOOT) {
            self.boot_current_task_candidate()
        } else {
            self.current_task_candidate(task_ref)
        };
        let (flow_ref, cpu_ref) = candidate
            .map(|candidate| {
                (
                    candidate.task.active_flow(),
                    candidate
                        .flow
                        .cpu_ref()
                        .unwrap_or(crate::objects::cpu::CpuRef::invalid()),
                )
            })
            .unwrap_or((TaskFlowRef::NONE, crate::objects::cpu::CpuRef::invalid()));
        CurrentTaskError::new(
            code,
            CurrentTaskDiagnostic::new(tp, task_ref, flow_ref, cpu_ref),
        )
    }

    #[cfg(app_smoke)]
    pub(crate) fn resolve_current_task_for_test(
        &self,
        tp: usize,
        requested_ref: TaskRef,
    ) -> Result<CurrentTask, CurrentTaskError> {
        self.resolve_current_task_identity(tp, requested_ref)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn setup_smoke_scheduler_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            scheduler_test_stacks,
            ..
        } = self;
        let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Online,
                State::Online,
            );
        };
        scheduler.setup_smoke_scheduler_task(entry, scheduler_test_stacks, scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn enqueue_smoke_scheduler_task(&mut self) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            ..
        } = self;
        cpu_group
            .boot_scheduler_mut()
            .ok_or_else(|| {
                crate::objects::state::EventError::failed(
                    crate::objects::state::EventErrorCode::ConditionFailed,
                    LifecycleEvent::Setup,
                    State::Base,
                    State::Online,
                    State::Online,
                )
            })?
            .enqueue_smoke_scheduler_task(scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn setup_smoke_mutex_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            scheduler_test_stacks,
            ..
        } = self;
        let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Online,
                State::Online,
            );
        };
        scheduler.setup_smoke_mutex_task(entry, scheduler_test_stacks, scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn enqueue_smoke_mutex_task(&mut self) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            ..
        } = self;
        cpu_group
            .boot_scheduler_mut()
            .ok_or_else(missing_scheduler_error)?
            .enqueue_smoke_mutex_task(scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn setup_smoke_rwsem_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            scheduler_test_stacks,
            ..
        } = self;
        let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Online,
                State::Online,
            );
        };
        scheduler.setup_smoke_rwsem_task(entry, scheduler_test_stacks, scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn enqueue_smoke_rwsem_task(&mut self) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            ..
        } = self;
        cpu_group
            .boot_scheduler_mut()
            .ok_or_else(missing_scheduler_error)?
            .enqueue_smoke_rwsem_task(scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn setup_smoke_rwlock_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            scheduler_test_stacks,
            ..
        } = self;
        let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Online,
                State::Online,
            );
        };
        scheduler.setup_smoke_rwlock_task(entry, scheduler_test_stacks, scheduler_test_tasks)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn enqueue_smoke_rwlock_task(&mut self) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            ..
        } = self;
        cpu_group
            .boot_scheduler_mut()
            .ok_or_else(missing_scheduler_error)?
            .enqueue_smoke_rwlock_task(scheduler_test_tasks)
    }

    pub fn schedule_current(&mut self) -> EventResult {
        let current_task = self.current_task().map_err(current_task_event_error)?;
        let sender_flow_ref = self
            .current_task_flow_ref()
            .map_err(current_task_event_error)?;
        let current_cpu = self.current_cpu().map_err(current_task_event_error)?;
        self.schedule_from_refs(
            sender_flow_ref,
            current_task.task_ref(),
            current_cpu.cpu_ref(),
        )
    }

    fn schedule_from_refs(
        &mut self,
        sender_flow_ref: TaskFlowRef,
        current_task_ref: TaskRef,
        current_cpu_ref: crate::objects::cpu::CpuRef,
    ) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            ..
        } = self;
        let Some((scheduler, local_interrupt)) = cpu_group.boot_scheduler_and_local_interrupt_mut()
        else {
            return crate::objects::state::failed_condition(
                crate::objects::state::LifecycleEvent::Setup,
                crate::objects::state::State::Base,
                crate::objects::state::State::Online,
                crate::objects::state::State::Online,
            );
        };
        let mut task_access = SchedulerTaskAccess::new(
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            scheduler_test_tasks,
        );
        scheduler.schedule(
            sender_flow_ref,
            current_task_ref,
            current_cpu_ref,
            &mut task_access,
            local_interrupt,
        )?;
        let current_task = self.current_task().map_err(current_task_event_error)?;
        if self.scheduler().switch_to_entry_prev_ref() != current_task.task_ref()
            && self.scheduler().switch_to_entry_next_ref() == current_task.task_ref()
            && self.scheduler().schedule_exit_current_ref() != current_task.task_ref()
        {
            self.scheduler_mut().record_schedule_exit(current_task)?;
        }
        Ok(())
    }

    #[cfg(app_smoke)]
    pub(crate) fn schedule_from_refs_for_test(
        &mut self,
        sender_flow_ref: TaskFlowRef,
        current_task_ref: TaskRef,
        current_cpu_ref: crate::objects::cpu::CpuRef,
    ) -> EventResult {
        self.schedule_from_refs(sender_flow_ref, current_task_ref, current_cpu_ref)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn declare_current_scheduler_sleep(&mut self) -> EventResult {
        let task_ref = self.current_task_ref().map_err(current_task_event_error)?;
        let Some(task) = self.task_mut_for_ref(task_ref) else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::OnCpu,
                State::OnCpu,
            );
        };
        task.declare_scheduler_sleep()
    }

    pub fn replace_current_user_task(
        &mut self,
        previous: crate::objects::task::TaskRef,
        next: crate::objects::task::TaskRef,
        next_pid: usize,
    ) -> EventResult {
        let current_task = self.current_task().map_err(current_task_event_error)?;
        self.replace_current_user_task_with_capability(previous, next, next_pid, current_task)
    }

    /// Commits a terminal user-task switch using the selector proof captured
    /// synchronously before the exiting Flow was retired. The old Task is not
    /// reclaimed until `dispatch_task_after_switch` has established `next`.
    #[cfg_attr(not(any(app_smoke, app_user_boot)), allow(dead_code))]
    pub fn replace_terminal_user_task(
        &mut self,
        previous: crate::objects::task::TaskRef,
        next: crate::objects::task::TaskRef,
        next_pid: usize,
        current_task: CurrentTask,
    ) -> EventResult {
        self.replace_current_user_task_with_capability(previous, next, next_pid, current_task)
    }

    fn replace_current_user_task_with_capability(
        &mut self,
        previous: crate::objects::task::TaskRef,
        next: crate::objects::task::TaskRef,
        next_pid: usize,
        current_task: CurrentTask,
    ) -> EventResult {
        if !current_task.task_ref().same_identity(previous) {
            return crate::objects::state::failed_condition(
                LifecycleEvent::Continue,
                State::OnCpu,
                State::OnCpu,
                State::OnCpu,
            );
        }
        self.scheduler_mut()
            .replace_user_task_on_runqueue(previous, next, next_pid)?;
        {
            let Self {
                cpu_group,
                scheduler_test_tasks,
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                ..
            } = self;
            let scheduler = cpu_group.boot_scheduler_mut().ok_or_else(|| {
                EventError::failed(
                    EventErrorCode::ConditionFailed,
                    LifecycleEvent::Setup,
                    State::Base,
                    State::Online,
                    State::Online,
                )
            })?;
            let task_access = SchedulerTaskAccess::new(
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                scheduler_test_tasks,
            );
            scheduler.prepare_simulated_task_switch(previous, next, current_task, &task_access)?;
        }
        self.establish_simulated_task_identity(next)?;
        self.finish_task_switch(next)
    }

    pub fn finish_task_switch(&mut self, next: crate::objects::task::TaskRef) -> EventResult {
        let Self {
            cpu_group,
            scheduler_test_tasks,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            ..
        } = self;
        let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
            return failed_condition(
                LifecycleEvent::Continue,
                State::Base,
                State::Online,
                State::Online,
            );
        };
        let mut task_access = SchedulerTaskAccess::new(
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            scheduler_test_tasks,
        );
        scheduler.dispatch_task_after_switch(next, &mut task_access)?;
        self.refresh_current_cpu_trap_entry_task(next)?;
        let current_task = self
            .resolve_current_task_identity(crate::arch::riscv64::csr::read_tp(), next)
            .map_err(current_task_event_error)?;
        self.scheduler_mut().record_schedule_exit(current_task)
    }

    fn refresh_current_cpu_trap_entry_task(&mut self, task_ref: TaskRef) -> EventResult {
        let task_identity = crate::arch::riscv64::csr::read_tp();
        let Some(candidate) = self.current_task_candidate(task_ref) else {
            return Err(current_task_event_error(self.current_task_error(
                CurrentTaskErrorCode::UnknownIdentity,
                task_identity,
                task_ref,
            )));
        };
        let Some(cpu_ref) = candidate.flow.cpu_ref() else {
            return Err(current_task_event_error(self.current_task_error(
                CurrentTaskErrorCode::MissingCpu,
                task_identity,
                task_ref,
            )));
        };
        let root_ref = candidate.task.root_trap_flow_ref();
        if !self
            .cpu_group
            .cpu_mut(cpu_ref.logical_id())
            .is_some_and(|cpu| {
                cpu.trap_mut()
                    .refresh_entry_task_root(task_identity, root_ref)
            })
        {
            return Err(current_task_event_error(self.current_task_error(
                CurrentTaskErrorCode::UnknownIdentity,
                task_identity,
                task_ref,
            )));
        }
        Ok(())
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn smoke_identity_switch(&mut self) -> EventResult {
        let current_task = self.current_task().map_err(current_task_event_error)?;
        let current = current_task.task_ref();
        let Self {
            cpu_group,
            scheduler_test_tasks,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            ..
        } = self;
        let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Online,
                State::Online,
            );
        };
        let task_access = SchedulerTaskAccess::new(
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            scheduler_test_tasks,
        );
        scheduler.prepare_simulated_task_switch(current, current, current_task, &task_access)
    }

    fn establish_simulated_task_identity(&self, task_ref: TaskRef) -> EventResult {
        let identity = match task_ref {
            TaskRef::BOOT => Some(self.boot_task.carrier_address()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.task() as *const Task as usize),
            TaskRef::KTHREADD => Some(self.kthreadd_task.task() as *const Task as usize),
            _ if task_ref.is_user() => self.user_task_set.task_identity_ptr(task_ref),
            _ => self
                .scheduler_test_tasks
                .current_task_candidate_by_ref(task_ref)
                .map(|candidate| candidate.task as *const Task as usize),
        };
        let Some(identity) = identity else {
            return Err(current_task_event_error(self.current_task_error(
                CurrentTaskErrorCode::UnknownIdentity,
                crate::arch::riscv64::csr::read_tp(),
                task_ref,
            )));
        };
        crate::arch::riscv64::csr::write_tp(identity);
        Ok(())
    }

    pub fn cleanup_current_task_for_shutdown(&mut self) -> bool {
        if self.user_task_set.active_task_ref().is_valid() {
            let Some(task_ref) = self.user_task_set.prepare_active_task_for_shutdown() else {
                return false;
            };
            let Self {
                cpu_group,
                scheduler_test_tasks,
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                ..
            } = self;
            let mut task_access = SchedulerTaskAccess::new(
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                scheduler_test_tasks,
            );
            cpu_group
                .boot_scheduler_mut()
                .is_some_and(|scheduler| scheduler.suspend_task(task_ref, &mut task_access).is_ok())
        } else {
            if self
                .user_app_flow
                .cleanup_active_flow_for_shutdown(&mut self.kernel_init_task)
                .is_err()
                || {
                    let Self {
                        cpu_group,
                        scheduler_test_tasks,
                        kernel_init_task,
                        kernel_init_flow,
                        user_app_flow,
                        kthreadd_task,
                        kthreadd_flow,
                        boot_idle_flow,
                        user_task_set,
                        ..
                    } = self;
                    let mut task_access = SchedulerTaskAccess::new(
                        kernel_init_task,
                        kernel_init_flow,
                        user_app_flow,
                        kthreadd_task,
                        kthreadd_flow,
                        boot_idle_flow,
                        user_task_set,
                        scheduler_test_tasks,
                    );
                    cpu_group.boot_scheduler_mut().is_none_or(|scheduler| {
                        scheduler
                            .suspend_task(
                                crate::objects::task::TaskRef::KERNEL_INIT,
                                &mut task_access,
                            )
                            .is_err()
                    })
                }
            {
                return false;
            }
            self.user_app_flow
                .cleanup_task_after_shutdown_suspend(&mut self.kernel_init_task)
                .is_ok()
        }
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn commit_user_dispatch(&mut self, next: crate::objects::task::TaskRef) -> EventResult {
        if !next.is_user() {
            return crate::objects::state::failed_condition(
                crate::objects::state::LifecycleEvent::Setup,
                crate::objects::state::State::Destroyed,
                crate::objects::state::State::Online,
                crate::objects::state::State::Online,
            );
        }
        let current_task = self.current_task().map_err(current_task_event_error)?;
        let previous = current_task.task_ref();
        if previous.same_identity(next) {
            return Ok(());
        }
        {
            let Self {
                cpu_group,
                scheduler_test_tasks,
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                ..
            } = self;
            let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
                return failed_condition(
                    LifecycleEvent::Setup,
                    State::Base,
                    State::Online,
                    State::Online,
                );
            };
            let task_access = SchedulerTaskAccess::new(
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                scheduler_test_tasks,
            );
            scheduler.prepare_simulated_task_switch(previous, next, current_task, &task_access)?;
        }
        self.establish_simulated_task_identity(next)?;
        self.finish_task_switch(next)
    }

    #[cfg_attr(not(app_user_boot), allow(dead_code))]
    pub fn commit_terminal_kernel_init_dispatch(
        &mut self,
        current_task: CurrentTask,
    ) -> EventResult {
        self.commit_kernel_init_dispatch_with_capability(current_task)
    }

    fn commit_kernel_init_dispatch_with_capability(
        &mut self,
        current_task: CurrentTask,
    ) -> EventResult {
        let next = crate::objects::task::TaskRef::KERNEL_INIT;
        let previous = current_task.task_ref();
        if previous.same_identity(next) {
            return Ok(());
        }
        {
            let Self {
                cpu_group,
                scheduler_test_tasks,
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                ..
            } = self;
            let Some(scheduler) = cpu_group.boot_scheduler_mut() else {
                return failed_condition(
                    LifecycleEvent::Setup,
                    State::Base,
                    State::Online,
                    State::Online,
                );
            };
            let task_access = SchedulerTaskAccess::new(
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                scheduler_test_tasks,
            );
            scheduler.prepare_simulated_task_switch(previous, next, current_task, &task_access)?;
        }
        self.establish_simulated_task_identity(next)?;
        self.finish_task_switch(next)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_scheduler_entry_ran(&mut self) -> EventResult {
        self.finish_task_switch(crate::objects::task::TaskRef::SMOKE_SCHEDULER)?;
        self.scheduler_test_tasks
            .smoke_scheduler_task_mut()
            .mark_entry_ran()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_scheduler_yielded_back(&mut self) -> EventResult {
        self.scheduler_test_tasks
            .smoke_scheduler_task_mut()
            .mark_yielded_back()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_mutex_entry_ran(&mut self) -> EventResult {
        self.finish_task_switch(crate::objects::task::TaskRef::SMOKE_MUTEX)?;
        self.scheduler_test_tasks
            .smoke_mutex_task_mut()
            .mark_entry_ran()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_mutex_yielded_back(&mut self) -> EventResult {
        self.scheduler_test_tasks
            .smoke_mutex_task_mut()
            .mark_yielded_back()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_rwsem_entry_ran(&mut self) -> EventResult {
        self.finish_task_switch(crate::objects::task::TaskRef::SMOKE_RWSEM)?;
        self.scheduler_test_tasks
            .smoke_rwsem_task_mut()
            .mark_entry_ran()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_rwsem_yielded_back(&mut self) -> EventResult {
        self.scheduler_test_tasks
            .smoke_rwsem_task_mut()
            .mark_yielded_back()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_rwlock_entry_ran(&mut self) -> EventResult {
        self.finish_task_switch(crate::objects::task::TaskRef::SMOKE_RWLOCK)?;
        self.scheduler_test_tasks
            .smoke_rwlock_task_mut()
            .mark_entry_ran()
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn mark_smoke_rwlock_yielded_back(&mut self) -> EventResult {
        self.scheduler_test_tasks
            .smoke_rwlock_task_mut()
            .mark_yielded_back()
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

fn current_task_event_error(error: CurrentTaskError) -> EventError {
    let diagnostic = error.diagnostic();
    let task_ref = diagnostic.task_ref();
    let flow_ref = diagnostic.flow_ref();
    let cpu_ref = diagnostic.cpu_ref();
    crate::objects::printk::write_fmt(format_args!(
        "CurrentTask resolution failed: code={} tp={:#x} TaskRef={}:{} FlowRef={}:{} CpuRef={}\n",
        error.code() as usize,
        diagnostic.tp(),
        task_ref.slot(),
        task_ref.generation(),
        flow_ref.slot(),
        flow_ref.generation(),
        cpu_ref.logical_id(),
    ));
    EventError::failed(
        EventErrorCode::ConditionFailed,
        LifecycleEvent::Continue,
        State::Online,
        State::OnCpu,
        State::OnCpu,
    )
}

fn missing_scheduler_error() -> EventError {
    EventError::failed(
        EventErrorCode::ConditionFailed,
        LifecycleEvent::Setup,
        State::Base,
        State::Online,
        State::Online,
    )
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
