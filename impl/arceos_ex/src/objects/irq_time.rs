use core::{
    mem::size_of,
    sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
};

use super::{
    config::Config,
    cpu_group::CpuGroup,
    device::DeviceRef,
    device_tree::{DeviceNodeRef, DevicePropertyRef, DeviceTree},
    fdt_reader::{read_be_u32, read_cells},
    interrupt_stream::InterruptStream,
    ioremap::Ioremap,
    mm_core::{PageAllocator, PageMetadataMap, PageTableCaches, SlubSubsystem, VmallocAllocator},
    per_cpu_storage::PerCpuStorage,
    sbi::Sbi,
    softirq::Softirq,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::StaticBranch,
};
use crate::{arch::riscv64, trace::Checkpoint};

const DEFAULT_TIMEBASE_HZ: u64 = 10_000_000;
const IRQCHIP_RUN_RECORD_CAPACITY: usize = 8;
const PLIC_COMPATIBLE_SIFIVE: &[u8] = b"sifive,plic-1.0.0";
const PLIC_COMPATIBLE_RISCV: &[u8] = b"riscv,plic0";
#[cfg(not(plic_provider_linux_object))]
const PLIC_PRIORITY_BASE: usize = 0;
#[cfg(not(plic_provider_linux_object))]
const PLIC_PRIORITY_PER_ID: usize = 4;
const PLIC_CONTEXT_ENABLE_BASE: usize = 0x2000;
const PLIC_CONTEXT_ENABLE_SIZE: usize = 0x80;
const PLIC_CONTEXT_BASE: usize = 0x200000;
const PLIC_CONTEXT_SIZE: usize = 0x1000;
const PLIC_CONTEXT_THRESHOLD: usize = 0x00;
const PLIC_CONTEXT_CLAIM: usize = 0x4;
const PLIC_IRQ_MAPPING_CAPACITY: usize = 32;
const PLIC_LOGICAL_IRQ_BASE: usize = 32;
const IRQ_ACTION_CAPACITY: usize = 16;

static TIMER_INTERRUPT_COUNT: AtomicUsize = AtomicUsize::new(0);
static ONESHOT_DEADLINE: AtomicU64 = AtomicU64::new(0);
static ONESHOT_CALLBACK: AtomicUsize = AtomicUsize::new(0);

pub type ClockEventCallback = fn(u64);
pub type IrqChipInitFn = for<'dt> fn(
    &mut Plic,
    DeviceNodeRef<'dt>,
    &RiscvIntc,
    &CpuGroup,
    &mut VmallocAllocator,
    &mut PageTableCaches,
    &mut PageAllocator,
    &PageMetadataMap,
    &Config,
    &mut Ioremap,
) -> EventResult;

unsafe extern "C" {
    static __irqchip_init_start: u8;
    static __irqchip_init_end: u8;
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct IrqChipInitEntry {
    name: &'static str,
    compatible: &'static [u8],
    init: IrqChipInitFn,
}

impl IrqChipInitEntry {
    pub const fn new(name: &'static str, compatible: &'static [u8], init: IrqChipInitFn) -> Self {
        Self {
            name,
            compatible,
            init,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn compatible(&self) -> &'static [u8] {
        self.compatible
    }

    fn run(
        &self,
        plic: &mut Plic,
        node: DeviceNodeRef<'_>,
        riscv_intc: &RiscvIntc,
        cpu_group: &CpuGroup,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
    ) -> EventResult {
        (self.init)(
            plic,
            node,
            riscv_intc,
            cpu_group,
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            ioremap,
        )
    }
}

#[derive(Clone, Copy)]
pub struct IrqChipInitRunRecord {
    name: &'static str,
    compatible: &'static [u8],
    matched: bool,
    callback_invoked: bool,
    return_ok: bool,
}

impl IrqChipInitRunRecord {
    const fn empty() -> Self {
        Self {
            name: "",
            compatible: b"",
            matched: false,
            callback_invoked: false,
            return_ok: false,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn compatible(&self) -> &'static [u8] {
        self.compatible
    }

    pub const fn matched(&self) -> bool {
        self.matched
    }

    pub const fn callback_invoked(&self) -> bool {
        self.callback_invoked
    }

    pub const fn return_ok(&self) -> bool {
        self.return_ok
    }
}

crate::irqchip_declare!(sifive_plic, "sifive,plic-1.0.0", plic_irqchip_init);
crate::irqchip_declare!(riscv_plic0, "riscv,plic0", plic_irqchip_init);

fn irqchip_init_range() -> &'static [IrqChipInitEntry] {
    let start = &raw const __irqchip_init_start as *const u8;
    let end = &raw const __irqchip_init_end as *const u8;
    let start_addr = start as usize;
    let end_addr = end as usize;
    let entry_size = size_of::<IrqChipInitEntry>();
    if start_addr == 0
        || end_addr < start_addr
        || entry_size == 0
        || (end_addr - start_addr) % entry_size != 0
    {
        return &[];
    }

    let count = (end_addr - start_addr) / entry_size;
    unsafe { core::slice::from_raw_parts(start as *const IrqChipInitEntry, count) }
}

fn irqchip_init_range_valid(range: &[IrqChipInitEntry]) -> bool {
    let mut index = 0usize;
    while index < range.len() {
        if range[index].name().is_empty() || range[index].compatible().is_empty() {
            return false;
        }
        index += 1;
    }
    true
}

fn plic_irqchip_init(
    plic: &mut Plic,
    node: DeviceNodeRef<'_>,
    riscv_intc: &RiscvIntc,
    cpu_group: &CpuGroup,
    vmalloc_allocator: &mut VmallocAllocator,
    page_table_caches: &mut PageTableCaches,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
    config: &Config,
    ioremap: &mut Ioremap,
) -> EventResult {
    plic.preset_from_irqchip(
        node,
        riscv_intc,
        cpu_group,
        vmalloc_allocator,
        page_table_caches,
        page_allocator,
        page_metadata_map,
        config,
        ioremap,
    )
}

pub struct IrqController {
    lifecycle: Lifecycle,
    descriptors_ready: bool,
    domain_ready: bool,
    allocator_minimal_ready: bool,
}

impl IrqController {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            descriptors_ready: false,
            domain_ready: false,
            allocator_minimal_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn descriptors_ready(&self) -> bool {
        self.descriptors_ready
    }

    pub const fn domain_ready(&self) -> bool {
        self.domain_ready
    }

    pub const fn allocator_minimal_ready(&self) -> bool {
        self.allocator_minimal_ready
    }

    pub fn setup(
        &mut self,
        device_tree: &DeviceTree,
        page_allocator: &PageAllocator,
        slub_subsystem: &SlubSubsystem,
        per_cpu_storage: &PerCpuStorage,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || page_allocator.state() != State::Ready
            || slub_subsystem.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || cpu_group.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.descriptors_ready = true;
        self.domain_ready = true;
        self.allocator_minimal_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IrqControllerReady,
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum IrqHandlerKind {
    None,
    Ns16550aUart,
    VirtioMmio,
}

pub struct IrqAction {
    lifecycle: Lifecycle,
    logical_irq: LogicalIrq,
    device: DeviceRef,
    handler_kind: IrqHandlerKind,
    handler_bound: bool,
    hardirq_context_required: bool,
    mapped_irq_required: bool,
    duplicate_registration_rejected: bool,
    unmapped_registration_rejected: bool,
    source_enabled: bool,
    dispatch_ready: bool,
}

impl IrqAction {
    const fn empty() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            logical_irq: LogicalIrq::invalid(),
            device: DeviceRef::new(usize::MAX),
            handler_kind: IrqHandlerKind::None,
            handler_bound: false,
            hardirq_context_required: false,
            mapped_irq_required: false,
            duplicate_registration_rejected: false,
            unmapped_registration_rejected: false,
            source_enabled: false,
            dispatch_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn logical_irq(&self) -> LogicalIrq {
        self.logical_irq
    }

    pub const fn device(&self) -> DeviceRef {
        self.device
    }

    pub const fn handler_kind(&self) -> IrqHandlerKind {
        self.handler_kind
    }

    pub const fn handler_bound(&self) -> bool {
        self.handler_bound
    }

    pub const fn hardirq_context_required(&self) -> bool {
        self.hardirq_context_required
    }

    pub const fn mapped_irq_required(&self) -> bool {
        self.mapped_irq_required
    }

    pub const fn duplicate_registration_rejected(&self) -> bool {
        self.duplicate_registration_rejected
    }

    pub const fn unmapped_registration_rejected(&self) -> bool {
        self.unmapped_registration_rejected
    }

    pub const fn source_not_enabled(&self) -> bool {
        !self.source_enabled
    }

    pub const fn dispatch_ready(&self) -> bool {
        self.dispatch_ready
    }

    fn setup_from_request(
        &mut self,
        logical_irq: LogicalIrq,
        device: DeviceRef,
        handler_kind: IrqHandlerKind,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || !logical_irq.is_valid()
            || handler_kind == IrqHandlerKind::None
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.logical_irq = logical_irq;
        self.device = device;
        self.handler_kind = handler_kind;
        self.handler_bound = true;
        self.hardirq_context_required = true;
        self.mapped_irq_required = true;
        self.duplicate_registration_rejected = true;
        self.unmapped_registration_rejected = true;
        self.source_enabled = false;
        self.dispatch_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct IrqHandlerRegistry {
    lifecycle: Lifecycle,
    action_table_ready: bool,
    owner_irq_core: bool,
    requires_mapped_logical_irq: bool,
    duplicate_policy_ready: bool,
    unmapped_reject_ready: bool,
    hardirq_context_guard_ready: bool,
    source_enable_deferred: bool,
    dispatch_ready: bool,
    dispatch_requires_hardirq_context: bool,
    dispatch_calls: AtomicUsize,
    duplicate_registration_rejected: bool,
    unmapped_registration_rejected: bool,
    actions: [IrqAction; IRQ_ACTION_CAPACITY],
    action_count: usize,
}

impl IrqHandlerRegistry {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            action_table_ready: false,
            owner_irq_core: false,
            requires_mapped_logical_irq: false,
            duplicate_policy_ready: false,
            unmapped_reject_ready: false,
            hardirq_context_guard_ready: false,
            source_enable_deferred: false,
            dispatch_ready: false,
            dispatch_requires_hardirq_context: false,
            dispatch_calls: AtomicUsize::new(0),
            duplicate_registration_rejected: false,
            unmapped_registration_rejected: false,
            actions: [const { IrqAction::empty() }; IRQ_ACTION_CAPACITY],
            action_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn action_table_ready(&self) -> bool {
        self.action_table_ready
    }

    pub const fn owner_irq_core(&self) -> bool {
        self.owner_irq_core
    }

    pub const fn requires_mapped_logical_irq(&self) -> bool {
        self.requires_mapped_logical_irq
    }

    pub const fn duplicate_policy_ready(&self) -> bool {
        self.duplicate_policy_ready
    }

    pub const fn unmapped_reject_ready(&self) -> bool {
        self.unmapped_reject_ready
    }

    pub const fn hardirq_context_guard_ready(&self) -> bool {
        self.hardirq_context_guard_ready
    }

    pub const fn source_enable_deferred(&self) -> bool {
        self.source_enable_deferred
    }

    pub const fn dispatch_ready(&self) -> bool {
        self.dispatch_ready
    }

    pub const fn dispatch_requires_hardirq_context(&self) -> bool {
        self.dispatch_requires_hardirq_context
    }

    #[allow(dead_code)]
    pub fn dispatch_calls(&self) -> usize {
        self.dispatch_calls.load(Ordering::Acquire)
    }

    #[allow(dead_code)]
    pub const fn duplicate_registration_rejected(&self) -> bool {
        self.duplicate_registration_rejected
    }

    #[allow(dead_code)]
    pub const fn unmapped_registration_rejected(&self) -> bool {
        self.unmapped_registration_rejected
    }

    pub const fn action_count(&self) -> usize {
        self.action_count
    }

    pub fn action_for_logical_irq(&self, logical_irq: LogicalIrq) -> Option<&IrqAction> {
        let mut index = 0usize;
        while index < self.action_count {
            let action = &self.actions[index];
            if action.logical_irq() == logical_irq {
                return Some(action);
            }
            index += 1;
        }
        None
    }

    pub fn has_handler_for_logical_irq(&self, logical_irq: LogicalIrq) -> bool {
        self.action_for_logical_irq(logical_irq)
            .is_some_and(|action| action.state() == State::Ready && action.handler_bound())
    }

    pub fn setup(
        &mut self,
        irq_controller: &IrqController,
        plic_irq_domain: &PlicIrqDomain,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || irq_controller.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || !plic_irq_domain.mapping_table_ready()
            || !plic_irq_domain.enable_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.action_table_ready = true;
        self.owner_irq_core = true;
        self.requires_mapped_logical_irq = true;
        self.duplicate_policy_ready = true;
        self.unmapped_reject_ready = true;
        self.hardirq_context_guard_ready = true;
        self.source_enable_deferred = true;
        self.dispatch_ready = true;
        self.dispatch_requires_hardirq_context = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IrqHandlerRegistryReady,
        )
    }

    pub fn request_irq(
        &mut self,
        plic_irq_domain: &PlicIrqDomain,
        logical_irq: LogicalIrq,
        device: DeviceRef,
        handler_kind: IrqHandlerKind,
    ) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.action_table_ready
            || !self.owner_irq_core
            || !self.requires_mapped_logical_irq
            || !self.duplicate_policy_ready
            || !self.unmapped_reject_ready
            || !self.hardirq_context_guard_ready
            || !logical_irq.is_valid()
            || handler_kind == IrqHandlerKind::None
        {
            return false;
        }
        if plic_irq_domain
            .mapping_for_logical_irq(logical_irq)
            .is_none()
        {
            self.unmapped_registration_rejected = true;
            return false;
        }
        if self.action_for_logical_irq(logical_irq).is_some() {
            self.duplicate_registration_rejected = true;
            return false;
        }
        if self.action_count >= IRQ_ACTION_CAPACITY {
            return false;
        }

        if !super::plic_provider::record_irq_action_request(logical_irq, device, handler_kind) {
            return false;
        }

        let index = self.action_count;
        if self.actions[index]
            .setup_from_request(logical_irq, device, handler_kind)
            .is_err()
        {
            return false;
        }
        self.action_count += 1;
        true
    }

    pub fn dispatch(&self, logical_irq: LogicalIrq) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.dispatch_ready
            || !self.dispatch_requires_hardirq_context
            || !logical_irq.is_valid()
        {
            return false;
        }
        let Some(action) = self.action_for_logical_irq(logical_irq) else {
            return false;
        };
        if action.state() != State::Ready
            || !action.handler_bound()
            || !action.hardirq_context_required()
            || !action.mapped_irq_required()
            || !action.dispatch_ready()
        {
            return false;
        }

        let handler_kind = action.handler_kind();
        match handler_kind {
            IrqHandlerKind::Ns16550aUart => crate::objects::ns16550a::handle_uart_irq(),
            IrqHandlerKind::VirtioMmio => crate::objects::virtio_mmio::handle_virtio_mmio_irq(),
            IrqHandlerKind::None => return false,
        }
        self.dispatch_calls.fetch_add(1, Ordering::AcqRel);
        true
    }
}

pub struct IrqChipInitTable {
    lifecycle: Lifecycle,
    static_entries_ready: bool,
    lds_section_ready: bool,
    entry_view_ready: bool,
    interrupt_controller_scan_ready: bool,
    parent_first_order_ready: bool,
    init_irq_called_irqchip_init: bool,
    irqchip_init_called_of_irq_init: bool,
    of_irq_init_traversed_lds_section: bool,
    plic_callback_invoked: bool,
    entry_count: usize,
    run_records: [IrqChipInitRunRecord; IRQCHIP_RUN_RECORD_CAPACITY],
    run_count: usize,
}

impl IrqChipInitTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            static_entries_ready: false,
            lds_section_ready: false,
            entry_view_ready: false,
            interrupt_controller_scan_ready: false,
            parent_first_order_ready: false,
            init_irq_called_irqchip_init: false,
            irqchip_init_called_of_irq_init: false,
            of_irq_init_traversed_lds_section: false,
            plic_callback_invoked: false,
            entry_count: 0,
            run_records: [IrqChipInitRunRecord::empty(); IRQCHIP_RUN_RECORD_CAPACITY],
            run_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn static_entries_ready(&self) -> bool {
        self.static_entries_ready
    }

    pub const fn lds_section_ready(&self) -> bool {
        self.lds_section_ready
    }

    pub const fn entry_view_ready(&self) -> bool {
        self.entry_view_ready
    }

    pub const fn interrupt_controller_scan_ready(&self) -> bool {
        self.interrupt_controller_scan_ready
    }

    pub const fn parent_first_order_ready(&self) -> bool {
        self.parent_first_order_ready
    }

    pub const fn init_irq_called_irqchip_init(&self) -> bool {
        self.init_irq_called_irqchip_init
    }

    pub const fn irqchip_init_called_of_irq_init(&self) -> bool {
        self.irqchip_init_called_of_irq_init
    }

    pub const fn of_irq_init_traversed_lds_section(&self) -> bool {
        self.of_irq_init_traversed_lds_section
    }

    pub const fn plic_callback_invoked(&self) -> bool {
        self.plic_callback_invoked
    }

    pub const fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub const fn run_count(&self) -> usize {
        self.run_count
    }

    pub fn run_record(&self, index: usize) -> Option<IrqChipInitRunRecord> {
        if index >= self.run_count {
            return None;
        }
        Some(self.run_records[index])
    }

    pub fn contains_entry(&self, compatible: &[u8]) -> bool {
        let entries = irqchip_init_range();
        let mut index = 0usize;
        while index < entries.len() {
            if entries[index].compatible() == compatible {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn preset(
        &mut self,
        irq_controller: &IrqController,
        device_tree: &DeviceTree,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || irq_controller.state() != State::Ready
            || device_tree.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let entries = irqchip_init_range();
        if entries.is_empty()
            || !irqchip_init_range_valid(entries)
            || !entries
                .iter()
                .any(|entry| entry.compatible() == PLIC_COMPATIBLE_SIFIVE)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.static_entries_ready = true;
        self.lds_section_ready = true;
        self.entry_view_ready = true;
        self.interrupt_controller_scan_ready = true;
        self.parent_first_order_ready = true;
        self.entry_count = entries.len();
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::IrqChipInitTablePrepared,
        )
    }

    pub fn setup(
        &mut self,
        plic_driver: &PlicDriver,
        device_tree: &DeviceTree,
        riscv_intc: &RiscvIntc,
        cpu_group: &CpuGroup,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic: &mut Plic,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || plic_driver.state() != State::Prepared
            || device_tree.state() != State::Ready
            || riscv_intc.state() != State::Ready
            || cpu_group.state() != State::Ready
            || vmalloc_allocator.state() != State::Ready
            || page_table_caches.state() != State::Ready
            || page_allocator.state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || config.state() != State::Online
            || ioremap.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.init_irq_called_irqchip_init = true;
        self.irqchip_init_called_of_irq_init = true;
        let result = self.of_irq_init(
            plic_driver,
            device_tree,
            riscv_intc,
            cpu_group,
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            ioremap,
            plic,
        );
        if result.is_err() || !self.plic_callback_invoked {
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
            Checkpoint::IrqChipInitTableReady,
        )
    }

    fn of_irq_init(
        &mut self,
        plic_driver: &PlicDriver,
        device_tree: &DeviceTree,
        riscv_intc: &RiscvIntc,
        cpu_group: &CpuGroup,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic: &mut Plic,
    ) -> EventResult {
        let entries = irqchip_init_range();
        if entries.len() != self.entry_count || !irqchip_init_range_valid(entries) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.of_irq_init_traversed_lds_section = true;
        let Some(node) = find_plic_interrupt_controller_node(device_tree) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        };

        let mut index = 0usize;
        while index < entries.len() {
            let entry = &entries[index];
            let matched = node.has_compatible(entry.compatible());
            if self.run_count < IRQCHIP_RUN_RECORD_CAPACITY {
                self.run_records[self.run_count] = IrqChipInitRunRecord {
                    name: entry.name(),
                    compatible: entry.compatible(),
                    matched,
                    callback_invoked: false,
                    return_ok: false,
                };
                self.run_count += 1;
            }
            if matched {
                if !plic_driver.entry_registered()
                    || !plic_driver.init_callback_bound()
                    || !plic_driver.compatible_covers_qemu_virt()
                {
                    return failed_condition(
                        LifecycleEvent::Setup,
                        self.lifecycle.state(),
                        State::Prepared,
                        State::Ready,
                    );
                }
                let result = entry.run(
                    plic,
                    node,
                    riscv_intc,
                    cpu_group,
                    vmalloc_allocator,
                    page_table_caches,
                    page_allocator,
                    page_metadata_map,
                    config,
                    ioremap,
                );
                let return_ok = result.is_ok();
                if let Some(record) = self.run_records.get_mut(self.run_count - 1) {
                    record.callback_invoked = true;
                    record.return_ok = return_ok;
                }
                if return_ok {
                    self.plic_callback_invoked = true;
                    return Ok(());
                }
                return result;
            }
            index += 1;
        }

        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }
}

pub struct PlicDriver {
    lifecycle: Lifecycle,
    entry_registered: bool,
    registered_in_lds_section: bool,
    init_callback_bound: bool,
    compatible_covers_qemu_virt: bool,
    probe_depends_on_device_tree: bool,
    probe_runs_in_irq_time_init: bool,
    not_platform_bus_probe: bool,
}

impl PlicDriver {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            entry_registered: false,
            registered_in_lds_section: false,
            init_callback_bound: false,
            compatible_covers_qemu_virt: false,
            probe_depends_on_device_tree: false,
            probe_runs_in_irq_time_init: false,
            not_platform_bus_probe: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn entry_registered(&self) -> bool {
        self.entry_registered
    }

    pub const fn registered_in_lds_section(&self) -> bool {
        self.registered_in_lds_section
    }

    pub const fn init_callback_bound(&self) -> bool {
        self.init_callback_bound
    }

    pub const fn compatible_covers_qemu_virt(&self) -> bool {
        self.compatible_covers_qemu_virt
    }

    pub const fn probe_depends_on_device_tree(&self) -> bool {
        self.probe_depends_on_device_tree
    }

    pub const fn probe_runs_in_irq_time_init(&self) -> bool {
        self.probe_runs_in_irq_time_init
    }

    pub const fn not_platform_bus_probe(&self) -> bool {
        self.not_platform_bus_probe
    }

    pub fn preset(
        &mut self,
        irqchip_table: &IrqChipInitTable,
        device_tree: &DeviceTree,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || irqchip_table.state() != State::Prepared
            || device_tree.state() != State::Ready
            || !irqchip_table.contains_entry(PLIC_COMPATIBLE_SIFIVE)
            || find_plic_interrupt_controller_node(device_tree).is_none()
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.entry_registered = true;
        self.registered_in_lds_section = true;
        self.init_callback_bound = true;
        self.compatible_covers_qemu_virt = true;
        self.probe_depends_on_device_tree = true;
        self.probe_runs_in_irq_time_init = true;
        self.not_platform_bus_probe = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PlicDriverPrepared,
        )
    }
}

pub struct RiscvIntc {
    lifecycle: Lifecycle,
    domain_ready: bool,
    boot_cpu_local_causes_ready: bool,
    timer_pin_ready: bool,
    software_pin_ready: bool,
    external_pin_ready: bool,
    boot_cpu_timer_irq_ready: bool,
    boot_cpu_software_irq_ready: bool,
    boot_cpu_external_irq_reserved: bool,
}

impl RiscvIntc {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            domain_ready: false,
            boot_cpu_local_causes_ready: false,
            timer_pin_ready: false,
            software_pin_ready: false,
            external_pin_ready: false,
            boot_cpu_timer_irq_ready: false,
            boot_cpu_software_irq_ready: false,
            boot_cpu_external_irq_reserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn domain_ready(&self) -> bool {
        self.domain_ready
    }

    pub const fn boot_cpu_local_causes_ready(&self) -> bool {
        self.boot_cpu_local_causes_ready
    }

    pub const fn timer_pin_ready(&self) -> bool {
        self.timer_pin_ready
    }

    pub const fn software_pin_ready(&self) -> bool {
        self.software_pin_ready
    }

    pub const fn external_pin_ready(&self) -> bool {
        self.external_pin_ready
    }

    pub const fn boot_cpu_timer_irq_ready(&self) -> bool {
        self.boot_cpu_timer_irq_ready
    }

    pub const fn boot_cpu_software_irq_ready(&self) -> bool {
        self.boot_cpu_software_irq_ready
    }

    pub const fn boot_cpu_external_irq_reserved(&self) -> bool {
        self.boot_cpu_external_irq_reserved
    }

    pub fn setup(
        &mut self,
        device_tree: &DeviceTree,
        irq_controller: &IrqController,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || irq_controller.state() != State::Ready
            || cpu_group.state() != State::Ready
            || device_tree.find_node(b"/cpus").is_none()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.domain_ready = true;
        self.boot_cpu_local_causes_ready = true;
        self.timer_pin_ready = true;
        self.software_pin_ready = true;
        self.external_pin_ready = true;
        self.boot_cpu_timer_irq_ready = true;
        self.boot_cpu_software_irq_ready = true;
        self.boot_cpu_external_irq_reserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RiscvIntcReady,
        )
    }
}

pub struct IrqDispatchTree {
    lifecycle: Lifecycle,
    fallback_route_ready: bool,
    timer_route_ready: bool,
    software_route_reserved: bool,
    external_route_ready: bool,
    external_route_uses_plic_chained_handler: bool,
    external_route_uses_plic_irq_domain: bool,
    external_route_claims_before_dispatch: bool,
    external_route_completes_after_handler: bool,
    boot_cpu_route_ready: bool,
}

impl IrqDispatchTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fallback_route_ready: false,
            timer_route_ready: false,
            software_route_reserved: false,
            external_route_ready: false,
            external_route_uses_plic_chained_handler: false,
            external_route_uses_plic_irq_domain: false,
            external_route_claims_before_dispatch: false,
            external_route_completes_after_handler: false,
            boot_cpu_route_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fallback_route_ready(&self) -> bool {
        self.fallback_route_ready
    }

    pub const fn timer_route_ready(&self) -> bool {
        self.timer_route_ready
    }

    pub const fn software_route_reserved(&self) -> bool {
        self.software_route_reserved
    }

    pub const fn external_route_ready(&self) -> bool {
        self.external_route_ready
    }

    pub const fn external_route_uses_plic_chained_handler(&self) -> bool {
        self.external_route_uses_plic_chained_handler
    }

    pub const fn external_route_uses_plic_irq_domain(&self) -> bool {
        self.external_route_uses_plic_irq_domain
    }

    pub const fn external_route_claims_before_dispatch(&self) -> bool {
        self.external_route_claims_before_dispatch
    }

    pub const fn external_route_completes_after_handler(&self) -> bool {
        self.external_route_completes_after_handler
    }

    pub const fn boot_cpu_route_ready(&self) -> bool {
        self.boot_cpu_route_ready
    }

    pub fn setup(
        &mut self,
        irq_controller: &IrqController,
        riscv_intc: &RiscvIntc,
        interrupt_stream: &mut InterruptStream,
        cpu_group: &CpuGroup,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || irq_controller.state() != State::Ready
            || riscv_intc.state() != State::Ready
            || interrupt_stream.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !riscv_intc.boot_cpu_timer_irq_ready()
            || !riscv_intc.boot_cpu_external_irq_reserved()
            || plic.state() != State::Ready
            || !plic.chained_handler_ready()
            || !plic.claim_action_ready()
            || !plic.complete_action_ready()
            || plic_irq_domain.state() != State::Ready
            || !plic_irq_domain.dispatch_ops_ready()
            || irq_handler_registry.state() != State::Ready
            || !irq_handler_registry.dispatch_ready()
            || !irq_handler_registry.dispatch_requires_hardirq_context()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        interrupt_stream.bind_timer_handler()?;
        interrupt_stream.bind_external_handler()?;
        self.fallback_route_ready = true;
        self.timer_route_ready = true;
        self.software_route_reserved = true;
        self.external_route_ready = true;
        self.external_route_uses_plic_chained_handler = true;
        self.external_route_uses_plic_irq_domain = true;
        self.external_route_claims_before_dispatch = true;
        self.external_route_completes_after_handler = true;
        self.boot_cpu_route_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IrqDispatchTreeReady,
        )
    }
}

pub struct Tick {
    lifecycle: Lifecycle,
    broadcast: TickBroadcast,
    control_ready: bool,
    nohz_trimmed: bool,
    boot_cpu_tick_device_ready: bool,
}

impl Tick {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            broadcast: TickBroadcast::new(),
            control_ready: false,
            nohz_trimmed: false,
            boot_cpu_tick_device_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn broadcast(&self) -> &TickBroadcast {
        &self.broadcast
    }

    pub const fn control_ready(&self) -> bool {
        self.control_ready
    }

    pub const fn nohz_trimmed(&self) -> bool {
        self.nohz_trimmed
    }

    pub const fn boot_cpu_tick_device_ready(&self) -> bool {
        self.boot_cpu_tick_device_ready
    }

    pub fn preset(&mut self, cpu_group: &CpuGroup, per_cpu_storage: &PerCpuStorage) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.broadcast.preset()?;
        self.control_ready = true;
        self.nohz_trimmed = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TickPrepared,
        )
    }

    pub fn setup(&mut self, hrtimer_core: &HrtimerCore, timer: &RiscvTimerProvider) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || hrtimer_core.state() != State::Ready
            || timer.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.broadcast.setup(hrtimer_core, timer)?;
        self.boot_cpu_tick_device_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::TickReady,
        )
    }
}

pub struct TickBroadcast {
    lifecycle: Lifecycle,
    masks_ready: bool,
    clockevent_ready: bool,
}

impl TickBroadcast {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            masks_ready: false,
            clockevent_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn masks_ready(&self) -> bool {
        self.masks_ready
    }

    pub const fn clockevent_ready(&self) -> bool {
        self.clockevent_ready
    }

    fn preset(&mut self) -> EventResult {
        self.masks_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TickBroadcastPrepared,
        )
    }

    fn setup(&mut self, hrtimer_core: &HrtimerCore, timer: &RiscvTimerProvider) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || hrtimer_core.state() != State::Ready
            || timer.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.clockevent_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::TickBroadcastReady,
        )
    }
}

pub struct TimerWheel {
    lifecycle: Lifecycle,
    cpu_timer_bases_ready: bool,
    posix_cpu_timer_work_ready: bool,
    timer_softirq_registered: bool,
}

impl TimerWheel {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cpu_timer_bases_ready: false,
            posix_cpu_timer_work_ready: false,
            timer_softirq_registered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cpu_timer_bases_ready(&self) -> bool {
        self.cpu_timer_bases_ready
    }

    pub const fn posix_cpu_timer_work_ready(&self) -> bool {
        self.posix_cpu_timer_work_ready
    }

    pub const fn timer_softirq_registered(&self) -> bool {
        self.timer_softirq_registered
    }

    pub fn setup(
        &mut self,
        per_cpu_storage: &PerCpuStorage,
        cpu_group: &CpuGroup,
        softirq: &mut Softirq,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || per_cpu_storage.state() != State::Ready
            || cpu_group.state() != State::Ready
            || softirq.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        softirq.register_timer_action()?;
        self.cpu_timer_bases_ready = true;
        self.posix_cpu_timer_work_ready = true;
        self.timer_softirq_registered = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::TimerWheelReady,
        )
    }
}

pub struct HrtimerCore {
    lifecycle: Lifecycle,
    boot_cpu_base_ready: bool,
    hrtimer_softirq_registered: bool,
}

impl HrtimerCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_cpu_base_ready: false,
            hrtimer_softirq_registered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn boot_cpu_base_ready(&self) -> bool {
        self.boot_cpu_base_ready
    }

    pub const fn hrtimer_softirq_registered(&self) -> bool {
        self.hrtimer_softirq_registered
    }

    pub fn setup(
        &mut self,
        per_cpu_storage: &PerCpuStorage,
        cpu_group: &CpuGroup,
        softirq: &mut Softirq,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || per_cpu_storage.state() != State::Ready
            || cpu_group.state() != State::Ready
            || softirq.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        softirq.register_hrtimer_action()?;
        self.boot_cpu_base_ready = true;
        self.hrtimer_softirq_registered = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::HrtimerCoreReady,
        )
    }
}

pub struct Timekeeper {
    lifecycle: Lifecycle,
    clocksource_core: ClocksourceCore,
    jiffies_clocksource: JiffiesClocksource,
    wall_time_ready: bool,
    monotonic_time_ready: bool,
    raw_time_ready: bool,
}

impl Timekeeper {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            clocksource_core: ClocksourceCore::new(),
            jiffies_clocksource: JiffiesClocksource::new(),
            wall_time_ready: false,
            monotonic_time_ready: false,
            raw_time_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn clocksource_core(&self) -> &ClocksourceCore {
        &self.clocksource_core
    }

    pub const fn jiffies_clocksource(&self) -> &JiffiesClocksource {
        &self.jiffies_clocksource
    }

    pub const fn wall_time_ready(&self) -> bool {
        self.wall_time_ready
    }

    pub const fn monotonic_time_ready(&self) -> bool {
        self.monotonic_time_ready
    }

    pub const fn raw_time_ready(&self) -> bool {
        self.raw_time_ready
    }

    pub fn setup(&mut self, tick: &Tick, static_branch: &StaticBranch) -> EventResult {
        if self.lifecycle.state() != State::Base
            || tick.state() != State::Prepared
            || static_branch.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.clocksource_core.preset()?;
        self.jiffies_clocksource.preset()?;
        self.wall_time_ready = true;
        self.monotonic_time_ready = true;
        self.raw_time_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::TimekeeperReady,
        )
    }
}

pub struct ClocksourceCore {
    lifecycle: Lifecycle,
    registry_ready: bool,
    riscv_clocksource_registered: bool,
}

impl ClocksourceCore {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registry_ready: false,
            riscv_clocksource_registered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn registry_ready(&self) -> bool {
        self.registry_ready
    }

    pub const fn riscv_clocksource_registered(&self) -> bool {
        self.riscv_clocksource_registered
    }

    fn preset(&mut self) -> EventResult {
        self.registry_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::ClocksourceCorePrepared,
        )
    }

    fn register_riscv_clocksource(&mut self) {
        self.riscv_clocksource_registered = true;
    }
}

pub struct JiffiesClocksource {
    lifecycle: Lifecycle,
    available: bool,
}

impl JiffiesClocksource {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            available: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn available(&self) -> bool {
        self.available
    }

    fn preset(&mut self) -> EventResult {
        self.available = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::JiffiesClocksourcePrepared,
        )
    }
}

pub struct RiscvTimerProvider {
    lifecycle: Lifecycle,
    timebase_hz: u64,
    clocksource_registered: bool,
    clockevent_registered: bool,
    irq_mapping_ready: bool,
    sbi_programming_ready: bool,
}

impl RiscvTimerProvider {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            timebase_hz: 0,
            clocksource_registered: false,
            clockevent_registered: false,
            irq_mapping_ready: false,
            sbi_programming_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn timebase_hz(&self) -> u64 {
        self.timebase_hz
    }

    pub const fn clocksource_registered(&self) -> bool {
        self.clocksource_registered
    }

    pub const fn clockevent_registered(&self) -> bool {
        self.clockevent_registered
    }

    pub const fn irq_mapping_ready(&self) -> bool {
        self.irq_mapping_ready
    }

    pub const fn sbi_programming_ready(&self) -> bool {
        self.sbi_programming_ready
    }

    pub fn setup(
        &mut self,
        device_tree: &DeviceTree,
        riscv_intc: &RiscvIntc,
        irq_dispatch_tree: &IrqDispatchTree,
        timekeeper: &mut Timekeeper,
        hrtimer_core: &HrtimerCore,
        tick: &Tick,
        sbi: &Sbi,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || riscv_intc.state() != State::Ready
            || irq_dispatch_tree.state() != State::Ready
            || timekeeper.state() != State::Ready
            || hrtimer_core.state() != State::Ready
            || tick.state() != State::Prepared
            || sbi.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.timebase_hz = read_timebase_frequency(device_tree).unwrap_or(DEFAULT_TIMEBASE_HZ);
        timekeeper.clocksource_core.register_riscv_clocksource();
        self.clocksource_registered = true;
        self.clockevent_registered = true;
        self.irq_mapping_ready =
            riscv_intc.boot_cpu_timer_irq_ready() && irq_dispatch_tree.timer_route_ready();
        self.sbi_programming_ready = true;
        if self.timebase_hz == 0 || !self.irq_mapping_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RiscvTimerProviderReady,
        )
    }

    pub fn read_time(&self) -> Option<u64> {
        if self.lifecycle.state() != State::Ready {
            return None;
        }

        Some(riscv64::sbi::read_time())
    }

    pub fn schedule_oneshot(&self, delta_ticks: u64, callback: ClockEventCallback) -> Option<u64> {
        if self.lifecycle.state() != State::Ready
            || delta_ticks == 0
            || ONESHOT_CALLBACK.load(Ordering::Acquire) != 0
        {
            return None;
        }

        let now = riscv64::sbi::read_time();
        let deadline = now.wrapping_add(delta_ticks);
        ONESHOT_DEADLINE.store(deadline, Ordering::Relaxed);
        ONESHOT_CALLBACK.store(callback as usize, Ordering::Release);
        riscv64::csr::enable_supervisor_timer_interrupt();
        riscv64::sbi::set_timer(deadline);
        Some(deadline)
    }
}

pub struct SbiIpi {
    lifecycle: Lifecycle,
    irq_mapping_ready: bool,
    send_action_ready: bool,
    enable_deferred: bool,
}

impl SbiIpi {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            irq_mapping_ready: false,
            send_action_ready: false,
            enable_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn irq_mapping_ready(&self) -> bool {
        self.irq_mapping_ready
    }

    pub const fn send_action_ready(&self) -> bool {
        self.send_action_ready
    }

    pub const fn enable_deferred(&self) -> bool {
        self.enable_deferred
    }

    pub fn setup(
        &mut self,
        sbi: &Sbi,
        riscv_intc: &RiscvIntc,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || sbi.state() != State::Ready
            || riscv_intc.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !riscv_intc.boot_cpu_software_irq_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.irq_mapping_ready = true;
        self.send_action_ready = true;
        self.enable_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SbiIpiReady,
        )
    }
}

pub struct IpiMux {
    lifecycle: Lifecycle,
    domain_ready: bool,
    per_cpu_bits_ready: bool,
    virtual_ipi_range_ready: bool,
    parent_software_irq_ready: bool,
    secondary_enable_deferred: bool,
}

impl IpiMux {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            domain_ready: false,
            per_cpu_bits_ready: false,
            virtual_ipi_range_ready: false,
            parent_software_irq_ready: false,
            secondary_enable_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn domain_ready(&self) -> bool {
        self.domain_ready
    }

    pub const fn per_cpu_bits_ready(&self) -> bool {
        self.per_cpu_bits_ready
    }

    pub const fn virtual_ipi_range_ready(&self) -> bool {
        self.virtual_ipi_range_ready
    }

    pub const fn parent_software_irq_ready(&self) -> bool {
        self.parent_software_irq_ready
    }

    pub const fn secondary_enable_deferred(&self) -> bool {
        self.secondary_enable_deferred
    }

    pub fn setup(
        &mut self,
        sbi_ipi: &SbiIpi,
        per_cpu_storage: &PerCpuStorage,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || sbi_ipi.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !sbi_ipi.send_action_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.domain_ready = true;
        self.per_cpu_bits_ready = true;
        self.virtual_ipi_range_ready = true;
        self.parent_software_irq_ready = true;
        self.secondary_enable_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IpiMuxReady,
        )
    }
}

pub struct Plic {
    lifecycle: Lifecycle,
    matched_compatible: bool,
    setup_called_by_of_irq_init: bool,
    interrupt_controller_node_ready: bool,
    provider_discovery_reserved: bool,
    external_parent_reserved: bool,
    output_connected_to_riscv_intc_external_input: bool,
    mmio_resource_ready: bool,
    ioremapped: bool,
    vm_ioremap: bool,
    mapbase: usize,
    mapsize: usize,
    membase: usize,
    source_count: u32,
    context_id: usize,
    claim_addr: usize,
    enable_addr: usize,
    threshold_addr: usize,
    external_input_context_ready: bool,
    threshold_ready: bool,
    priority_ready: bool,
    source_enable_ready: bool,
    chained_handler_ready: bool,
    claim_action_ready: bool,
    complete_action_ready: bool,
    claim_reads_claim_register: bool,
    claim_zero_means_no_pending: bool,
    complete_writes_claimed_source: bool,
    claim_loop_until_zero: bool,
    zero_claim_stops_dispatch: bool,
    completes_each_claimed_source: bool,
    claim_before_dispatch: bool,
    complete_after_handler: bool,
    uart_source_trigger_deferred: bool,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    claim_count: AtomicUsize,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    zero_claim_count: AtomicUsize,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    dispatch_count: AtomicUsize,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    complete_count: AtomicUsize,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    loop_exit_count: AtomicUsize,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    last_claimed_source: AtomicU32,
    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    last_completed_source: AtomicU32,
}

impl Plic {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            matched_compatible: false,
            setup_called_by_of_irq_init: false,
            interrupt_controller_node_ready: false,
            provider_discovery_reserved: false,
            external_parent_reserved: false,
            output_connected_to_riscv_intc_external_input: false,
            mmio_resource_ready: false,
            ioremapped: false,
            vm_ioremap: false,
            mapbase: 0,
            mapsize: 0,
            membase: 0,
            source_count: 0,
            context_id: 0,
            claim_addr: 0,
            enable_addr: 0,
            threshold_addr: 0,
            external_input_context_ready: false,
            threshold_ready: false,
            priority_ready: false,
            source_enable_ready: false,
            chained_handler_ready: false,
            claim_action_ready: false,
            complete_action_ready: false,
            claim_reads_claim_register: false,
            claim_zero_means_no_pending: false,
            complete_writes_claimed_source: false,
            claim_loop_until_zero: false,
            zero_claim_stops_dispatch: false,
            completes_each_claimed_source: false,
            claim_before_dispatch: false,
            complete_after_handler: false,
            uart_source_trigger_deferred: false,
            claim_count: AtomicUsize::new(0),
            zero_claim_count: AtomicUsize::new(0),
            dispatch_count: AtomicUsize::new(0),
            complete_count: AtomicUsize::new(0),
            loop_exit_count: AtomicUsize::new(0),
            last_claimed_source: AtomicU32::new(0),
            last_completed_source: AtomicU32::new(0),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn matched_compatible(&self) -> bool {
        self.matched_compatible
    }

    pub const fn setup_called_by_of_irq_init(&self) -> bool {
        self.setup_called_by_of_irq_init
    }

    pub const fn interrupt_controller_node_ready(&self) -> bool {
        self.interrupt_controller_node_ready
    }

    pub const fn provider_discovery_reserved(&self) -> bool {
        self.provider_discovery_reserved
    }

    pub const fn external_parent_reserved(&self) -> bool {
        self.external_parent_reserved
    }

    pub const fn output_connected_to_riscv_intc_external_input(&self) -> bool {
        self.output_connected_to_riscv_intc_external_input
    }

    pub const fn mmio_resource_ready(&self) -> bool {
        self.mmio_resource_ready
    }

    pub const fn ioremapped(&self) -> bool {
        self.ioremapped
    }

    pub const fn vm_ioremap(&self) -> bool {
        self.vm_ioremap
    }

    pub const fn mapbase(&self) -> usize {
        self.mapbase
    }

    pub const fn mapsize(&self) -> usize {
        self.mapsize
    }

    pub const fn membase(&self) -> usize {
        self.membase
    }

    pub const fn source_count(&self) -> u32 {
        self.source_count
    }

    #[allow(dead_code)]
    pub const fn context_id(&self) -> usize {
        self.context_id
    }

    #[allow(dead_code)]
    pub const fn claim_addr(&self) -> usize {
        self.claim_addr
    }

    #[allow(dead_code)]
    pub const fn enable_addr(&self) -> usize {
        self.enable_addr
    }

    #[allow(dead_code)]
    pub const fn threshold_addr(&self) -> usize {
        self.threshold_addr
    }

    pub const fn external_input_context_ready(&self) -> bool {
        self.external_input_context_ready
    }

    pub const fn threshold_ready(&self) -> bool {
        self.threshold_ready
    }

    pub const fn priority_ready(&self) -> bool {
        self.priority_ready
    }

    pub const fn source_enable_ready(&self) -> bool {
        self.source_enable_ready
    }

    pub const fn chained_handler_ready(&self) -> bool {
        self.chained_handler_ready
    }

    pub const fn claim_action_ready(&self) -> bool {
        self.claim_action_ready
    }

    pub const fn complete_action_ready(&self) -> bool {
        self.complete_action_ready
    }

    pub const fn claim_reads_claim_register(&self) -> bool {
        self.claim_reads_claim_register
    }

    pub const fn claim_zero_means_no_pending(&self) -> bool {
        self.claim_zero_means_no_pending
    }

    pub const fn complete_writes_claimed_source(&self) -> bool {
        self.complete_writes_claimed_source
    }

    pub const fn claim_loop_until_zero(&self) -> bool {
        self.claim_loop_until_zero
    }

    pub const fn zero_claim_stops_dispatch(&self) -> bool {
        self.zero_claim_stops_dispatch
    }

    pub const fn completes_each_claimed_source(&self) -> bool {
        self.completes_each_claimed_source
    }

    pub const fn claim_before_dispatch(&self) -> bool {
        self.claim_before_dispatch
    }

    pub const fn complete_after_handler(&self) -> bool {
        self.complete_after_handler
    }

    pub const fn uart_source_trigger_deferred(&self) -> bool {
        self.uart_source_trigger_deferred
    }

    #[allow(dead_code)]
    pub fn claim_count(&self) -> usize {
        super::plic_provider::claim_count(self)
    }

    #[allow(dead_code)]
    pub fn zero_claim_count(&self) -> usize {
        super::plic_provider::zero_claim_count(self)
    }

    #[allow(dead_code)]
    pub fn dispatch_count(&self) -> usize {
        super::plic_provider::dispatch_count(self)
    }

    #[allow(dead_code)]
    pub fn complete_count(&self) -> usize {
        super::plic_provider::complete_count(self)
    }

    #[allow(dead_code)]
    pub fn loop_exit_count(&self) -> usize {
        super::plic_provider::loop_exit_count(self)
    }

    #[allow(dead_code)]
    pub fn last_claimed_source(&self) -> u32 {
        super::plic_provider::last_claimed_source(self)
    }

    #[allow(dead_code)]
    pub fn last_completed_source(&self) -> u32 {
        super::plic_provider::last_completed_source(self)
    }

    fn preset_from_irqchip(
        &mut self,
        node: DeviceNodeRef<'_>,
        riscv_intc: &RiscvIntc,
        cpu_group: &CpuGroup,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || riscv_intc.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !riscv_intc.boot_cpu_external_irq_reserved()
            || !node.property(b"interrupt-controller").is_some()
            || !plic_node_matches_supported_compatible(node)
            || !plic_context_parent_has_external_input(node, cpu_group)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        let Some(resource) = plic_mmio_resource(node) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        let Some(source_count) = plic_source_count(node) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        if resource.size == 0 || source_count == 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        let Some(context_id) = plic_external_context_index(node, cpu_group) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        let Some(mapping) = ioremap.map_system_irqchip_mmio(
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            "plic",
            resource.base,
            resource.size,
        ) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };

        if !mapping.owner_is_system_irqchip()
            || mapping.phys_base() != resource.base
            || mapping.size() != resource.size
            || !mapping.uses_vm_ioremap()
            || !mapping.uses_io_page_protection()
            || ioremap.mapping_for_system_irqchip("plic") != Some(mapping)
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        let Some(claim_addr) = plic_context_claim_addr(mapping.membase(), context_id) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        let Some(enable_addr) = plic_context_enable_addr(mapping.membase(), context_id) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        let Some(threshold_addr) = plic_context_threshold_addr(mapping.membase(), context_id)
        else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };

        self.matched_compatible = true;
        self.setup_called_by_of_irq_init = true;
        self.interrupt_controller_node_ready = true;
        self.provider_discovery_reserved = true;
        self.external_parent_reserved = true;
        self.output_connected_to_riscv_intc_external_input = true;
        self.mmio_resource_ready = true;
        self.ioremapped = mapping.membase_cookie_ready();
        self.vm_ioremap = mapping.uses_vm_ioremap();
        self.mapbase = resource.base;
        self.mapsize = resource.size;
        self.membase = mapping.membase();
        self.source_count = source_count;
        self.context_id = context_id;
        self.claim_addr = claim_addr;
        self.enable_addr = enable_addr;
        self.threshold_addr = threshold_addr;
        self.external_input_context_ready = true;
        self.threshold_ready = true;
        self.priority_ready = true;
        self.source_enable_ready = true;
        self.chained_handler_ready = true;
        self.claim_action_ready = true;
        self.complete_action_ready = true;
        self.claim_reads_claim_register = true;
        self.claim_zero_means_no_pending = true;
        self.complete_writes_claimed_source = true;
        self.claim_loop_until_zero = true;
        self.zero_claim_stops_dispatch = true;
        self.completes_each_claimed_source = true;
        self.claim_before_dispatch = true;
        self.complete_after_handler = true;
        self.uart_source_trigger_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::PlicReady,
        )
    }

    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    pub(crate) fn native_handle_external_interrupt(
        &self,
        domain: &PlicIrqDomain,
        registry: &IrqHandlerRegistry,
    ) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.chained_handler_ready
            || !self.claim_action_ready
            || !self.complete_action_ready
            || !self.claim_loop_until_zero
            || !self.zero_claim_stops_dispatch
            || !self.completes_each_claimed_source
            || !self.claim_before_dispatch
            || !self.complete_after_handler
            || domain.state() != State::Ready
            || !domain.dispatch_ops_ready()
            || registry.state() != State::Ready
            || !registry.dispatch_ready()
        {
            return false;
        }

        let mut any_dispatched = false;
        loop {
            let source = self.claim();
            if source == 0 {
                self.loop_exit_count.fetch_add(1, Ordering::AcqRel);
                break;
            }

            let dispatched = domain
                .resolve_hwirq(source)
                .is_some_and(|logical_irq| registry.dispatch(logical_irq));
            if dispatched {
                self.dispatch_count.fetch_add(1, Ordering::AcqRel);
                any_dispatched = true;
            }
            self.complete(source);
        }

        any_dispatched
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_enable_source(&self, source: u32) -> bool {
        if self.lifecycle.state() != State::Ready
            || !self.source_enable_ready
            || self.enable_addr == 0
            || self.threshold_addr == 0
            || source == 0
            || source > self.source_count
        {
            return false;
        }

        let Some(priority_addr) = plic_priority_addr(self.membase, source) else {
            return false;
        };
        let Some(enable_addr) = plic_source_enable_addr(self.enable_addr, source) else {
            return false;
        };
        let mask = 1u32 << (source % 32);

        unsafe {
            core::ptr::write_volatile(priority_addr as *mut u32, 1);
            let enabled = core::ptr::read_volatile(enable_addr as *const u32);
            core::ptr::write_volatile(enable_addr as *mut u32, enabled | mask);
            core::ptr::write_volatile(self.threshold_addr as *mut u32, 0);
        }

        unsafe { core::ptr::read_volatile(enable_addr as *const u32) & mask != 0 }
    }

    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    fn claim(&self) -> u32 {
        if self.claim_addr == 0 || !self.claim_action_ready {
            return 0;
        }

        let source = unsafe { core::ptr::read_volatile(self.claim_addr as *const u32) };
        if source == 0 {
            self.zero_claim_count.fetch_add(1, Ordering::AcqRel);
            return 0;
        }

        self.claim_count.fetch_add(1, Ordering::AcqRel);
        self.last_claimed_source.store(source, Ordering::Release);
        source
    }

    #[cfg_attr(plic_provider_linux_object, allow(dead_code))]
    fn complete(&self, source: u32) {
        if self.claim_addr == 0 || source == 0 || !self.complete_action_ready {
            return;
        }

        unsafe {
            core::ptr::write_volatile(self.claim_addr as *mut u32, source);
        }
        self.complete_count.fetch_add(1, Ordering::AcqRel);
        self.last_completed_source.store(source, Ordering::Release);
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_claim_count(&self) -> usize {
        self.claim_count.load(Ordering::Acquire)
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_zero_claim_count(&self) -> usize {
        self.zero_claim_count.load(Ordering::Acquire)
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_dispatch_count(&self) -> usize {
        self.dispatch_count.load(Ordering::Acquire)
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_complete_count(&self) -> usize {
        self.complete_count.load(Ordering::Acquire)
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_loop_exit_count(&self) -> usize {
        self.loop_exit_count.load(Ordering::Acquire)
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_last_claimed_source(&self) -> u32 {
        self.last_claimed_source.load(Ordering::Acquire)
    }

    #[cfg(not(plic_provider_linux_object))]
    pub(crate) fn native_last_completed_source(&self) -> u32 {
        self.last_completed_source.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct LogicalIrq {
    value: usize,
}

impl LogicalIrq {
    pub const fn invalid() -> Self {
        Self { value: 0 }
    }

    pub const fn new(value: usize) -> Self {
        Self { value }
    }

    pub const fn is_valid(self) -> bool {
        self.value != 0
    }

    #[cfg_attr(not(plic_provider_linux_object), allow(dead_code))]
    pub const fn as_usize(self) -> usize {
        self.value
    }
}

pub struct PlicIrqMapping {
    lifecycle: Lifecycle,
    source: u32,
    logical_irq: LogicalIrq,
    domain_bound: bool,
    source_valid: bool,
    source_zero_rejected: bool,
    source_range_checked: bool,
    duplicate_source_idempotent: bool,
    source_gate_defined: bool,
    source_gate_closed: bool,
    source_enable_deferred: bool,
    source_enabled: bool,
    handler_registered: bool,
}

impl PlicIrqMapping {
    const fn empty() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            source: 0,
            logical_irq: LogicalIrq::invalid(),
            domain_bound: false,
            source_valid: false,
            source_zero_rejected: false,
            source_range_checked: false,
            duplicate_source_idempotent: false,
            source_gate_defined: false,
            source_gate_closed: false,
            source_enable_deferred: false,
            source_enabled: false,
            handler_registered: false,
        }
    }

    pub const fn source(&self) -> u32 {
        self.source
    }

    pub const fn logical_irq(&self) -> LogicalIrq {
        self.logical_irq
    }

    pub const fn domain_bound(&self) -> bool {
        self.domain_bound
    }

    pub const fn source_valid(&self) -> bool {
        self.source_valid
    }

    pub const fn source_zero_rejected(&self) -> bool {
        self.source_zero_rejected
    }

    pub const fn source_range_checked(&self) -> bool {
        self.source_range_checked
    }

    pub const fn duplicate_source_idempotent(&self) -> bool {
        self.duplicate_source_idempotent
    }

    pub const fn source_gate_defined(&self) -> bool {
        self.source_gate_defined
    }

    pub const fn source_gate_closed(&self) -> bool {
        self.source_gate_closed
    }

    pub const fn source_gate_open(&self) -> bool {
        self.source_gate_defined && !self.source_gate_closed && self.source_enabled
    }

    pub const fn source_enable_deferred(&self) -> bool {
        self.source_enable_deferred
    }

    pub const fn source_not_enabled(&self) -> bool {
        !self.source_enabled
    }

    pub const fn source_enabled(&self) -> bool {
        self.source_enabled
    }

    pub const fn handler_not_registered(&self) -> bool {
        !self.handler_registered
    }

    fn setup_from_domain(
        &mut self,
        source: u32,
        logical_irq: LogicalIrq,
        source_count: u32,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || source == 0
            || source > source_count
            || !logical_irq.is_valid()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.source = source;
        self.logical_irq = logical_irq;
        self.domain_bound = true;
        self.source_valid = true;
        self.source_zero_rejected = true;
        self.source_range_checked = true;
        self.duplicate_source_idempotent = true;
        self.source_gate_defined = true;
        self.source_gate_closed = true;
        self.source_enable_deferred = true;
        self.source_enabled = false;
        self.handler_registered = false;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn enable_source_gate(&mut self, plic: &Plic) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.source_gate_defined
            || !self.source_gate_closed
            || !self.source_enable_deferred
            || self.source_enabled
            || !super::plic_provider::enable_mapped_source(plic, self.source, self.logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.source_gate_closed = false;
        self.source_enable_deferred = false;
        self.source_enabled = true;
        Ok(())
    }
}

pub struct PlicIrqDomain {
    lifecycle: Lifecycle,
    owner_bound: bool,
    hwirq_valid_range_ready: bool,
    logical_irq_allocator_ready: bool,
    mapping_table_ready: bool,
    translate_specifier_ready: bool,
    dispatch_ops_ready: bool,
    source_zero_reserved: bool,
    one_cell_specifier: bool,
    enable_deferred: bool,
    source_count: u32,
    next_logical_irq: usize,
    mappings: [PlicIrqMapping; PLIC_IRQ_MAPPING_CAPACITY],
    mapping_count: usize,
}

impl PlicIrqDomain {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            owner_bound: false,
            hwirq_valid_range_ready: false,
            logical_irq_allocator_ready: false,
            mapping_table_ready: false,
            translate_specifier_ready: false,
            dispatch_ops_ready: false,
            source_zero_reserved: false,
            one_cell_specifier: false,
            enable_deferred: false,
            source_count: 0,
            next_logical_irq: PLIC_LOGICAL_IRQ_BASE,
            mappings: [const { PlicIrqMapping::empty() }; PLIC_IRQ_MAPPING_CAPACITY],
            mapping_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn owner_bound(&self) -> bool {
        self.owner_bound
    }

    pub const fn hwirq_valid_range_ready(&self) -> bool {
        self.hwirq_valid_range_ready
    }

    pub const fn logical_irq_allocator_ready(&self) -> bool {
        self.logical_irq_allocator_ready
    }

    pub const fn mapping_table_ready(&self) -> bool {
        self.mapping_table_ready
    }

    pub const fn translate_specifier_ready(&self) -> bool {
        self.translate_specifier_ready
    }

    pub const fn dispatch_ops_ready(&self) -> bool {
        self.dispatch_ops_ready
    }

    pub const fn source_zero_reserved(&self) -> bool {
        self.source_zero_reserved
    }

    pub const fn one_cell_specifier(&self) -> bool {
        self.one_cell_specifier
    }

    pub const fn enable_deferred(&self) -> bool {
        self.enable_deferred
    }

    pub const fn source_count(&self) -> u32 {
        self.source_count
    }

    pub const fn mapping_count(&self) -> usize {
        self.mapping_count
    }

    pub fn mapping_for_source(&self, source: u32) -> Option<&PlicIrqMapping> {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = &self.mappings[index];
            if mapping.source() == source {
                return Some(mapping);
            }
            index += 1;
        }
        None
    }

    pub fn mapping_for_logical_irq(&self, logical_irq: LogicalIrq) -> Option<&PlicIrqMapping> {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = &self.mappings[index];
            if mapping.logical_irq() == logical_irq {
                return Some(mapping);
            }
            index += 1;
        }
        None
    }

    pub fn preset(&mut self, plic: &Plic, irq_controller: &IrqController) -> EventResult {
        if self.lifecycle.state() != State::Base
            || plic.state() != State::Ready
            || irq_controller.state() != State::Ready
            || plic.source_count() == 0
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.owner_bound = true;
        self.translate_specifier_ready = true;
        self.source_count = plic.source_count();
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PlicIrqDomainPrepared,
        )
    }

    pub fn setup(&mut self, plic: &Plic, irq_controller: &IrqController) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || plic.state() != State::Ready
            || irq_controller.state() != State::Ready
            || self.source_count != plic.source_count()
            || !self.owner_bound
            || !self.translate_specifier_ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.hwirq_valid_range_ready = true;
        self.logical_irq_allocator_ready = true;
        self.mapping_table_ready = true;
        self.dispatch_ops_ready = true;
        self.source_zero_reserved = true;
        self.one_cell_specifier = true;
        self.enable_deferred = true;
        self.next_logical_irq = PLIC_LOGICAL_IRQ_BASE;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::PlicIrqDomainReady,
        )
    }

    pub fn translate_one_cell_specifier(&self, specifier: &[u32]) -> Option<u32> {
        super::plic_provider::translate_one_cell_specifier(self, specifier)
    }

    pub(crate) fn native_translate_one_cell_specifier(&self, specifier: &[u32]) -> Option<u32> {
        if self.lifecycle.state() != State::Ready
            || !self.translate_specifier_ready
            || !self.one_cell_specifier
            || specifier.len() != 1
        {
            return None;
        }
        let source = specifier[0];
        if source == 0 || source > self.source_count {
            return None;
        }
        Some(source)
    }

    pub fn map_source(&mut self, source: u32) -> Option<LogicalIrq> {
        super::plic_provider::map_source(self, source)
    }

    pub(crate) fn native_map_source(&mut self, source: u32) -> Option<LogicalIrq> {
        if self.lifecycle.state() != State::Ready
            || !self.logical_irq_allocator_ready
            || !self.mapping_table_ready
            || source == 0
            || source > self.source_count
        {
            return None;
        }
        if let Some(mapping) = self.mapping_for_source(source) {
            return Some(mapping.logical_irq());
        }
        if self.mapping_count >= PLIC_IRQ_MAPPING_CAPACITY {
            return None;
        }
        let logical_irq = LogicalIrq::new(self.next_logical_irq);
        let mapping_index = self.mapping_count;
        if self.mappings[mapping_index]
            .setup_from_domain(source, logical_irq, self.source_count)
            .is_err()
        {
            return None;
        }
        self.mapping_count += 1;
        self.next_logical_irq = self.next_logical_irq.saturating_add(1);
        Some(logical_irq)
    }

    pub fn resolve_hwirq(&self, source: u32) -> Option<LogicalIrq> {
        super::plic_provider::resolve_hwirq(self, source)
    }

    pub(crate) fn native_resolve_hwirq(&self, source: u32) -> Option<LogicalIrq> {
        if self.lifecycle.state() != State::Ready
            || !self.dispatch_ops_ready
            || source == 0
            || source > self.source_count
        {
            return None;
        }

        self.mapping_for_source(source)
            .map(|mapping| mapping.logical_irq())
    }

    pub fn enable_source_gate(&mut self, plic: &Plic, source: u32) -> EventResult {
        if self.lifecycle.state() != State::Ready || plic.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        let mut index = 0usize;
        while index < self.mapping_count {
            if self.mappings[index].source() == source {
                return self.mappings[index].enable_source_gate(plic);
            }
            index += 1;
        }

        failed_condition(
            LifecycleEvent::Enable,
            self.lifecycle.state(),
            State::Ready,
            State::Ready,
        )
    }
}

#[derive(Clone, Copy)]
struct PlicMmioResource {
    base: usize,
    size: usize,
}

fn plic_mmio_resource(node: DeviceNodeRef<'_>) -> Option<PlicMmioResource> {
    let reg = node.property(b"reg")?.raw_value();
    let reg_base = reg.as_ptr() as usize;
    let parent = node.parent()?;
    let address_cells = read_cells_u32(parent.property(b"#address-cells")).unwrap_or(2);
    let size_cells = read_cells_u32(parent.property(b"#size-cells")).unwrap_or(2);
    let (base, used) = read_cells(reg_base, reg.len(), address_cells)?;
    let (size, _) = read_cells(reg_base.checked_add(used)?, reg.len() - used, size_cells)?;
    Some(PlicMmioResource {
        base: usize::try_from(base).ok()?,
        size: usize::try_from(size).ok()?,
    })
}

fn plic_source_count(node: DeviceNodeRef<'_>) -> Option<u32> {
    read_property_u32(node.property(b"riscv,ndev"))
}

fn plic_context_parent_has_external_input(node: DeviceNodeRef<'_>, cpu_group: &CpuGroup) -> bool {
    plic_external_context_index(node, cpu_group).is_some()
}

fn plic_external_context_index(node: DeviceNodeRef<'_>, cpu_group: &CpuGroup) -> Option<usize> {
    let boot_intc_phandle = boot_hart_intc_phandle(node, cpu_group.boot_hartid())?;
    let Some(property) = node.property(b"interrupts-extended") else {
        return None;
    };
    let value = property.raw_value();
    let start = value.as_ptr() as usize;
    let Some(end) = start.checked_add(value.len()) else {
        return None;
    };
    if value.len() < 8 {
        return None;
    }

    let mut cursor = start;
    let mut index = 0usize;
    while cursor + 8 <= end {
        let Some(phandle) = read_be_u32(cursor, end) else {
            return None;
        };
        let Some(cause) = read_be_u32(cursor + 4, end) else {
            return None;
        };
        if phandle == boot_intc_phandle && cause == riscv64::SUPERVISOR_EXTERNAL_IRQ as u32 {
            return Some(index);
        }
        cursor += 8;
        index += 1;
    }
    None
}

fn boot_hart_intc_phandle(plic_node: DeviceNodeRef<'_>, boot_hartid: usize) -> Option<u32> {
    let cpus = plic_node
        .parent()
        .and_then(|parent| parent.parent())
        .and_then(|root| root.children().find(|node| node.name() == b"cpus"))?;
    let address_cells = read_cells_u32(cpus.property(b"#address-cells")).unwrap_or(1);
    for cpu in cpus.children() {
        let Some(reg) = cpu.property(b"reg") else {
            continue;
        };
        let (hartid, _) = read_cells(
            reg.raw_value().as_ptr() as usize,
            reg.raw_value().len(),
            address_cells,
        )?;
        if usize::try_from(hartid).ok()? != boot_hartid {
            continue;
        }
        for child in cpu.children() {
            if child.property(b"interrupt-controller").is_some()
                && child.has_compatible(b"riscv,cpu-intc")
            {
                if let Some(phandle) = read_property_u32(child.property(b"phandle")) {
                    return Some(phandle);
                }
                if let Some(phandle) = read_property_u32(child.property(b"linux,phandle")) {
                    return Some(phandle);
                }
            }
        }
    }
    None
}

fn plic_context_claim_addr(membase: usize, context_id: usize) -> Option<usize> {
    membase
        .checked_add(PLIC_CONTEXT_BASE)?
        .checked_add(context_id.checked_mul(PLIC_CONTEXT_SIZE)?)?
        .checked_add(PLIC_CONTEXT_CLAIM)
}

fn plic_context_enable_addr(membase: usize, context_id: usize) -> Option<usize> {
    membase
        .checked_add(PLIC_CONTEXT_ENABLE_BASE)?
        .checked_add(context_id.checked_mul(PLIC_CONTEXT_ENABLE_SIZE)?)
}

fn plic_context_threshold_addr(membase: usize, context_id: usize) -> Option<usize> {
    membase
        .checked_add(PLIC_CONTEXT_BASE)?
        .checked_add(context_id.checked_mul(PLIC_CONTEXT_SIZE)?)?
        .checked_add(PLIC_CONTEXT_THRESHOLD)
}

#[cfg(not(plic_provider_linux_object))]
fn plic_priority_addr(membase: usize, source: u32) -> Option<usize> {
    membase.checked_add(PLIC_PRIORITY_BASE)?.checked_add(
        usize::try_from(source)
            .ok()?
            .checked_mul(PLIC_PRIORITY_PER_ID)?,
    )
}

#[cfg(not(plic_provider_linux_object))]
fn plic_source_enable_addr(enable_base: usize, source: u32) -> Option<usize> {
    enable_base.checked_add(
        usize::try_from(source / 32)
            .ok()?
            .checked_mul(size_of::<u32>())?,
    )
}

fn plic_node_matches_supported_compatible(node: DeviceNodeRef<'_>) -> bool {
    node.has_compatible(PLIC_COMPATIBLE_SIFIVE) || node.has_compatible(PLIC_COMPATIBLE_RISCV)
}

pub struct UartExternalIrqEnable {
    lifecycle: Lifecycle,
    plic_source_gate_open: bool,
    root_external_input_gate_open: bool,
    uart_interrupt_output_deferred: bool,
}

impl UartExternalIrqEnable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            plic_source_gate_open: false,
            root_external_input_gate_open: false,
            uart_interrupt_output_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn plic_source_gate_open(&self) -> bool {
        self.plic_source_gate_open
    }

    pub const fn root_external_input_gate_open(&self) -> bool {
        self.root_external_input_gate_open
    }

    pub const fn uart_interrupt_output_deferred(&self) -> bool {
        self.uart_interrupt_output_deferred
    }

    pub fn setup(
        &mut self,
        plic: &Plic,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
        interrupt_stream: &mut InterruptStream,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        if self.lifecycle.state() != State::Base
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::ns16550a::uart8250_port_irq_resource_ready()
            || !super::ns16550a::uart8250_port_logical_irq_ready()
            || !super::ns16550a::uart8250_irq_handler_registered()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| mapping.logical_irq() != logical_irq)
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
            || interrupt_stream.state() != State::Online
            || !interrupt_stream.external_handler_ready()
            || !interrupt_stream.supervisor_external_input_gate_defined()
            || !interrupt_stream.supervisor_external_input_gate_closed()
            || !interrupt_stream.supervisor_external_input_enable_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        plic_irq_domain.enable_source_gate(plic, source)?;
        interrupt_stream.enable_supervisor_external_input()?;
        let Some(mapping) = plic_irq_domain.mapping_for_source(source) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        if !mapping.source_gate_open() || !interrupt_stream.supervisor_external_input_gate_open() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.plic_source_gate_open = true;
        self.root_external_input_gate_open = true;
        self.uart_interrupt_output_deferred =
            super::ns16550a::uart8250_interrupt_output_still_deferred();
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct UartInterruptChainProbe {
    lifecycle: Lifecycle,
    uart_trigger_committed: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_observed: bool,
    plic_complete_observed: bool,
    plic_loop_exit_observed: bool,
    irq_cycle_closed: bool,
    console_polling_preserved: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct Serial8250ConsoleIrqTxProbe {
    lifecycle: Lifecycle,
    interrupt_driven_enabled: bool,
    printk_frontend_submitted: bool,
    tx_queue_kicked: bool,
    uart_handler_drained_tx: bool,
    plic_claim_observed: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    tx_queue_empty_after_irq: bool,
    local_irq_guard_observed: bool,
}

pub struct Serial8250RxLoopbackProbe {
    lifecycle: Lifecycle,
    rx_runtime_enabled: bool,
    loopback_stimulus_committed: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_received_rx: bool,
    flip_buffer_pushed: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    irq_cycle_closed: bool,
    last_byte_matched: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct Serial8250ConsoleBurstIrqTxProbe {
    lifecycle: Lifecycle,
    printk_frontend_submitted: bool,
    multiple_records_submitted: bool,
    tx_queue_kicked: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_drained_tx: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    tx_queue_empty_after_irq: bool,
    write_count_matched: bool,
    drain_count_matched: bool,
    last_byte_matched: bool,
    local_irq_guard_observed: bool,
    no_overflow_observed: bool,
    tty_xmit_fifo_unchanged: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct Serial8250ConsoleLongIrqTxProbe {
    lifecycle: Lifecycle,
    printk_frontend_submitted: bool,
    tx_load_size_observed: bool,
    message_exceeds_single_load: bool,
    tx_queue_kicked: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_drained_tx: bool,
    multiple_irq_rounds_observed: bool,
    tx_load_budget_observed: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    tx_queue_empty_after_irq: bool,
    write_count_matched: bool,
    drain_count_matched: bool,
    last_byte_matched: bool,
    local_irq_guard_observed: bool,
    no_overflow_observed: bool,
    tty_xmit_fifo_unchanged: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct Serial8250ConsoleLongBurstIrqTxProbe {
    lifecycle: Lifecycle,
    printk_frontend_submitted: bool,
    multiple_records_submitted: bool,
    tx_load_size_observed: bool,
    each_record_exceeds_single_load: bool,
    tx_queue_kicked: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_drained_tx: bool,
    multiple_irq_rounds_observed: bool,
    tx_load_budget_observed: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    tx_queue_empty_after_irq: bool,
    write_count_matched: bool,
    drain_count_matched: bool,
    last_byte_matched: bool,
    local_irq_guard_observed: bool,
    no_overflow_observed: bool,
    tty_xmit_fifo_unchanged: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct Serial8250ConsoleTxQuiesceProbe {
    lifecycle: Lifecycle,
    no_printk_write: bool,
    tx_queue_empty_observed: bool,
    thri_stopped_observed: bool,
    no_spurious_plic_claim: bool,
    no_spurious_irq_dispatch: bool,
    no_uart_tx_drain: bool,
    no_tty_xmit_fifo_mutation: bool,
    no_overflow_observed: bool,
}

pub struct Serial8250RxBatchLoopbackProbe {
    lifecycle: Lifecycle,
    batch_stimulus_committed: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_received_batch: bool,
    flip_buffer_batch_pushed: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    irq_cycle_closed: bool,
    bounded_drain_observed: bool,
    batch_count_matched: bool,
    last_byte_matched: bool,
    no_overflow_observed: bool,
}

pub struct TtyXmitFifoProbe {
    lifecycle: Lifecycle,
    enqueue_committed: bool,
    dequeue_committed: bool,
    byte_round_trip: bool,
    queue_empty_after_dequeue: bool,
    distinct_from_printk_console_tx: bool,
    runtime_tx_deferred: bool,
    printk_tx_queue_unchanged: bool,
    no_uart_thri_kick: bool,
    no_overflow_observed: bool,
    no_underflow_observed: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct TtyWriteRuntimeTxProbe {
    lifecycle: Lifecycle,
    xmit_fifo_enqueued: bool,
    start_tx_committed: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_observed: bool,
    xmit_fifo_drained: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    queue_empty_after_irq: bool,
    printk_tx_queue_unchanged: bool,
    local_irq_guard_observed: bool,
    last_byte_matched: bool,
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub struct TtyWriteBatchRuntimeTxProbe {
    lifecycle: Lifecycle,
    fixed_bounded_batch: bool,
    xmit_fifo_batch_enqueued: bool,
    start_tx_committed: bool,
    plic_claim_observed: bool,
    irq_dispatch_observed: bool,
    uart_handler_observed: bool,
    xmit_fifo_batch_drained: bool,
    plic_complete_observed: bool,
    zero_claim_loop_exit_observed: bool,
    queue_empty_after_irq: bool,
    printk_tx_queue_unchanged: bool,
    local_irq_guard_observed: bool,
    bounded_drain_observed: bool,
    batch_count_matched: bool,
    last_byte_matched: bool,
    no_overflow_observed: bool,
    no_underflow_observed: bool,
}

const UART_IRQ_CYCLE_SPIN_LIMIT: usize = 20_000_000;
#[cfg(checkpoint_handler_uart_irq_chain)]
const SERIAL8250_IRQ_TX_PROBE_MESSAGE: &str = "serial8250 irq console\n";
#[cfg(checkpoint_handler_uart_irq_chain)]
const SERIAL8250_BURST_IRQ_TX_PROBE_MESSAGES: &[&str] = &["printk burst A\n", "printk burst B\n"];
#[cfg(checkpoint_handler_uart_irq_chain)]
const SERIAL8250_LONG_IRQ_TX_PROBE_MESSAGE: &str = "printk long tx load 0123456789abcdef\n";
#[cfg(checkpoint_handler_uart_irq_chain)]
const SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES: &[&str] = &[
    "printk long burst A 0123456789abcdef\n",
    "printk long burst B fedcba9876543210\n",
    "printk long burst C 0011223344556677\n",
];
const SERIAL8250_RX_LOOPBACK_BYTE: u8 = b'R';
const SERIAL8250_RX_BATCH_LOOPBACK_BYTES: &[u8] = b"rx42";
const TTY_XMIT_FIFO_PROBE_BYTE: u8 = b'T';
#[cfg(checkpoint_handler_uart_irq_chain)]
const TTY_WRITE_RUNTIME_TX_PROBE_BYTE: u8 = b'W';
#[cfg(checkpoint_handler_uart_irq_chain)]
const TTY_WRITE_RUNTIME_TX_BATCH_BYTES: &[u8] = b"tx04";

#[derive(Clone, Copy, Eq, PartialEq)]
struct UartIrqCycleSnapshot {
    requests: usize,
    rx_requests: usize,
    claims: usize,
    zero_claims: usize,
    plic_dispatches: usize,
    completes: usize,
    loop_exits: usize,
    irq_dispatches: usize,
    handler_calls: usize,
    handled: usize,
    rx_handled: usize,
    thri_disabled: usize,
    flip_pushes: usize,
    flip_inserted: usize,
}

impl UartInterruptChainProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            uart_trigger_committed: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_observed: false,
            plic_complete_observed: false,
            plic_loop_exit_observed: false,
            irq_cycle_closed: false,
            console_polling_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn uart_trigger_committed(&self) -> bool {
        self.uart_trigger_committed
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_observed(&self) -> bool {
        self.uart_handler_observed
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn plic_loop_exit_observed(&self) -> bool {
        self.plic_loop_exit_observed
    }

    pub const fn irq_cycle_closed(&self) -> bool {
        self.irq_cycle_closed
    }

    pub const fn console_polling_preserved(&self) -> bool {
        self.console_polling_preserved
    }

    pub fn setup(
        &mut self,
        uart_external_irq_enable: &UartExternalIrqEnable,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        if self.lifecycle.state() != State::Base
            || uart_external_irq_enable.state() != State::Ready
            || !uart_external_irq_enable.plic_source_gate_open()
            || !uart_external_irq_enable.root_external_input_gate_open()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::ns16550a::uart8250_port_logical_irq_ready()
            || !super::ns16550a::uart8250_irq_handler_registered()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);

        if !super::ns16550a::trigger_uart8250_thre_interrupt_once() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !wait_uart_irq_cycle_closed(plic, irq_handler_registry, baseline, source) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        self.uart_trigger_committed = super::ns16550a::uart8250_interrupt_trigger_ready()
            && observed.requests > baseline.requests
            && super::ns16550a::uart8250_thre_interrupt_handled();
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_observed = observed.handler_calls > baseline.handler_calls
            && observed.thri_disabled > baseline.thri_disabled;
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.plic_loop_exit_observed = plic.claim_loop_until_zero()
            && plic.zero_claim_stops_dispatch()
            && plic.completes_each_claimed_source()
            && observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits
            && observed.completes.saturating_sub(baseline.completes)
                == observed.claims.saturating_sub(baseline.claims);
        self.irq_cycle_closed =
            uart_irq_cycle_completed_and_closed(plic, observed, baseline, source);
        self.console_polling_preserved =
            super::ns16550a::uart8250_interrupt_output_still_deferred();

        if !self.uart_trigger_committed
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_observed
            || !self.plic_complete_observed
            || !self.plic_loop_exit_observed
            || !self.irq_cycle_closed
            || !self.console_polling_preserved
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl Serial8250ConsoleIrqTxProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            interrupt_driven_enabled: false,
            printk_frontend_submitted: false,
            tx_queue_kicked: false,
            uart_handler_drained_tx: false,
            plic_claim_observed: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            tx_queue_empty_after_irq: false,
            local_irq_guard_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn interrupt_driven_enabled(&self) -> bool {
        self.interrupt_driven_enabled
    }

    pub const fn printk_frontend_submitted(&self) -> bool {
        self.printk_frontend_submitted
    }

    pub const fn tx_queue_kicked(&self) -> bool {
        self.tx_queue_kicked
    }

    pub const fn uart_handler_drained_tx(&self) -> bool {
        self.uart_handler_drained_tx
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn tx_queue_empty_after_irq(&self) -> bool {
        self.tx_queue_empty_after_irq
    }

    pub const fn local_irq_guard_observed(&self) -> bool {
        self.local_irq_guard_observed
    }

    pub fn setup(
        &mut self,
        uart_external_irq_enable: &UartExternalIrqEnable,
        uart_interrupt_chain_probe: &UartInterruptChainProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        if self.lifecycle.state() != State::Base
            || uart_external_irq_enable.state() != State::Ready
            || !uart_external_irq_enable.plic_source_gate_open()
            || !uart_external_irq_enable.root_external_input_gate_open()
            || uart_interrupt_chain_probe.state() != State::Ready
            || !uart_interrupt_chain_probe.irq_cycle_closed()
            || !uart_interrupt_chain_probe.console_polling_preserved()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::printk::console_handoff_complete()
            || !super::ns16550a::serial8250_console_registered()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !super::ns16550a::serial8250_runtime_console_tx_ready()
            && !super::ns16550a::enable_serial8250_interrupt_driven_console()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !super::ns16550a::uart8250_interrupt_driven_configured() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_drains = super::ns16550a::serial8250_tx_irq_drain_count();
        let baseline_write_calls =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();

        super::printk::write_str(SERIAL8250_IRQ_TX_PROBE_MESSAGE);

        let expected_tx_bytes = SERIAL8250_IRQ_TX_PROBE_MESSAGE.len().saturating_add(1);
        if !wait_serial8250_irq_tx_closed(
            plic,
            irq_handler_registry,
            baseline,
            baseline_drains,
            expected_tx_bytes,
            source,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        self.interrupt_driven_enabled = super::ns16550a::uart8250_interrupt_driven_ready();
        self.printk_frontend_submitted =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
                > baseline_write_calls;
        self.tx_queue_kicked = super::ns16550a::serial8250_tx_irq_kick_count() > baseline_kicks;
        self.uart_handler_drained_tx = super::ns16550a::serial8250_tx_irq_drain_count()
            >= baseline_drains.saturating_add(expected_tx_bytes);
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.tx_queue_empty_after_irq = super::ns16550a::serial8250_tx_queue_len() == 0
            && super::ns16550a::serial8250_tx_irq_empty_stop_count() != 0
            && !super::ns16550a::serial8250_tx_queue_overflowed();
        self.local_irq_guard_observed =
            super::ns16550a::serial8250_tx_queue_guarded_by_local_irq_save();

        if !self.interrupt_driven_enabled
            || !self.printk_frontend_submitted
            || !self.tx_queue_kicked
            || !self.uart_handler_drained_tx
            || !self.plic_claim_observed
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.tx_queue_empty_after_irq
            || !self.local_irq_guard_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl Serial8250ConsoleBurstIrqTxProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            printk_frontend_submitted: false,
            multiple_records_submitted: false,
            tx_queue_kicked: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_drained_tx: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            tx_queue_empty_after_irq: false,
            write_count_matched: false,
            drain_count_matched: false,
            last_byte_matched: false,
            local_irq_guard_observed: false,
            no_overflow_observed: false,
            tty_xmit_fifo_unchanged: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn printk_frontend_submitted(&self) -> bool {
        self.printk_frontend_submitted
    }

    pub const fn multiple_records_submitted(&self) -> bool {
        self.multiple_records_submitted
    }

    pub const fn tx_queue_kicked(&self) -> bool {
        self.tx_queue_kicked
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_drained_tx(&self) -> bool {
        self.uart_handler_drained_tx
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn tx_queue_empty_after_irq(&self) -> bool {
        self.tx_queue_empty_after_irq
    }

    pub const fn write_count_matched(&self) -> bool {
        self.write_count_matched
    }

    pub const fn drain_count_matched(&self) -> bool {
        self.drain_count_matched
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub const fn local_irq_guard_observed(&self) -> bool {
        self.local_irq_guard_observed
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub const fn tty_xmit_fifo_unchanged(&self) -> bool {
        self.tty_xmit_fifo_unchanged
    }

    pub fn setup(
        &mut self,
        console_irq_tx_probe: &Serial8250ConsoleIrqTxProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        let expected_records = SERIAL8250_BURST_IRQ_TX_PROBE_MESSAGES.len();
        let expected_tx_bytes = serial8250_console_burst_expected_tx_bytes();
        let expected_last = serial8250_console_burst_expected_last_byte();
        if self.lifecycle.state() != State::Base
            || console_irq_tx_probe.state() != State::Ready
            || !console_irq_tx_probe.interrupt_driven_enabled()
            || !console_irq_tx_probe.tx_queue_empty_after_irq()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::printk::console_handoff_complete()
            || !super::ns16550a::serial8250_console_registered()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || super::ns16550a::serial8250_tx_queue_len() != 0
            || expected_records <= 1
            || expected_tx_bytes == 0
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_write_calls =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();
        let baseline_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_drains = super::ns16550a::serial8250_tx_irq_drain_count();
        let baseline_tx_bytes = super::ns16550a::serial8250_tx_byte_count_available_for_irq_probe();
        let baseline_crlf = super::ns16550a::serial8250_tx_crlf_insertion_count();
        let baseline_tty_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_tty_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_tty_runtime_drains = super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count();

        for message in SERIAL8250_BURST_IRQ_TX_PROBE_MESSAGES {
            super::printk::write_str(message);
        }

        if !wait_serial8250_irq_tx_closed(
            plic,
            irq_handler_registry,
            baseline,
            baseline_drains,
            expected_tx_bytes,
            source,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let write_delta = super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
            .saturating_sub(baseline_write_calls);
        let drain_delta =
            super::ns16550a::serial8250_tx_irq_drain_count().saturating_sub(baseline_drains);
        let tx_byte_delta = super::ns16550a::serial8250_tx_byte_count_available_for_irq_probe()
            .saturating_sub(baseline_tx_bytes);
        let crlf_delta =
            super::ns16550a::serial8250_tx_crlf_insertion_count().saturating_sub(baseline_crlf);
        self.printk_frontend_submitted = write_delta == expected_records;
        self.multiple_records_submitted = expected_records > 1;
        self.tx_queue_kicked = super::ns16550a::serial8250_tx_irq_kick_count() > baseline_kicks;
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_drained_tx = observed.handler_calls > baseline.handler_calls
            && observed.handled > baseline.handled
            && drain_delta >= expected_tx_bytes;
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.tx_queue_empty_after_irq = super::ns16550a::serial8250_tx_queue_len() == 0
            && super::ns16550a::serial8250_tx_irq_empty_stop_count() != 0
            && !super::ns16550a::serial8250_tx_queue_overflowed();
        self.write_count_matched = write_delta == expected_records;
        self.drain_count_matched =
            drain_delta == expected_tx_bytes && tx_byte_delta == expected_tx_bytes;
        self.last_byte_matched = super::ns16550a::serial8250_last_tx_byte() == expected_last;
        self.local_irq_guard_observed =
            super::ns16550a::serial8250_tx_queue_guarded_by_local_irq_save();
        self.no_overflow_observed = !super::ns16550a::serial8250_tx_queue_overflowed();
        self.tty_xmit_fifo_unchanged = super::ns16550a::tty_xmit_fifo_enqueue_count()
            == baseline_tty_enqueues
            && super::ns16550a::tty_xmit_fifo_dequeue_count() == baseline_tty_dequeues
            && super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count()
                == baseline_tty_runtime_drains
            && crlf_delta == serial8250_console_burst_expected_crlf_insertions();

        if !self.printk_frontend_submitted
            || !self.multiple_records_submitted
            || !self.tx_queue_kicked
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_drained_tx
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.tx_queue_empty_after_irq
            || !self.write_count_matched
            || !self.drain_count_matched
            || !self.last_byte_matched
            || !self.local_irq_guard_observed
            || !self.no_overflow_observed
            || !self.tty_xmit_fifo_unchanged
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::Serial8250ConsoleBurstIrqTxReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl Serial8250ConsoleLongIrqTxProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            printk_frontend_submitted: false,
            tx_load_size_observed: false,
            message_exceeds_single_load: false,
            tx_queue_kicked: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_drained_tx: false,
            multiple_irq_rounds_observed: false,
            tx_load_budget_observed: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            tx_queue_empty_after_irq: false,
            write_count_matched: false,
            drain_count_matched: false,
            last_byte_matched: false,
            local_irq_guard_observed: false,
            no_overflow_observed: false,
            tty_xmit_fifo_unchanged: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn printk_frontend_submitted(&self) -> bool {
        self.printk_frontend_submitted
    }

    pub const fn tx_load_size_observed(&self) -> bool {
        self.tx_load_size_observed
    }

    pub const fn message_exceeds_single_load(&self) -> bool {
        self.message_exceeds_single_load
    }

    pub const fn tx_queue_kicked(&self) -> bool {
        self.tx_queue_kicked
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_drained_tx(&self) -> bool {
        self.uart_handler_drained_tx
    }

    pub const fn multiple_irq_rounds_observed(&self) -> bool {
        self.multiple_irq_rounds_observed
    }

    pub const fn tx_load_budget_observed(&self) -> bool {
        self.tx_load_budget_observed
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn tx_queue_empty_after_irq(&self) -> bool {
        self.tx_queue_empty_after_irq
    }

    pub const fn write_count_matched(&self) -> bool {
        self.write_count_matched
    }

    pub const fn drain_count_matched(&self) -> bool {
        self.drain_count_matched
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub const fn local_irq_guard_observed(&self) -> bool {
        self.local_irq_guard_observed
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub const fn tty_xmit_fifo_unchanged(&self) -> bool {
        self.tty_xmit_fifo_unchanged
    }

    pub fn setup(
        &mut self,
        console_burst_irq_tx_probe: &Serial8250ConsoleBurstIrqTxProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        let tx_load_size = super::ns16550a::serial8250_runtime_tx_load_size();
        let expected_tx_bytes =
            serial8250_console_message_expected_tx_bytes(SERIAL8250_LONG_IRQ_TX_PROBE_MESSAGE);
        let expected_last = SERIAL8250_LONG_IRQ_TX_PROBE_MESSAGE
            .as_bytes()
            .last()
            .copied()
            .unwrap_or(0);
        if self.lifecycle.state() != State::Base
            || console_burst_irq_tx_probe.state() != State::Ready
            || !console_burst_irq_tx_probe.tx_queue_empty_after_irq()
            || !console_burst_irq_tx_probe.tty_xmit_fifo_unchanged()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::printk::console_handoff_complete()
            || !super::ns16550a::serial8250_console_registered()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || super::ns16550a::serial8250_tx_queue_len() != 0
            || tx_load_size == 0
            || expected_tx_bytes <= tx_load_size
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_write_calls =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();
        let baseline_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_drains = super::ns16550a::serial8250_tx_irq_drain_count();
        let baseline_budget_hits = super::ns16550a::serial8250_tx_irq_budget_hit_count();
        let baseline_tx_bytes = super::ns16550a::serial8250_tx_byte_count_available_for_irq_probe();
        let baseline_crlf = super::ns16550a::serial8250_tx_crlf_insertion_count();
        let baseline_tty_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_tty_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_tty_runtime_drains = super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count();

        super::printk::write_str(SERIAL8250_LONG_IRQ_TX_PROBE_MESSAGE);

        if !wait_serial8250_long_irq_tx_closed(
            plic,
            irq_handler_registry,
            baseline,
            baseline_drains,
            baseline_budget_hits,
            expected_tx_bytes,
            source,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let write_delta = super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
            .saturating_sub(baseline_write_calls);
        let drain_delta =
            super::ns16550a::serial8250_tx_irq_drain_count().saturating_sub(baseline_drains);
        let budget_hit_delta = super::ns16550a::serial8250_tx_irq_budget_hit_count()
            .saturating_sub(baseline_budget_hits);
        let tx_byte_delta = super::ns16550a::serial8250_tx_byte_count_available_for_irq_probe()
            .saturating_sub(baseline_tx_bytes);
        let crlf_delta =
            super::ns16550a::serial8250_tx_crlf_insertion_count().saturating_sub(baseline_crlf);
        let claim_delta = observed.claims.saturating_sub(baseline.claims);
        let complete_delta = observed.completes.saturating_sub(baseline.completes);
        let handler_delta = observed
            .handler_calls
            .saturating_sub(baseline.handler_calls);
        let handled_delta = observed.handled.saturating_sub(baseline.handled);

        self.printk_frontend_submitted = write_delta == 1;
        self.tx_load_size_observed = tx_load_size != 0;
        self.message_exceeds_single_load = expected_tx_bytes > tx_load_size;
        self.tx_queue_kicked = super::ns16550a::serial8250_tx_irq_kick_count() > baseline_kicks;
        self.plic_claim_observed = claim_delta != 0 && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_drained_tx =
            handler_delta != 0 && handled_delta != 0 && drain_delta >= expected_tx_bytes;
        self.multiple_irq_rounds_observed =
            handler_delta >= 2 && claim_delta >= 2 && complete_delta >= 2;
        self.tx_load_budget_observed = budget_hit_delta != 0;
        self.plic_complete_observed = complete_delta != 0 && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.tx_queue_empty_after_irq = super::ns16550a::serial8250_tx_queue_len() == 0
            && super::ns16550a::serial8250_tx_irq_empty_stop_count() != 0
            && !super::ns16550a::serial8250_tx_queue_overflowed();
        self.write_count_matched = write_delta == 1;
        self.drain_count_matched =
            drain_delta == expected_tx_bytes && tx_byte_delta == expected_tx_bytes;
        self.last_byte_matched = super::ns16550a::serial8250_last_tx_byte() == expected_last;
        self.local_irq_guard_observed =
            super::ns16550a::serial8250_tx_queue_guarded_by_local_irq_save();
        self.no_overflow_observed = !super::ns16550a::serial8250_tx_queue_overflowed();
        self.tty_xmit_fifo_unchanged = super::ns16550a::tty_xmit_fifo_enqueue_count()
            == baseline_tty_enqueues
            && super::ns16550a::tty_xmit_fifo_dequeue_count() == baseline_tty_dequeues
            && super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count()
                == baseline_tty_runtime_drains
            && crlf_delta == count_newlines(SERIAL8250_LONG_IRQ_TX_PROBE_MESSAGE);

        if !self.printk_frontend_submitted
            || !self.tx_load_size_observed
            || !self.message_exceeds_single_load
            || !self.tx_queue_kicked
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_drained_tx
            || !self.multiple_irq_rounds_observed
            || !self.tx_load_budget_observed
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.tx_queue_empty_after_irq
            || !self.write_count_matched
            || !self.drain_count_matched
            || !self.last_byte_matched
            || !self.local_irq_guard_observed
            || !self.no_overflow_observed
            || !self.tty_xmit_fifo_unchanged
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::Serial8250ConsoleLongIrqTxReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl Serial8250ConsoleLongBurstIrqTxProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            printk_frontend_submitted: false,
            multiple_records_submitted: false,
            tx_load_size_observed: false,
            each_record_exceeds_single_load: false,
            tx_queue_kicked: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_drained_tx: false,
            multiple_irq_rounds_observed: false,
            tx_load_budget_observed: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            tx_queue_empty_after_irq: false,
            write_count_matched: false,
            drain_count_matched: false,
            last_byte_matched: false,
            local_irq_guard_observed: false,
            no_overflow_observed: false,
            tty_xmit_fifo_unchanged: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn printk_frontend_submitted(&self) -> bool {
        self.printk_frontend_submitted
    }

    pub const fn multiple_records_submitted(&self) -> bool {
        self.multiple_records_submitted
    }

    pub const fn tx_load_size_observed(&self) -> bool {
        self.tx_load_size_observed
    }

    pub const fn each_record_exceeds_single_load(&self) -> bool {
        self.each_record_exceeds_single_load
    }

    pub const fn tx_queue_kicked(&self) -> bool {
        self.tx_queue_kicked
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_drained_tx(&self) -> bool {
        self.uart_handler_drained_tx
    }

    pub const fn multiple_irq_rounds_observed(&self) -> bool {
        self.multiple_irq_rounds_observed
    }

    pub const fn tx_load_budget_observed(&self) -> bool {
        self.tx_load_budget_observed
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn tx_queue_empty_after_irq(&self) -> bool {
        self.tx_queue_empty_after_irq
    }

    pub const fn write_count_matched(&self) -> bool {
        self.write_count_matched
    }

    pub const fn drain_count_matched(&self) -> bool {
        self.drain_count_matched
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub const fn local_irq_guard_observed(&self) -> bool {
        self.local_irq_guard_observed
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub const fn tty_xmit_fifo_unchanged(&self) -> bool {
        self.tty_xmit_fifo_unchanged
    }

    pub fn setup(
        &mut self,
        console_long_irq_tx_probe: &Serial8250ConsoleLongIrqTxProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        let tx_load_size = super::ns16550a::serial8250_runtime_tx_load_size();
        let expected_records = SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES.len();
        let expected_tx_bytes = serial8250_console_long_burst_expected_tx_bytes();
        let expected_last = serial8250_console_long_burst_expected_last_byte();
        let expected_rounds = serial8250_console_long_burst_expected_irq_rounds(tx_load_size);
        let expected_budget_hits = serial8250_console_long_burst_expected_budget_hits(tx_load_size);
        if self.lifecycle.state() != State::Base
            || console_long_irq_tx_probe.state() != State::Ready
            || !console_long_irq_tx_probe.tx_queue_empty_after_irq()
            || !console_long_irq_tx_probe.multiple_irq_rounds_observed()
            || !console_long_irq_tx_probe.tx_load_budget_observed()
            || !console_long_irq_tx_probe.tty_xmit_fifo_unchanged()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::printk::console_handoff_complete()
            || !super::ns16550a::serial8250_console_registered()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || super::ns16550a::serial8250_tx_queue_len() != 0
            || tx_load_size == 0
            || expected_records <= 1
            || expected_tx_bytes <= tx_load_size
            || expected_rounds <= 1
            || !serial8250_console_long_burst_each_record_exceeds_single_load(tx_load_size)
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_write_calls =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();
        let baseline_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_drains = super::ns16550a::serial8250_tx_irq_drain_count();
        let baseline_budget_hits = super::ns16550a::serial8250_tx_irq_budget_hit_count();
        let baseline_tx_bytes = super::ns16550a::serial8250_tx_byte_count_available_for_irq_probe();
        let baseline_crlf = super::ns16550a::serial8250_tx_crlf_insertion_count();
        let baseline_tty_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_tty_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_tty_runtime_drains = super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count();

        for message in SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES {
            super::printk::write_str(message);
        }

        if !wait_serial8250_long_burst_irq_tx_closed(
            plic,
            irq_handler_registry,
            baseline,
            baseline_drains,
            baseline_budget_hits,
            expected_tx_bytes,
            expected_rounds,
            expected_budget_hits,
            source,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let write_delta = super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
            .saturating_sub(baseline_write_calls);
        let drain_delta =
            super::ns16550a::serial8250_tx_irq_drain_count().saturating_sub(baseline_drains);
        let budget_hit_delta = super::ns16550a::serial8250_tx_irq_budget_hit_count()
            .saturating_sub(baseline_budget_hits);
        let tx_byte_delta = super::ns16550a::serial8250_tx_byte_count_available_for_irq_probe()
            .saturating_sub(baseline_tx_bytes);
        let crlf_delta =
            super::ns16550a::serial8250_tx_crlf_insertion_count().saturating_sub(baseline_crlf);
        let claim_delta = observed.claims.saturating_sub(baseline.claims);
        let complete_delta = observed.completes.saturating_sub(baseline.completes);
        let handler_delta = observed
            .handler_calls
            .saturating_sub(baseline.handler_calls);
        let handled_delta = observed.handled.saturating_sub(baseline.handled);

        self.printk_frontend_submitted = write_delta == expected_records;
        self.multiple_records_submitted = expected_records > 1;
        self.tx_load_size_observed = tx_load_size != 0;
        self.each_record_exceeds_single_load =
            serial8250_console_long_burst_each_record_exceeds_single_load(tx_load_size);
        self.tx_queue_kicked = super::ns16550a::serial8250_tx_irq_kick_count() > baseline_kicks;
        self.plic_claim_observed =
            claim_delta >= expected_rounds && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed
            .plic_dispatches
            .saturating_sub(baseline.plic_dispatches)
            >= expected_rounds
            && observed
                .irq_dispatches
                .saturating_sub(baseline.irq_dispatches)
                >= expected_rounds;
        self.uart_handler_drained_tx = handler_delta >= expected_rounds
            && handled_delta >= expected_rounds
            && drain_delta >= expected_tx_bytes;
        self.multiple_irq_rounds_observed = handler_delta >= expected_rounds
            && claim_delta >= expected_rounds
            && complete_delta >= expected_rounds;
        self.tx_load_budget_observed = budget_hit_delta >= expected_budget_hits;
        self.plic_complete_observed =
            complete_delta >= expected_rounds && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.tx_queue_empty_after_irq = super::ns16550a::serial8250_tx_queue_len() == 0
            && super::ns16550a::serial8250_tx_irq_empty_stop_count() != 0
            && !super::ns16550a::serial8250_tx_queue_overflowed();
        self.write_count_matched = write_delta == expected_records;
        self.drain_count_matched =
            drain_delta == expected_tx_bytes && tx_byte_delta == expected_tx_bytes;
        self.last_byte_matched = super::ns16550a::serial8250_last_tx_byte() == expected_last;
        self.local_irq_guard_observed =
            super::ns16550a::serial8250_tx_queue_guarded_by_local_irq_save();
        self.no_overflow_observed = !super::ns16550a::serial8250_tx_queue_overflowed();
        self.tty_xmit_fifo_unchanged = super::ns16550a::tty_xmit_fifo_enqueue_count()
            == baseline_tty_enqueues
            && super::ns16550a::tty_xmit_fifo_dequeue_count() == baseline_tty_dequeues
            && super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count()
                == baseline_tty_runtime_drains
            && crlf_delta == serial8250_console_long_burst_expected_crlf_insertions();

        if !self.printk_frontend_submitted
            || !self.multiple_records_submitted
            || !self.tx_load_size_observed
            || !self.each_record_exceeds_single_load
            || !self.tx_queue_kicked
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_drained_tx
            || !self.multiple_irq_rounds_observed
            || !self.tx_load_budget_observed
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.tx_queue_empty_after_irq
            || !self.write_count_matched
            || !self.drain_count_matched
            || !self.last_byte_matched
            || !self.local_irq_guard_observed
            || !self.no_overflow_observed
            || !self.tty_xmit_fifo_unchanged
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::Serial8250ConsoleLongBurstIrqTxReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl Serial8250ConsoleTxQuiesceProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            no_printk_write: false,
            tx_queue_empty_observed: false,
            thri_stopped_observed: false,
            no_spurious_plic_claim: false,
            no_spurious_irq_dispatch: false,
            no_uart_tx_drain: false,
            no_tty_xmit_fifo_mutation: false,
            no_overflow_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn no_printk_write(&self) -> bool {
        self.no_printk_write
    }

    pub const fn tx_queue_empty_observed(&self) -> bool {
        self.tx_queue_empty_observed
    }

    pub const fn thri_stopped_observed(&self) -> bool {
        self.thri_stopped_observed
    }

    pub const fn no_spurious_plic_claim(&self) -> bool {
        self.no_spurious_plic_claim
    }

    pub const fn no_spurious_irq_dispatch(&self) -> bool {
        self.no_spurious_irq_dispatch
    }

    pub const fn no_uart_tx_drain(&self) -> bool {
        self.no_uart_tx_drain
    }

    pub const fn no_tty_xmit_fifo_mutation(&self) -> bool {
        self.no_tty_xmit_fifo_mutation
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub fn setup(
        &mut self,
        console_long_burst_irq_tx_probe: &Serial8250ConsoleLongBurstIrqTxProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        if self.lifecycle.state() != State::Base
            || console_long_burst_irq_tx_probe.state() != State::Ready
            || !console_long_burst_irq_tx_probe.tx_queue_empty_after_irq()
            || !console_long_burst_irq_tx_probe.tx_load_budget_observed()
            || !console_long_burst_irq_tx_probe.no_overflow_observed()
            || !console_long_burst_irq_tx_probe.tty_xmit_fifo_unchanged()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::printk::console_handoff_complete()
            || !super::ns16550a::serial8250_console_registered()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || super::ns16550a::serial8250_tx_queue_len() != 0
            || super::ns16550a::serial8250_tx_irq_empty_stop_count() == 0
            || super::ns16550a::serial8250_tx_queue_overflowed()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_write_calls =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();
        let baseline_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_drains = super::ns16550a::serial8250_tx_irq_drain_count();
        let baseline_empty_stops = super::ns16550a::serial8250_tx_irq_empty_stop_count();
        let baseline_tty_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_tty_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_tty_runtime_drains = super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count();

        wait_serial8250_tx_quiesce_window();

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        self.no_printk_write =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
                == baseline_write_calls
                && super::ns16550a::serial8250_tx_irq_kick_count() == baseline_kicks;
        self.tx_queue_empty_observed = super::ns16550a::serial8250_tx_queue_len() == 0;
        self.thri_stopped_observed = super::ns16550a::serial8250_tx_irq_empty_stop_count()
            == baseline_empty_stops
            && observed.thri_disabled == baseline.thri_disabled;
        self.no_spurious_plic_claim = observed.claims == baseline.claims
            && observed.zero_claims == baseline.zero_claims
            && observed.completes == baseline.completes
            && observed.loop_exits == baseline.loop_exits;
        self.no_spurious_irq_dispatch = observed.plic_dispatches == baseline.plic_dispatches
            && observed.irq_dispatches == baseline.irq_dispatches
            && observed.handler_calls == baseline.handler_calls
            && observed.handled == baseline.handled;
        self.no_uart_tx_drain = super::ns16550a::serial8250_tx_irq_drain_count() == baseline_drains;
        self.no_tty_xmit_fifo_mutation = super::ns16550a::tty_xmit_fifo_enqueue_count()
            == baseline_tty_enqueues
            && super::ns16550a::tty_xmit_fifo_dequeue_count() == baseline_tty_dequeues
            && super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count()
                == baseline_tty_runtime_drains;
        self.no_overflow_observed = !super::ns16550a::serial8250_tx_queue_overflowed();

        if !self.no_printk_write
            || !self.tx_queue_empty_observed
            || !self.thri_stopped_observed
            || !self.no_spurious_plic_claim
            || !self.no_spurious_irq_dispatch
            || !self.no_uart_tx_drain
            || !self.no_tty_xmit_fifo_mutation
            || !self.no_overflow_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::Serial8250ConsoleTxQuiesceReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

impl Serial8250RxLoopbackProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            rx_runtime_enabled: false,
            loopback_stimulus_committed: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_received_rx: false,
            flip_buffer_pushed: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            irq_cycle_closed: false,
            last_byte_matched: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn rx_runtime_enabled(&self) -> bool {
        self.rx_runtime_enabled
    }

    pub const fn loopback_stimulus_committed(&self) -> bool {
        self.loopback_stimulus_committed
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_received_rx(&self) -> bool {
        self.uart_handler_received_rx
    }

    pub const fn flip_buffer_pushed(&self) -> bool {
        self.flip_buffer_pushed
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn irq_cycle_closed(&self) -> bool {
        self.irq_cycle_closed
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub fn setup(
        &mut self,
        uart_external_irq_enable: &UartExternalIrqEnable,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        if self.lifecycle.state() != State::Base
            || uart_external_irq_enable.state() != State::Ready
            || !uart_external_irq_enable.plic_source_gate_open()
            || !uart_external_irq_enable.root_external_input_gate_open()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::ns16550a::uart8250_interrupt_driven_configured()
            || !super::ns16550a::serial8250_runtime_port_ready()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || !super::ns16550a::serial8250_runtime_rx_deferred()
            || !super::ns16550a::tty_flip_buffer_empty()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !super::ns16550a::enable_serial8250_runtime_rx() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.rx_runtime_enabled = super::ns16550a::serial8250_runtime_rx_enabled();
        if !self.rx_runtime_enabled {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if !super::ns16550a::trigger_serial8250_rx_loopback_once(SERIAL8250_RX_LOOPBACK_BYTE) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !wait_serial8250_rx_loopback_closed(
            plic,
            irq_handler_registry,
            baseline,
            source,
            SERIAL8250_RX_LOOPBACK_BYTE,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        self.loopback_stimulus_committed = observed.rx_requests > baseline.rx_requests;
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_received_rx = observed.handler_calls > baseline.handler_calls
            && observed.rx_handled > baseline.rx_handled;
        self.flip_buffer_pushed = observed.flip_pushes > baseline.flip_pushes
            && observed.flip_inserted > baseline.flip_inserted
            && super::ns16550a::tty_flip_buffer_pushed();
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.irq_cycle_closed =
            uart_rx_cycle_completed_and_closed(plic, observed, baseline, source);
        self.last_byte_matched =
            super::ns16550a::serial8250_runtime_rx_last_byte() == SERIAL8250_RX_LOOPBACK_BYTE;

        if !self.loopback_stimulus_committed
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_received_rx
            || !self.flip_buffer_pushed
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.irq_cycle_closed
            || !self.last_byte_matched
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::Serial8250RxLoopbackReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

impl Serial8250RxBatchLoopbackProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            batch_stimulus_committed: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_received_batch: false,
            flip_buffer_batch_pushed: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            irq_cycle_closed: false,
            bounded_drain_observed: false,
            batch_count_matched: false,
            last_byte_matched: false,
            no_overflow_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn batch_stimulus_committed(&self) -> bool {
        self.batch_stimulus_committed
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_received_batch(&self) -> bool {
        self.uart_handler_received_batch
    }

    pub const fn flip_buffer_batch_pushed(&self) -> bool {
        self.flip_buffer_batch_pushed
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn irq_cycle_closed(&self) -> bool {
        self.irq_cycle_closed
    }

    pub const fn bounded_drain_observed(&self) -> bool {
        self.bounded_drain_observed
    }

    pub const fn batch_count_matched(&self) -> bool {
        self.batch_count_matched
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub fn setup(
        &mut self,
        rx_loopback_probe: &Serial8250RxLoopbackProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        let expected_len = SERIAL8250_RX_BATCH_LOOPBACK_BYTES.len();
        let expected_last = SERIAL8250_RX_BATCH_LOOPBACK_BYTES[expected_len - 1];
        if self.lifecycle.state() != State::Base
            || rx_loopback_probe.state() != State::Ready
            || !rx_loopback_probe.rx_runtime_enabled()
            || !rx_loopback_probe.irq_cycle_closed()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::ns16550a::serial8250_runtime_rx_enabled()
            || !super::ns16550a::serial8250_runtime_rx_fifo_enabled()
            || expected_len == 0
            || expected_len > super::ns16550a::serial8250_runtime_rx_drain_limit()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if !super::ns16550a::trigger_serial8250_rx_loopback_batch(
            SERIAL8250_RX_BATCH_LOOPBACK_BYTES,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !wait_serial8250_rx_batch_loopback_closed(
            plic,
            irq_handler_registry,
            baseline,
            source,
            expected_len,
            expected_last,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let inserted_delta = observed
            .flip_inserted
            .saturating_sub(baseline.flip_inserted);
        self.batch_stimulus_committed = observed.rx_requests > baseline.rx_requests;
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_received_batch = observed.handler_calls > baseline.handler_calls
            && observed.rx_handled > baseline.rx_handled;
        self.flip_buffer_batch_pushed = observed.flip_pushes > baseline.flip_pushes
            && inserted_delta == expected_len
            && super::ns16550a::tty_flip_buffer_last_pushed_len() == expected_len;
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.irq_cycle_closed =
            uart_rx_cycle_completed_and_closed(plic, observed, baseline, source);
        self.bounded_drain_observed = expected_len
            <= super::ns16550a::serial8250_runtime_rx_drain_limit()
            && super::ns16550a::tty_flip_buffer_last_pushed_len() == expected_len;
        self.batch_count_matched = inserted_delta == expected_len;
        self.last_byte_matched =
            super::ns16550a::serial8250_runtime_rx_last_byte() == expected_last;
        self.no_overflow_observed = !super::ns16550a::tty_flip_buffer_overflowed();

        if !self.batch_stimulus_committed
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_received_batch
            || !self.flip_buffer_batch_pushed
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.irq_cycle_closed
            || !self.bounded_drain_observed
            || !self.batch_count_matched
            || !self.last_byte_matched
            || !self.no_overflow_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::Serial8250RxBatchLoopbackReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

impl TtyXmitFifoProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            enqueue_committed: false,
            dequeue_committed: false,
            byte_round_trip: false,
            queue_empty_after_dequeue: false,
            distinct_from_printk_console_tx: false,
            runtime_tx_deferred: false,
            printk_tx_queue_unchanged: false,
            no_uart_thri_kick: false,
            no_overflow_observed: false,
            no_underflow_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn enqueue_committed(&self) -> bool {
        self.enqueue_committed
    }

    pub const fn dequeue_committed(&self) -> bool {
        self.dequeue_committed
    }

    pub const fn byte_round_trip(&self) -> bool {
        self.byte_round_trip
    }

    pub const fn queue_empty_after_dequeue(&self) -> bool {
        self.queue_empty_after_dequeue
    }

    pub const fn distinct_from_printk_console_tx(&self) -> bool {
        self.distinct_from_printk_console_tx
    }

    pub const fn runtime_tx_deferred(&self) -> bool {
        self.runtime_tx_deferred
    }

    pub const fn printk_tx_queue_unchanged(&self) -> bool {
        self.printk_tx_queue_unchanged
    }

    pub const fn no_uart_thri_kick(&self) -> bool {
        self.no_uart_thri_kick
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub const fn no_underflow_observed(&self) -> bool {
        self.no_underflow_observed
    }

    pub fn setup(&mut self, rx_batch_probe: &Serial8250RxBatchLoopbackProbe) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rx_batch_probe.state() != State::Ready
            || !rx_batch_probe.no_overflow_observed()
            || !super::ns16550a::tty_xmit_fifo_ready()
            || !super::ns16550a::tty_xmit_fifo_deferred_from_console_tx()
            || !super::ns16550a::serial8250_runtime_port_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_console_queue = super::ns16550a::serial8250_tx_queue_len();
        let baseline_console_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_console_drains = super::ns16550a::serial8250_tx_irq_drain_count();

        if !super::ns16550a::probe_tty_xmit_fifo_round_trip(TTY_XMIT_FIFO_PROBE_BYTE) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.enqueue_committed = super::ns16550a::tty_xmit_fifo_enqueue_count() > baseline_enqueues;
        self.dequeue_committed = super::ns16550a::tty_xmit_fifo_dequeue_count() > baseline_dequeues;
        self.byte_round_trip = super::ns16550a::tty_xmit_fifo_last_enqueued()
            == TTY_XMIT_FIFO_PROBE_BYTE
            && super::ns16550a::tty_xmit_fifo_last_dequeued() == TTY_XMIT_FIFO_PROBE_BYTE
            && super::ns16550a::tty_xmit_fifo_round_trip_ready();
        self.queue_empty_after_dequeue = super::ns16550a::tty_xmit_fifo_queue_len() == 0;
        self.distinct_from_printk_console_tx =
            super::ns16550a::tty_xmit_fifo_deferred_from_console_tx();
        self.runtime_tx_deferred = super::ns16550a::tty_xmit_fifo_deferred_from_console_tx();
        self.printk_tx_queue_unchanged = super::ns16550a::serial8250_tx_queue_len()
            == baseline_console_queue
            && super::ns16550a::serial8250_tx_irq_drain_count() == baseline_console_drains;
        self.no_uart_thri_kick =
            super::ns16550a::serial8250_tx_irq_kick_count() == baseline_console_kicks;
        self.no_overflow_observed = !super::ns16550a::tty_xmit_fifo_overflowed();
        self.no_underflow_observed = !super::ns16550a::tty_xmit_fifo_underflowed();

        if !self.enqueue_committed
            || !self.dequeue_committed
            || !self.byte_round_trip
            || !self.queue_empty_after_dequeue
            || !self.distinct_from_printk_console_tx
            || !self.runtime_tx_deferred
            || !self.printk_tx_queue_unchanged
            || !self.no_uart_thri_kick
            || !self.no_overflow_observed
            || !self.no_underflow_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::TtyXmitFifoProbeReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl TtyWriteRuntimeTxProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            xmit_fifo_enqueued: false,
            start_tx_committed: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_observed: false,
            xmit_fifo_drained: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            queue_empty_after_irq: false,
            printk_tx_queue_unchanged: false,
            local_irq_guard_observed: false,
            last_byte_matched: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn xmit_fifo_enqueued(&self) -> bool {
        self.xmit_fifo_enqueued
    }

    pub const fn start_tx_committed(&self) -> bool {
        self.start_tx_committed
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_observed(&self) -> bool {
        self.uart_handler_observed
    }

    pub const fn xmit_fifo_drained(&self) -> bool {
        self.xmit_fifo_drained
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn queue_empty_after_irq(&self) -> bool {
        self.queue_empty_after_irq
    }

    pub const fn printk_tx_queue_unchanged(&self) -> bool {
        self.printk_tx_queue_unchanged
    }

    pub const fn local_irq_guard_observed(&self) -> bool {
        self.local_irq_guard_observed
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub fn setup(
        &mut self,
        xmit_fifo_probe: &TtyXmitFifoProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        if self.lifecycle.state() != State::Base
            || xmit_fifo_probe.state() != State::Ready
            || !xmit_fifo_probe.runtime_tx_deferred()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::ns16550a::tty_xmit_fifo_ready()
            || !super::ns16550a::serial8250_runtime_port_ready()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || super::ns16550a::serial8250_tx_queue_len() != 0
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_kicks = super::ns16550a::tty_xmit_fifo_runtime_tx_kick_count();
        let baseline_drains = super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count();
        let baseline_console_queue = super::ns16550a::serial8250_tx_queue_len();
        let baseline_console_writes =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();
        let baseline_console_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_console_drains = super::ns16550a::serial8250_tx_irq_drain_count();

        if !super::ns16550a::start_tty_xmit_fifo_runtime_tx(TTY_WRITE_RUNTIME_TX_PROBE_BYTE) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !wait_tty_write_runtime_tx_closed(
            plic,
            irq_handler_registry,
            baseline,
            baseline_drains,
            1,
            source,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        self.xmit_fifo_enqueued =
            super::ns16550a::tty_xmit_fifo_enqueue_count() > baseline_enqueues;
        self.start_tx_committed = super::ns16550a::tty_xmit_fifo_runtime_tx_kick_count()
            > baseline_kicks
            && super::ns16550a::tty_xmit_fifo_runtime_tx_integrated();
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_observed = observed.handler_calls > baseline.handler_calls
            && observed.handled > baseline.handled
            && observed.thri_disabled > baseline.thri_disabled;
        self.xmit_fifo_drained = super::ns16550a::tty_xmit_fifo_dequeue_count() > baseline_dequeues
            && super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count() > baseline_drains;
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.queue_empty_after_irq = super::ns16550a::tty_xmit_fifo_queue_len() == 0
            && super::ns16550a::tty_xmit_fifo_runtime_tx_empty_stop_count() != 0
            && !super::ns16550a::tty_xmit_fifo_overflowed()
            && !super::ns16550a::tty_xmit_fifo_underflowed();
        self.printk_tx_queue_unchanged = super::ns16550a::serial8250_tx_queue_len()
            == baseline_console_queue
            && super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
                == baseline_console_writes
            && super::ns16550a::serial8250_tx_irq_kick_count() == baseline_console_kicks
            && super::ns16550a::serial8250_tx_irq_drain_count() == baseline_console_drains;
        self.local_irq_guard_observed =
            super::ns16550a::tty_xmit_fifo_runtime_tx_guarded_by_local_irq_save();
        self.last_byte_matched = super::ns16550a::tty_xmit_fifo_last_enqueued()
            == TTY_WRITE_RUNTIME_TX_PROBE_BYTE
            && super::ns16550a::tty_xmit_fifo_last_dequeued() == TTY_WRITE_RUNTIME_TX_PROBE_BYTE;

        if !self.xmit_fifo_enqueued
            || !self.start_tx_committed
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_observed
            || !self.xmit_fifo_drained
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.queue_empty_after_irq
            || !self.printk_tx_queue_unchanged
            || !self.local_irq_guard_observed
            || !self.last_byte_matched
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::TtyWriteRuntimeTxReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
impl TtyWriteBatchRuntimeTxProbe {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fixed_bounded_batch: false,
            xmit_fifo_batch_enqueued: false,
            start_tx_committed: false,
            plic_claim_observed: false,
            irq_dispatch_observed: false,
            uart_handler_observed: false,
            xmit_fifo_batch_drained: false,
            plic_complete_observed: false,
            zero_claim_loop_exit_observed: false,
            queue_empty_after_irq: false,
            printk_tx_queue_unchanged: false,
            local_irq_guard_observed: false,
            bounded_drain_observed: false,
            batch_count_matched: false,
            last_byte_matched: false,
            no_overflow_observed: false,
            no_underflow_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fixed_bounded_batch(&self) -> bool {
        self.fixed_bounded_batch
    }

    pub const fn xmit_fifo_batch_enqueued(&self) -> bool {
        self.xmit_fifo_batch_enqueued
    }

    pub const fn start_tx_committed(&self) -> bool {
        self.start_tx_committed
    }

    pub const fn plic_claim_observed(&self) -> bool {
        self.plic_claim_observed
    }

    pub const fn irq_dispatch_observed(&self) -> bool {
        self.irq_dispatch_observed
    }

    pub const fn uart_handler_observed(&self) -> bool {
        self.uart_handler_observed
    }

    pub const fn xmit_fifo_batch_drained(&self) -> bool {
        self.xmit_fifo_batch_drained
    }

    pub const fn plic_complete_observed(&self) -> bool {
        self.plic_complete_observed
    }

    pub const fn zero_claim_loop_exit_observed(&self) -> bool {
        self.zero_claim_loop_exit_observed
    }

    pub const fn queue_empty_after_irq(&self) -> bool {
        self.queue_empty_after_irq
    }

    pub const fn printk_tx_queue_unchanged(&self) -> bool {
        self.printk_tx_queue_unchanged
    }

    pub const fn local_irq_guard_observed(&self) -> bool {
        self.local_irq_guard_observed
    }

    pub const fn bounded_drain_observed(&self) -> bool {
        self.bounded_drain_observed
    }

    pub const fn batch_count_matched(&self) -> bool {
        self.batch_count_matched
    }

    pub const fn last_byte_matched(&self) -> bool {
        self.last_byte_matched
    }

    pub const fn no_overflow_observed(&self) -> bool {
        self.no_overflow_observed
    }

    pub const fn no_underflow_observed(&self) -> bool {
        self.no_underflow_observed
    }

    pub fn setup(
        &mut self,
        runtime_tx_probe: &TtyWriteRuntimeTxProbe,
        plic: &Plic,
        plic_irq_domain: &PlicIrqDomain,
        irq_handler_registry: &IrqHandlerRegistry,
    ) -> EventResult {
        let source = super::ns16550a::uart8250_port_irq_source();
        let logical_irq = super::ns16550a::uart8250_port_logical_irq();
        let expected_len = TTY_WRITE_RUNTIME_TX_BATCH_BYTES.len();
        let expected_last = TTY_WRITE_RUNTIME_TX_BATCH_BYTES[expected_len - 1];
        if self.lifecycle.state() != State::Base
            || runtime_tx_probe.state() != State::Ready
            || !runtime_tx_probe.queue_empty_after_irq()
            || !runtime_tx_probe.printk_tx_queue_unchanged()
            || plic.state() != State::Ready
            || plic_irq_domain.state() != State::Ready
            || irq_handler_registry.state() != State::Ready
            || !super::ns16550a::tty_xmit_fifo_ready()
            || !super::ns16550a::tty_xmit_fifo_runtime_tx_integrated()
            || !super::ns16550a::serial8250_runtime_port_ready()
            || !super::ns16550a::serial8250_runtime_console_tx_ready()
            || super::ns16550a::tty_xmit_fifo_queue_len() != 0
            || super::ns16550a::serial8250_tx_queue_len() != 0
            || expected_len <= 1
            || expected_len > super::ns16550a::tty_xmit_fifo_capacity()
            || !logical_irq.is_valid()
            || plic_irq_domain
                .mapping_for_source(source)
                .is_none_or(|mapping| {
                    mapping.logical_irq() != logical_irq || !mapping.source_gate_open()
                })
            || !irq_handler_registry.has_handler_for_logical_irq(logical_irq)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let baseline = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let baseline_enqueues = super::ns16550a::tty_xmit_fifo_enqueue_count();
        let baseline_dequeues = super::ns16550a::tty_xmit_fifo_dequeue_count();
        let baseline_kicks = super::ns16550a::tty_xmit_fifo_runtime_tx_kick_count();
        let baseline_drains = super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count();
        let baseline_console_queue = super::ns16550a::serial8250_tx_queue_len();
        let baseline_console_writes =
            super::ns16550a::serial8250_write_call_count_available_for_irq_probe();
        let baseline_console_kicks = super::ns16550a::serial8250_tx_irq_kick_count();
        let baseline_console_drains = super::ns16550a::serial8250_tx_irq_drain_count();

        if !super::ns16550a::start_tty_xmit_fifo_runtime_tx_batch(TTY_WRITE_RUNTIME_TX_BATCH_BYTES)
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !wait_tty_write_runtime_tx_closed(
            plic,
            irq_handler_registry,
            baseline,
            baseline_drains,
            expected_len,
            source,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let observed = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let enqueue_delta =
            super::ns16550a::tty_xmit_fifo_enqueue_count().saturating_sub(baseline_enqueues);
        let dequeue_delta =
            super::ns16550a::tty_xmit_fifo_dequeue_count().saturating_sub(baseline_dequeues);
        let drain_delta =
            super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count().saturating_sub(baseline_drains);
        self.fixed_bounded_batch =
            expected_len > 1 && expected_len <= super::ns16550a::tty_xmit_fifo_capacity();
        self.xmit_fifo_batch_enqueued = enqueue_delta == expected_len;
        self.start_tx_committed = super::ns16550a::tty_xmit_fifo_runtime_tx_kick_count()
            > baseline_kicks
            && super::ns16550a::tty_xmit_fifo_runtime_tx_integrated();
        self.plic_claim_observed =
            observed.claims > baseline.claims && plic.last_claimed_source() == source;
        self.irq_dispatch_observed = observed.plic_dispatches > baseline.plic_dispatches
            && observed.irq_dispatches > baseline.irq_dispatches;
        self.uart_handler_observed = observed.handler_calls > baseline.handler_calls
            && observed.handled > baseline.handled
            && observed.thri_disabled > baseline.thri_disabled;
        self.xmit_fifo_batch_drained = dequeue_delta == expected_len && drain_delta == expected_len;
        self.plic_complete_observed =
            observed.completes > baseline.completes && plic.last_completed_source() == source;
        self.zero_claim_loop_exit_observed = observed.zero_claims > baseline.zero_claims
            && observed.loop_exits > baseline.loop_exits;
        self.queue_empty_after_irq = super::ns16550a::tty_xmit_fifo_queue_len() == 0
            && super::ns16550a::tty_xmit_fifo_runtime_tx_empty_stop_count() != 0
            && !super::ns16550a::tty_xmit_fifo_overflowed()
            && !super::ns16550a::tty_xmit_fifo_underflowed();
        self.printk_tx_queue_unchanged = super::ns16550a::serial8250_tx_queue_len()
            == baseline_console_queue
            && super::ns16550a::serial8250_write_call_count_available_for_irq_probe()
                == baseline_console_writes
            && super::ns16550a::serial8250_tx_irq_kick_count() == baseline_console_kicks
            && super::ns16550a::serial8250_tx_irq_drain_count() == baseline_console_drains;
        self.local_irq_guard_observed =
            super::ns16550a::tty_xmit_fifo_runtime_tx_guarded_by_local_irq_save();
        self.bounded_drain_observed = drain_delta == expected_len
            && expected_len <= super::ns16550a::tty_xmit_fifo_capacity();
        self.batch_count_matched = enqueue_delta == expected_len
            && dequeue_delta == expected_len
            && drain_delta == expected_len;
        self.last_byte_matched = super::ns16550a::tty_xmit_fifo_last_enqueued() == expected_last
            && super::ns16550a::tty_xmit_fifo_last_dequeued() == expected_last;
        self.no_overflow_observed = !super::ns16550a::tty_xmit_fifo_overflowed();
        self.no_underflow_observed = !super::ns16550a::tty_xmit_fifo_underflowed();

        if !self.fixed_bounded_batch
            || !self.xmit_fifo_batch_enqueued
            || !self.start_tx_committed
            || !self.plic_claim_observed
            || !self.irq_dispatch_observed
            || !self.uart_handler_observed
            || !self.xmit_fifo_batch_drained
            || !self.plic_complete_observed
            || !self.zero_claim_loop_exit_observed
            || !self.queue_empty_after_irq
            || !self.printk_tx_queue_unchanged
            || !self.local_irq_guard_observed
            || !self.bounded_drain_observed
            || !self.batch_count_matched
            || !self.last_byte_matched
            || !self.no_overflow_observed
            || !self.no_underflow_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        crate::trace::checkpoint(Checkpoint::TtyWriteBatchRuntimeTxReady);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

fn uart_irq_cycle_snapshot(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
) -> UartIrqCycleSnapshot {
    UartIrqCycleSnapshot {
        requests: super::ns16550a::uart8250_thre_interrupt_request_count(),
        rx_requests: super::ns16550a::uart8250_rx_interrupt_request_count(),
        claims: plic.claim_count(),
        zero_claims: plic.zero_claim_count(),
        plic_dispatches: plic.dispatch_count(),
        completes: plic.complete_count(),
        loop_exits: plic.loop_exit_count(),
        irq_dispatches: irq_handler_registry.dispatch_calls(),
        handler_calls: super::ns16550a::uart8250_irq_handler_call_count(),
        handled: super::ns16550a::uart8250_thre_interrupt_handled_count(),
        rx_handled: super::ns16550a::uart8250_rx_interrupt_handled_count(),
        thri_disabled: super::ns16550a::uart8250_thri_disabled_by_handler_count(),
        flip_pushes: super::ns16550a::tty_flip_buffer_push_count(),
        flip_inserted: super::ns16550a::tty_flip_buffer_total_inserted(),
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn wait_tty_write_runtime_tx_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    baseline_drains: usize,
    expected_drain_delta: usize,
    source: u32,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if current.requests > baseline.requests
            && current.claims > baseline.claims
            && current.plic_dispatches > baseline.plic_dispatches
            && current.irq_dispatches > baseline.irq_dispatches
            && current.handler_calls > baseline.handler_calls
            && current.handled > baseline.handled
            && current.thri_disabled > baseline.thri_disabled
            && current.completes > baseline.completes
            && current.zero_claims > baseline.zero_claims
            && current.loop_exits > baseline.loop_exits
            && super::ns16550a::tty_xmit_fifo_runtime_tx_drain_count()
                >= baseline_drains.saturating_add(expected_drain_delta)
            && super::ns16550a::tty_xmit_fifo_queue_len() == 0
            && plic.last_claimed_source() == source
            && plic.last_completed_source() == source
            && irq_handler_registry.dispatch_calls() > baseline.irq_dispatches
        {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn wait_serial8250_tx_quiesce_window() {
    let mut spins = 0usize;
    while spins < UART_IRQ_CYCLE_SPIN_LIMIT / 16 {
        core::hint::spin_loop();
        spins += 1;
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_burst_expected_tx_bytes() -> usize {
    let mut total = 0usize;
    for message in SERIAL8250_BURST_IRQ_TX_PROBE_MESSAGES {
        total = total.saturating_add(serial8250_console_message_expected_tx_bytes(message));
    }
    total
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_message_expected_tx_bytes(message: &str) -> usize {
    message.len().saturating_add(count_newlines(message))
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_burst_expected_crlf_insertions() -> usize {
    let mut total = 0usize;
    for message in SERIAL8250_BURST_IRQ_TX_PROBE_MESSAGES {
        total = total.saturating_add(count_newlines(message));
    }
    total
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_burst_expected_last_byte() -> u8 {
    let Some(last_message) = SERIAL8250_BURST_IRQ_TX_PROBE_MESSAGES.last() else {
        return 0;
    };
    last_message.as_bytes().last().copied().unwrap_or(0)
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_long_burst_expected_tx_bytes() -> usize {
    let mut total = 0usize;
    for message in SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES {
        total = total.saturating_add(serial8250_console_message_expected_tx_bytes(message));
    }
    total
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_long_burst_expected_crlf_insertions() -> usize {
    let mut total = 0usize;
    for message in SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES {
        total = total.saturating_add(count_newlines(message));
    }
    total
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_long_burst_expected_last_byte() -> u8 {
    let Some(last_message) = SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES.last() else {
        return 0;
    };
    last_message.as_bytes().last().copied().unwrap_or(0)
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_long_burst_each_record_exceeds_single_load(tx_load_size: usize) -> bool {
    if tx_load_size == 0 {
        return false;
    }
    for message in SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES {
        if serial8250_console_message_expected_tx_bytes(message) <= tx_load_size {
            return false;
        }
    }
    true
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_long_burst_expected_irq_rounds(tx_load_size: usize) -> usize {
    let mut total = 0usize;
    for message in SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES {
        total = total.saturating_add(serial8250_expected_tx_irq_rounds(
            serial8250_console_message_expected_tx_bytes(message),
            tx_load_size,
        ));
    }
    total
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_console_long_burst_expected_budget_hits(tx_load_size: usize) -> usize {
    let mut total = 0usize;
    for message in SERIAL8250_LONG_BURST_IRQ_TX_PROBE_MESSAGES {
        let rounds = serial8250_expected_tx_irq_rounds(
            serial8250_console_message_expected_tx_bytes(message),
            tx_load_size,
        );
        total = total.saturating_add(rounds.saturating_sub(1));
    }
    total
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn serial8250_expected_tx_irq_rounds(total_bytes: usize, tx_load_size: usize) -> usize {
    if total_bytes == 0 || tx_load_size == 0 {
        return 0;
    }
    total_bytes.saturating_add(tx_load_size - 1) / tx_load_size
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn count_newlines(message: &str) -> usize {
    let mut count = 0usize;
    for byte in message.as_bytes() {
        if *byte == b'\n' {
            count = count.saturating_add(1);
        }
    }
    count
}

fn wait_serial8250_rx_batch_loopback_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    source: u32,
    expected_len: usize,
    expected_last: u8,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if current.rx_requests > baseline.rx_requests
            && current.claims > baseline.claims
            && current.plic_dispatches > baseline.plic_dispatches
            && current.irq_dispatches > baseline.irq_dispatches
            && current.handler_calls > baseline.handler_calls
            && current.rx_handled > baseline.rx_handled
            && current.flip_pushes > baseline.flip_pushes
            && current.flip_inserted.saturating_sub(baseline.flip_inserted) == expected_len
            && current.completes > baseline.completes
            && current.zero_claims > baseline.zero_claims
            && current.loop_exits > baseline.loop_exits
            && super::ns16550a::serial8250_runtime_rx_enabled()
            && super::ns16550a::serial8250_runtime_rx_fifo_enabled()
            && super::ns16550a::tty_flip_buffer_last_pushed_len() == expected_len
            && super::ns16550a::serial8250_runtime_rx_last_byte() == expected_last
            && !super::ns16550a::tty_flip_buffer_overflowed()
            && plic.last_claimed_source() == source
            && plic.last_completed_source() == source
            && irq_handler_registry.dispatch_calls() > baseline.irq_dispatches
        {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

fn wait_serial8250_rx_loopback_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    source: u32,
    expected_byte: u8,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if current.rx_requests > baseline.rx_requests
            && current.claims > baseline.claims
            && current.plic_dispatches > baseline.plic_dispatches
            && current.irq_dispatches > baseline.irq_dispatches
            && current.handler_calls > baseline.handler_calls
            && current.rx_handled > baseline.rx_handled
            && current.flip_pushes > baseline.flip_pushes
            && current.flip_inserted > baseline.flip_inserted
            && current.completes > baseline.completes
            && current.zero_claims > baseline.zero_claims
            && current.loop_exits > baseline.loop_exits
            && super::ns16550a::serial8250_runtime_rx_enabled()
            && super::ns16550a::serial8250_runtime_rx_last_byte() == expected_byte
            && plic.last_claimed_source() == source
            && plic.last_completed_source() == source
            && irq_handler_registry.dispatch_calls() > baseline.irq_dispatches
        {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

fn uart_rx_cycle_completed_and_closed(
    plic: &Plic,
    current: UartIrqCycleSnapshot,
    baseline: UartIrqCycleSnapshot,
    source: u32,
) -> bool {
    let claim_delta = current.claims.saturating_sub(baseline.claims);
    let complete_delta = current.completes.saturating_sub(baseline.completes);
    let zero_claim_delta = current.zero_claims.saturating_sub(baseline.zero_claims);
    let loop_exit_delta = current.loop_exits.saturating_sub(baseline.loop_exits);

    current.rx_requests > baseline.rx_requests
        && current.claims > baseline.claims
        && current.plic_dispatches > baseline.plic_dispatches
        && current.irq_dispatches > baseline.irq_dispatches
        && current.handler_calls > baseline.handler_calls
        && current.rx_handled > baseline.rx_handled
        && current.flip_pushes > baseline.flip_pushes
        && current.flip_inserted > baseline.flip_inserted
        && current.completes > baseline.completes
        && current.zero_claims > baseline.zero_claims
        && current.loop_exits > baseline.loop_exits
        && claim_delta == complete_delta
        && zero_claim_delta == loop_exit_delta
        && plic.last_claimed_source() == source
        && plic.last_completed_source() == source
}

fn wait_uart_irq_cycle_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    source: u32,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if uart_irq_cycle_completed_and_closed(plic, current, baseline, source) {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn wait_serial8250_irq_tx_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    baseline_drains: usize,
    expected_tx_bytes: usize,
    source: u32,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        if current.requests > baseline.requests
            && current.claims > baseline.claims
            && current.plic_dispatches > baseline.plic_dispatches
            && current.irq_dispatches > baseline.irq_dispatches
            && current.handler_calls > baseline.handler_calls
            && current.handled > baseline.handled
            && current.thri_disabled > baseline.thri_disabled
            && current.completes > baseline.completes
            && current.zero_claims > baseline.zero_claims
            && current.loop_exits > baseline.loop_exits
            && super::ns16550a::serial8250_tx_irq_drain_count()
                >= baseline_drains.saturating_add(expected_tx_bytes)
            && super::ns16550a::serial8250_tx_queue_len() == 0
            && plic.last_claimed_source() == source
            && plic.last_completed_source() == source
            && irq_handler_registry.dispatch_calls() > baseline.irq_dispatches
        {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn wait_serial8250_long_irq_tx_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    baseline_drains: usize,
    baseline_budget_hits: usize,
    expected_tx_bytes: usize,
    source: u32,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let claim_delta = current.claims.saturating_sub(baseline.claims);
        let complete_delta = current.completes.saturating_sub(baseline.completes);
        let handler_delta = current.handler_calls.saturating_sub(baseline.handler_calls);
        if current.requests > baseline.requests
            && claim_delta >= 2
            && current.plic_dispatches > baseline.plic_dispatches
            && current.irq_dispatches > baseline.irq_dispatches
            && handler_delta >= 2
            && current.handled.saturating_sub(baseline.handled) >= 2
            && current.thri_disabled > baseline.thri_disabled
            && complete_delta >= 2
            && current.zero_claims > baseline.zero_claims
            && current.loop_exits > baseline.loop_exits
            && super::ns16550a::serial8250_tx_irq_drain_count()
                >= baseline_drains.saturating_add(expected_tx_bytes)
            && super::ns16550a::serial8250_tx_irq_budget_hit_count() > baseline_budget_hits
            && super::ns16550a::serial8250_tx_queue_len() == 0
            && plic.last_claimed_source() == source
            && plic.last_completed_source() == source
            && irq_handler_registry.dispatch_calls() > baseline.irq_dispatches
        {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn wait_serial8250_long_burst_irq_tx_closed(
    plic: &Plic,
    irq_handler_registry: &IrqHandlerRegistry,
    baseline: UartIrqCycleSnapshot,
    baseline_drains: usize,
    baseline_budget_hits: usize,
    expected_tx_bytes: usize,
    expected_rounds: usize,
    expected_budget_hits: usize,
    source: u32,
) -> bool {
    let mut spins = 0usize;

    while spins < UART_IRQ_CYCLE_SPIN_LIMIT {
        let current = uart_irq_cycle_snapshot(plic, irq_handler_registry);
        let claim_delta = current.claims.saturating_sub(baseline.claims);
        let plic_dispatch_delta = current
            .plic_dispatches
            .saturating_sub(baseline.plic_dispatches);
        let irq_dispatch_delta = current
            .irq_dispatches
            .saturating_sub(baseline.irq_dispatches);
        let complete_delta = current.completes.saturating_sub(baseline.completes);
        let handler_delta = current.handler_calls.saturating_sub(baseline.handler_calls);
        let handled_delta = current.handled.saturating_sub(baseline.handled);
        let budget_hit_delta = super::ns16550a::serial8250_tx_irq_budget_hit_count()
            .saturating_sub(baseline_budget_hits);
        if current.requests > baseline.requests
            && claim_delta >= expected_rounds
            && plic_dispatch_delta >= expected_rounds
            && irq_dispatch_delta >= expected_rounds
            && handler_delta >= expected_rounds
            && handled_delta >= expected_rounds
            && current.thri_disabled > baseline.thri_disabled
            && complete_delta >= expected_rounds
            && current.zero_claims > baseline.zero_claims
            && current.loop_exits > baseline.loop_exits
            && super::ns16550a::serial8250_tx_irq_drain_count()
                >= baseline_drains.saturating_add(expected_tx_bytes)
            && budget_hit_delta >= expected_budget_hits
            && super::ns16550a::serial8250_tx_queue_len() == 0
            && plic.last_claimed_source() == source
            && plic.last_completed_source() == source
            && irq_handler_registry
                .dispatch_calls()
                .saturating_sub(baseline.irq_dispatches)
                >= expected_rounds
        {
            return true;
        }

        core::hint::spin_loop();
        spins += 1;
    }

    false
}

fn uart_irq_cycle_completed_and_closed(
    plic: &Plic,
    current: UartIrqCycleSnapshot,
    baseline: UartIrqCycleSnapshot,
    source: u32,
) -> bool {
    let claim_delta = current.claims.saturating_sub(baseline.claims);
    let complete_delta = current.completes.saturating_sub(baseline.completes);
    let zero_claim_delta = current.zero_claims.saturating_sub(baseline.zero_claims);
    let loop_exit_delta = current.loop_exits.saturating_sub(baseline.loop_exits);

    current.requests > baseline.requests
        && current.claims > baseline.claims
        && current.plic_dispatches > baseline.plic_dispatches
        && current.irq_dispatches > baseline.irq_dispatches
        && current.handler_calls > baseline.handler_calls
        && current.handled > baseline.handled
        && current.thri_disabled > baseline.thri_disabled
        && current.completes > baseline.completes
        && current.zero_claims > baseline.zero_claims
        && current.loop_exits > baseline.loop_exits
        && claim_delta == complete_delta
        && zero_claim_delta == loop_exit_delta
        && plic.last_claimed_source() == source
        && plic.last_completed_source() == source
}

fn find_plic_interrupt_controller_node(device_tree: &DeviceTree) -> Option<DeviceNodeRef<'_>> {
    let root = device_tree.root()?;
    find_plic_interrupt_controller_node_from(root)
}

fn find_plic_interrupt_controller_node_from(node: DeviceNodeRef<'_>) -> Option<DeviceNodeRef<'_>> {
    if node.property(b"interrupt-controller").is_some()
        && plic_node_matches_supported_compatible(node)
    {
        return Some(node);
    }

    for child in node.children() {
        if let Some(found) = find_plic_interrupt_controller_node_from(child) {
            return Some(found);
        }
    }

    None
}

pub struct SmpCallFunction {
    lifecycle: Lifecycle,
    call_single_queue_ready: bool,
    ipi_route_ready: bool,
    ipi_mux_ready: bool,
    possible_cpu_count: usize,
}

impl SmpCallFunction {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            call_single_queue_ready: false,
            ipi_route_ready: false,
            ipi_mux_ready: false,
            possible_cpu_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn call_single_queue_ready(&self) -> bool {
        self.call_single_queue_ready
    }

    pub const fn ipi_route_ready(&self) -> bool {
        self.ipi_route_ready
    }

    pub const fn ipi_mux_ready(&self) -> bool {
        self.ipi_mux_ready
    }

    pub const fn possible_cpu_count(&self) -> usize {
        self.possible_cpu_count
    }

    pub fn setup(
        &mut self,
        ipi_mux: &IpiMux,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || ipi_mux.state() != State::Ready
            || cpu_group.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || !ipi_mux.virtual_ipi_range_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.call_single_queue_ready = true;
        self.ipi_route_ready = true;
        self.ipi_mux_ready = true;
        self.possible_cpu_count = cpu_group.possible_cpu_count();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SmpCallFunctionReady,
        )
    }
}

pub fn timer_interrupt_count() -> usize {
    TIMER_INTERRUPT_COUNT.load(Ordering::Relaxed)
}

pub fn handle_external_interrupt() {
    let ctx = crate::context::context_ref();
    super::plic_provider::handle_external_interrupt(
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    );
}

pub fn handle_timer_interrupt() {
    riscv64::csr::disable_supervisor_timer_interrupt();
    TIMER_INTERRUPT_COUNT.fetch_add(1, Ordering::Relaxed);
    let callback = ONESHOT_CALLBACK.swap(0, Ordering::AcqRel);
    let deadline = ONESHOT_DEADLINE.swap(0, Ordering::AcqRel);
    if callback != 0 {
        let callback: ClockEventCallback = unsafe { core::mem::transmute(callback) };
        callback(deadline);
    }
}

fn read_timebase_frequency(device_tree: &DeviceTree) -> Option<u64> {
    let cpus = device_tree.find_node(b"/cpus")?;
    if let Some(value) = cpus.property(b"timebase-frequency") {
        return read_u32_property(value.raw_value()).map(u64::from);
    }

    for cpu in cpus.children() {
        if let Some(value) = cpu.property(b"timebase-frequency") {
            return read_u32_property(value.raw_value()).map(u64::from);
        }
    }

    None
}

fn read_cells_u32(property: Option<DevicePropertyRef<'_>>) -> Option<usize> {
    usize::try_from(read_property_u32(property)?).ok()
}

fn read_property_u32(property: Option<DevicePropertyRef<'_>>) -> Option<u32> {
    read_u32_property(property?.raw_value())
}

fn read_u32_property(value: &[u8]) -> Option<u32> {
    let start = value.as_ptr() as usize;
    let end = start.checked_add(value.len())?;
    read_be_u32(start, end)
}
