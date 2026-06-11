use super::{
    cpu_group::CpuGroup,
    earlycon,
    irq_time::{HrtimerCore, RiscvTimerProvider, Timekeeper},
    mm_core::{KmallocCaches, SlubAllocator},
    printk,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::StaticBranch,
    static_objects::StaticObjects,
};
use crate::trace::Checkpoint;

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
            || !earlycon::is_online()
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
}

impl SchedClock {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            running_key_enabled: false,
            reader_ready: false,
            timer_ready: false,
            timer_period: 0,
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

    pub fn setup(
        &mut self,
        hrtimer_core: &HrtimerCore,
        timekeeper: &Timekeeper,
        timer_provider: &RiscvTimerProvider,
        static_branch: &StaticBranch,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || hrtimer_core.state() != State::Ready
            || timekeeper.state() != State::Ready
            || timer_provider.state() != State::Ready
            || timer_provider.timebase_hz() == 0
            || static_branch.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.running_key_enabled = true;
        self.reader_ready = true;
        self.timer_ready = true;
        self.timer_period = SCHED_CLOCK_TIMER_PERIOD;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SchedClockReady,
        )
    }

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

pub fn slub_flush_workqueue_ready(
    slub_allocator: &SlubAllocator,
    kmalloc_caches: &KmallocCaches,
) -> bool {
    slub_allocator.state() == State::Ready
        && slub_allocator.flush_workqueue_ready()
        && kmalloc_caches.state() == State::Ready
}
