use super::{
    irq_time::IrqDispatchTree,
    mm_core::PageAllocator,
    runtime_core::RuntimeCoreBoundary,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    workqueue::Workqueue,
};
use crate::{context::Context, trace::Checkpoint};
use core::mem::size_of;

pub const INITCALL_LEVEL_COUNT: usize = 8;
pub const INITCALL_ENTRY_COUNT: usize = 8;
pub const PLATFORM_BUS_ACTION_SLOT_COUNT: usize = 4;

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
pub struct InitcallEntryDescriptor {
    level: InitcallLevelName,
    name: &'static str,
    entry: InitcallEntryFn,
}

impl InitcallEntryDescriptor {
    pub const fn new(
        level: InitcallLevelName,
        name: &'static str,
        entry: InitcallEntryFn,
    ) -> Self {
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

#[used]
#[unsafe(link_section = ".initcall.pure")]
static INITCALL_PURE_SMOKE: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Pure,
    "pure_smoke_initcall",
    pure_smoke_initcall,
);

#[used]
#[unsafe(link_section = ".initcall.core")]
static INITCALL_CORE_SMOKE: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Core,
    "core_smoke_initcall",
    core_smoke_initcall,
);

#[used]
#[unsafe(link_section = ".initcall.postcore")]
static INITCALL_POSTCORE_SMOKE: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Postcore,
    "postcore_smoke_initcall",
    postcore_smoke_initcall,
);

#[used]
#[unsafe(link_section = ".initcall.arch")]
static INITCALL_ARCH_PLATFORM: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Arch,
    "of_platform_default_populate_init",
    of_platform_default_populate_initcall,
);

#[used]
#[unsafe(link_section = ".initcall.subsys")]
static INITCALL_SUBSYS_SMOKE: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Subsys,
    "subsys_smoke_initcall",
    subsys_smoke_initcall,
);

#[used]
#[unsafe(link_section = ".initcall.fs")]
static INITCALL_FS_SMOKE: InitcallEntryDescriptor =
    InitcallEntryDescriptor::new(InitcallLevelName::Fs, "fs_smoke_initcall", fs_smoke_initcall);

#[used]
#[unsafe(link_section = ".initcall.device")]
static INITCALL_DEVICE_SMOKE: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Device,
    "device_smoke_initcall",
    device_smoke_initcall,
);

#[used]
#[unsafe(link_section = ".initcall.late")]
static INITCALL_LATE_SMOKE: InitcallEntryDescriptor = InitcallEntryDescriptor::new(
    InitcallLevelName::Late,
    "late_smoke_initcall",
    late_smoke_initcall,
);

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

fn of_platform_default_populate_initcall(ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: of_platform_default_populate_init\n");
    if ctx.platform_bus_type.state() == State::Ready && ctx.platform_bus_type.registered() {
        InitcallReturn::Ok
    } else {
        InitcallReturn::Error(-1)
    }
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

fn static_initcall_descriptors() -> impl Iterator<Item = &'static InitcallEntryDescriptor> {
    [
        initcall_range(&raw const __initcall_pure_start, &raw const __initcall_pure_end),
        initcall_range(&raw const __initcall_core_start, &raw const __initcall_core_end),
        initcall_range(
            &raw const __initcall_postcore_start,
            &raw const __initcall_postcore_end,
        ),
        initcall_range(&raw const __initcall_arch_start, &raw const __initcall_arch_end),
        initcall_range(&raw const __initcall_subsys_start, &raw const __initcall_subsys_end),
        initcall_range(&raw const __initcall_fs_start, &raw const __initcall_fs_end),
        initcall_range(
            &raw const __initcall_device_start,
            &raw const __initcall_device_end,
        ),
        initcall_range(&raw const __initcall_late_start, &raw const __initcall_late_end),
    ]
    .into_iter()
    .flatten()
}

fn initcall_range(start: *const u8, end: *const u8) -> &'static [InitcallEntryDescriptor] {
    let start_addr = start as usize;
    let end_addr = end as usize;
    let entry_size = size_of::<InitcallEntryDescriptor>();
    if start_addr == 0
        || end_addr < start_addr
        || entry_size == 0
        || (end_addr - start_addr) % entry_size != 0
    {
        return &[];
    }

    let count = (end_addr - start_addr) / entry_size;
    unsafe { core::slice::from_raw_parts(start as *const InitcallEntryDescriptor, count) }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BusDeviceRef {
    MockPlatformDevice,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BusDriverRef {
    MockPlatformDriver,
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
pub struct InitcallEntry {
    level: InitcallLevelName,
    name: &'static str,
    skipped: bool,
    return_code: isize,
    run_context_checked: bool,
}

impl InitcallEntry {
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

pub struct PlatformBusDevice {
    lifecycle: Lifecycle,
    early_platform_cleanup_deferred: bool,
    static_device_registered: bool,
    device_name_bound: bool,
    register_return_zero: bool,
}

impl PlatformBusDevice {
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

pub struct PlatformBusType {
    lifecycle: Lifecycle,
    registered: bool,
    devices_kset_ready: bool,
    drivers_kset_ready: bool,
    autoprobe_enabled: bool,
    ops_bound: bool,
    register_return_zero: bool,
    device_refs: [Option<BusDeviceRef>; PLATFORM_BUS_ACTION_SLOT_COUNT],
    device_count: usize,
    driver_refs: [Option<BusDriverRef>; PLATFORM_BUS_ACTION_SLOT_COUNT],
    driver_count: usize,
    probe_driver_deferred_count: usize,
    probe_device_deferred_count: usize,
    probe_driver_scanned_devices: bool,
    probe_device_scanned_drivers: bool,
}

impl PlatformBusType {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registered: false,
            devices_kset_ready: false,
            drivers_kset_ready: false,
            autoprobe_enabled: false,
            ops_bound: false,
            register_return_zero: false,
            device_refs: [None; PLATFORM_BUS_ACTION_SLOT_COUNT],
            device_count: 0,
            driver_refs: [None; PLATFORM_BUS_ACTION_SLOT_COUNT],
            driver_count: 0,
            probe_driver_deferred_count: 0,
            probe_device_deferred_count: 0,
            probe_driver_scanned_devices: false,
            probe_device_scanned_drivers: false,
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

    pub const fn device_count(&self) -> usize {
        self.device_count
    }

    pub const fn driver_count(&self) -> usize {
        self.driver_count
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

    pub fn contains_device(&self, device: BusDeviceRef) -> bool {
        let mut index = 0usize;
        while index < self.device_count {
            if self.device_refs[index] == Some(device) {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn contains_driver(&self, driver: BusDriverRef) -> bool {
        let mut index = 0usize;
        while index < self.driver_count {
            if self.driver_refs[index] == Some(driver) {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn setup(
        &mut self,
        driver_core_base: &DriverCoreBase,
        platform_bus_device: &PlatformBusDevice,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || driver_core_base.state() != State::Ready
            || !driver_core_base.bus_registry_ready()
            || platform_bus_device.state() != State::Ready
            || !platform_bus_device.static_device_registered()
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

    pub fn add_device(&mut self, device: BusDeviceRef) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.devices_kset_ready
            || self.contains_device(device)
            || self.device_count >= PLATFORM_BUS_ACTION_SLOT_COUNT
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.device_refs[self.device_count] = Some(device);
        self.device_count += 1;
        Ok(())
    }

    pub fn add_driver(&mut self, driver: BusDriverRef) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.drivers_kset_ready
            || self.contains_driver(driver)
            || self.driver_count >= PLATFORM_BUS_ACTION_SLOT_COUNT
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.driver_refs[self.driver_count] = Some(driver);
        self.driver_count += 1;
        Ok(())
    }

    pub fn probe_driver(&mut self, driver: BusDriverRef) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.devices_kset_ready
            || self.device_count == 0
            || !is_bus_driver_ref_ready(driver)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.probe_driver_scanned_devices = true;
        self.probe_driver_deferred_count += 1;
        Ok(())
    }

    pub fn probe_device(&mut self, device: BusDeviceRef) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.registered
            || !self.drivers_kset_ready
            || self.driver_count == 0
            || !is_bus_device_ref_ready(device)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.probe_device_scanned_drivers = true;
        self.probe_device_deferred_count += 1;
        Ok(())
    }
}

const fn is_bus_device_ref_ready(device: BusDeviceRef) -> bool {
    matches!(device, BusDeviceRef::MockPlatformDevice)
}

const fn is_bus_driver_ref_ready(driver: BusDriverRef) -> bool {
    matches!(driver, BusDriverRef::MockPlatformDriver)
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

    pub fn setup(&mut self, platform_bus_type: &PlatformBusType) -> EventResult {
        if self.lifecycle.state() != State::Base
            || platform_bus_type.state() != State::Ready
            || !platform_bus_type.registered()
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
        platform_bus_device: &PlatformBusDevice,
        platform_bus_type: &PlatformBusType,
        driver_core_deferred: &DriverCoreDeferred,
        irq_dispatch_tree: &IrqDispatchTree,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || driver_core_base.state() != State::Ready
            || !driver_core_base.device_registry_ready()
            || !driver_core_base.bus_registry_ready()
            || platform_bus_device.state() != State::Ready
            || !platform_bus_device.static_device_registered()
            || platform_bus_type.state() != State::Ready
            || !platform_bus_type.registered()
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
    entries: [InitcallEntry; INITCALL_ENTRY_COUNT],
    descriptors: [Option<InitcallEntryDescriptor>; INITCALL_ENTRY_COUNT],
    entry_count: usize,
    run_order: [InitcallLevelName; INITCALL_ENTRY_COUNT],
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
            entries: [InitcallEntry::empty(); INITCALL_ENTRY_COUNT],
            descriptors: [None; INITCALL_ENTRY_COUNT],
            entry_count: 0,
            run_order: [InitcallLevelName::Pure; INITCALL_ENTRY_COUNT],
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

    pub fn entry(&self, index: usize) -> Option<InitcallEntry> {
        if index < self.entry_count {
            Some(self.entries[index])
        } else {
            None
        }
    }

    pub fn register(
        &mut self,
        level: InitcallLevelName,
        name: &'static str,
        entry: InitcallEntryFn,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.entry_count >= INITCALL_ENTRY_COUNT
            || self.contains_entry(name)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.descriptors[self.entry_count] = Some(InitcallEntryDescriptor::new(level, name, entry));
        self.entry_count += 1;
        Ok(())
    }

    pub fn register_static_entries(&mut self) -> EventResult {
        for descriptor in static_initcall_descriptors() {
            self.register(descriptor.level(), descriptor.name(), descriptor.entry)?;
        }
        Ok(())
    }

    pub fn preset(&mut self, ctor_table: &CtorTable, static_objects: &StaticObjects) -> EventResult {
        if self.lifecycle.state() != State::Base
            || ctor_table.state() != State::Ready
            || !ctor_table.constructors_empty_or_trimmed()
            || static_objects.state() != State::Online
            || self.entry_count == 0
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

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

    pub fn run_registered_entries_in_context(&mut self, ctx: ContextRef<'_>) -> EventResult {
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
        let names = [
            InitcallLevelName::Pure,
            InitcallLevelName::Core,
            InitcallLevelName::Postcore,
            InitcallLevelName::Arch,
            InitcallLevelName::Subsys,
            InitcallLevelName::Fs,
            InitcallLevelName::Device,
            InitcallLevelName::Late,
        ];

        let mut level_index = 0usize;
        let mut out_index = 0usize;
        self.run_count = 0;
        while level_index < INITCALL_LEVEL_COUNT {
            self.levels[level_index].entry_count = 0;
            level_index += 1;
        }
        level_index = 0;

        while level_index < INITCALL_LEVEL_COUNT {
            let level = names[level_index];
            let mut descriptor_index = 0usize;
            while descriptor_index < self.entry_count {
                if let Some(descriptor) = self.descriptors[descriptor_index] {
                    if descriptor.level() == level {
                        let result = descriptor.run(&mut *ctx);
                        self.entries[out_index] = InitcallEntry {
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
                    }
                }
                descriptor_index += 1;
            }
            self.levels[level_index].state = InitcallLevelState::Done;
            level_index += 1;
        }
    }

    fn contains_entry(&self, name: &'static str) -> bool {
        let mut index = 0usize;
        while index < self.entry_count {
            if let Some(descriptor) = self.descriptors[index] {
                if descriptor.name() == name {
                    return true;
                }
            }
            index += 1;
        }
        false
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
        platform_bus_device: &PlatformBusDevice,
        platform_bus_type: &PlatformBusType,
        driver_core: &DriverCoreDeferred,
        irq_proc_view: &IrqProcViewDeferred,
        ctor_table: &CtorTable,
        initcall_table: &InitcallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpuset.state() != State::Ready
            || driver_core_base.state() != State::Ready
            || !driver_core_base.device_registry_ready()
            || !driver_core_base.bus_registry_ready()
            || platform_bus_device.state() != State::Ready
            || !platform_bus_device.static_device_registered()
            || platform_bus_type.state() != State::Ready
            || !platform_bus_type.registered()
            || driver_core.state() != State::Ready
            || !driver_core.post_platform_deferred()
            || irq_proc_view.state() != State::Ready
            || ctor_table.state() != State::Ready
            || initcall_table.state() != State::Ready
            || !initcall_table.all_levels_ran()
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
    platform_bus_device: &PlatformBusDevice,
    platform_bus_type: &PlatformBusType,
    driver_core: &DriverCoreDeferred,
    irq_proc_view: &IrqProcViewDeferred,
    ctor_table: &CtorTable,
    initcall_table: &InitcallTable,
    boundary: &InitcallBoundary,
) -> bool {
    cpuset.state() == State::Ready
        && cpuset.trimmed_noop()
        && driver_core_base.state() == State::Ready
        && driver_core_base.device_registry_ready()
        && driver_core_base.bus_registry_ready()
        && driver_core_base.pre_platform_deferred()
        && driver_core_base.pre_platform_order_preserved()
        && platform_bus_device.state() == State::Ready
        && platform_bus_device.early_platform_cleanup_deferred()
        && platform_bus_device.static_device_registered()
        && platform_bus_device.device_name_bound()
        && platform_bus_device.register_return_zero()
        && platform_bus_type.state() == State::Ready
        && platform_bus_type.registered()
        && platform_bus_type.devices_kset_ready()
        && platform_bus_type.drivers_kset_ready()
        && platform_bus_type.autoprobe_enabled()
        && platform_bus_type.ops_bound()
        && platform_bus_type.register_return_zero()
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
        && initcall_table.entry_count() == INITCALL_ENTRY_COUNT
        && initcall_table.all_registered_entries_ran()
        && all_levels_done_public(initcall_table)
        && all_entries_checked_public(initcall_table)
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

fn all_entries_checked(entries: &[InitcallEntry; INITCALL_ENTRY_COUNT], count: usize) -> bool {
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
