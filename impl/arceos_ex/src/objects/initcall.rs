use super::{
    config::Config,
    device::{DeviceRef, PlatformDevice, PlatformDeviceStorage},
    device_tree::{DeviceNodeRef, DeviceTree},
    driver::{DeviceDriverRef, ProbeResult},
    ioremap::Ioremap,
    irq_time::{
        IrqDispatchTree, IrqHandlerKind, IrqHandlerRegistry, PlicIrqDomain,
        Serial8250RxBatchLoopbackProbe, Serial8250RxLoopbackProbe, TtyXmitFifoProbe,
        UartExternalIrqEnable, UartInterruptChainProbe,
    },
    mm_core::{PageAllocator, PageMetadataMap, PageTableCaches, VmallocAllocator},
    ns16550a,
    runtime_core::RuntimeCoreBoundary,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    workqueue::Workqueue,
};
use crate::{
    context::Context,
    trace::{self, Checkpoint},
};
use alloc::vec::Vec;
use core::mem::size_of;

pub const INITCALL_LEVEL_COUNT: usize = 8;
pub const INITCALL_RUN_RECORD_CAPACITY: usize = 16;

unsafe extern "C" {
    static __initcall_pure_start: u8;
    static __initcall_pure_end: u8;
    static __initcall_core_start: u8;
    static __initcall_core_end: u8;
    static __initcall_postcore_start: u8;
    static __initcall_postcore_end: u8;
    static __initcall_arch_start: u8;
    static __initcall_arch_end: u8;
    static __initcall_subsys_start: u8;
    static __initcall_subsys_end: u8;
    static __initcall_fs_start: u8;
    static __initcall_fs_end: u8;
    static __initcall_device_start: u8;
    static __initcall_device_end: u8;
    static __initcall_late_start: u8;
    static __initcall_late_end: u8;
}

pub type ContextRef<'a> = &'a mut Context;
pub type InitcallEntryFn = for<'a> fn(ContextRef<'a>) -> InitcallReturn;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InitcallReturn {
    Ok,
    Error(isize),
}

impl InitcallReturn {
    pub const fn code(self) -> isize {
        match self {
            Self::Ok => 0,
            Self::Error(code) => code,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InitcallEntry {
    level: InitcallLevelName,
    name: &'static str,
    entry: InitcallEntryFn,
}

impl InitcallEntry {
    pub const fn new(level: InitcallLevelName, name: &'static str, entry: InitcallEntryFn) -> Self {
        Self { level, name, entry }
    }

    pub const fn level(&self) -> InitcallLevelName {
        self.level
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub fn run(&self, ctx: ContextRef<'_>) -> InitcallReturn {
        (self.entry)(ctx)
    }
}

crate::pure_initcall!(pure_smoke_initcall);
crate::core_initcall!(core_smoke_initcall);
crate::postcore_initcall!(postcore_smoke_initcall);
crate::arch_initcall_sync!(of_platform_default_populate_init);
crate::subsys_initcall!(subsys_smoke_initcall);
crate::fs_initcall!(fs_smoke_initcall);
crate::device_initcall!(device_smoke_initcall);
crate::late_initcall!(late_smoke_initcall);

fn pure_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: pure_smoke_initcall\n");
    InitcallReturn::Ok
}

fn core_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: core_smoke_initcall\n");
    InitcallReturn::Ok
}

fn postcore_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: postcore_smoke_initcall\n");
    InitcallReturn::Ok
}

fn of_platform_default_populate_init(ctx: ContextRef<'_>) -> InitcallReturn {
    let result = {
        let device_tree = &ctx.device_tree;
        ctx.platform_bus
            .of_platform_default_populate_init(device_tree)
    };
    if result == InitcallReturn::Ok {
        crate::checkpoint::dispatch(Checkpoint::OfPlatformDefaultPopulateScanComplete, ctx);
    }
    result
}

fn subsys_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: subsys_smoke_initcall\n");
    InitcallReturn::Ok
}

fn fs_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: fs_smoke_initcall\n");
    InitcallReturn::Ok
}

fn device_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: device_smoke_initcall\n");
    InitcallReturn::Ok
}

fn late_smoke_initcall(_ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: late_smoke_initcall\n");
    InitcallReturn::Ok
}

fn static_initcall_ranges() -> [&'static [InitcallEntry]; INITCALL_LEVEL_COUNT] {
    [
        initcall_range(
            &raw const __initcall_pure_start,
            &raw const __initcall_pure_end,
        ),
        initcall_range(
            &raw const __initcall_core_start,
            &raw const __initcall_core_end,
        ),
        initcall_range(
            &raw const __initcall_postcore_start,
            &raw const __initcall_postcore_end,
        ),
        initcall_range(
            &raw const __initcall_arch_start,
            &raw const __initcall_arch_end,
        ),
        initcall_range(
            &raw const __initcall_subsys_start,
            &raw const __initcall_subsys_end,
        ),
        initcall_range(&raw const __initcall_fs_start, &raw const __initcall_fs_end),
        initcall_range(
            &raw const __initcall_device_start,
            &raw const __initcall_device_end,
        ),
        initcall_range(
            &raw const __initcall_late_start,
            &raw const __initcall_late_end,
        ),
    ]
}

fn initcall_range(start: *const u8, end: *const u8) -> &'static [InitcallEntry] {
    let start_addr = start as usize;
    let end_addr = end as usize;
    let entry_size = size_of::<InitcallEntry>();
    if start_addr == 0
        || end_addr < start_addr
        || entry_size == 0
        || (end_addr - start_addr) % entry_size != 0
    {
        return &[];
    }

    let count = (end_addr - start_addr) / entry_size;
    unsafe { core::slice::from_raw_parts(start as *const InitcallEntry, count) }
}

fn static_initcall_entry_count() -> usize {
    let ranges = static_initcall_ranges();
    let mut count = 0usize;
    let mut index = 0usize;
    while index < INITCALL_LEVEL_COUNT {
        count += ranges[index].len();
        index += 1;
    }
    count
}

fn static_initcall_ranges_valid(ranges: &[&'static [InitcallEntry]; INITCALL_LEVEL_COUNT]) -> bool {
    let mut level_index = 0usize;
    while level_index < INITCALL_LEVEL_COUNT {
        let expected = initcall_level_name(level_index);
        let range = ranges[level_index];
        let mut entry_index = 0usize;
        while entry_index < range.len() {
            if range[entry_index].level() != expected {
                return false;
            }
            entry_index += 1;
        }
        level_index += 1;
    }
    true
}

const fn initcall_level_name(index: usize) -> InitcallLevelName {
    match index {
        0 => InitcallLevelName::Pure,
        1 => InitcallLevelName::Core,
        2 => InitcallLevelName::Postcore,
        3 => InitcallLevelName::Arch,
        4 => InitcallLevelName::Subsys,
        5 => InitcallLevelName::Fs,
        6 => InitcallLevelName::Device,
        _ => InitcallLevelName::Late,
    }
}

#[derive(Clone, Copy)]
pub struct OfPlatformCandidate<'dt> {
    node_id: super::device_tree::DeviceNodeId,
    name: &'dt [u8],
    compatible: &'dt [u8],
}

impl<'dt> OfPlatformCandidate<'dt> {
    pub const fn node_id(&self) -> super::device_tree::DeviceNodeId {
        self.node_id
    }

    pub const fn name(&self) -> &'dt [u8] {
        self.name
    }

    pub const fn compatible(&self) -> &'dt [u8] {
        self.compatible
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InitcallLevelState {
    Pending,
    Done,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InitcallLevelName {
    Pure,
    Core,
    Postcore,
    Arch,
    Subsys,
    Fs,
    Device,
    Late,
}

#[derive(Clone, Copy)]
pub struct InitcallLevel {
    name: InitcallLevelName,
    state: InitcallLevelState,
    entry_count: usize,
}

impl InitcallLevel {
    const fn new(name: InitcallLevelName) -> Self {
        Self {
            name,
            state: InitcallLevelState::Pending,
            entry_count: 0,
        }
    }

    pub const fn name(&self) -> InitcallLevelName {
        self.name
    }

    pub const fn state(&self) -> InitcallLevelState {
        self.state
    }

    pub const fn entry_count(&self) -> usize {
        self.entry_count
    }
}

#[derive(Clone, Copy)]
pub struct InitcallRunRecord {
    level: InitcallLevelName,
    name: &'static str,
    skipped: bool,
    return_code: isize,
    run_context_checked: bool,
}

impl InitcallRunRecord {
    const fn empty() -> Self {
        Self {
            level: InitcallLevelName::Pure,
            name: "",
            skipped: false,
            return_code: 0,
            run_context_checked: false,
        }
    }

    pub const fn level(&self) -> InitcallLevelName {
        self.level
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn skipped(&self) -> bool {
        self.skipped
    }

    pub const fn return_code(&self) -> isize {
        self.return_code
    }

    pub const fn run_context_checked(&self) -> bool {
        self.run_context_checked
    }
}

pub struct CpusetSmpTrimmed {
    lifecycle: Lifecycle,
    trimmed_noop: bool,
}

impl CpusetSmpTrimmed {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trimmed_noop: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn trimmed_noop(&self) -> bool {
        self.trimmed_noop
    }

    pub fn setup(&mut self, runtime_core_boundary: &RuntimeCoreBoundary) -> EventResult {
        if self.lifecycle.state() != State::Base
            || runtime_core_boundary.state() != State::Ready
            || !runtime_core_boundary.do_basic_setup_next_boundary()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.trimmed_noop = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::CpusetSmpTrimmedReady,
        )
    }
}

pub struct DriverCoreBase {
    lifecycle: Lifecycle,
    device_registry_ready: bool,
    bus_registry_ready: bool,
    pre_platform_deferred: bool,
    pre_platform_order_preserved: bool,
}

impl DriverCoreBase {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            device_registry_ready: false,
            bus_registry_ready: false,
            pre_platform_deferred: false,
            pre_platform_order_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn device_registry_ready(&self) -> bool {
        self.device_registry_ready
    }

    pub const fn bus_registry_ready(&self) -> bool {
        self.bus_registry_ready
    }

    pub const fn pre_platform_deferred(&self) -> bool {
        self.pre_platform_deferred
    }

    pub const fn pre_platform_order_preserved(&self) -> bool {
        self.pre_platform_order_preserved
    }

    pub fn setup(
        &mut self,
        cpuset: &CpusetSmpTrimmed,
        page_allocator: &PageAllocator,
        workqueue: &Workqueue,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpuset.state() != State::Ready
            || !cpuset.trimmed_noop()
            || page_allocator.state() != State::Ready
            || !page_allocator.late_ready()
            || workqueue.state() != State::Ready
            || !workqueue.topology_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.device_registry_ready = true;
        self.bus_registry_ready = true;
        self.pre_platform_deferred = true;
        self.pre_platform_order_preserved = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct PlatformBusRootDevice {
    lifecycle: Lifecycle,
    early_platform_cleanup_deferred: bool,
    static_device_registered: bool,
    device_name_bound: bool,
    register_return_zero: bool,
}

impl PlatformBusRootDevice {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            early_platform_cleanup_deferred: false,
            static_device_registered: false,
            device_name_bound: false,
            register_return_zero: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn early_platform_cleanup_deferred(&self) -> bool {
        self.early_platform_cleanup_deferred
    }

    pub const fn static_device_registered(&self) -> bool {
        self.static_device_registered
    }

    pub const fn device_name_bound(&self) -> bool {
        self.device_name_bound
    }

    pub const fn register_return_zero(&self) -> bool {
        self.register_return_zero
    }

    pub fn setup(
        &mut self,
        driver_core_base: &DriverCoreBase,
        static_objects: &StaticObjects,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || driver_core_base.state() != State::Ready
            || !driver_core_base.device_registry_ready()
            || static_objects.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.early_platform_cleanup_deferred = true;
        self.static_device_registered = true;
        self.device_name_bound = true;
        self.register_return_zero = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[derive(Clone, Copy)]
struct PlatformBusProbeObservation {
    driver: DeviceDriverRef,
    device: DeviceRef,
    match_attempted: bool,
    matched: bool,
    probe_called: bool,
    probe_return_zero: bool,
    probe_result: Option<ProbeResult>,
    bound: bool,
}

impl PlatformBusProbeObservation {
    const fn new(driver: DeviceDriverRef, device: DeviceRef) -> Self {
        Self {
            driver,
            device,
            match_attempted: false,
            matched: false,
            probe_called: false,
            probe_return_zero: false,
            probe_result: None,
            bound: false,
        }
    }
}

pub struct PlatformBus {
    lifecycle: Lifecycle,
    registered: bool,
    devices_kset_ready: bool,
    drivers_kset_ready: bool,
    autoprobe_enabled: bool,
    ops_bound: bool,
    register_return_zero: bool,
    platform_devices: PlatformDeviceStorage,
    device_refs: Vec<DeviceRef>,
    driver_refs: Vec<DeviceDriverRef>,
    probe_driver_deferred_count: usize,
    probe_device_deferred_count: usize,
    probe_driver_scanned_devices: bool,
    probe_device_scanned_drivers: bool,
    probe_observations: Vec<PlatformBusProbeObservation>,
    ns16550a_driver_registered: bool,
    ns16550a_match_table_ready: bool,
    ns16550a_device_matched: bool,
    ns16550a_probe_called: bool,
    ns16550a_probe_return_zero: bool,
    ns16550a_bound_device: Option<DeviceRef>,
    ns16550a_probe_ioremaps_uart8250_port: bool,
    ns16550a_probe_registers_uart8250_port: bool,
    ns16550a_probe_records_uart_irq_resource: bool,
    ns16550a_probe_records_uart_irq_mapping: bool,
    ns16550a_probe_registers_uart_irq_handler: bool,
    ns16550a_probe_keeps_interrupt_output_deferred: bool,
    ns16550a_probe_registers_serial_console: bool,
    ns16550a_probe_triggers_console_handoff: bool,
    of_platform_source_tree_ready: bool,
    of_platform_root_children_scanned: bool,
    of_platform_strict_compatible_required: bool,
    of_platform_default_bus_match_table_used: bool,
    of_platform_bus_nodes_recurse: bool,
    of_platform_candidates_identified: bool,
    of_platform_candidates_are_available: bool,
    of_platform_candidate_names_printed: bool,
    of_platform_candidate_compatibles_printed: bool,
    of_platform_devices_created: bool,
    of_platform_devices_added: bool,
    of_platform_candidate_count: usize,
}

impl PlatformBus {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registered: false,
            devices_kset_ready: false,
            drivers_kset_ready: false,
            autoprobe_enabled: false,
            ops_bound: false,
            register_return_zero: false,
            platform_devices: PlatformDeviceStorage::new(),
            device_refs: Vec::new(),
            driver_refs: Vec::new(),
            probe_driver_deferred_count: 0,
            probe_device_deferred_count: 0,
            probe_driver_scanned_devices: false,
            probe_device_scanned_drivers: false,
            probe_observations: Vec::new(),
            ns16550a_driver_registered: false,
            ns16550a_match_table_ready: false,
            ns16550a_device_matched: false,
            ns16550a_probe_called: false,
            ns16550a_probe_return_zero: false,
            ns16550a_bound_device: None,
            ns16550a_probe_ioremaps_uart8250_port: false,
            ns16550a_probe_registers_uart8250_port: false,
            ns16550a_probe_records_uart_irq_resource: false,
            ns16550a_probe_records_uart_irq_mapping: false,
            ns16550a_probe_registers_uart_irq_handler: false,
            ns16550a_probe_keeps_interrupt_output_deferred: false,
            ns16550a_probe_registers_serial_console: false,
            ns16550a_probe_triggers_console_handoff: false,
            of_platform_source_tree_ready: false,
            of_platform_root_children_scanned: false,
            of_platform_strict_compatible_required: false,
            of_platform_default_bus_match_table_used: false,
            of_platform_bus_nodes_recurse: false,
            of_platform_candidates_identified: false,
            of_platform_candidates_are_available: false,
            of_platform_candidate_names_printed: false,
            of_platform_candidate_compatibles_printed: false,
            of_platform_devices_created: false,
            of_platform_devices_added: false,
            of_platform_candidate_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    pub const fn devices_kset_ready(&self) -> bool {
        self.devices_kset_ready
    }

    pub const fn drivers_kset_ready(&self) -> bool {
        self.drivers_kset_ready
    }

    pub const fn autoprobe_enabled(&self) -> bool {
        self.autoprobe_enabled
    }

    pub const fn ops_bound(&self) -> bool {
        self.ops_bound
    }

    pub const fn register_return_zero(&self) -> bool {
        self.register_return_zero
    }

    pub fn device_count(&self) -> usize {
        self.device_refs.len()
    }

    pub fn driver_count(&self) -> usize {
        self.driver_refs.len()
    }

    pub fn platform_device_count(&self) -> usize {
        self.platform_devices.len()
    }

    pub fn klist_device_count(&self) -> usize {
        self.device_refs.len()
    }

    pub fn klist_device_ref(&self, index: usize) -> Option<DeviceRef> {
        self.device_refs.get(index).copied()
    }

    pub fn platform_device(&self, device: DeviceRef) -> Option<&super::device::PlatformDevice> {
        self.platform_devices.get(device)
    }

    pub const fn probe_driver_deferred_count(&self) -> usize {
        self.probe_driver_deferred_count
    }

    pub const fn probe_device_deferred_count(&self) -> usize {
        self.probe_device_deferred_count
    }

    pub const fn probe_driver_scanned_devices(&self) -> bool {
        self.probe_driver_scanned_devices
    }

    pub const fn probe_device_scanned_drivers(&self) -> bool {
        self.probe_device_scanned_drivers
    }

    pub fn platform_device_discovered(&self, device: DeviceRef) -> bool {
        self.contains_device(device)
    }

    pub fn platform_driver_registered(&self, driver: DeviceDriverRef) -> bool {
        self.contains_driver(driver)
    }

    pub fn platform_match_attempted(&self, driver: DeviceDriverRef, device: DeviceRef) -> bool {
        self.probe_observation(driver, device)
            .is_some_and(|observation| observation.match_attempted)
    }

    pub fn platform_driver_matched_device(
        &self,
        driver: DeviceDriverRef,
        device: DeviceRef,
    ) -> bool {
        self.probe_observation(driver, device)
            .is_some_and(|observation| observation.matched)
    }

    pub fn platform_probe_called(&self, driver: DeviceDriverRef, device: DeviceRef) -> bool {
        self.probe_observation(driver, device)
            .is_some_and(|observation| observation.probe_called)
    }

    pub fn platform_probe_return_zero(&self, driver: DeviceDriverRef, device: DeviceRef) -> bool {
        self.probe_observation(driver, device)
            .is_some_and(|observation| observation.probe_return_zero)
    }

    pub fn platform_probe_result(
        &self,
        driver: DeviceDriverRef,
        device: DeviceRef,
    ) -> Option<ProbeResult> {
        self.probe_observation(driver, device)
            .and_then(|observation| observation.probe_result)
    }

    pub fn platform_device_bound(&self, driver: DeviceDriverRef, device: DeviceRef) -> bool {
        self.probe_observation(driver, device)
            .is_some_and(|observation| observation.bound)
    }

    pub const fn ns16550a_driver_registered(&self) -> bool {
        self.ns16550a_driver_registered
    }

    pub const fn ns16550a_match_table_ready(&self) -> bool {
        self.ns16550a_match_table_ready
    }

    pub const fn ns16550a_device_matched(&self) -> bool {
        self.ns16550a_device_matched
    }

    pub const fn ns16550a_probe_called(&self) -> bool {
        self.ns16550a_probe_called
    }

    pub const fn ns16550a_probe_return_zero(&self) -> bool {
        self.ns16550a_probe_return_zero
    }

    pub const fn ns16550a_bound_device(&self) -> Option<DeviceRef> {
        self.ns16550a_bound_device
    }

    pub const fn ns16550a_probe_ioremaps_uart8250_port(&self) -> bool {
        self.ns16550a_probe_ioremaps_uart8250_port
    }

    pub const fn ns16550a_probe_registers_uart8250_port(&self) -> bool {
        self.ns16550a_probe_registers_uart8250_port
    }

    pub const fn ns16550a_probe_records_uart_irq_resource(&self) -> bool {
        self.ns16550a_probe_records_uart_irq_resource
    }

    pub const fn ns16550a_probe_records_uart_irq_mapping(&self) -> bool {
        self.ns16550a_probe_records_uart_irq_mapping
    }

    pub const fn ns16550a_probe_registers_uart_irq_handler(&self) -> bool {
        self.ns16550a_probe_registers_uart_irq_handler
    }

    pub const fn ns16550a_probe_keeps_interrupt_output_deferred(&self) -> bool {
        self.ns16550a_probe_keeps_interrupt_output_deferred
    }

    pub const fn ns16550a_probe_registers_serial_console(&self) -> bool {
        self.ns16550a_probe_registers_serial_console
    }

    pub const fn ns16550a_probe_triggers_console_handoff(&self) -> bool {
        self.ns16550a_probe_triggers_console_handoff
    }

    pub const fn of_platform_source_tree_ready(&self) -> bool {
        self.of_platform_source_tree_ready
    }

    pub const fn of_platform_root_children_scanned(&self) -> bool {
        self.of_platform_root_children_scanned
    }

    pub const fn of_platform_strict_compatible_required(&self) -> bool {
        self.of_platform_strict_compatible_required
    }

    pub const fn of_platform_default_bus_match_table_used(&self) -> bool {
        self.of_platform_default_bus_match_table_used
    }

    pub const fn of_platform_bus_nodes_recurse(&self) -> bool {
        self.of_platform_bus_nodes_recurse
    }

    pub const fn of_platform_candidates_identified(&self) -> bool {
        self.of_platform_candidates_identified
    }

    pub const fn of_platform_candidates_are_available(&self) -> bool {
        self.of_platform_candidates_are_available
    }

    pub const fn of_platform_candidate_names_printed(&self) -> bool {
        self.of_platform_candidate_names_printed
    }

    pub const fn of_platform_candidate_compatibles_printed(&self) -> bool {
        self.of_platform_candidate_compatibles_printed
    }

    pub const fn of_platform_devices_created(&self) -> bool {
        self.of_platform_devices_created
    }

    pub const fn of_platform_devices_added(&self) -> bool {
        self.of_platform_devices_added
    }

    pub const fn of_platform_candidate_count(&self) -> usize {
        self.of_platform_candidate_count
    }

    pub fn contains_device(&self, device: DeviceRef) -> bool {
        self.device_refs.contains(&device) && self.platform_devices.contains(device)
    }

    pub fn contains_driver(&self, driver: DeviceDriverRef) -> bool {
        self.driver_refs.contains(&driver)
    }

    pub fn setup(
        &mut self,
        driver_core_base: &DriverCoreBase,
        platform_bus_root_device: &PlatformBusRootDevice,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || driver_core_base.state() != State::Ready
            || !driver_core_base.bus_registry_ready()
            || platform_bus_root_device.state() != State::Ready
            || !platform_bus_root_device.static_device_registered()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.registered = true;
        self.devices_kset_ready = true;
        self.drivers_kset_ready = true;
        self.autoprobe_enabled = true;
        self.ops_bound = true;
        self.register_return_zero = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn of_platform_default_populate_init(
        &mut self,
        device_tree: &DeviceTree,
    ) -> InitcallReturn {
        crate::objects::printk::write_str("initcall: of_platform_default_populate_init\n");
        if self.state() != State::Ready || !self.registered() || device_tree.state() != State::Ready
        {
            return InitcallReturn::Error(-1);
        }

        let Some(candidates) = collect_of_platform_candidates(device_tree) else {
            return InitcallReturn::Error(-1);
        };

        self.of_platform_source_tree_ready = true;
        self.of_platform_root_children_scanned = true;
        self.of_platform_strict_compatible_required = true;
        self.of_platform_default_bus_match_table_used = true;
        self.of_platform_bus_nodes_recurse = candidates.bus_nodes_seen != 0;
        self.of_platform_candidates_identified = candidates.count() != 0;
        self.of_platform_candidates_are_available = true;
        self.of_platform_candidate_count = candidates.count();

        print_of_platform_candidates(&candidates);
        self.of_platform_candidate_names_printed = candidates.count() != 0;
        self.of_platform_candidate_compatibles_printed = candidates.count() != 0;
        if !self.create_platform_devices_from_candidates(device_tree, &candidates) {
            return InitcallReturn::Error(-1);
        }
        self.of_platform_devices_created = self.platform_device_count() == candidates.count();
        self.of_platform_devices_added = self.klist_device_count() == candidates.count();
        trace::checkpoint(Checkpoint::OfPlatformDefaultPopulateScanComplete);
        InitcallReturn::Ok
    }

    fn create_platform_devices_from_candidates(
        &mut self,
        device_tree: &DeviceTree,
        candidates: &OfPlatformCandidateSet<'_>,
    ) -> bool {
        let mut index = 0usize;
        while index < candidates.count() {
            let Some(candidate) = candidates.entry(index) else {
                return false;
            };
            let Some(platform_device) =
                PlatformDevice::from_node_id(device_tree, candidate.node_id())
            else {
                return false;
            };
            let device_ref = self.platform_devices.push(platform_device);
            if self.add_device(device_ref).is_err() {
                return false;
            }
            index += 1;
        }
        true
    }

    pub fn add_device(&mut self, device: DeviceRef) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.devices_kset_ready
            || self.contains_device(device)
            || !self.platform_devices.contains(device)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.device_refs.push(device);
        Ok(())
    }

    pub fn add_platform_device_from_node(
        &mut self,
        device_tree: &DeviceTree,
        node_id: super::device_tree::DeviceNodeId,
    ) -> Result<DeviceRef, ()> {
        let Some(platform_device) = PlatformDevice::from_node_id(device_tree, node_id) else {
            return Err(());
        };
        let device_ref = self.platform_devices.push(platform_device);
        self.add_device(device_ref).map_err(|_| ())?;
        Ok(device_ref)
    }

    pub fn add_driver(&mut self, driver: DeviceDriverRef) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.drivers_kset_ready
            || self.contains_driver(driver)
            || !is_device_driver_ref_ready(driver)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.driver_refs.push(driver);
        if ns16550a::is_ns16550a_platform_driver(driver) {
            self.ns16550a_driver_registered = true;
            self.ns16550a_match_table_ready = true;
        }
        Ok(())
    }

    pub fn probe_driver(
        &mut self,
        driver: DeviceDriverRef,
        device_tree: &DeviceTree,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &mut IrqHandlerRegistry,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.devices_kset_ready
            || self.device_refs.is_empty()
            || !is_device_driver_ref_ready(driver)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.probe_driver_scanned_devices = true;
        if !self.contains_driver(driver) {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        let mut matched = false;
        let mut index = 0usize;
        while index < self.device_refs.len() {
            let device_ref = self.device_refs[index];
            let device_matched = self.driver_matches_device(driver, device_ref, device_tree);
            self.record_match_attempt(driver, device_ref, device_matched);
            if device_matched {
                matched = true;
                self.probe_and_bind(
                    driver,
                    device_ref,
                    device_tree,
                    vmalloc_allocator,
                    page_table_caches,
                    page_allocator,
                    page_metadata_map,
                    config,
                    ioremap,
                    plic_irq_domain,
                    irq_handler_registry,
                );
            }
            index += 1;
        }

        if !matched {
            self.probe_driver_deferred_count += 1;
        }
        Ok(())
    }

    pub fn probe_device(
        &mut self,
        device: DeviceRef,
        device_tree: &DeviceTree,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &mut IrqHandlerRegistry,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.drivers_kset_ready
            || self.driver_refs.is_empty()
            || !is_device_ref_ready(device)
            || !self.contains_device(device)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.probe_device_scanned_drivers = true;
        let mut index = 0usize;
        while index < self.driver_refs.len() {
            let driver = self.driver_refs[index];
            let matched = self.driver_matches_device(driver, device, device_tree);
            self.record_match_attempt(driver, device, matched);
            if matched {
                self.probe_and_bind(
                    driver,
                    device,
                    device_tree,
                    vmalloc_allocator,
                    page_table_caches,
                    page_allocator,
                    page_metadata_map,
                    config,
                    ioremap,
                    plic_irq_domain,
                    irq_handler_registry,
                );
                return Ok(());
            }
            index += 1;
        }

        self.probe_device_deferred_count += 1;
        Ok(())
    }

    pub fn platform_driver_register(
        &mut self,
        driver: DeviceDriverRef,
        device_tree: &DeviceTree,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &mut IrqHandlerRegistry,
    ) -> InitcallReturn {
        if self.add_driver(driver).is_err() {
            return InitcallReturn::Error(-1);
        }
        if self
            .probe_driver(
                driver,
                device_tree,
                vmalloc_allocator,
                page_table_caches,
                page_allocator,
                page_metadata_map,
                config,
                ioremap,
                plic_irq_domain,
                irq_handler_registry,
            )
            .is_err()
        {
            return InitcallReturn::Error(-1);
        }
        InitcallReturn::Ok
    }

    fn driver_matches_device(
        &self,
        driver: DeviceDriverRef,
        device_ref: DeviceRef,
        device_tree: &DeviceTree,
    ) -> bool {
        let Some(platform_device) = self.platform_device(device_ref) else {
            return false;
        };
        let Some(node) = device_tree.node(platform_device.dev().node_id()) else {
            return false;
        };
        node.compatibles().any(|compatible| {
            driver
                .driver()
                .driver()
                .of_match_table()
                .matches(compatible)
        })
    }

    fn probe_and_bind(
        &mut self,
        driver: DeviceDriverRef,
        device_ref: DeviceRef,
        device_tree: &DeviceTree,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &mut IrqHandlerRegistry,
    ) {
        let Some(platform_device) = self.platform_device(device_ref) else {
            return;
        };
        let node_id = platform_device.dev().node_id();
        let result = driver.driver().probe(
            device_tree,
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            ioremap,
            plic_irq_domain,
            irq_handler_registry,
            device_ref,
            node_id,
        );
        self.record_probe_result(driver, device_ref, result);
        if ns16550a::is_ns16550a_platform_driver(driver) {
            self.ns16550a_device_matched = true;
            self.ns16550a_probe_called = true;
            self.ns16550a_probe_return_zero = result == ProbeResult::Bound;
            if result == ProbeResult::Bound {
                self.ns16550a_bound_device = Some(device_ref);
                self.ns16550a_probe_ioremaps_uart8250_port = ns16550a::uart8250_port_ioremapped()
                    && ns16550a::uart8250_port_resources_ready()
                    && ioremap.mapping_count() != 0
                    && ioremap.mapping_for_device(device_ref).is_some();
                self.ns16550a_probe_registers_uart8250_port = ns16550a::uart8250_port_registered();
                self.ns16550a_probe_records_uart_irq_resource =
                    ns16550a::uart8250_port_irq_resource_ready();
                self.ns16550a_probe_records_uart_irq_mapping =
                    ns16550a::uart8250_port_logical_irq_ready()
                        && plic_irq_domain
                            .mapping_for_source(ns16550a::uart8250_port_irq_source())
                            .is_some_and(|mapping| {
                                mapping.logical_irq() == ns16550a::uart8250_port_logical_irq()
                                    && mapping.source_gate_defined()
                                    && mapping.source_gate_closed()
                                    && mapping.source_enable_deferred()
                                    && mapping.source_not_enabled()
                                    && mapping.handler_not_registered()
                            });
                self.ns16550a_probe_registers_uart_irq_handler =
                    ns16550a::uart8250_irq_handler_registered()
                        && irq_handler_registry
                            .action_for_logical_irq(ns16550a::uart8250_port_logical_irq())
                            .is_some_and(|action| {
                                action.device() == device_ref
                                    && action.handler_kind() == IrqHandlerKind::Ns16550aUart
                                    && action.handler_bound()
                                    && action.hardirq_context_required()
                                    && action.mapped_irq_required()
                                    && action.duplicate_registration_rejected()
                                    && action.unmapped_registration_rejected()
                                    && action.source_not_enabled()
                                    && action.dispatch_ready()
                            });
                self.ns16550a_probe_keeps_interrupt_output_deferred =
                    ns16550a::uart8250_interrupt_output_still_deferred();
                self.ns16550a_probe_registers_serial_console =
                    ns16550a::serial8250_console_registered();
                self.ns16550a_probe_triggers_console_handoff = ns16550a::handoff_triggered();
            }
            self.print_ns16550a_probe(device_ref);
        }
    }

    fn probe_observation(
        &self,
        driver: DeviceDriverRef,
        device: DeviceRef,
    ) -> Option<&PlatformBusProbeObservation> {
        self.probe_observations
            .iter()
            .find(|observation| observation.driver == driver && observation.device == device)
    }

    fn probe_observation_index(&self, driver: DeviceDriverRef, device: DeviceRef) -> Option<usize> {
        let mut index = 0usize;
        while index < self.probe_observations.len() {
            let observation = self.probe_observations[index];
            if observation.driver == driver && observation.device == device {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn probe_observation_mut(
        &mut self,
        driver: DeviceDriverRef,
        device: DeviceRef,
    ) -> &mut PlatformBusProbeObservation {
        if let Some(index) = self.probe_observation_index(driver, device) {
            return &mut self.probe_observations[index];
        }
        self.probe_observations
            .push(PlatformBusProbeObservation::new(driver, device));
        let index = self.probe_observations.len() - 1;
        &mut self.probe_observations[index]
    }

    fn record_match_attempt(&mut self, driver: DeviceDriverRef, device: DeviceRef, matched: bool) {
        let observation = self.probe_observation_mut(driver, device);
        observation.match_attempted = true;
        if matched {
            observation.matched = true;
        }
    }

    fn record_probe_result(
        &mut self,
        driver: DeviceDriverRef,
        device: DeviceRef,
        result: ProbeResult,
    ) {
        let observation = self.probe_observation_mut(driver, device);
        observation.probe_called = true;
        observation.probe_result = Some(result);
        observation.probe_return_zero = result == ProbeResult::Bound;
        if result == ProbeResult::Bound {
            observation.bound = true;
        }
    }

    fn print_ns16550a_probe(&self, device_ref: DeviceRef) {
        crate::objects::printk::write_str("platform_driver: ns16550a probed ");
        let Some(platform_device) = self.platform_device(device_ref) else {
            crate::objects::printk::write_str("<missing>\n");
            return;
        };
        crate::objects::printk::write_str("device=");
        crate::objects::printk::write_fmt(format_args!(
            "node#{}\n",
            platform_device.dev().node_id().index()
        ));
    }
}

const fn is_device_ref_ready(_device: DeviceRef) -> bool {
    true
}

fn is_device_driver_ref_ready(driver: DeviceDriverRef) -> bool {
    !driver.driver().driver().name().is_empty()
}

struct OfPlatformCandidateSet<'dt> {
    entries: Vec<OfPlatformCandidate<'dt>>,
    bus_nodes_seen: usize,
}

impl<'dt> OfPlatformCandidateSet<'dt> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            bus_nodes_seen: 0,
        }
    }

    fn push(&mut self, candidate: OfPlatformCandidate<'dt>) {
        self.entries.push(candidate);
    }

    fn entry(&self, index: usize) -> Option<OfPlatformCandidate<'dt>> {
        self.entries.get(index).copied()
    }

    fn count(&self) -> usize {
        self.entries.len()
    }
}

fn collect_of_platform_candidates(device_tree: &DeviceTree) -> Option<OfPlatformCandidateSet<'_>> {
    let root = device_tree.root()?;
    let mut candidates = OfPlatformCandidateSet::new();

    for child in root.children() {
        collect_of_platform_bus_create(child, &mut candidates)?;
    }

    Some(candidates)
}

fn collect_of_platform_bus_create<'dt>(
    node: DeviceNodeRef<'dt>,
    candidates: &mut OfPlatformCandidateSet<'dt>,
) -> Option<()> {
    let Some(compatible) = node.property(b"compatible") else {
        return Some(());
    };
    if !of_device_is_available(node) || of_platform_node_skipped(node) {
        return Some(());
    }

    candidates.push(OfPlatformCandidate {
        node_id: node.id(),
        name: node.name(),
        compatible: first_compatible(compatible.raw_value()),
    });

    if of_default_bus_match(node) {
        candidates.bus_nodes_seen += 1;
        for child in node.children() {
            collect_of_platform_bus_create(child, candidates)?;
        }
    }

    Some(())
}

fn of_device_is_available(node: DeviceNodeRef<'_>) -> bool {
    let Some(status) = node.property(b"status") else {
        return true;
    };
    let value = first_compatible(status.raw_value());
    value == b"okay" || value == b"ok"
}

fn of_platform_node_skipped(node: DeviceNodeRef<'_>) -> bool {
    node.has_compatible(b"operating-points-v2")
}

fn of_default_bus_match(node: DeviceNodeRef<'_>) -> bool {
    node.has_compatible(b"simple-bus")
        || node.has_compatible(b"simple-mfd")
        || node.has_compatible(b"isa")
}

fn first_compatible(value: &[u8]) -> &[u8] {
    &value[..cstr_slice_len(value)]
}

fn cstr_slice_len(value: &[u8]) -> usize {
    let mut len = 0usize;
    while len < value.len() && value[len] != 0 {
        len += 1;
    }
    len
}

fn print_of_platform_candidates(candidates: &OfPlatformCandidateSet<'_>) {
    crate::objects::printk::write_fmt(format_args!(
        "of_platform: candidates={}\n",
        candidates.count()
    ));

    let mut index = 0usize;
    while index < candidates.count() {
        let Some(candidate) = candidates.entry(index) else {
            return;
        };
        crate::objects::printk::write_str("of_platform: candidate name=");
        print_bytes(candidate.name());
        crate::objects::printk::write_str(" compatible=");
        print_bytes(candidate.compatible());
        crate::objects::printk::write_str("\n");
        index += 1;
    }
}

fn print_bytes(bytes: &[u8]) {
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_graphic() || byte == b' ' {
            crate::objects::printk::write_byte(byte);
        } else {
            crate::objects::printk::write_byte(b'.');
        }
        index += 1;
    }
}

pub struct DriverCoreDeferred {
    lifecycle: Lifecycle,
    post_platform_deferred: bool,
    entry_position_preserved: bool,
}

impl DriverCoreDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            post_platform_deferred: false,
            entry_position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn post_platform_deferred(&self) -> bool {
        self.post_platform_deferred
    }

    pub const fn entry_position_preserved(&self) -> bool {
        self.entry_position_preserved
    }

    pub fn setup(&mut self, platform_bus: &PlatformBus) -> EventResult {
        if self.lifecycle.state() != State::Base
            || platform_bus.state() != State::Ready
            || !platform_bus.registered()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.post_platform_deferred = true;
        self.entry_position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DriverCoreDeferredReady,
        )
    }
}

pub struct IrqProcViewDeferred {
    lifecycle: Lifecycle,
    setup_deferred: bool,
    proc_irq_export_deferred: bool,
}

impl IrqProcViewDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            proc_irq_export_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn setup_deferred(&self) -> bool {
        self.setup_deferred
    }

    pub const fn proc_irq_export_deferred(&self) -> bool {
        self.proc_irq_export_deferred
    }

    pub fn setup(
        &mut self,
        driver_core_base: &DriverCoreBase,
        platform_bus_root_device: &PlatformBusRootDevice,
        platform_bus: &PlatformBus,
        driver_core_deferred: &DriverCoreDeferred,
        irq_dispatch_tree: &IrqDispatchTree,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || driver_core_base.state() != State::Ready
            || !driver_core_base.device_registry_ready()
            || !driver_core_base.bus_registry_ready()
            || platform_bus_root_device.state() != State::Ready
            || !platform_bus_root_device.static_device_registered()
            || platform_bus.state() != State::Ready
            || !platform_bus.registered()
            || driver_core_deferred.state() != State::Ready
            || !driver_core_deferred.post_platform_deferred()
            || irq_dispatch_tree.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.setup_deferred = true;
        self.proc_irq_export_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IrqProcViewDeferredReady,
        )
    }
}

pub struct CtorTable {
    lifecycle: Lifecycle,
    position_preserved: bool,
    constructors_empty_or_trimmed: bool,
}

impl CtorTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            position_preserved: false,
            constructors_empty_or_trimmed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
    }

    pub const fn constructors_empty_or_trimmed(&self) -> bool {
        self.constructors_empty_or_trimmed
    }

    pub fn setup(
        &mut self,
        irq_proc_view: &IrqProcViewDeferred,
        static_objects: &StaticObjects,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || irq_proc_view.state() != State::Ready
            || static_objects.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.position_preserved = true;
        self.constructors_empty_or_trimmed = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::CtorTableReady,
        )
    }
}

pub struct InitcallTable {
    lifecycle: Lifecycle,
    static_ranges_ready: bool,
    level_count_ready: bool,
    all_levels_ran: bool,
    entries_recorded_as_properties: bool,
    registered_entries_collected: bool,
    level_mapping_ready: bool,
    run_levels_ready: bool,
    entry_operation_bindings_ready: bool,
    command_line_scratch_reused_per_level: bool,
    param_parser_applied: bool,
    filter_applied: bool,
    run_context_checked: bool,
    levels: [InitcallLevel; INITCALL_LEVEL_COUNT],
    entries: [InitcallRunRecord; INITCALL_RUN_RECORD_CAPACITY],
    entry_count: usize,
    run_order: [InitcallLevelName; INITCALL_RUN_RECORD_CAPACITY],
    run_count: usize,
}

impl InitcallTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            static_ranges_ready: false,
            level_count_ready: false,
            all_levels_ran: false,
            entries_recorded_as_properties: false,
            registered_entries_collected: false,
            level_mapping_ready: false,
            run_levels_ready: false,
            entry_operation_bindings_ready: false,
            command_line_scratch_reused_per_level: false,
            param_parser_applied: false,
            filter_applied: false,
            run_context_checked: false,
            levels: [
                InitcallLevel::new(InitcallLevelName::Pure),
                InitcallLevel::new(InitcallLevelName::Core),
                InitcallLevel::new(InitcallLevelName::Postcore),
                InitcallLevel::new(InitcallLevelName::Arch),
                InitcallLevel::new(InitcallLevelName::Subsys),
                InitcallLevel::new(InitcallLevelName::Fs),
                InitcallLevel::new(InitcallLevelName::Device),
                InitcallLevel::new(InitcallLevelName::Late),
            ],
            entries: [InitcallRunRecord::empty(); INITCALL_RUN_RECORD_CAPACITY],
            entry_count: 0,
            run_order: [InitcallLevelName::Pure; INITCALL_RUN_RECORD_CAPACITY],
            run_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn static_ranges_ready(&self) -> bool {
        self.static_ranges_ready
    }

    pub const fn level_count_ready(&self) -> bool {
        self.level_count_ready
    }

    pub const fn level_count(&self) -> usize {
        INITCALL_LEVEL_COUNT
    }

    pub const fn all_levels_ran(&self) -> bool {
        self.all_levels_ran
    }

    pub const fn entries_recorded_as_properties(&self) -> bool {
        self.entries_recorded_as_properties
    }

    pub const fn registered_entries_collected(&self) -> bool {
        self.registered_entries_collected
    }

    pub const fn level_mapping_ready(&self) -> bool {
        self.level_mapping_ready
    }

    pub const fn run_levels_ready(&self) -> bool {
        self.run_levels_ready
    }

    pub const fn entry_operation_bindings_ready(&self) -> bool {
        self.entry_operation_bindings_ready
    }

    pub const fn command_line_scratch_reused_per_level(&self) -> bool {
        self.command_line_scratch_reused_per_level
    }

    pub const fn param_parser_applied(&self) -> bool {
        self.param_parser_applied
    }

    pub const fn filter_applied(&self) -> bool {
        self.filter_applied
    }

    pub const fn run_context_checked(&self) -> bool {
        self.run_context_checked
    }

    pub const fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub const fn run_count(&self) -> usize {
        self.run_count
    }

    pub fn run_order_level(&self, index: usize) -> Option<InitcallLevelName> {
        if index < self.run_count {
            Some(self.run_order[index])
        } else {
            None
        }
    }

    pub fn level(&self, index: usize) -> Option<InitcallLevel> {
        if index < INITCALL_LEVEL_COUNT {
            Some(self.levels[index])
        } else {
            None
        }
    }

    pub fn entry(&self, index: usize) -> Option<InitcallRunRecord> {
        if index < self.run_count {
            Some(self.entries[index])
        } else {
            None
        }
    }

    pub fn preset(
        &mut self,
        ctor_table: &CtorTable,
        static_objects: &StaticObjects,
    ) -> EventResult {
        let ranges = static_initcall_ranges();
        let entry_count = static_initcall_entry_count();
        if self.lifecycle.state() != State::Base
            || ctor_table.state() != State::Ready
            || !ctor_table.constructors_empty_or_trimmed()
            || static_objects.state() != State::Online
            || entry_count == 0
            || entry_count > INITCALL_RUN_RECORD_CAPACITY
            || !static_initcall_ranges_valid(&ranges)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.entry_count = entry_count;
        self.static_ranges_ready = true;
        self.level_count_ready = true;
        self.entries_recorded_as_properties = true;
        self.registered_entries_collected = true;
        self.level_mapping_ready = true;
        self.run_levels_ready = true;
        self.entry_operation_bindings_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self, ctx: ContextRef<'_>) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || ctx.saved_command_line.state() != State::Ready
            || ctx.page_allocator.state() != State::Ready
            || !ctx.page_allocator.late_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.command_line_scratch_reused_per_level = true;
        self.param_parser_applied = true;
        self.filter_applied = true;
        self.run_context_checked = true;

        self.run_entries_in_level_order(ctx);
        self.all_levels_ran = all_levels_done(&self.levels);
        if !self.all_levels_ran || !all_entries_checked(&self.entries, self.run_count) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InitcallTableReady,
        )
    }

    fn run_entries_in_level_order(&mut self, ctx: ContextRef<'_>) {
        let ranges = static_initcall_ranges();
        let mut level_index = 0usize;
        let mut out_index = 0usize;
        self.run_count = 0;
        while level_index < INITCALL_LEVEL_COUNT {
            self.levels[level_index].entry_count = 0;
            level_index += 1;
        }
        level_index = 0;

        while level_index < INITCALL_LEVEL_COUNT {
            let level = initcall_level_name(level_index);
            let range = ranges[level_index];
            let mut descriptor_index = 0usize;
            while descriptor_index < range.len() {
                let descriptor = &range[descriptor_index];
                let result = descriptor.run(&mut *ctx);
                self.entries[out_index] = InitcallRunRecord {
                    level,
                    name: descriptor.name(),
                    skipped: false,
                    return_code: result.code(),
                    run_context_checked: true,
                };
                self.run_order[out_index] = level;
                out_index += 1;
                self.run_count = out_index;
                self.levels[level_index].entry_count += 1;
                descriptor_index += 1;
            }
            self.levels[level_index].state = InitcallLevelState::Done;
            level_index += 1;
        }
    }

    pub const fn all_registered_entries_ran(&self) -> bool {
        self.entry_count == self.run_count
    }
}

pub struct InitcallBoundary {
    lifecycle: Lifecycle,
    kunit_next_boundary: bool,
}

impl InitcallBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kunit_next_boundary: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kunit_next_boundary(&self) -> bool {
        self.kunit_next_boundary
    }

    pub fn setup(
        &mut self,
        cpuset: &CpusetSmpTrimmed,
        driver_core_base: &DriverCoreBase,
        platform_bus_root_device: &PlatformBusRootDevice,
        platform_bus: &PlatformBus,
        driver_core: &DriverCoreDeferred,
        irq_proc_view: &IrqProcViewDeferred,
        ctor_table: &CtorTable,
        initcall_table: &InitcallTable,
        uart_external_irq_enable: &UartExternalIrqEnable,
        uart_interrupt_chain_probe: &UartInterruptChainProbe,
        serial8250_rx_loopback_probe: &Serial8250RxLoopbackProbe,
        serial8250_rx_batch_loopback_probe: &Serial8250RxBatchLoopbackProbe,
        tty_xmit_fifo_probe: &TtyXmitFifoProbe,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpuset.state() != State::Ready
            || driver_core_base.state() != State::Ready
            || !driver_core_base.device_registry_ready()
            || !driver_core_base.bus_registry_ready()
            || platform_bus_root_device.state() != State::Ready
            || !platform_bus_root_device.static_device_registered()
            || platform_bus.state() != State::Ready
            || !platform_bus.registered()
            || !platform_bus.ns16550a_probe_ioremaps_uart8250_port()
            || driver_core.state() != State::Ready
            || !driver_core.post_platform_deferred()
            || irq_proc_view.state() != State::Ready
            || ctor_table.state() != State::Ready
            || initcall_table.state() != State::Ready
            || !initcall_table.all_levels_ran()
            || uart_external_irq_enable.state() != State::Ready
            || !uart_external_irq_enable.plic_source_gate_open()
            || !uart_external_irq_enable.root_external_input_gate_open()
            || !uart_external_irq_enable.uart_interrupt_output_deferred()
            || uart_interrupt_chain_probe.state() != State::Ready
            || !uart_interrupt_chain_probe.uart_trigger_committed()
            || !uart_interrupt_chain_probe.plic_claim_observed()
            || !uart_interrupt_chain_probe.irq_dispatch_observed()
            || !uart_interrupt_chain_probe.uart_handler_observed()
            || !uart_interrupt_chain_probe.plic_complete_observed()
            || !uart_interrupt_chain_probe.plic_loop_exit_observed()
            || !uart_interrupt_chain_probe.irq_cycle_closed()
            || !uart_interrupt_chain_probe.console_polling_preserved()
            || serial8250_rx_loopback_probe.state() != State::Ready
            || !serial8250_rx_loopback_probe.rx_runtime_enabled()
            || !serial8250_rx_loopback_probe.loopback_stimulus_committed()
            || !serial8250_rx_loopback_probe.plic_claim_observed()
            || !serial8250_rx_loopback_probe.irq_dispatch_observed()
            || !serial8250_rx_loopback_probe.uart_handler_received_rx()
            || !serial8250_rx_loopback_probe.flip_buffer_pushed()
            || !serial8250_rx_loopback_probe.plic_complete_observed()
            || !serial8250_rx_loopback_probe.zero_claim_loop_exit_observed()
            || !serial8250_rx_loopback_probe.irq_cycle_closed()
            || !serial8250_rx_loopback_probe.last_byte_matched()
            || serial8250_rx_batch_loopback_probe.state() != State::Ready
            || !serial8250_rx_batch_loopback_probe.batch_stimulus_committed()
            || !serial8250_rx_batch_loopback_probe.plic_claim_observed()
            || !serial8250_rx_batch_loopback_probe.irq_dispatch_observed()
            || !serial8250_rx_batch_loopback_probe.uart_handler_received_batch()
            || !serial8250_rx_batch_loopback_probe.flip_buffer_batch_pushed()
            || !serial8250_rx_batch_loopback_probe.plic_complete_observed()
            || !serial8250_rx_batch_loopback_probe.zero_claim_loop_exit_observed()
            || !serial8250_rx_batch_loopback_probe.irq_cycle_closed()
            || !serial8250_rx_batch_loopback_probe.bounded_drain_observed()
            || !serial8250_rx_batch_loopback_probe.batch_count_matched()
            || !serial8250_rx_batch_loopback_probe.last_byte_matched()
            || !serial8250_rx_batch_loopback_probe.no_overflow_observed()
            || tty_xmit_fifo_probe.state() != State::Ready
            || !tty_xmit_fifo_probe.enqueue_committed()
            || !tty_xmit_fifo_probe.dequeue_committed()
            || !tty_xmit_fifo_probe.byte_round_trip()
            || !tty_xmit_fifo_probe.queue_empty_after_dequeue()
            || !tty_xmit_fifo_probe.distinct_from_printk_console_tx()
            || !tty_xmit_fifo_probe.runtime_tx_deferred()
            || !tty_xmit_fifo_probe.printk_tx_queue_unchanged()
            || !tty_xmit_fifo_probe.no_uart_thri_kick()
            || !tty_xmit_fifo_probe.no_overflow_observed()
            || !tty_xmit_fifo_probe.no_underflow_observed()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.kunit_next_boundary = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::InitcallBoundaryReady,
        )
    }
}

pub fn initcall_phase_ready(
    cpuset: &CpusetSmpTrimmed,
    driver_core_base: &DriverCoreBase,
    platform_bus_root_device: &PlatformBusRootDevice,
    platform_bus: &PlatformBus,
    driver_core: &DriverCoreDeferred,
    irq_proc_view: &IrqProcViewDeferred,
    ctor_table: &CtorTable,
    initcall_table: &InitcallTable,
    uart_external_irq_enable: &UartExternalIrqEnable,
    uart_interrupt_chain_probe: &UartInterruptChainProbe,
    serial8250_rx_loopback_probe: &Serial8250RxLoopbackProbe,
    serial8250_rx_batch_loopback_probe: &Serial8250RxBatchLoopbackProbe,
    tty_xmit_fifo_probe: &TtyXmitFifoProbe,
    boundary: &InitcallBoundary,
) -> bool {
    cpuset.state() == State::Ready
        && cpuset.trimmed_noop()
        && driver_core_base.state() == State::Ready
        && driver_core_base.device_registry_ready()
        && driver_core_base.bus_registry_ready()
        && driver_core_base.pre_platform_deferred()
        && driver_core_base.pre_platform_order_preserved()
        && platform_bus_root_device.state() == State::Ready
        && platform_bus_root_device.early_platform_cleanup_deferred()
        && platform_bus_root_device.static_device_registered()
        && platform_bus_root_device.device_name_bound()
        && platform_bus_root_device.register_return_zero()
        && platform_bus.state() == State::Ready
        && platform_bus.registered()
        && platform_bus.devices_kset_ready()
        && platform_bus.drivers_kset_ready()
        && platform_bus.autoprobe_enabled()
        && platform_bus.ops_bound()
        && platform_bus.register_return_zero()
        && platform_bus.of_platform_source_tree_ready()
        && platform_bus.of_platform_root_children_scanned()
        && platform_bus.of_platform_strict_compatible_required()
        && platform_bus.of_platform_default_bus_match_table_used()
        && platform_bus.of_platform_bus_nodes_recurse()
        && platform_bus.of_platform_candidates_identified()
        && platform_bus.of_platform_candidates_are_available()
        && platform_bus.of_platform_candidate_names_printed()
        && platform_bus.of_platform_candidate_compatibles_printed()
        && platform_bus.of_platform_devices_created()
        && platform_bus.of_platform_devices_added()
        && platform_bus.of_platform_candidate_count() != 0
        && platform_bus.platform_device_count() == platform_bus.of_platform_candidate_count()
        && platform_bus.klist_device_count() == platform_bus.of_platform_candidate_count()
        && platform_bus.ns16550a_driver_registered()
        && platform_bus.ns16550a_match_table_ready()
        && platform_bus.ns16550a_device_matched()
        && platform_bus.ns16550a_probe_called()
        && platform_bus.ns16550a_probe_return_zero()
        && platform_bus.ns16550a_bound_device().is_some()
        && platform_bus.ns16550a_probe_ioremaps_uart8250_port()
        && platform_bus.ns16550a_probe_registers_uart8250_port()
        && platform_bus.ns16550a_probe_registers_serial_console()
        && platform_bus.ns16550a_probe_triggers_console_handoff()
        && driver_core.state() == State::Ready
        && driver_core.post_platform_deferred()
        && driver_core.entry_position_preserved()
        && irq_proc_view.state() == State::Ready
        && irq_proc_view.setup_deferred()
        && irq_proc_view.proc_irq_export_deferred()
        && ctor_table.state() == State::Ready
        && ctor_table.position_preserved()
        && ctor_table.constructors_empty_or_trimmed()
        && initcall_table.state() == State::Ready
        && initcall_table.static_ranges_ready()
        && initcall_table.level_count_ready()
        && initcall_table.level_count() == INITCALL_LEVEL_COUNT
        && initcall_table.all_levels_ran()
        && initcall_table.entries_recorded_as_properties()
        && initcall_table.command_line_scratch_reused_per_level()
        && initcall_table.param_parser_applied()
        && initcall_table.filter_applied()
        && initcall_table.run_context_checked()
        && initcall_table.all_registered_entries_ran()
        && all_levels_done_public(initcall_table)
        && all_entries_checked_public(initcall_table)
        && uart_external_irq_enable.state() == State::Ready
        && uart_external_irq_enable.plic_source_gate_open()
        && uart_external_irq_enable.root_external_input_gate_open()
        && uart_external_irq_enable.uart_interrupt_output_deferred()
        && uart_interrupt_chain_probe.state() == State::Ready
        && uart_interrupt_chain_probe.uart_trigger_committed()
        && uart_interrupt_chain_probe.plic_claim_observed()
        && uart_interrupt_chain_probe.irq_dispatch_observed()
        && uart_interrupt_chain_probe.uart_handler_observed()
        && uart_interrupt_chain_probe.plic_complete_observed()
        && uart_interrupt_chain_probe.plic_loop_exit_observed()
        && uart_interrupt_chain_probe.irq_cycle_closed()
        && uart_interrupt_chain_probe.console_polling_preserved()
        && serial8250_rx_loopback_probe.state() == State::Ready
        && serial8250_rx_loopback_probe.rx_runtime_enabled()
        && serial8250_rx_loopback_probe.loopback_stimulus_committed()
        && serial8250_rx_loopback_probe.plic_claim_observed()
        && serial8250_rx_loopback_probe.irq_dispatch_observed()
        && serial8250_rx_loopback_probe.uart_handler_received_rx()
        && serial8250_rx_loopback_probe.flip_buffer_pushed()
        && serial8250_rx_loopback_probe.plic_complete_observed()
        && serial8250_rx_loopback_probe.zero_claim_loop_exit_observed()
        && serial8250_rx_loopback_probe.irq_cycle_closed()
        && serial8250_rx_loopback_probe.last_byte_matched()
        && serial8250_rx_batch_loopback_probe.state() == State::Ready
        && serial8250_rx_batch_loopback_probe.batch_stimulus_committed()
        && serial8250_rx_batch_loopback_probe.plic_claim_observed()
        && serial8250_rx_batch_loopback_probe.irq_dispatch_observed()
        && serial8250_rx_batch_loopback_probe.uart_handler_received_batch()
        && serial8250_rx_batch_loopback_probe.flip_buffer_batch_pushed()
        && serial8250_rx_batch_loopback_probe.plic_complete_observed()
        && serial8250_rx_batch_loopback_probe.zero_claim_loop_exit_observed()
        && serial8250_rx_batch_loopback_probe.irq_cycle_closed()
        && serial8250_rx_batch_loopback_probe.bounded_drain_observed()
        && serial8250_rx_batch_loopback_probe.batch_count_matched()
        && serial8250_rx_batch_loopback_probe.last_byte_matched()
        && serial8250_rx_batch_loopback_probe.no_overflow_observed()
        && tty_xmit_fifo_probe.state() == State::Ready
        && tty_xmit_fifo_probe.enqueue_committed()
        && tty_xmit_fifo_probe.dequeue_committed()
        && tty_xmit_fifo_probe.byte_round_trip()
        && tty_xmit_fifo_probe.queue_empty_after_dequeue()
        && tty_xmit_fifo_probe.distinct_from_printk_console_tx()
        && tty_xmit_fifo_probe.runtime_tx_deferred()
        && tty_xmit_fifo_probe.printk_tx_queue_unchanged()
        && tty_xmit_fifo_probe.no_uart_thri_kick()
        && tty_xmit_fifo_probe.no_overflow_observed()
        && tty_xmit_fifo_probe.no_underflow_observed()
        && boundary.state() == State::Ready
        && boundary.kunit_next_boundary()
}

fn all_levels_done(levels: &[InitcallLevel; INITCALL_LEVEL_COUNT]) -> bool {
    let mut index = 0usize;
    while index < INITCALL_LEVEL_COUNT {
        if levels[index].state() != InitcallLevelState::Done {
            return false;
        }
        index += 1;
    }
    true
}

fn all_entries_checked(
    entries: &[InitcallRunRecord; INITCALL_RUN_RECORD_CAPACITY],
    count: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        if entries[index].skipped()
            || entries[index].return_code() != 0
            || !entries[index].run_context_checked()
        {
            return false;
        }
        index += 1;
    }
    true
}

fn all_levels_done_public(initcall_table: &InitcallTable) -> bool {
    let mut index = 0usize;
    while index < INITCALL_LEVEL_COUNT {
        let Some(level) = initcall_table.level(index) else {
            return false;
        };
        if level.state() != InitcallLevelState::Done {
            return false;
        }
        index += 1;
    }
    true
}

fn all_entries_checked_public(initcall_table: &InitcallTable) -> bool {
    let mut index = 0usize;
    while index < initcall_table.run_count() {
        let Some(entry) = initcall_table.entry(index) else {
            return false;
        };
        if entry.skipped() || entry.return_code() != 0 || !entry.run_context_checked() {
            return false;
        }
        index += 1;
    }
    true
}
