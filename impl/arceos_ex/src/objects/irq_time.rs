use core::{
    mem::size_of,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

use super::{
    config::Config,
    cpu_group::CpuGroup,
    device_tree::{DeviceNodeRef, DevicePropertyRef, DeviceTree},
    fdt_reader::{read_be_u32, read_cells},
    interrupt_stream::InterruptStream,
    ioremap::Ioremap,
    mm_core::{PageAllocator, PageMetadataMap, PageTableCaches, SlubAllocator, VmallocAllocator},
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
const RISCV_IRQ_S_EXT: u32 = 9;
const RISCV_IRQ_M_EXT: u32 = 11;

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
        slub_allocator: &SlubAllocator,
        per_cpu_storage: &PerCpuStorage,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || page_allocator.state() != State::Ready
            || slub_allocator.state() != State::Ready
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
    external_route_deferred: bool,
    boot_cpu_route_ready: bool,
}

impl IrqDispatchTree {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fallback_route_ready: false,
            timer_route_ready: false,
            software_route_reserved: false,
            external_route_deferred: false,
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

    pub const fn external_route_deferred(&self) -> bool {
        self.external_route_deferred
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
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || irq_controller.state() != State::Ready
            || riscv_intc.state() != State::Ready
            || interrupt_stream.state() != State::Ready
            || cpu_group.state() != State::Ready
            || !riscv_intc.boot_cpu_timer_irq_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        interrupt_stream.bind_timer_handler()?;
        self.fallback_route_ready = true;
        self.timer_route_ready = true;
        self.software_route_reserved = true;
        self.external_route_deferred = true;
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
    external_input_context_ready: bool,
    threshold_ready: bool,
    priority_ready: bool,
    source_enable_ready: bool,
    external_irq_route_deferred: bool,
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
            external_input_context_ready: false,
            threshold_ready: false,
            priority_ready: false,
            source_enable_ready: false,
            external_irq_route_deferred: false,
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

    pub const fn external_irq_route_deferred(&self) -> bool {
        self.external_irq_route_deferred
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
            || !plic_context_parent_has_external_input(node)
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
        self.external_input_context_ready = true;
        self.threshold_ready = true;
        self.priority_ready = true;
        self.source_enable_ready = true;
        self.external_irq_route_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Ready,
            Checkpoint::PlicReady,
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

fn plic_context_parent_has_external_input(node: DeviceNodeRef<'_>) -> bool {
    let Some(property) = node.property(b"interrupts-extended") else {
        return false;
    };
    let value = property.raw_value();
    let start = value.as_ptr() as usize;
    let Some(end) = start.checked_add(value.len()) else {
        return false;
    };
    if value.len() < 8 {
        return false;
    }

    let mut cursor = start;
    while cursor + 8 <= end {
        let Some(_phandle) = read_be_u32(cursor, end) else {
            return false;
        };
        let Some(cause) = read_be_u32(cursor + 4, end) else {
            return false;
        };
        if cause == RISCV_IRQ_S_EXT || cause == RISCV_IRQ_M_EXT {
            return true;
        }
        cursor += 8;
    }
    false
}

fn plic_node_matches_supported_compatible(node: DeviceNodeRef<'_>) -> bool {
    node.has_compatible(PLIC_COMPATIBLE_SIFIVE) || node.has_compatible(PLIC_COMPATIBLE_RISCV)
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
