use core::{
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};

use super::{
    device_tree::DeviceTree,
    irq_time::{LogicalIrq, Plic},
    state::{failed_condition, EventResult, LifecycleEvent, State},
};

type LinuxInitcall = unsafe extern "C" fn() -> i32;
type LinuxPlatformProbe = unsafe extern "C" fn(*mut c_void) -> i32;
type LinuxCpuHotplugStartup = unsafe extern "C" fn(u32) -> i32;
type LinuxIrqDomainAlloc = unsafe extern "C" fn(*mut c_void, u32, u32, *mut c_void) -> i32;
type LinuxIrqFlowHandler = unsafe extern "C" fn(*mut c_void);
type LinuxIrqChipCallback = unsafe extern "C" fn(*mut c_void);

const PLIC_COMPATIBLE_SIFIVE: &[u8] = b"sifive,plic-1.0.0";
const PLIC_COMPATIBLE_RISCV: &[u8] = b"riscv,plic0";
const LINUX_RV_IRQ_EXT: u32 = 9;

unsafe extern "C" {
    static __linux_initcall6_start: usize;
    static __linux_initcall6_end: usize;
}

static PLATFORM_DRIVER_REGISTERED: AtomicBool = AtomicBool::new(false);
static PLATFORM_DRIVER_PTR: AtomicUsize = AtomicUsize::new(0);
static PLATFORM_DRIVER_MATCHED: AtomicBool = AtomicBool::new(false);
static PLATFORM_DRIVER_PROBE_ENTERED: AtomicBool = AtomicBool::new(false);
static PLATFORM_DRIVER_PROBE_RET: AtomicUsize = AtomicUsize::new(usize::MAX);
static LINUX_PLIC_MEMBASE: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_SOURCE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CONTEXT_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CONTEXT_ID: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_DOMAIN_PTR: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_PARENT_IRQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHAINED_IRQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHAINED_HANDLER: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHAINED_IS_CHAINED: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_DOMAIN_OPS: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_DOMAIN_HOST_DATA: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_EVENT_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_INITCALL_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_DRIVER_REGISTER_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_PLATFORM_MATCH_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_PROBE_ENTER_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_PROBE_RETURN_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_DOMAIN_INSTANTIATE_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_FIND_PARENT_DOMAIN_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CREATE_MAPPING_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_SET_HANDLER_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CPUHP_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_SYSCORE_SEQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_OF_IOMAP_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_OF_IRQ_COUNT_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_OF_IRQ_PARSE_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_OF_IRQ_PARSE_SUCCESSES: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_OF_MATCH_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_OF_PROPERTY_NDEV_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_RUNTIME_CLAIM_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_RUNTIME_ZERO_CLAIM_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_RUNTIME_DISPATCH_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_RUNTIME_COMPLETE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_RUNTIME_LOOP_EXIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_SAVED_RUST_TP: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_RUNTIME_LAST_CLAIMED_SOURCE: AtomicU32 = AtomicU32::new(0);
static LINUX_PLIC_RUNTIME_LAST_COMPLETED_SOURCE: AtomicU32 = AtomicU32::new(0);
static LINUX_PLIC_CHIP_ENABLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHIP_DISABLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHIP_MASK_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHIP_UNMASK_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHIP_EOI_COUNT: AtomicUsize = AtomicUsize::new(0);
static LINUX_PLIC_CHIP_CALLBACK_PROBE_SUCCESSES: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
struct LinuxPlatformDriver {
    probe: usize,
    remove: usize,
    shutdown: usize,
    suspend: usize,
    resume: usize,
    driver: LinuxDeviceDriver,
}

#[repr(C)]
struct LinuxDeviceDriver {
    name: usize,
    bus: usize,
    owner: usize,
    mod_name: usize,
    suppress_bind_attrs: bool,
    probe_type: u32,
    of_match_table: *const LinuxOfDeviceId,
}

#[repr(C)]
pub struct LinuxOfDeviceId {
    name: [u8; 32],
    device_type: [u8; 32],
    compatible: [u8; 128],
    data: usize,
}

#[repr(C, align(8))]
struct LinuxPlatformDeviceProbeView {
    bytes_before_fwnode: [u8; LINUX_PLATFORM_DEVICE_FWNODE_OFFSET],
    fwnode: *mut LinuxOfFwnodeView,
}

#[repr(C, align(8))]
struct LinuxOfFwnodeView {
    bytes_before_ops: [u8; LINUX_FWNODE_OPS_OFFSET],
    ops: *const usize,
}

#[repr(C)]
pub struct LinuxOfPhandleArgs {
    np: *mut c_void,
    args_count: i32,
    args: [u32; 16],
}

#[repr(C)]
struct LinuxIrqFwspec {
    fwnode: *mut c_void,
    param_count: i32,
    param: [u32; LINUX_IRQ_FWSPEC_PARAM_COUNT],
}

#[repr(C, align(16))]
struct LinuxThreadInfoView {
    bytes: [u8; LINUX_THREAD_INFO_SIZE],
}

#[repr(C, align(8))]
#[derive(Clone, Copy)]
struct LinuxIrqDescView {
    bytes: [u8; LINUX_IRQ_DESC_SIZE],
}

#[derive(Clone, Copy)]
struct LinuxPlicLeafIrqRecord {
    in_use: bool,
    virq: u32,
    hwirq: u32,
    chip: usize,
    flow_handler: usize,
}

impl LinuxPlicLeafIrqRecord {
    const fn empty() -> Self {
        Self {
            in_use: false,
            virq: 0,
            hwirq: 0,
            chip: 0,
            flow_handler: 0,
        }
    }
}

#[derive(Clone, Copy)]
enum LinuxIrqChipCallbackSlot {
    Enable,
    Disable,
    Mask,
    Unmask,
    Eoi,
}

impl LinuxIrqChipCallbackSlot {
    const fn offset(self) -> usize {
        match self {
            Self::Enable => LINUX_IRQ_CHIP_IRQ_ENABLE_OFFSET,
            Self::Disable => LINUX_IRQ_CHIP_IRQ_DISABLE_OFFSET,
            Self::Mask => LINUX_IRQ_CHIP_IRQ_MASK_OFFSET,
            Self::Unmask => LINUX_IRQ_CHIP_IRQ_UNMASK_OFFSET,
            Self::Eoi => LINUX_IRQ_CHIP_IRQ_EOI_OFFSET,
        }
    }

    fn note_success(self) {
        match self {
            Self::Enable => {
                LINUX_PLIC_CHIP_ENABLE_COUNT.fetch_add(1, Ordering::AcqRel);
            }
            Self::Disable => {
                LINUX_PLIC_CHIP_DISABLE_COUNT.fetch_add(1, Ordering::AcqRel);
            }
            Self::Mask => {
                LINUX_PLIC_CHIP_MASK_COUNT.fetch_add(1, Ordering::AcqRel);
            }
            Self::Unmask => {
                LINUX_PLIC_CHIP_UNMASK_COUNT.fetch_add(1, Ordering::AcqRel);
            }
            Self::Eoi => {
                LINUX_PLIC_CHIP_EOI_COUNT.fetch_add(1, Ordering::AcqRel);
            }
        };
    }
}

#[derive(Clone, Copy)]
pub struct LinuxPlicBoundaryFacts {
    pub initcall_seq: usize,
    pub driver_register_seq: usize,
    pub platform_match_seq: usize,
    pub probe_enter_seq: usize,
    pub probe_return_seq: usize,
    pub domain_instantiate_seq: usize,
    pub find_parent_domain_seq: usize,
    pub create_mapping_seq: usize,
    pub set_handler_seq: usize,
    pub cpuhp_seq: usize,
    pub syscore_seq: usize,
    pub driver_registered: bool,
    pub driver_matched: bool,
    pub probe_entered: bool,
    pub probe_return: Option<i32>,
    pub driver_ptr: usize,
    pub probe_ptr: usize,
    pub platform_device_ptr: usize,
    pub fwnode_ptr: usize,
    pub fwnode_ops_ptr: usize,
    pub expected_fwnode_ops_ptr: usize,
    pub membase: usize,
    pub source_count: usize,
    pub context_count: usize,
    pub context_id: usize,
    pub domain_ptr: usize,
    pub domain_ops: usize,
    pub domain_host_data: usize,
    pub parent_irq: usize,
    pub chained_irq: usize,
    pub chained_handler: usize,
    pub chained_is_chained: usize,
    pub thread_info_base: usize,
    pub thread_info_cpu: u32,
    pub per_cpu_offset0: usize,
    pub of_iomap_calls: usize,
    pub of_irq_count_calls: usize,
    pub of_irq_parse_calls: usize,
    pub of_irq_parse_successes: usize,
    pub of_match_calls: usize,
    pub of_property_ndev_calls: usize,
    pub heap_used: usize,
    pub chip_enable_count: usize,
    pub chip_disable_count: usize,
    pub chip_mask_count: usize,
    pub chip_unmask_count: usize,
    pub chip_eoi_count: usize,
    pub chip_callback_probe_successes: usize,
}

const LINUX_PLATFORM_DEVICE_FWNODE_OFFSET: usize = 744;
const LINUX_FWNODE_OPS_OFFSET: usize = 8;
const LINUX_PLIC_HEAP_SIZE: usize = 16 * 1024;
const LINUX_IRQ_DOMAIN_SIZE: usize = 256;
const LINUX_IRQ_DOMAIN_OPS_OFFSET: usize = 24;
const LINUX_IRQ_DOMAIN_HOST_DATA_OFFSET: usize = 32;
const LINUX_IRQ_DOMAIN_INFO_OPS_OFFSET: usize = 48;
const LINUX_IRQ_DOMAIN_INFO_HOST_DATA_OFFSET: usize = 56;
const LINUX_IRQ_DOMAIN_OPS_ALLOC_OFFSET: usize = 40;
const LINUX_IRQ_DESC_SIZE: usize = 512;
const LINUX_IRQ_CHIP_SIZE: usize = 128;
const LINUX_IRQ_FWSPEC_PARAM_COUNT: usize = 16;
const LINUX_PLIC_LEAF_IRQ_CAPACITY: usize = 32;
const LINUX_IRQ_DESC_IRQ_DATA_OFFSET: usize = 48;
const LINUX_IRQ_DESC_IRQ_DATA_IRQ_OFFSET: usize = LINUX_IRQ_DESC_IRQ_DATA_OFFSET + 4;
const LINUX_IRQ_DESC_IRQ_DATA_HWIRQ_OFFSET: usize = LINUX_IRQ_DESC_IRQ_DATA_OFFSET + 8;
const LINUX_IRQ_DESC_IRQ_DATA_COMMON_OFFSET: usize = LINUX_IRQ_DESC_IRQ_DATA_OFFSET + 16;
const LINUX_IRQ_DESC_IRQ_DATA_CHIP_OFFSET: usize = LINUX_IRQ_DESC_IRQ_DATA_OFFSET + 24;
const LINUX_IRQ_DESC_IRQ_DATA_CHIP_DATA_OFFSET: usize = LINUX_IRQ_DESC_IRQ_DATA_OFFSET + 48;
const LINUX_IRQ_COMMON_EFFECTIVE_AFFINITY_OFFSET: usize = 32;
const LINUX_IRQ_CHIP_IRQ_ENABLE_OFFSET: usize = 24;
const LINUX_IRQ_CHIP_IRQ_DISABLE_OFFSET: usize = 32;
const LINUX_IRQ_CHIP_IRQ_MASK_OFFSET: usize = 48;
const LINUX_IRQ_CHIP_IRQ_UNMASK_OFFSET: usize = 64;
const LINUX_IRQ_CHIP_IRQ_EOI_OFFSET: usize = 72;
const LINUX_THREAD_INFO_SIZE: usize = 2048;
const LINUX_THREAD_INFO_CPU_OFFSET: usize = 32;
const LINUX_TASK_STACK_CANARY_OFFSET: usize = 1232;
const LINUX_BOOT_CPU_ID: u32 = 0;
const DEBUG_LINUX_PLIC_SHIM: bool = false;

static mut LINUX_PLIC_FWNODE_VIEW: LinuxOfFwnodeView = LinuxOfFwnodeView {
    bytes_before_ops: [0; LINUX_FWNODE_OPS_OFFSET],
    ops: &raw const of_fwnode_ops as *const usize,
};

static mut LINUX_PLIC_PLATFORM_DEVICE_VIEW: LinuxPlatformDeviceProbeView =
    LinuxPlatformDeviceProbeView {
        bytes_before_fwnode: [0; LINUX_PLATFORM_DEVICE_FWNODE_OFFSET],
        fwnode: &raw mut LINUX_PLIC_FWNODE_VIEW,
    };

static mut LINUX_PLIC_PARENT_INTC_NODE: usize = 0;
static mut LINUX_PLIC_IRQ_DOMAIN: [u8; LINUX_IRQ_DOMAIN_SIZE] = [0; LINUX_IRQ_DOMAIN_SIZE];
static mut LINUX_PLIC_INTC_DOMAIN: [u8; LINUX_IRQ_DOMAIN_SIZE] = [0; LINUX_IRQ_DOMAIN_SIZE];
static mut LINUX_PLIC_PARENT_IRQ_DESC: [u8; LINUX_IRQ_DESC_SIZE] = [0; LINUX_IRQ_DESC_SIZE];
static mut LINUX_PLIC_PARENT_IRQ_CHIP: [u8; LINUX_IRQ_CHIP_SIZE] = [0; LINUX_IRQ_CHIP_SIZE];
static mut LINUX_PLIC_LEAF_IRQ_RECORDS: [LinuxPlicLeafIrqRecord; LINUX_PLIC_LEAF_IRQ_CAPACITY] =
    [const { LinuxPlicLeafIrqRecord::empty() }; LINUX_PLIC_LEAF_IRQ_CAPACITY];
static mut LINUX_PLIC_LEAF_IRQ_DESCS: [LinuxIrqDescView; LINUX_PLIC_LEAF_IRQ_CAPACITY] = [const {
    LinuxIrqDescView {
        bytes: [0; LINUX_IRQ_DESC_SIZE],
    }
};
    LINUX_PLIC_LEAF_IRQ_CAPACITY];
static mut LINUX_PLIC_THREAD_INFO: LinuxThreadInfoView = LinuxThreadInfoView {
    bytes: [0; LINUX_THREAD_INFO_SIZE],
};
static mut LINUX_PLIC_HEAP: [u8; LINUX_PLIC_HEAP_SIZE] = [0; LINUX_PLIC_HEAP_SIZE];
static LINUX_PLIC_HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

fn trap(symbol: &str) -> ! {
    crate::arch::riscv64::sbi::putstr("linux plic shim trap: ");
    crate::arch::riscv64::sbi::putstr(symbol);
    crate::arch::riscv64::sbi::putstr("\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

fn debug(msg: &str) {
    if DEBUG_LINUX_PLIC_SHIM {
        crate::arch::riscv64::sbi::putstr(msg);
    }
}

fn next_event_seq() -> usize {
    LINUX_PLIC_EVENT_SEQ.fetch_add(1, Ordering::AcqRel) + 1
}

pub fn run_linux_initcall6() -> EventResult {
    let start = &raw const __linux_initcall6_start as *const usize as usize;
    let end = &raw const __linux_initcall6_end as *const usize as usize;
    let entry_size = size_of::<LinuxInitcall>();
    if start == 0 || end <= start || entry_size == 0 || (end - start) % entry_size != 0 {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    let count = (end - start) / entry_size;
    let entries = unsafe { core::slice::from_raw_parts(start as *const LinuxInitcall, count) };
    LINUX_PLIC_INITCALL_SEQ.store(next_event_seq(), Ordering::Release);
    let mut index = 0usize;
    while index < entries.len() {
        let ret = unsafe { entries[index]() };
        if ret != 0 {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Base,
                State::Ready,
                State::Ready,
            );
        }
        index += 1;
    }

    Ok(())
}

pub fn platform_driver_registered() -> bool {
    PLATFORM_DRIVER_REGISTERED.load(Ordering::Acquire)
}

pub fn platform_driver_probe_ptr() -> usize {
    let driver = PLATFORM_DRIVER_PTR.load(Ordering::Acquire) as *const LinuxPlatformDriver;
    if driver.is_null() {
        return 0;
    }

    unsafe { (*driver).probe }
}

pub fn runtime_claim_count() -> usize {
    LINUX_PLIC_RUNTIME_CLAIM_COUNT.load(Ordering::Acquire)
}

pub fn runtime_zero_claim_count() -> usize {
    LINUX_PLIC_RUNTIME_ZERO_CLAIM_COUNT.load(Ordering::Acquire)
}

pub fn runtime_dispatch_count() -> usize {
    LINUX_PLIC_RUNTIME_DISPATCH_COUNT.load(Ordering::Acquire)
}

pub fn runtime_complete_count() -> usize {
    LINUX_PLIC_RUNTIME_COMPLETE_COUNT.load(Ordering::Acquire)
}

pub fn runtime_loop_exit_count() -> usize {
    LINUX_PLIC_RUNTIME_LOOP_EXIT_COUNT.load(Ordering::Acquire)
}

pub fn runtime_last_claimed_source() -> u32 {
    LINUX_PLIC_RUNTIME_LAST_CLAIMED_SOURCE.load(Ordering::Acquire)
}

pub fn runtime_last_completed_source() -> u32 {
    LINUX_PLIC_RUNTIME_LAST_COMPLETED_SOURCE.load(Ordering::Acquire)
}

pub fn boundary_facts() -> LinuxPlicBoundaryFacts {
    let probe_ret = PLATFORM_DRIVER_PROBE_RET.load(Ordering::Acquire);
    let thread_info_base = linux_thread_info_base() as usize;
    let thread_info_cpu = unsafe {
        core::ptr::read(
            linux_thread_info_base()
                .add(LINUX_THREAD_INFO_CPU_OFFSET)
                .cast::<u32>(),
        )
    };
    LinuxPlicBoundaryFacts {
        initcall_seq: LINUX_PLIC_INITCALL_SEQ.load(Ordering::Acquire),
        driver_register_seq: LINUX_PLIC_DRIVER_REGISTER_SEQ.load(Ordering::Acquire),
        platform_match_seq: LINUX_PLIC_PLATFORM_MATCH_SEQ.load(Ordering::Acquire),
        probe_enter_seq: LINUX_PLIC_PROBE_ENTER_SEQ.load(Ordering::Acquire),
        probe_return_seq: LINUX_PLIC_PROBE_RETURN_SEQ.load(Ordering::Acquire),
        domain_instantiate_seq: LINUX_PLIC_DOMAIN_INSTANTIATE_SEQ.load(Ordering::Acquire),
        find_parent_domain_seq: LINUX_PLIC_FIND_PARENT_DOMAIN_SEQ.load(Ordering::Acquire),
        create_mapping_seq: LINUX_PLIC_CREATE_MAPPING_SEQ.load(Ordering::Acquire),
        set_handler_seq: LINUX_PLIC_SET_HANDLER_SEQ.load(Ordering::Acquire),
        cpuhp_seq: LINUX_PLIC_CPUHP_SEQ.load(Ordering::Acquire),
        syscore_seq: LINUX_PLIC_SYSCORE_SEQ.load(Ordering::Acquire),
        driver_registered: PLATFORM_DRIVER_REGISTERED.load(Ordering::Acquire),
        driver_matched: PLATFORM_DRIVER_MATCHED.load(Ordering::Acquire),
        probe_entered: PLATFORM_DRIVER_PROBE_ENTERED.load(Ordering::Acquire),
        probe_return: if probe_ret == usize::MAX {
            None
        } else {
            Some(probe_ret as i32)
        },
        driver_ptr: PLATFORM_DRIVER_PTR.load(Ordering::Acquire),
        probe_ptr: platform_driver_probe_ptr(),
        platform_device_ptr: &raw mut LINUX_PLIC_PLATFORM_DEVICE_VIEW as *mut c_void as usize,
        fwnode_ptr: &raw mut LINUX_PLIC_FWNODE_VIEW as *mut LinuxOfFwnodeView as usize,
        fwnode_ops_ptr: unsafe {
            core::ptr::read(
                (&raw const LINUX_PLIC_FWNODE_VIEW)
                    .cast::<u8>()
                    .add(LINUX_FWNODE_OPS_OFFSET)
                    .cast::<*const usize>(),
            ) as usize
        },
        expected_fwnode_ops_ptr: &raw const of_fwnode_ops as *const usize as usize,
        membase: LINUX_PLIC_MEMBASE.load(Ordering::Acquire),
        source_count: LINUX_PLIC_SOURCE_COUNT.load(Ordering::Acquire),
        context_count: LINUX_PLIC_CONTEXT_COUNT.load(Ordering::Acquire),
        context_id: LINUX_PLIC_CONTEXT_ID.load(Ordering::Acquire),
        domain_ptr: LINUX_PLIC_DOMAIN_PTR.load(Ordering::Acquire),
        domain_ops: LINUX_PLIC_DOMAIN_OPS.load(Ordering::Acquire),
        domain_host_data: LINUX_PLIC_DOMAIN_HOST_DATA.load(Ordering::Acquire),
        parent_irq: LINUX_PLIC_PARENT_IRQ.load(Ordering::Acquire),
        chained_irq: LINUX_PLIC_CHAINED_IRQ.load(Ordering::Acquire),
        chained_handler: LINUX_PLIC_CHAINED_HANDLER.load(Ordering::Acquire),
        chained_is_chained: LINUX_PLIC_CHAINED_IS_CHAINED.load(Ordering::Acquire),
        thread_info_base,
        thread_info_cpu,
        per_cpu_offset0: unsafe { core::ptr::read((&raw const __per_cpu_offset).cast::<usize>()) },
        of_iomap_calls: LINUX_PLIC_OF_IOMAP_CALLS.load(Ordering::Acquire),
        of_irq_count_calls: LINUX_PLIC_OF_IRQ_COUNT_CALLS.load(Ordering::Acquire),
        of_irq_parse_calls: LINUX_PLIC_OF_IRQ_PARSE_CALLS.load(Ordering::Acquire),
        of_irq_parse_successes: LINUX_PLIC_OF_IRQ_PARSE_SUCCESSES.load(Ordering::Acquire),
        of_match_calls: LINUX_PLIC_OF_MATCH_CALLS.load(Ordering::Acquire),
        of_property_ndev_calls: LINUX_PLIC_OF_PROPERTY_NDEV_CALLS.load(Ordering::Acquire),
        heap_used: LINUX_PLIC_HEAP_OFFSET.load(Ordering::Acquire),
        chip_enable_count: LINUX_PLIC_CHIP_ENABLE_COUNT.load(Ordering::Acquire),
        chip_disable_count: LINUX_PLIC_CHIP_DISABLE_COUNT.load(Ordering::Acquire),
        chip_mask_count: LINUX_PLIC_CHIP_MASK_COUNT.load(Ordering::Acquire),
        chip_unmask_count: LINUX_PLIC_CHIP_UNMASK_COUNT.load(Ordering::Acquire),
        chip_eoi_count: LINUX_PLIC_CHIP_EOI_COUNT.load(Ordering::Acquire),
        chip_callback_probe_successes: LINUX_PLIC_CHIP_CALLBACK_PROBE_SUCCESSES
            .load(Ordering::Acquire),
    }
}

pub fn platform_driver_match_and_probe(device_tree: &DeviceTree, plic: &Plic) -> EventResult {
    let driver = PLATFORM_DRIVER_PTR.load(Ordering::Acquire) as *const LinuxPlatformDriver;
    let Some(source_count) = plic_node_source_count(device_tree) else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    let Some(context_count) = plic_node_context_count(device_tree) else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    };
    if driver.is_null()
        || !plic_node_available(device_tree)
        || plic.membase() == 0
        || plic.source_count() == 0
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }
    if !driver_matches_qemu_plic(driver) {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    let probe = unsafe { (*driver).probe };
    if probe == 0 {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    LINUX_PLIC_MEMBASE.store(plic.membase(), Ordering::Release);
    LINUX_PLIC_SOURCE_COUNT.store(source_count as usize, Ordering::Release);
    LINUX_PLIC_CONTEXT_COUNT.store(context_count, Ordering::Release);
    LINUX_PLIC_CONTEXT_ID.store(plic.context_id(), Ordering::Release);
    PLATFORM_DRIVER_MATCHED.store(true, Ordering::Release);
    PLATFORM_DRIVER_PROBE_ENTERED.store(true, Ordering::Release);
    LINUX_PLIC_PLATFORM_MATCH_SEQ.store(next_event_seq(), Ordering::Release);
    LINUX_PLIC_PROBE_ENTER_SEQ.store(next_event_seq(), Ordering::Release);
    debug("linux plic shim: enter probe\n");
    let ret = unsafe {
        let probe: LinuxPlatformProbe = core::mem::transmute(probe);
        call_linux_platform_probe(
            probe,
            &raw mut LINUX_PLIC_PLATFORM_DEVICE_VIEW as *mut c_void,
        )
    };
    debug("linux plic shim: leave probe\n");
    PLATFORM_DRIVER_PROBE_RET.store(ret as usize, Ordering::Release);
    LINUX_PLIC_PROBE_RETURN_SEQ.store(next_event_seq(), Ordering::Release);
    if ret != 0 {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    Ok(())
}

pub fn handle_external_interrupt() {
    let handler = LINUX_PLIC_CHAINED_HANDLER.load(Ordering::Acquire);
    if handler == 0 {
        return;
    }

    prepare_linux_parent_irq_desc();
    let desc = &raw mut LINUX_PLIC_PARENT_IRQ_DESC as *mut u8 as *mut c_void;
    let handler: unsafe extern "C" fn(*mut c_void) = unsafe { core::mem::transmute(handler) };
    unsafe { call_linux_chained_irq_handler(handler, desc) };
}

fn plic_node_available(device_tree: &DeviceTree) -> bool {
    device_tree
        .find_node(b"/soc/plic@c000000")
        .is_some_and(|node| {
            node.has_compatible(PLIC_COMPATIBLE_SIFIVE)
                || node.has_compatible(PLIC_COMPATIBLE_RISCV)
        })
}

fn plic_node_source_count(device_tree: &DeviceTree) -> Option<u32> {
    let node = device_tree.find_node(b"/soc/plic@c000000")?;
    let property = node.property(b"riscv,ndev")?;
    let value = property.raw_value();
    if value.len() < 4 {
        return None;
    }
    Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn plic_node_context_count(device_tree: &DeviceTree) -> Option<usize> {
    let node = device_tree.find_node(b"/soc/plic@c000000")?;
    let property = node.property(b"interrupts-extended")?;
    let value = property.raw_value();
    if value.len() < 8 || value.len() % 8 != 0 {
        return None;
    }
    Some(value.len() / 8)
}

fn driver_matches_qemu_plic(driver: *const LinuxPlatformDriver) -> bool {
    let mut entry = unsafe { (*driver).driver.of_match_table };
    if entry.is_null() {
        return false;
    }

    let mut scanned = 0usize;
    while scanned < 16 {
        let compatible = unsafe { cstr_bytes(&(*entry).compatible) };
        if compatible.is_empty() {
            return false;
        }
        if compatible == PLIC_COMPATIBLE_SIFIVE || compatible == PLIC_COMPATIBLE_RISCV {
            return true;
        }
        entry = unsafe { entry.add(1) };
        scanned += 1;
    }

    false
}

fn cstr_bytes(bytes: &[u8]) -> &[u8] {
    let mut len = 0usize;
    while len < bytes.len() && bytes[len] != 0 {
        len += 1;
    }
    &bytes[..len]
}

unsafe fn cstr_ptr_eq(ptr: *const u8, expected: &[u8]) -> bool {
    let mut index = 0usize;
    while index < expected.len() {
        if unsafe { *ptr.add(index) } != expected[index] {
            return false;
        }
        if expected[index] == 0 {
            return true;
        }
        index += 1;
    }
    false
}

fn linux_plic_alloc(size: usize, align: usize) -> *mut c_void {
    debug("linux plic shim: alloc enter\n");
    if size == 0 || align == 0 || !align.is_power_of_two() {
        return core::ptr::null_mut();
    }

    let mut current = LINUX_PLIC_HEAP_OFFSET.load(Ordering::Acquire);
    loop {
        let aligned = (current + align - 1) & !(align - 1);
        let Some(next) = aligned.checked_add(size) else {
            return core::ptr::null_mut();
        };
        if next > LINUX_PLIC_HEAP_SIZE {
            return core::ptr::null_mut();
        }
        match LINUX_PLIC_HEAP_OFFSET.compare_exchange(
            current,
            next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                let ptr = unsafe { (&raw mut LINUX_PLIC_HEAP).cast::<u8>().add(aligned) };
                unsafe { core::ptr::write_bytes(ptr, 0, size) };
                debug("linux plic shim: alloc leave\n");
                return ptr.cast::<c_void>();
            }
            Err(observed) => current = observed,
        }
    }
}

unsafe fn call_linux_platform_probe(probe: LinuxPlatformProbe, pdev: *mut c_void) -> i32 {
    prepare_linux_thread_info();
    let saved_sstatus = crate::arch::riscv64::csr::save_and_disable_supervisor_interrupts();
    let saved_tp = current_rust_tp_for_linux_call();
    remember_rust_tp(saved_tp);
    crate::arch::riscv64::csr::write_tp(linux_thread_info_base() as usize);
    let ret = unsafe { probe(pdev) };
    crate::arch::riscv64::csr::write_tp(saved_tp);
    crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
    ret
}

unsafe fn call_linux_chained_irq_handler(
    handler: unsafe extern "C" fn(*mut c_void),
    desc: *mut c_void,
) {
    prepare_linux_thread_info();
    let saved_sstatus = crate::arch::riscv64::csr::save_and_disable_supervisor_interrupts();
    let saved_tp = current_rust_tp_for_linux_call();
    remember_rust_tp(saved_tp);
    crate::arch::riscv64::csr::write_tp(linux_thread_info_base() as usize);
    unsafe { handler(desc) };
    crate::arch::riscv64::csr::write_tp(saved_tp);
    crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
    LINUX_PLIC_RUNTIME_ZERO_CLAIM_COUNT.fetch_add(1, Ordering::AcqRel);
    LINUX_PLIC_RUNTIME_LOOP_EXIT_COUNT.fetch_add(1, Ordering::AcqRel);
}

fn prepare_linux_thread_info() {
    unsafe {
        let base = linux_thread_info_base();
        core::ptr::write_bytes(base, 0, LINUX_THREAD_INFO_SIZE);
        core::ptr::write(
            base.add(LINUX_THREAD_INFO_CPU_OFFSET).cast::<u32>(),
            LINUX_BOOT_CPU_ID,
        );
        core::ptr::write(base.add(LINUX_TASK_STACK_CANARY_OFFSET).cast::<usize>(), 0);
    }
}

fn prepare_linux_parent_irq_desc() {
    unsafe {
        let desc = (&raw mut LINUX_PLIC_PARENT_IRQ_DESC).cast::<u8>();
        let chip = (&raw mut LINUX_PLIC_PARENT_IRQ_CHIP).cast::<u8>();
        prepare_linux_irq_desc(
            desc,
            LINUX_RV_IRQ_EXT,
            LINUX_RV_IRQ_EXT as usize,
            chip as usize,
            0,
        );
        core::ptr::write_bytes(chip, 0, LINUX_IRQ_CHIP_SIZE);
        core::ptr::write(
            chip.add(LINUX_IRQ_CHIP_IRQ_EOI_OFFSET).cast::<usize>(),
            linux_plic_parent_irq_eoi as usize,
        );
    }
}

unsafe fn prepare_linux_irq_desc(
    desc: *mut u8,
    virq: u32,
    hwirq: usize,
    chip: usize,
    chip_data: usize,
) {
    unsafe {
        core::ptr::write_bytes(desc, 0, LINUX_IRQ_DESC_SIZE);
        core::ptr::write(
            desc.add(LINUX_IRQ_DESC_IRQ_DATA_IRQ_OFFSET).cast::<u32>(),
            virq,
        );
        core::ptr::write(
            desc.add(LINUX_IRQ_DESC_IRQ_DATA_HWIRQ_OFFSET)
                .cast::<usize>(),
            hwirq,
        );
        core::ptr::write(
            desc.add(LINUX_IRQ_DESC_IRQ_DATA_COMMON_OFFSET)
                .cast::<usize>(),
            desc as usize,
        );
        core::ptr::write(
            desc.add(LINUX_IRQ_DESC_IRQ_DATA_CHIP_OFFSET)
                .cast::<usize>(),
            chip,
        );
        core::ptr::write(
            desc.add(LINUX_IRQ_DESC_IRQ_DATA_CHIP_DATA_OFFSET)
                .cast::<usize>(),
            chip_data,
        );
        core::ptr::write(
            desc.add(LINUX_IRQ_COMMON_EFFECTIVE_AFFINITY_OFFSET)
                .cast::<usize>(),
            1,
        );
    }
}

fn linux_irq_data_from_desc(desc: *mut u8) -> *mut c_void {
    unsafe { desc.add(LINUX_IRQ_DESC_IRQ_DATA_OFFSET).cast::<c_void>() }
}

fn linux_leaf_record_index_by_hwirq(hwirq: u32) -> Option<usize> {
    let records = (&raw const LINUX_PLIC_LEAF_IRQ_RECORDS).cast::<LinuxPlicLeafIrqRecord>();
    let mut index = 0usize;
    while index < LINUX_PLIC_LEAF_IRQ_CAPACITY {
        let record = unsafe { core::ptr::read(records.add(index)) };
        if record.in_use && record.hwirq == hwirq {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn linux_leaf_record_index_by_virq(virq: u32) -> Option<usize> {
    let records = (&raw const LINUX_PLIC_LEAF_IRQ_RECORDS).cast::<LinuxPlicLeafIrqRecord>();
    let mut index = 0usize;
    while index < LINUX_PLIC_LEAF_IRQ_CAPACITY {
        let record = unsafe { core::ptr::read(records.add(index)) };
        if record.in_use && record.virq == virq {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn linux_leaf_record_index_by_desc(desc: *const c_void) -> Option<usize> {
    let descs = (&raw const LINUX_PLIC_LEAF_IRQ_DESCS).cast::<LinuxIrqDescView>();
    let mut index = 0usize;
    while index < LINUX_PLIC_LEAF_IRQ_CAPACITY {
        let expected = unsafe { descs.add(index).cast::<c_void>() };
        if expected == desc {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn linux_leaf_desc_ptr(index: usize) -> *mut u8 {
    unsafe {
        (&raw mut LINUX_PLIC_LEAF_IRQ_DESCS)
            .cast::<LinuxIrqDescView>()
            .add(index)
            .cast::<u8>()
    }
}

fn linux_leaf_irq_data_ptr(index: usize) -> *mut c_void {
    linux_irq_data_from_desc(linux_leaf_desc_ptr(index))
}

fn linux_leaf_record(index: usize) -> LinuxPlicLeafIrqRecord {
    let records = (&raw const LINUX_PLIC_LEAF_IRQ_RECORDS).cast::<LinuxPlicLeafIrqRecord>();
    unsafe { core::ptr::read(records.add(index)) }
}

fn linux_store_leaf_irq_record(
    virq: u32,
    hwirq: u32,
    chip: usize,
    chip_data: usize,
    flow_handler: usize,
) -> bool {
    let index = linux_leaf_record_index_by_hwirq(hwirq)
        .or_else(|| linux_leaf_record_index_by_virq(virq))
        .or_else(|| {
            let records = (&raw const LINUX_PLIC_LEAF_IRQ_RECORDS).cast::<LinuxPlicLeafIrqRecord>();
            let mut index = 0usize;
            while index < LINUX_PLIC_LEAF_IRQ_CAPACITY {
                let record = unsafe { core::ptr::read(records.add(index)) };
                if !record.in_use {
                    return Some(index);
                }
                index += 1;
            }
            None
        });
    let Some(index) = index else {
        return false;
    };

    unsafe {
        let desc = linux_leaf_desc_ptr(index);
        prepare_linux_irq_desc(desc, virq, hwirq as usize, chip, chip_data);
        let records = (&raw mut LINUX_PLIC_LEAF_IRQ_RECORDS).cast::<LinuxPlicLeafIrqRecord>();
        core::ptr::write(
            records.add(index),
            LinuxPlicLeafIrqRecord {
                in_use: true,
                virq,
                hwirq,
                chip,
                flow_handler,
            },
        );
    }
    true
}

unsafe fn call_linux_domain_alloc(domain: *mut c_void, virq: u32, hwirq: u32) -> i32 {
    let ops = LINUX_PLIC_DOMAIN_OPS.load(Ordering::Acquire);
    if domain.is_null() || ops == 0 {
        return -22;
    }

    let alloc = unsafe {
        core::ptr::read(
            (ops as *const u8)
                .add(LINUX_IRQ_DOMAIN_OPS_ALLOC_OFFSET)
                .cast::<usize>(),
        )
    };
    if alloc == 0 {
        return -22;
    }

    let mut fwspec = LinuxIrqFwspec {
        fwnode: core::ptr::null_mut(),
        param_count: 1,
        param: [0; LINUX_IRQ_FWSPEC_PARAM_COUNT],
    };
    fwspec.param[0] = hwirq;
    let alloc: LinuxIrqDomainAlloc = unsafe { core::mem::transmute(alloc) };
    unsafe { alloc(domain, virq, 1, (&raw mut fwspec).cast::<c_void>()) }
}

unsafe fn call_linux_chip_callback(
    record: LinuxPlicLeafIrqRecord,
    irq_data: *mut c_void,
    slot: LinuxIrqChipCallbackSlot,
) -> bool {
    if record.chip == 0 || irq_data.is_null() {
        return false;
    }
    let callback = unsafe {
        core::ptr::read(
            (record.chip as *const u8)
                .add(slot.offset())
                .cast::<usize>(),
        )
    };
    if callback == 0 {
        return false;
    }
    let callback: LinuxIrqChipCallback = unsafe { core::mem::transmute(callback) };
    unsafe { callback(irq_data) };
    slot.note_success();
    true
}

fn ensure_linux_leaf_irq(domain: *mut c_void, hwirq: u32, virq: u32) -> Option<usize> {
    if let Some(index) = linux_leaf_record_index_by_hwirq(hwirq) {
        return Some(index);
    }
    let ret = unsafe { call_linux_domain_alloc(domain, virq, hwirq) };
    if ret != 0 {
        return None;
    }
    linux_leaf_record_index_by_hwirq(hwirq)
}

pub fn enable_mapped_source(source: u32, logical_irq: LogicalIrq) -> bool {
    if source == 0 || !logical_irq.is_valid() {
        return false;
    }
    let domain = LINUX_PLIC_DOMAIN_PTR.load(Ordering::Acquire) as *mut c_void;
    let Ok(virq) = u32::try_from(logical_irq.as_usize()) else {
        return false;
    };

    prepare_linux_thread_info();
    let saved_sstatus = crate::arch::riscv64::csr::save_and_disable_supervisor_interrupts();
    let saved_tp = current_rust_tp_for_linux_call();
    remember_rust_tp(saved_tp);
    crate::arch::riscv64::csr::write_tp(linux_thread_info_base() as usize);

    let enabled = ensure_linux_leaf_irq(domain, source, virq).is_some_and(|index| {
        let record = linux_leaf_record(index);
        let irq_data = linux_leaf_irq_data_ptr(index);
        unsafe { call_linux_chip_callback(record, irq_data, LinuxIrqChipCallbackSlot::Enable) }
    });

    crate::arch::riscv64::csr::write_tp(saved_tp);
    crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
    enabled
}

pub fn probe_uart_leaf_chip_callbacks(source: u32, logical_irq: LogicalIrq) -> bool {
    if source == 0 || !logical_irq.is_valid() {
        return false;
    }
    let domain = LINUX_PLIC_DOMAIN_PTR.load(Ordering::Acquire) as *mut c_void;
    let Ok(virq) = u32::try_from(logical_irq.as_usize()) else {
        return false;
    };

    prepare_linux_thread_info();
    let saved_sstatus = crate::arch::riscv64::csr::save_and_disable_supervisor_interrupts();
    let saved_tp = current_rust_tp_for_linux_call();
    remember_rust_tp(saved_tp);
    crate::arch::riscv64::csr::write_tp(linux_thread_info_base() as usize);

    let callbacks_ok = ensure_linux_leaf_irq(domain, source, virq).is_some_and(|index| {
        let record = linux_leaf_record(index);
        let irq_data = linux_leaf_irq_data_ptr(index);
        let disable_ok = unsafe {
            call_linux_chip_callback(record, irq_data, LinuxIrqChipCallbackSlot::Disable)
        };
        let enable_ok =
            unsafe { call_linux_chip_callback(record, irq_data, LinuxIrqChipCallbackSlot::Enable) };
        let mask_ok =
            unsafe { call_linux_chip_callback(record, irq_data, LinuxIrqChipCallbackSlot::Mask) };
        let unmask_ok =
            unsafe { call_linux_chip_callback(record, irq_data, LinuxIrqChipCallbackSlot::Unmask) };
        disable_ok && enable_ok && mask_ok && unmask_ok
    });

    crate::arch::riscv64::csr::write_tp(saved_tp);
    crate::arch::riscv64::csr::restore_supervisor_interrupts(saved_sstatus);
    if callbacks_ok {
        LINUX_PLIC_CHIP_CALLBACK_PROBE_SUCCESSES.fetch_add(1, Ordering::AcqRel);
    }
    callbacks_ok
}

fn linux_thread_info_base() -> *mut u8 {
    (&raw mut LINUX_PLIC_THREAD_INFO).cast::<u8>()
}

fn linux_thread_info_addr() -> usize {
    linux_thread_info_base() as usize
}

fn current_rust_tp_for_linux_call() -> usize {
    let current = crate::arch::riscv64::csr::read_tp();
    if current != linux_thread_info_addr() {
        return current;
    }

    LINUX_PLIC_SAVED_RUST_TP.load(Ordering::Acquire)
}

fn remember_rust_tp(tp: usize) {
    if tp != 0 && tp != linux_thread_info_addr() {
        LINUX_PLIC_SAVED_RUST_TP.store(tp, Ordering::Release);
    }
}

extern "C" fn linux_plic_parent_irq_eoi(_data: *mut c_void) {}

#[unsafe(no_mangle)]
pub static mut __cpu_online_mask: [usize; 1] = [1];

#[unsafe(no_mangle)]
pub static mut __cpu_present_mask: [usize; 1] = [1];

#[unsafe(no_mangle)]
pub static mut __mmiowb_state: usize = 0;

#[unsafe(no_mangle)]
pub static mut __per_cpu_offset: [usize; 8] = [0; 8];

#[unsafe(no_mangle)]
pub static cpu_bit_bitmap: [usize; 65] = [
    0,
    1usize << 0,
    1usize << 1,
    1usize << 2,
    1usize << 3,
    1usize << 4,
    1usize << 5,
    1usize << 6,
    1usize << 7,
    1usize << 8,
    1usize << 9,
    1usize << 10,
    1usize << 11,
    1usize << 12,
    1usize << 13,
    1usize << 14,
    1usize << 15,
    1usize << 16,
    1usize << 17,
    1usize << 18,
    1usize << 19,
    1usize << 20,
    1usize << 21,
    1usize << 22,
    1usize << 23,
    1usize << 24,
    1usize << 25,
    1usize << 26,
    1usize << 27,
    1usize << 28,
    1usize << 29,
    1usize << 30,
    1usize << 31,
    1usize << 32,
    1usize << 33,
    1usize << 34,
    1usize << 35,
    1usize << 36,
    1usize << 37,
    1usize << 38,
    1usize << 39,
    1usize << 40,
    1usize << 41,
    1usize << 42,
    1usize << 43,
    1usize << 44,
    1usize << 45,
    1usize << 46,
    1usize << 47,
    1usize << 48,
    1usize << 49,
    1usize << 50,
    1usize << 51,
    1usize << 52,
    1usize << 53,
    1usize << 54,
    1usize << 55,
    1usize << 56,
    1usize << 57,
    1usize << 58,
    1usize << 59,
    1usize << 60,
    1usize << 61,
    1usize << 62,
    1usize << 63,
];

#[unsafe(no_mangle)]
pub static mut kmalloc_caches: [usize; 16] = [0; 16];

#[unsafe(no_mangle)]
pub static nr_cpu_ids: u32 = 1;

#[unsafe(no_mangle)]
pub static of_fwnode_ops: [usize; 8] = [0; 8];

#[unsafe(no_mangle)]
pub extern "C" fn ___ratelimit() -> i32 {
    trap("___ratelimit")
}

#[unsafe(no_mangle)]
pub extern "C" fn __cpuhp_setup_state(
    _state: i32,
    _name: *const u8,
    invoke: bool,
    startup: *mut c_void,
    _teardown: *mut c_void,
    _multi_instance: bool,
) -> i32 {
    debug("linux plic shim: cpuhp\n");
    LINUX_PLIC_CPUHP_SEQ.store(next_event_seq(), Ordering::Release);
    if invoke && !startup.is_null() {
        let startup: LinuxCpuHotplugStartup = unsafe { core::mem::transmute(startup) };
        return unsafe { startup(LINUX_BOOT_CPU_ID) };
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn __irq_set_handler(
    irq: u32,
    handler: *mut c_void,
    is_chained: i32,
    _name: *const u8,
) {
    debug("linux plic shim: set handler\n");
    LINUX_PLIC_SET_HANDLER_SEQ.store(next_event_seq(), Ordering::Release);
    LINUX_PLIC_CHAINED_IRQ.store(irq as usize, Ordering::Release);
    LINUX_PLIC_CHAINED_HANDLER.store(handler as usize, Ordering::Release);
    LINUX_PLIC_CHAINED_IS_CHAINED.store(is_chained as usize, Ordering::Release);
}

#[unsafe(no_mangle)]
pub extern "C" fn __kmalloc_cache_noprof(
    _cache: *mut c_void,
    _flags: usize,
    size: usize,
) -> *mut c_void {
    debug("linux plic shim: kmalloc_cache\n");
    linux_plic_alloc(size, size_of::<usize>())
}

#[unsafe(no_mangle)]
pub extern "C" fn __kmalloc_noprof(size: usize, _flags: usize) -> *mut c_void {
    debug("linux plic shim: kmalloc\n");
    linux_plic_alloc(size, size_of::<usize>())
}

#[unsafe(no_mangle)]
pub extern "C" fn __platform_driver_register(driver: *mut c_void, _owner: *mut c_void) -> i32 {
    if driver.is_null() {
        return -1;
    }

    PLATFORM_DRIVER_PTR.store(driver as usize, Ordering::Release);
    PLATFORM_DRIVER_REGISTERED.store(true, Ordering::Release);
    LINUX_PLIC_DRIVER_REGISTER_SEQ.store(next_event_seq(), Ordering::Release);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn __raw_spin_lock_init(lock: *mut c_void) {
    if !lock.is_null() {
        unsafe { core::ptr::write_bytes(lock, 0, 4) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn __stack_chk_fail() -> ! {
    trap("__stack_chk_fail")
}

#[unsafe(no_mangle)]
pub extern "C" fn _printk(_fmt: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn _raw_spin_lock_irqsave(_lock: *mut c_void) -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn _raw_spin_unlock_irqrestore(_lock: *mut c_void, _flags: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn bitmap_free() {
    trap("bitmap_free")
}

#[unsafe(no_mangle)]
pub extern "C" fn bitmap_zalloc(nbits: usize, _flags: usize) -> *mut usize {
    debug("linux plic shim: bitmap_zalloc\n");
    let words = nbits.div_ceil(usize::BITS as usize);
    linux_plic_alloc(words * size_of::<usize>(), size_of::<usize>()).cast::<usize>()
}

#[unsafe(no_mangle)]
pub extern "C" fn devm_platform_ioremap_resource() -> *mut c_void {
    trap("devm_platform_ioremap_resource")
}

#[unsafe(no_mangle)]
pub extern "C" fn disable_percpu_irq(_irq: u32) {
    debug("linux plic shim: disable percpu irq\n");
}

#[unsafe(no_mangle)]
pub extern "C" fn enable_percpu_irq(_irq: u32, _irq_type: u32) {
    debug("linux plic shim: enable percpu irq\n");
}

#[unsafe(no_mangle)]
pub extern "C" fn generic_handle_domain_irq(domain: *mut c_void, hwirq: usize) -> i32 {
    let expected_domain = LINUX_PLIC_DOMAIN_PTR.load(Ordering::Acquire) as *mut c_void;
    if domain.is_null() || domain != expected_domain || hwirq == 0 {
        return -22;
    }

    let Ok(source) = u32::try_from(hwirq) else {
        return -22;
    };
    LINUX_PLIC_RUNTIME_CLAIM_COUNT.fetch_add(1, Ordering::AcqRel);
    LINUX_PLIC_RUNTIME_LAST_CLAIMED_SOURCE.store(source, Ordering::Release);

    let linux_tp = crate::arch::riscv64::csr::read_tp();
    let rust_tp = LINUX_PLIC_SAVED_RUST_TP.load(Ordering::Acquire);
    if rust_tp != 0 {
        crate::arch::riscv64::csr::write_tp(rust_tp);
    }
    let ctx = crate::context::context_ref();
    let Some(logical_irq) = ctx.plic_irq_domain.resolve_hwirq(source) else {
        crate::arch::riscv64::csr::disable_supervisor_interrupts();
        crate::arch::riscv64::csr::write_tp(linux_tp);
        return -22;
    };
    let Ok(virq) = u32::try_from(logical_irq.as_usize()) else {
        crate::arch::riscv64::csr::disable_supervisor_interrupts();
        crate::arch::riscv64::csr::write_tp(linux_tp);
        return -22;
    };
    crate::arch::riscv64::csr::disable_supervisor_interrupts();
    crate::arch::riscv64::csr::write_tp(linux_tp);

    let Some(index) = ensure_linux_leaf_irq(domain, source, virq) else {
        return -22;
    };
    let record = linux_leaf_record(index);
    if record.flow_handler == 0 {
        return -22;
    }
    let handler: LinuxIrqFlowHandler = unsafe { core::mem::transmute(record.flow_handler) };
    let desc = linux_leaf_desc_ptr(index).cast::<c_void>();
    unsafe { handler(desc) };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_edge_irq(desc: *mut c_void) {
    handle_fasteoi_irq(desc)
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_fasteoi_irq(desc: *mut c_void) {
    let Some(index) = linux_leaf_record_index_by_desc(desc) else {
        return;
    };
    let record = linux_leaf_record(index);
    let irq_data = linux_leaf_irq_data_ptr(index);

    let linux_tp = crate::arch::riscv64::csr::read_tp();
    let rust_tp = LINUX_PLIC_SAVED_RUST_TP.load(Ordering::Acquire);
    let mut dispatched = false;
    if rust_tp != 0 {
        crate::arch::riscv64::csr::write_tp(rust_tp);
        let ctx = crate::context::context_ref();
        dispatched = ctx
            .irq_handler_registry
            .dispatch(LogicalIrq::new(record.virq as usize));
        crate::arch::riscv64::csr::disable_supervisor_interrupts();
        crate::arch::riscv64::csr::write_tp(linux_tp);
    }

    if unsafe { call_linux_chip_callback(record, irq_data, LinuxIrqChipCallbackSlot::Eoi) } {
        LINUX_PLIC_RUNTIME_COMPLETE_COUNT.fetch_add(1, Ordering::AcqRel);
        LINUX_PLIC_RUNTIME_LAST_COMPLETED_SOURCE.store(record.hwirq, Ordering::Release);
    }
    if dispatched {
        LINUX_PLIC_RUNTIME_DISPATCH_COUNT.fetch_add(1, Ordering::AcqRel);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn iounmap() {
    trap("iounmap")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_create_mapping_affinity(
    _domain: *mut c_void,
    hwirq: usize,
    _affinity: *const c_void,
) -> u32 {
    debug("linux plic shim: create mapping\n");
    LINUX_PLIC_CREATE_MAPPING_SEQ.store(next_event_seq(), Ordering::Release);
    let irq = if hwirq == LINUX_RV_IRQ_EXT as usize {
        9
    } else {
        u32::try_from(32usize.saturating_add(hwirq)).unwrap_or(0)
    };
    LINUX_PLIC_PARENT_IRQ.store(irq as usize, Ordering::Release);
    irq
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_free_irqs_top() {
    trap("irq_domain_free_irqs_top")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_instantiate(info: *mut c_void) -> *mut c_void {
    debug("linux plic shim: domain instantiate\n");
    if info.is_null() {
        return core::ptr::null_mut();
    }
    LINUX_PLIC_DOMAIN_INSTANTIATE_SEQ.store(next_event_seq(), Ordering::Release);
    let domain = &raw mut LINUX_PLIC_IRQ_DOMAIN as *mut u8 as *mut c_void;
    LINUX_PLIC_DOMAIN_PTR.store(domain as usize, Ordering::Release);
    unsafe {
        let info_bytes = info.cast::<u8>();
        let fwnode = *info_bytes.cast::<usize>();
        if fwnode == 0 {
            return core::ptr::null_mut();
        }
        let ops = *info_bytes
            .add(LINUX_IRQ_DOMAIN_INFO_OPS_OFFSET)
            .cast::<usize>();
        let host_data = *info_bytes
            .add(LINUX_IRQ_DOMAIN_INFO_HOST_DATA_OFFSET)
            .cast::<usize>();
        LINUX_PLIC_DOMAIN_OPS.store(ops, Ordering::Release);
        LINUX_PLIC_DOMAIN_HOST_DATA.store(host_data, Ordering::Release);
        let domain_bytes = domain.cast::<u8>();
        core::ptr::write_bytes(domain_bytes, 0, LINUX_IRQ_DOMAIN_SIZE);
        core::ptr::write(
            domain_bytes
                .add(LINUX_IRQ_DOMAIN_OPS_OFFSET)
                .cast::<usize>(),
            ops,
        );
        core::ptr::write(
            domain_bytes
                .add(LINUX_IRQ_DOMAIN_HOST_DATA_OFFSET)
                .cast::<usize>(),
            host_data,
        );
    }
    domain
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_set_info(
    domain: *mut c_void,
    virq: u32,
    hwirq: usize,
    chip: *const c_void,
    chip_data: *mut c_void,
    handler: *mut c_void,
    _handler_data: *mut c_void,
    _handler_name: *const u8,
) {
    let expected_domain = LINUX_PLIC_DOMAIN_PTR.load(Ordering::Acquire) as *mut c_void;
    if domain.is_null()
        || domain != expected_domain
        || virq == 0
        || hwirq == 0
        || chip.is_null()
        || handler.is_null()
    {
        return;
    }
    let Ok(hwirq) = u32::try_from(hwirq) else {
        return;
    };
    let _ = linux_store_leaf_irq_record(
        virq,
        hwirq,
        chip as usize,
        chip_data as usize,
        handler as usize,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_translate_onecell(
    _domain: *mut c_void,
    fwspec: *mut c_void,
    out_hwirq: *mut usize,
    out_type: *mut u32,
) -> i32 {
    if fwspec.is_null() || out_hwirq.is_null() || out_type.is_null() {
        return -22;
    }
    let fwspec = unsafe { &*fwspec.cast::<LinuxIrqFwspec>() };
    if fwspec.param_count < 1 {
        return -22;
    }
    unsafe {
        *out_hwirq = fwspec.param[0] as usize;
        *out_type = 0;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_domain_translate_twocell() -> i32 {
    trap("irq_domain_translate_twocell")
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_find_matching_fwspec(_fwspec: *const c_void, _bus_token: u32) -> *mut c_void {
    debug("linux plic shim: find fwspec\n");
    LINUX_PLIC_FIND_PARENT_DOMAIN_SEQ.store(next_event_seq(), Ordering::Release);
    &raw mut LINUX_PLIC_INTC_DOMAIN as *mut u8 as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_get_irq_data(irq: u32) -> *mut c_void {
    let parent = LINUX_PLIC_PARENT_IRQ.load(Ordering::Acquire);
    if parent != 0 && irq as usize == parent {
        prepare_linux_parent_irq_desc();
        let desc = (&raw mut LINUX_PLIC_PARENT_IRQ_DESC).cast::<u8>();
        return linux_irq_data_from_desc(desc);
    }

    if let Some(index) = linux_leaf_record_index_by_virq(irq) {
        return linux_leaf_irq_data_ptr(index);
    }
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_modify_status(_irq: u32, _clear: u32, _set: u32) {}

#[unsafe(no_mangle)]
pub extern "C" fn irq_set_affinity(_irq: u32, _mask: *const c_void) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn kfree() {
    trap("kfree")
}

#[unsafe(no_mangle)]
pub extern "C" fn of_iomap(_node: *mut c_void, index: i32) -> *mut c_void {
    debug("linux plic shim: of_iomap\n");
    LINUX_PLIC_OF_IOMAP_CALLS.fetch_add(1, Ordering::AcqRel);
    if index != 0 {
        return core::ptr::null_mut();
    }
    LINUX_PLIC_MEMBASE.load(Ordering::Acquire) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn of_irq_count(_node: *mut c_void) -> i32 {
    debug("linux plic shim: of_irq_count\n");
    LINUX_PLIC_OF_IRQ_COUNT_CALLS.fetch_add(1, Ordering::AcqRel);
    i32::try_from(LINUX_PLIC_CONTEXT_COUNT.load(Ordering::Acquire)).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn of_irq_parse_one(
    _node: *mut c_void,
    index: i32,
    out: *mut LinuxOfPhandleArgs,
) -> i32 {
    debug("linux plic shim: of_irq_parse_one\n");
    LINUX_PLIC_OF_IRQ_PARSE_CALLS.fetch_add(1, Ordering::AcqRel);
    let context_id = LINUX_PLIC_CONTEXT_ID.load(Ordering::Acquire);
    if out.is_null() || index < 0 || usize::try_from(index).ok() != Some(context_id) {
        return -1;
    }
    unsafe {
        (*out).np = &raw mut LINUX_PLIC_PARENT_INTC_NODE as *mut usize as *mut c_void;
        (*out).args_count = 1;
        (*out).args[0] = LINUX_RV_IRQ_EXT;
    }
    LINUX_PLIC_OF_IRQ_PARSE_SUCCESSES.fetch_add(1, Ordering::AcqRel);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn of_match_node(
    matches: *const LinuxOfDeviceId,
    _node: *mut c_void,
) -> *const LinuxOfDeviceId {
    debug("linux plic shim: of_match_node\n");
    LINUX_PLIC_OF_MATCH_CALLS.fetch_add(1, Ordering::AcqRel);
    let mut entry = matches;
    let mut scanned = 0usize;
    while !entry.is_null() && scanned < 16 {
        let compatible = unsafe { cstr_bytes(&(*entry).compatible) };
        if compatible.is_empty() {
            return core::ptr::null();
        }
        if compatible == PLIC_COMPATIBLE_SIFIVE || compatible == PLIC_COMPATIBLE_RISCV {
            return entry;
        }
        entry = unsafe { entry.add(1) };
        scanned += 1;
    }
    core::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn of_property_read_variable_u32_array(
    _node: *mut c_void,
    propname: *const u8,
    out_values: *mut u32,
    sz_min: usize,
    _sz_max: usize,
) -> i32 {
    debug("linux plic shim: of_property_u32_array\n");
    if propname.is_null() || out_values.is_null() || sz_min == 0 {
        return -22;
    }
    if unsafe { cstr_ptr_eq(propname, b"riscv,ndev\0") } {
        unsafe { *out_values = LINUX_PLIC_SOURCE_COUNT.load(Ordering::Acquire) as u32 };
        LINUX_PLIC_OF_PROPERTY_NDEV_CALLS.fetch_add(1, Ordering::AcqRel);
        return 1;
    }
    -22
}

#[unsafe(no_mangle)]
pub extern "C" fn register_syscore_ops(_ops: *mut c_void) -> i32 {
    debug("linux plic shim: syscore\n");
    LINUX_PLIC_SYSCORE_SEQ.store(next_event_seq(), Ordering::Release);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn riscv_get_intc_hwnode() -> *mut c_void {
    &raw mut LINUX_PLIC_PARENT_INTC_NODE as *mut usize as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn riscv_hartid_to_cpuid(hartid: usize) -> i32 {
    if hartid == 0 {
        0
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn riscv_of_parent_hartid(_node: *mut c_void, hartid: *mut usize) -> i32 {
    if hartid.is_null() {
        return -1;
    }
    unsafe { *hartid = 0 };
    0
}
