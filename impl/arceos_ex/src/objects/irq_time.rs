use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::{
    cpu_group::CpuGroup,
    device_tree::DeviceTree,
    fdt_reader::read_be_u32,
    interrupt_stream::InterruptStream,
    mm_core::{PageAllocator, SlubAllocator},
    per_cpu_storage::PerCpuStorage,
    sbi::Sbi,
    softirq::Softirq,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::StaticBranch,
};
use crate::{arch::riscv64, trace::Checkpoint};

const DEFAULT_TIMEBASE_HZ: u64 = 10_000_000;

static TIMER_INTERRUPT_COUNT: AtomicUsize = AtomicUsize::new(0);
static ONESHOT_DEADLINE: AtomicU64 = AtomicU64::new(0);
static ONESHOT_CALLBACK: AtomicUsize = AtomicUsize::new(0);

pub type ClockEventCallback = fn(u64);

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
    provider_discovery_reserved: bool,
    external_parent_reserved: bool,
    external_irq_route_deferred: bool,
}

impl Plic {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            provider_discovery_reserved: false,
            external_parent_reserved: false,
            external_irq_route_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn provider_discovery_reserved(&self) -> bool {
        self.provider_discovery_reserved
    }

    pub const fn external_parent_reserved(&self) -> bool {
        self.external_parent_reserved
    }

    pub const fn external_irq_route_deferred(&self) -> bool {
        self.external_irq_route_deferred
    }

    pub fn preset(&mut self, device_tree: &DeviceTree, riscv_intc: &RiscvIntc) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || riscv_intc.state() != State::Ready
            || !riscv_intc.boot_cpu_external_irq_reserved()
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.provider_discovery_reserved = true;
        self.external_parent_reserved = true;
        self.external_irq_route_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PlicPrepared,
        )
    }
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

fn read_u32_property(value: &[u8]) -> Option<u32> {
    let start = value.as_ptr() as usize;
    let end = start.checked_add(value.len())?;
    read_be_u32(start, end)
}
