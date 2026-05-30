#[cfg(checkpoint_sbi_char)]
use core::sync::atomic::AtomicU8;
use core::sync::atomic::{AtomicBool, Ordering};

const CHECKPOINT_PROBE: &str = match option_env!("CHECKPOINT_PROBE") {
    Some(value) => value,
    None => "",
};
const PROBE_MEMBLOCK_ONLINE_STOP: &str = "memblock-online-stop";

#[cfg(checkpoint_sbi_char)]
const TRACE_MODE_EARLY_BYTE: u8 = 0;
#[cfg(checkpoint_sbi_char)]
const TRACE_MODE_NAMED_STRING: u8 = 1;

#[cfg(checkpoint_sbi_char)]
static TRACE_MODE: AtomicU8 = AtomicU8::new(TRACE_MODE_EARLY_BYTE);

static POST_VM_CHECKPOINTS_ENABLED: AtomicBool = AtomicBool::new(false);
static CHECKPOINT_HANDLER_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn enable_post_vm_checkpoints() {
    POST_VM_CHECKPOINTS_ENABLED.store(true, Ordering::Release);

    #[cfg(checkpoint_sbi_char)]
    enable_named_checkpoints();
}

#[cfg(checkpoint_sbi_char)]
fn enable_named_checkpoints() {
    if TRACE_MODE.swap(TRACE_MODE_NAMED_STRING, Ordering::Relaxed) == TRACE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(b'\n');
    }
}

pub fn checkpoint(checkpoint: Checkpoint) {
    if !control_handlers_configured() {
        trace_checkpoint(checkpoint);
        return;
    }

    if CHECKPOINT_HANDLER_ACTIVE.load(Ordering::Acquire) {
        checkpoint_reentry_shutdown();
    }

    trace_checkpoint(checkpoint);

    match dispatch_post_vm_handlers(checkpoint) {
        CheckpointOutcome::Continue => {}
        CheckpointOutcome::FailAndShutdown => {
            crate::arch::riscv64::sbi::putstr("checkpoint fail: ");
            crate::arch::riscv64::sbi::putstr(checkpoint.name());
            crate::arch::riscv64::sbi::putchar(b'\n');
            crate::arch::riscv64::sbi::system_shutdown();
        }
        CheckpointOutcome::StopAndShutdown => {
            crate::arch::riscv64::sbi::putstr("checkpoint stop: ");
            crate::arch::riscv64::sbi::putstr(checkpoint.name());
            crate::arch::riscv64::sbi::putchar(b'\n');
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }
}

#[cfg(checkpoint_sbi_char)]
fn trace_checkpoint(checkpoint: Checkpoint) {
    if TRACE_MODE.load(Ordering::Relaxed) == TRACE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(checkpoint.early_byte());
        return;
    }

    crate::arch::riscv64::sbi::putstr("trace: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putchar(b'\n');
}

#[cfg(not(checkpoint_sbi_char))]
fn trace_checkpoint(_checkpoint: Checkpoint) {}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CheckpointOutcome {
    Continue,
    FailAndShutdown,
    StopAndShutdown,
}

fn dispatch_post_vm_handlers(checkpoint: Checkpoint) -> CheckpointOutcome {
    if !POST_VM_CHECKPOINTS_ENABLED.load(Ordering::Acquire) || !control_handlers_configured() {
        return CheckpointOutcome::Continue;
    }

    if CHECKPOINT_HANDLER_ACTIVE.swap(true, Ordering::AcqRel) {
        checkpoint_reentry_shutdown();
    }

    let outcome = configured_control_handlers(checkpoint);
    CHECKPOINT_HANDLER_ACTIVE.store(false, Ordering::Release);
    outcome
}

fn configured_control_handlers(checkpoint: Checkpoint) -> CheckpointOutcome {
    match CHECKPOINT_PROBE {
        PROBE_MEMBLOCK_ONLINE_STOP => memblock_online_stop_handler(checkpoint),
        _ => unknown_probe_shutdown(),
    }
}

fn control_handlers_configured() -> bool {
    !CHECKPOINT_PROBE.is_empty()
}

fn memblock_online_stop_handler(checkpoint: Checkpoint) -> CheckpointOutcome {
    if checkpoint == Checkpoint::MemBlockOnline {
        CheckpointOutcome::StopAndShutdown
    } else {
        CheckpointOutcome::Continue
    }
}

fn unknown_probe_shutdown() -> ! {
    crate::arch::riscv64::sbi::putstr("checkpoint probe unknown: ");
    crate::arch::riscv64::sbi::putstr(CHECKPOINT_PROBE);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn checkpoint_reentry_shutdown() -> ! {
    crate::arch::riscv64::sbi::putstr("checkpoint reentry\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Checkpoint {
    StartupTimelineStarted,
    #[allow(dead_code)]
    StartupTimelineReady,
    PreparePhaseReady,
    PreparePhaseOnline,
    BootPhaseStarted,
    #[allow(dead_code)]
    BootPhaseReady,
    EntryPreludePhaseStarted,
    #[allow(dead_code)]
    EntryPreludePhaseReady,
    #[allow(dead_code)]
    EntrySuccessorPhaseStarted,
    #[allow(dead_code)]
    EntrySuccessorPhaseReady,
    EntryPreludePhaseDestroyed,
    InterruptStreamPrepared,
    InterruptStreamReady,
    KernelImagePrepared,
    RootStreamPrepared,
    KernelImageReady,
    BootCpuPrepared,
    BootCpuReady,
    BootCpuOnline,
    CpuGroupPrepared,
    InitTaskPrepared,
    InitStackPrepared,
    EventStreamPrepared,
    ExceptionStreamPrepared,
    TrampolineVmReady,
    TrampolineVmOnline,
    TrampolineVmDestroyed,
    RawDtbPrepared,
    RawDtbReady,
    FixMapReady,
    EarlyVmPrepared,
    EarlyVmReady,
    EarlyVmOnline,
    VmPrepared,
    VmReady,
    VmOnline,
    KernelImageOnline,
    EventStreamReady,
    InitTaskOnline,
    InitStackReady,
    InitStackOnline,
    SocPrepared,
    EarlyDtbPrepared,
    EarlyDtbReady,
    EarlyDtbDestroyed,
    PlatformCpuInfoReady,
    PlatformCpuInfoOnline,
    PhysicalMemoryReady,
    PhysicalMemoryOnline,
    CpuIdMapPrepared,
    CpuIdMapReady,
    CommandLinePrepared,
    KernelCmdlineReady,
    InitMmReady,
    EarlyIoremapReady,
    SbiReady,
    EarlyParamReady,
    MemBlockPrepared,
    MemBlockReady,
    MemBlockOnline,
    MemBlockOffline,
    SwapperVmReady,
    SwapperVmOnline,
    EarlyVmDestroyed,
    #[allow(dead_code)]
    PrintkBufferPrepared,
    #[allow(dead_code)]
    EarlyConPrepared,
    #[allow(dead_code)]
    EarlyConReady,
    #[allow(dead_code)]
    EarlyConOnline,
    CorePreparePhaseStarted,
    CorePreparePhaseReady,
    DeviceTreeReady,
    ZonesReady,
    ResourceTreeReady,
    CpuGroupReady,
    CacheBlockInfoReady,
    CpuCapabilitiesReady,
    DmaCachePolicyReady,
    StaticBranchReady,
    CommandLineReady,
    SavedCommandLineReady,
    StaticCommandLineReady,
    SetupNrCpuIdsCheckpoint,
    PerCpuStaticImageReady,
    PerCpuFirstChunkReady,
    PerCpuOffsetTableReady,
    PerCpuStorageReady,
    CpuHotplugStateReady,
    SecondParseEarlyParamCheckpoint,
    BootParamReady,
    PrintUnknownBootoptionsCheckpoint,
    PayloadParamReady,
    RandomnessPrepared,
    PrintkBufferReady,
    ExceptionTableReady,
    ExceptionStreamReady,
    MmCoreInitPhaseStarted,
    MmCoreInitPhaseReady,
    MemoryTopologyReady,
    BootMemoryNodeReady,
    BootZoneSetReady,
    BootZonelistSetReady,
    PageAllocatorPrepared,
    MemoryDebugHardeningReady,
    StackDepotReady,
    SwiotlbReady,
    PageAllocatorReady,
    SlubAllocatorPrepared,
    SlubCacheRegistryReady,
    KmallocCachesReady,
    SlubAllocatorReady,
    PageTableLockCacheReady,
    PageTableCachesReady,
    VmapAreaCacheReady,
    VmapAddressSpaceReady,
    VmapNodeSetReady,
    VmapBlockQueuesReady,
    VfreeDeferredSetReady,
    VmallocAllocatorReady,
    MmStructCacheReady,
    BreakpointExceptionHandled,
    PayloadPhaseReady,
    PayloadPhaseOnline,
}

impl Checkpoint {
    #[allow(dead_code)]
    const fn early_byte(self) -> u8 {
        match self {
            Self::EntryPreludePhaseStarted => b'A',
            Self::InterruptStreamPrepared => b'I',
            Self::KernelImagePrepared => b'K',
            Self::RootStreamPrepared => b'O',
            Self::KernelImageReady => b'Z',
            Self::BootCpuPrepared => b'H',
            Self::CpuGroupPrepared => b'G',
            Self::InitTaskPrepared => b'T',
            Self::InitStackPrepared => b'S',
            Self::EventStreamPrepared => b'V',
            Self::ExceptionStreamPrepared => b'9',
            Self::TrampolineVmReady => b'Q',
            Self::RawDtbPrepared => b'Y',
            Self::RawDtbReady => b'W',
            Self::FixMapReady => b'M',
            Self::EarlyVmPrepared => b'N',
            Self::EarlyVmReady => b'J',
            Self::VmPrepared => b'U',
            _ => b'?',
        }
    }

    #[allow(dead_code)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::StartupTimelineStarted => "StartupTimeline.Started",
            Self::StartupTimelineReady => "StartupTimeline.Ready",
            Self::PreparePhaseReady => "PreparePhase.Ready",
            Self::PreparePhaseOnline => "PreparePhase.Online",
            Self::BootPhaseStarted => "BootPhase.Started",
            Self::BootPhaseReady => "BootPhase.Ready",
            Self::EntryPreludePhaseStarted => "EntryPreludePhase.Started",
            Self::EntryPreludePhaseReady => "EntryPreludePhase.Ready",
            Self::EntrySuccessorPhaseStarted => "EntrySuccessorPhase.Started",
            Self::EntrySuccessorPhaseReady => "EntrySuccessorPhase.Ready",
            Self::EntryPreludePhaseDestroyed => "EntryPreludePhase.Destroyed",
            Self::InterruptStreamPrepared => "InterruptStream.Prepared",
            Self::InterruptStreamReady => "InterruptStream.Ready",
            Self::KernelImagePrepared => "KernelImage.Prepared",
            Self::RootStreamPrepared => "RootStream.Prepared",
            Self::KernelImageReady => "KernelImage.Ready",
            Self::BootCpuPrepared => "BootCPU.Prepared",
            Self::BootCpuReady => "BootCPU.Ready",
            Self::BootCpuOnline => "BootCPU.Online",
            Self::CpuGroupPrepared => "CpuGroup.Prepared",
            Self::InitTaskPrepared => "InitTask.Prepared",
            Self::InitStackPrepared => "InitStack.Prepared",
            Self::EventStreamPrepared => "EventStream.Prepared",
            Self::ExceptionStreamPrepared => "ExceptionStream.Prepared",
            Self::TrampolineVmReady => "TrampolineVm.Ready",
            Self::TrampolineVmOnline => "TrampolineVm.Online",
            Self::TrampolineVmDestroyed => "TrampolineVm.Destroyed",
            Self::RawDtbPrepared => "RawDtb.Prepared",
            Self::RawDtbReady => "RawDtb.Ready",
            Self::FixMapReady => "FixMap.Ready",
            Self::EarlyVmPrepared => "EarlyVm.Prepared",
            Self::EarlyVmReady => "EarlyVm.Ready",
            Self::EarlyVmOnline => "EarlyVm.Online",
            Self::VmPrepared => "Vm.Prepared",
            Self::VmReady => "Vm.Ready",
            Self::VmOnline => "Vm.Online",
            Self::KernelImageOnline => "KernelImage.Online",
            Self::EventStreamReady => "EventStream.Ready",
            Self::InitTaskOnline => "InitTask.Online",
            Self::InitStackReady => "InitStack.Ready",
            Self::InitStackOnline => "InitStack.Online",
            Self::SocPrepared => "Soc.Prepared",
            Self::EarlyDtbPrepared => "EarlyDtb.Prepared",
            Self::EarlyDtbReady => "EarlyDtb.Ready",
            Self::EarlyDtbDestroyed => "EarlyDtb.Destroyed",
            Self::PlatformCpuInfoReady => "PlatformCpuInfo.Ready",
            Self::PlatformCpuInfoOnline => "PlatformCpuInfo.Online",
            Self::PhysicalMemoryReady => "PhysicalMemory.Ready",
            Self::PhysicalMemoryOnline => "PhysicalMemory.Online",
            Self::CpuIdMapPrepared => "CpuIdMap.Prepared",
            Self::CpuIdMapReady => "CpuIdMap.Ready",
            Self::CommandLinePrepared => "CommandLine.Prepared",
            Self::KernelCmdlineReady => "KernelCmdline.Ready",
            Self::InitMmReady => "InitMM.Ready",
            Self::EarlyIoremapReady => "EarlyIoremap.Ready",
            Self::SbiReady => "SBI.Ready",
            Self::EarlyParamReady => "EarlyParam.Ready",
            Self::MemBlockPrepared => "MemBlock.Prepared",
            Self::MemBlockReady => "MemBlock.Ready",
            Self::MemBlockOnline => "MemBlock.Online",
            Self::MemBlockOffline => "MemBlock.Offline",
            Self::SwapperVmReady => "SwapperVm.Ready",
            Self::SwapperVmOnline => "SwapperVm.Online",
            Self::EarlyVmDestroyed => "EarlyVm.Destroyed",
            Self::PrintkBufferPrepared => "PrintkBuffer.Prepared",
            Self::EarlyConPrepared => "EarlyCon.Prepared",
            Self::EarlyConReady => "EarlyCon.Ready",
            Self::EarlyConOnline => "EarlyCon.Online",
            Self::CorePreparePhaseStarted => "CorePreparePhase.Started",
            Self::CorePreparePhaseReady => "CorePreparePhase.Ready",
            Self::DeviceTreeReady => "DeviceTree.Ready",
            Self::ZonesReady => "Zones.Ready",
            Self::ResourceTreeReady => "ResourceTree.Ready",
            Self::CpuGroupReady => "CpuGroup.Ready",
            Self::CacheBlockInfoReady => "CacheBlockInfo.Ready",
            Self::CpuCapabilitiesReady => "CpuCapabilities.Ready",
            Self::DmaCachePolicyReady => "DmaCachePolicy.Ready",
            Self::StaticBranchReady => "StaticBranch.Ready",
            Self::CommandLineReady => "CommandLine.Ready",
            Self::SavedCommandLineReady => "SavedCommandLine.Ready",
            Self::StaticCommandLineReady => "StaticCommandLine.Ready",
            Self::SetupNrCpuIdsCheckpoint => "SetupNrCpuIds.Checkpoint",
            Self::PerCpuStaticImageReady => "PerCpuStaticImage.Ready",
            Self::PerCpuFirstChunkReady => "PerCpuFirstChunk.Ready",
            Self::PerCpuOffsetTableReady => "PerCpuOffsetTable.Ready",
            Self::PerCpuStorageReady => "PerCpuStorage.Ready",
            Self::CpuHotplugStateReady => "CpuHotplugState.Ready",
            Self::SecondParseEarlyParamCheckpoint => "SecondParseEarlyParam.Checkpoint",
            Self::BootParamReady => "BootParam.Ready",
            Self::PrintUnknownBootoptionsCheckpoint => "PrintUnknownBootoptions.Checkpoint",
            Self::PayloadParamReady => "PayloadParam.Ready",
            Self::RandomnessPrepared => "Randomness.Prepared",
            Self::PrintkBufferReady => "PrintkBuffer.Ready",
            Self::ExceptionTableReady => "ExceptionTable.Ready",
            Self::ExceptionStreamReady => "ExceptionStream.Ready",
            Self::MmCoreInitPhaseStarted => "MmCoreInitPhase.Started",
            Self::MmCoreInitPhaseReady => "MmCoreInitPhase.Ready",
            Self::MemoryTopologyReady => "MemoryTopology.Ready",
            Self::BootMemoryNodeReady => "BootMemoryNode.Ready",
            Self::BootZoneSetReady => "BootZoneSet.Ready",
            Self::BootZonelistSetReady => "BootZonelistSet.Ready",
            Self::PageAllocatorPrepared => "PageAllocator.Prepared",
            Self::MemoryDebugHardeningReady => "MemoryDebugHardening.Ready",
            Self::StackDepotReady => "StackDepot.Ready",
            Self::SwiotlbReady => "Swiotlb.Ready",
            Self::PageAllocatorReady => "PageAllocator.Ready",
            Self::SlubAllocatorPrepared => "SlubAllocator.Prepared",
            Self::SlubCacheRegistryReady => "SlubCacheRegistry.Ready",
            Self::KmallocCachesReady => "KmallocCaches.Ready",
            Self::SlubAllocatorReady => "SlubAllocator.Ready",
            Self::PageTableLockCacheReady => "PageTableLockCache.Ready",
            Self::PageTableCachesReady => "PageTableCaches.Ready",
            Self::VmapAreaCacheReady => "VmapAreaCache.Ready",
            Self::VmapAddressSpaceReady => "VmapAddressSpace.Ready",
            Self::VmapNodeSetReady => "VmapNodeSet.Ready",
            Self::VmapBlockQueuesReady => "VmapBlockQueues.Ready",
            Self::VfreeDeferredSetReady => "VfreeDeferredSet.Ready",
            Self::VmallocAllocatorReady => "VmallocAllocator.Ready",
            Self::MmStructCacheReady => "MmStructCache.Ready",
            Self::BreakpointExceptionHandled => "BreakpointException.Handled",
            Self::PayloadPhaseReady => "PayloadPhase.Ready",
            Self::PayloadPhaseOnline => "PayloadPhase.Online",
        }
    }
}
