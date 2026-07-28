use super::{
    config::Config,
    cpu_group::CpuGroup,
    earlycon,
    interrupt_type::InterruptType,
    irq_time::{HrtimerCore, RiscvTimerProvider, Timekeeper},
    mm_core::{KmallocCaches, PageAllocator, SlubSubsystem},
    printk,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_branch::StaticBranch,
    static_objects::StaticObjects,
};
use crate::checkpoint::Checkpoint;

const SCHED_CLOCK_TIMER_PERIOD: u64 = 1_000_000;
const N_TTY_LINE_DISCIPLINE: usize = 0;

pub struct Console {
    lifecycle: Lifecycle,
    line_discipline_registry: TtyLineDisciplineRegistry,
    driver_set: ConsoleDriverSet,
    initcall_table_scanned: bool,
    real_device_probe_deferred: bool,
    earlycon_handoff_conditional: bool,
    registry_ready: bool,
    boot_console_registered: bool,
    serial_console_registered: bool,
    preferred_console_from_stdout: bool,
    printk_route_serial_console: bool,
    boot_console_unregistered: bool,
    handoff_complete: bool,
}

// Console and timing observation getters are exercised by smoke/KUnit configurations.
#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Console {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            line_discipline_registry: TtyLineDisciplineRegistry::new(),
            driver_set: ConsoleDriverSet::new(),
            initcall_table_scanned: false,
            real_device_probe_deferred: false,
            earlycon_handoff_conditional: false,
            registry_ready: false,
            boot_console_registered: false,
            serial_console_registered: false,
            preferred_console_from_stdout: false,
            printk_route_serial_console: false,
            boot_console_unregistered: false,
            handoff_complete: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn line_discipline_registry(&self) -> &TtyLineDisciplineRegistry {
        &self.line_discipline_registry
    }

    pub const fn driver_set(&self) -> &ConsoleDriverSet {
        &self.driver_set
    }

    pub const fn initcall_table_scanned(&self) -> bool {
        self.initcall_table_scanned
    }

    pub const fn real_device_probe_deferred(&self) -> bool {
        self.real_device_probe_deferred
    }

    pub const fn earlycon_handoff_conditional(&self) -> bool {
        self.earlycon_handoff_conditional
    }

    pub const fn registry_ready(&self) -> bool {
        self.registry_ready
    }

    pub const fn boot_console_registered(&self) -> bool {
        self.boot_console_registered
    }

    pub const fn serial_console_registered(&self) -> bool {
        self.serial_console_registered
    }

    pub const fn preferred_console_from_stdout(&self) -> bool {
        self.preferred_console_from_stdout
    }

    pub const fn printk_route_serial_console(&self) -> bool {
        self.printk_route_serial_console
    }

    pub const fn boot_console_unregistered(&self) -> bool {
        self.boot_console_unregistered
    }

    pub const fn handoff_complete(&self) -> bool {
        self.handoff_complete
    }

    pub fn preset(&mut self, static_objects: &StaticObjects) -> EventResult {
        if self.lifecycle.state() != State::Base
            || !printk::is_ready()
            || !(earlycon::is_online() || printk::console_handoff_complete())
            || static_objects.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.line_discipline_registry.preset()?;
        self.driver_set.preset(static_objects)?;
        self.initcall_table_scanned = true;
        self.real_device_probe_deferred = true;
        self.earlycon_handoff_conditional = true;

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::ConsolePrepared,
        )
    }

    pub fn refresh_handoff(&mut self) {
        self.registry_ready = printk::boot_console_registered()
            && printk::serial8250_console_registered()
            && printk::preferred_console_from_stdout();
        self.boot_console_registered = printk::boot_console_registered();
        self.serial_console_registered = printk::serial8250_console_registered();
        self.preferred_console_from_stdout = printk::preferred_console_from_stdout();
        self.printk_route_serial_console = printk::route() == printk::PrintkRoute::Serial8250;
        self.boot_console_unregistered =
            printk::boot_console_unregistered() && printk::boot_console_removed_from_registry();
        self.handoff_complete = printk::console_handoff_complete();
        if self.handoff_complete {
            self.real_device_probe_deferred = false;
            self.earlycon_handoff_conditional = false;
        }
    }
}

pub struct TtyLineDisciplineRegistry {
    lifecycle: Lifecycle,
    n_tty_registered: bool,
    registered_slot: usize,
}

impl TtyLineDisciplineRegistry {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            n_tty_registered: false,
            registered_slot: usize::MAX,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn n_tty_registered(&self) -> bool {
        self.n_tty_registered
    }

    pub const fn registered_slot(&self) -> usize {
        self.registered_slot
    }

    fn preset(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.n_tty_registered = true;
        self.registered_slot = N_TTY_LINE_DISCIPLINE;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TtyLineDisciplineRegistryPrepared,
        )
    }
}

pub struct ConsoleDriverSet {
    lifecycle: Lifecycle,
    early_registered: bool,
    serial_probe_deferred: bool,
    boot_console_unregister_deferred: bool,
}

impl ConsoleDriverSet {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            early_registered: false,
            serial_probe_deferred: false,
            boot_console_unregister_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn early_registered(&self) -> bool {
        self.early_registered
    }

    pub const fn serial_probe_deferred(&self) -> bool {
        self.serial_probe_deferred
    }

    pub const fn boot_console_unregister_deferred(&self) -> bool {
        self.boot_console_unregister_deferred
    }

    fn preset(&mut self, static_objects: &StaticObjects) -> EventResult {
        if self.lifecycle.state() != State::Base || static_objects.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.early_registered = true;
        self.serial_probe_deferred = true;
        self.boot_console_unregister_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::ConsoleDriverSetPrepared,
        )
    }
}

pub struct SchedClock {
    lifecycle: Lifecycle,
    running_key_enabled: bool,
    reader_ready: bool,
    timer_ready: bool,
    timer_period: u64,
    setup_local_irq_disable_enable_used: bool,
    setup_local_irq_guard_bound_to_boot_cpu: bool,
}

impl SchedClock {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            running_key_enabled: false,
            reader_ready: false,
            timer_ready: false,
            timer_period: 0,
            setup_local_irq_disable_enable_used: false,
            setup_local_irq_guard_bound_to_boot_cpu: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn running_key_enabled(&self) -> bool {
        self.running_key_enabled
    }

    pub const fn reader_ready(&self) -> bool {
        self.reader_ready
    }

    pub const fn timer_ready(&self) -> bool {
        self.timer_ready
    }

    pub const fn timer_period(&self) -> u64 {
        self.timer_period
    }

    pub const fn setup_local_irq_disable_enable_used(&self) -> bool {
        self.setup_local_irq_disable_enable_used
    }

    pub fn setup_local_irq_guard_used_by(&self, boot_cpu_local_interrupt: &InterruptType) -> bool {
        self.lifecycle.state() == State::Ready
            && self.setup_local_irq_disable_enable_used
            && self.setup_local_irq_guard_bound_to_boot_cpu
            && boot_cpu_local_interrupt.local_state() == State::Ready
            && boot_cpu_local_interrupt.enabled()
    }

    pub fn setup(
        &mut self,
        hrtimer_core: &HrtimerCore,
        timekeeper: &Timekeeper,
        timer_provider: &RiscvTimerProvider,
        static_branch: &StaticBranch,
        boot_cpu_local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || hrtimer_core.state() != State::Ready
            || timekeeper.state() != State::Ready
            || timer_provider.state() != State::Ready
            || timer_provider.timebase_hz() == 0
            || static_branch.state() != State::Ready
            || boot_cpu_local_interrupt.local_state() != State::Ready
            || !boot_cpu_local_interrupt.enabled()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        /*
         * SchedClockLocalInterruptContext:
         * Linux sched_clock_init() wraps generic_sched_clock_init() with
         * local_irq_disable()/local_irq_enable().
         */
        boot_cpu_local_interrupt.disable()?;
        self.running_key_enabled = true;
        self.reader_ready = true;
        self.timer_ready = true;
        self.timer_period = SCHED_CLOCK_TIMER_PERIOD;
        self.setup_local_irq_disable_enable_used = true;
        self.setup_local_irq_guard_bound_to_boot_cpu = true;
        boot_cpu_local_interrupt.enable()?;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SchedClockReady,
        )
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn read(&self, timer_provider: &RiscvTimerProvider) -> Option<u64> {
        if self.lifecycle.state() != State::Ready || !self.reader_ready {
            return None;
        }

        timer_provider.read_time()
    }
}

pub struct DelayLoop {
    lifecycle: Lifecycle,
    lpj_fine: u64,
    boot_cpu_loops_per_jiffy: u64,
    global_loops_per_jiffy: u64,
    delay_actions_ready: bool,
}

impl DelayLoop {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            lpj_fine: 0,
            boot_cpu_loops_per_jiffy: 0,
            global_loops_per_jiffy: 0,
            delay_actions_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn lpj_fine(&self) -> u64 {
        self.lpj_fine
    }

    pub const fn boot_cpu_loops_per_jiffy(&self) -> u64 {
        self.boot_cpu_loops_per_jiffy
    }

    pub const fn global_loops_per_jiffy(&self) -> u64 {
        self.global_loops_per_jiffy
    }

    pub const fn delay_actions_ready(&self) -> bool {
        self.delay_actions_ready
    }

    pub fn setup(
        &mut self,
        timer_provider: &RiscvTimerProvider,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || timer_provider.state() != State::Ready
            || timer_provider.timebase_hz() == 0
            || cpu_group.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lpj_fine = (timer_provider.timebase_hz() / 100).max(1);
        self.boot_cpu_loops_per_jiffy = self.lpj_fine;
        self.global_loops_per_jiffy = self.lpj_fine;
        self.delay_actions_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DelayLoopReady,
        )
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn udelay(&self, timer_provider: &RiscvTimerProvider, usec: u64) -> Option<u64> {
        if self.lifecycle.state() != State::Ready || !self.delay_actions_ready {
            return None;
        }

        let start = timer_provider.read_time()?;
        let ticks = timer_provider.timebase_hz().saturating_mul(usec) / 1_000_000;
        let wait_ticks = ticks.max(1);
        loop {
            let now = timer_provider.read_time()?;
            let elapsed = now.wrapping_sub(start);
            if elapsed >= wait_ticks {
                return Some(elapsed);
            }
            core::hint::spin_loop();
        }
    }
}

pub struct IrqOpenPrepareTrimmedPaths {
    lifecycle: Lifecycle,
    panic_later_clear: bool,
    lockdep_init_trimmed_noop: bool,
    lockdep_trimmed_because_config_debug_lock_alloc_disabled: bool,
    locking_selftest_trimmed_noop: bool,
    locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled: bool,
    initrd_bounds_trimmed: bool,
    initrd_trimmed_because_config_blk_dev_initrd_disabled: bool,
    page_allocator_per_cpu_pagesets_deferred: bool,
    page_allocator_deferred_bound: bool,
    numa_policy_trimmed_noop: bool,
    numa_policy_trimmed_because_config_numa_disabled: bool,
    acpi_early_trimmed_noop: bool,
    acpi_early_trimmed_because_config_acpi_disabled: bool,
    late_time_init_hook_trimmed_noop: bool,
    late_time_init_hook_unset_on_riscv: bool,
    arch_cpu_finalize_init_trimmed_noop: bool,
    arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled: bool,
    position_preserved: bool,
}

impl IrqOpenPrepareTrimmedPaths {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            panic_later_clear: false,
            lockdep_init_trimmed_noop: false,
            lockdep_trimmed_because_config_debug_lock_alloc_disabled: false,
            locking_selftest_trimmed_noop: false,
            locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled: false,
            initrd_bounds_trimmed: false,
            initrd_trimmed_because_config_blk_dev_initrd_disabled: false,
            page_allocator_per_cpu_pagesets_deferred: false,
            page_allocator_deferred_bound: false,
            numa_policy_trimmed_noop: false,
            numa_policy_trimmed_because_config_numa_disabled: false,
            acpi_early_trimmed_noop: false,
            acpi_early_trimmed_because_config_acpi_disabled: false,
            late_time_init_hook_trimmed_noop: false,
            late_time_init_hook_unset_on_riscv: false,
            arch_cpu_finalize_init_trimmed_noop: false,
            arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled: false,
            position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn panic_later_clear(&self) -> bool {
        self.panic_later_clear
    }

    pub const fn lockdep_init_trimmed_noop(&self) -> bool {
        self.lockdep_init_trimmed_noop
    }

    pub const fn lockdep_trimmed_because_config_debug_lock_alloc_disabled(&self) -> bool {
        self.lockdep_trimmed_because_config_debug_lock_alloc_disabled
    }

    pub const fn locking_selftest_trimmed_noop(&self) -> bool {
        self.locking_selftest_trimmed_noop
    }

    pub const fn locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled(
        &self,
    ) -> bool {
        self.locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled
    }

    pub const fn initrd_bounds_trimmed(&self) -> bool {
        self.initrd_bounds_trimmed
    }

    pub const fn initrd_trimmed_because_config_blk_dev_initrd_disabled(&self) -> bool {
        self.initrd_trimmed_because_config_blk_dev_initrd_disabled
    }

    pub const fn page_allocator_per_cpu_pagesets_deferred(&self) -> bool {
        self.page_allocator_per_cpu_pagesets_deferred
    }

    pub const fn page_allocator_deferred_bound(&self) -> bool {
        self.page_allocator_deferred_bound
    }

    pub const fn numa_policy_trimmed_noop(&self) -> bool {
        self.numa_policy_trimmed_noop
    }

    pub const fn numa_policy_trimmed_because_config_numa_disabled(&self) -> bool {
        self.numa_policy_trimmed_because_config_numa_disabled
    }

    pub const fn acpi_early_trimmed_noop(&self) -> bool {
        self.acpi_early_trimmed_noop
    }

    pub const fn acpi_early_trimmed_because_config_acpi_disabled(&self) -> bool {
        self.acpi_early_trimmed_because_config_acpi_disabled
    }

    pub const fn late_time_init_hook_trimmed_noop(&self) -> bool {
        self.late_time_init_hook_trimmed_noop
    }

    pub const fn late_time_init_hook_unset_on_riscv(&self) -> bool {
        self.late_time_init_hook_unset_on_riscv
    }

    pub const fn arch_cpu_finalize_init_trimmed_noop(&self) -> bool {
        self.arch_cpu_finalize_init_trimmed_noop
    }

    pub const fn arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled(
        &self,
    ) -> bool {
        self.arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
    }

    pub fn preset(
        &mut self,
        config: &Config,
        console: &Console,
        page_allocator: &PageAllocator,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || config.state() != State::Online
            || config.debug_lock_alloc_enabled()
            || config.debug_locking_api_selftests_enabled()
            || config.blk_dev_initrd_enabled()
            || config.numa_enabled()
            || config.acpi_enabled()
            || config.riscv_late_time_init_hook_set()
            || config.arch_has_cpu_finalize_init()
            || console.state() != State::Prepared
            || page_allocator.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.panic_later_clear = true;
        crate::checkpoint::checkpoint(Checkpoint::PanicLaterClearCheckpoint);
        self.lockdep_init_trimmed_noop = true;
        self.lockdep_trimmed_because_config_debug_lock_alloc_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::LockdepInitNoop);
        self.locking_selftest_trimmed_noop = true;
        self.locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::LockingSelftestNoop);
        self.initrd_bounds_trimmed = true;
        self.initrd_trimmed_because_config_blk_dev_initrd_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::InitrdBoundsTrimmed);
        self.page_allocator_per_cpu_pagesets_deferred = true;
        self.page_allocator_deferred_bound = true;
        crate::checkpoint::checkpoint(Checkpoint::PageAllocatorPerCpuPagesetsDeferred);
        self.numa_policy_trimmed_noop = true;
        self.numa_policy_trimmed_because_config_numa_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::NumaPolicyNoop);
        self.acpi_early_trimmed_noop = true;
        self.acpi_early_trimmed_because_config_acpi_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::AcpiEarlyNoop);
        self.late_time_init_hook_trimmed_noop = true;
        self.late_time_init_hook_unset_on_riscv = true;
        crate::checkpoint::checkpoint(Checkpoint::LateTimeInitNoop);
        self.position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::IrqOpenPrepareTrimmedPathsPrepared,
        )
    }

    pub fn setup(
        &mut self,
        config: &Config,
        sched_clock: &SchedClock,
        delay_loop: &DelayLoop,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || config.state() != State::Online
            || config.arch_has_cpu_finalize_init()
            || sched_clock.state() != State::Ready
            || delay_loop.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.arch_cpu_finalize_init_trimmed_noop = true;
        self.arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::ArchCpuFinalizeNoop);
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::IrqOpenPrepareTrimmedPathsReady,
        )
    }
}

pub fn slub_flush_workqueue_ready(
    slub_subsystem: &SlubSubsystem,
    kmalloc_caches: &KmallocCaches,
) -> bool {
    slub_subsystem.state() == State::Ready
        && slub_subsystem.flush_workqueue_ready()
        && kmalloc_caches.state() == State::Ready
}
