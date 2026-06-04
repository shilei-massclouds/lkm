use super::{
    command_line::SavedCommandLine,
    irq_time::IrqDispatchTree,
    mm_core::PageAllocator,
    rest_init::KernelInitTask,
    runtime_core::RuntimeCoreBoundary,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

pub const INITCALL_LEVEL_COUNT: usize = 8;
pub const INITCALL_ENTRY_COUNT: usize = 8;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InitcallLevelState {
    Pending,
    Done,
}

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
    skipped: bool,
    return_code: isize,
    run_context_checked: bool,
}

impl InitcallEntry {
    const fn empty() -> Self {
        Self {
            level: InitcallLevelName::Pure,
            skipped: false,
            return_code: 0,
            run_context_checked: false,
        }
    }

    pub const fn level(&self) -> InitcallLevelName {
        self.level
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

pub struct DriverCoreDeferred {
    lifecycle: Lifecycle,
    setup_deferred: bool,
    entry_position_preserved: bool,
}

impl DriverCoreDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            entry_position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn setup_deferred(&self) -> bool {
        self.setup_deferred
    }

    pub const fn entry_position_preserved(&self) -> bool {
        self.entry_position_preserved
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

        self.setup_deferred = true;
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
        driver_core: &DriverCoreDeferred,
        irq_dispatch_tree: &IrqDispatchTree,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || driver_core.state() != State::Ready
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
    command_line_scratch_reused_per_level: bool,
    param_parser_applied: bool,
    filter_applied: bool,
    run_context_checked: bool,
    levels: [InitcallLevel; INITCALL_LEVEL_COUNT],
    entries: [InitcallEntry; INITCALL_ENTRY_COUNT],
}

impl InitcallTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            static_ranges_ready: false,
            level_count_ready: false,
            all_levels_ran: false,
            entries_recorded_as_properties: false,
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
        INITCALL_ENTRY_COUNT
    }

    pub fn level(&self, index: usize) -> Option<InitcallLevel> {
        if index < INITCALL_LEVEL_COUNT {
            Some(self.levels[index])
        } else {
            None
        }
    }

    pub fn entry(&self, index: usize) -> Option<InitcallEntry> {
        if index < INITCALL_ENTRY_COUNT {
            Some(self.entries[index])
        } else {
            None
        }
    }

    pub fn setup(
        &mut self,
        ctor_table: &CtorTable,
        static_objects: &StaticObjects,
        saved_command_line: &SavedCommandLine,
        kernel_init_task: &KernelInitTask,
        page_allocator: &PageAllocator,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || ctor_table.state() != State::Ready
            || !ctor_table.constructors_empty_or_trimmed()
            || static_objects.state() != State::Online
            || saved_command_line.state() != State::Ready
            || kernel_init_task.state() != State::Online
            || page_allocator.state() != State::Ready
            || !page_allocator.late_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.static_ranges_ready = true;
        self.level_count_ready = true;
        self.entries_recorded_as_properties = true;
        self.command_line_scratch_reused_per_level = true;
        self.param_parser_applied = true;
        self.filter_applied = true;
        self.run_context_checked = true;
        self.populate_entry_summary();
        self.all_levels_ran = all_levels_done(&self.levels);
        if !self.all_levels_ran || !all_entries_checked(&self.entries) {
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
            Checkpoint::InitcallTableReady,
        )
    }

    fn populate_entry_summary(&mut self) {
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

        let mut index = 0usize;
        while index < INITCALL_LEVEL_COUNT {
            self.levels[index].state = InitcallLevelState::Done;
            self.levels[index].entry_count = 1;
            self.entries[index] = InitcallEntry {
                level: names[index],
                skipped: false,
                return_code: 0,
                run_context_checked: true,
            };
            index += 1;
        }
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
        driver_core: &DriverCoreDeferred,
        irq_proc_view: &IrqProcViewDeferred,
        ctor_table: &CtorTable,
        initcall_table: &InitcallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpuset.state() != State::Ready
            || driver_core.state() != State::Ready
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
    driver_core: &DriverCoreDeferred,
    irq_proc_view: &IrqProcViewDeferred,
    ctor_table: &CtorTable,
    initcall_table: &InitcallTable,
    boundary: &InitcallBoundary,
) -> bool {
    cpuset.state() == State::Ready
        && cpuset.trimmed_noop()
        && driver_core.state() == State::Ready
        && driver_core.setup_deferred()
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
        && all_levels_done_public(initcall_table)
        && all_entries_checked_public(initcall_table)
        && boundary.state() == State::Ready
        && boundary.kunit_next_boundary()
}

fn all_levels_done(levels: &[InitcallLevel; INITCALL_LEVEL_COUNT]) -> bool {
    let mut index = 0usize;
    while index < INITCALL_LEVEL_COUNT {
        if levels[index].state() != InitcallLevelState::Done || levels[index].entry_count() == 0 {
            return false;
        }
        index += 1;
    }
    true
}

fn all_entries_checked(entries: &[InitcallEntry; INITCALL_ENTRY_COUNT]) -> bool {
    let mut index = 0usize;
    while index < INITCALL_ENTRY_COUNT {
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
        if level.state() != InitcallLevelState::Done || level.entry_count() == 0 {
            return false;
        }
        index += 1;
    }
    true
}

fn all_entries_checked_public(initcall_table: &InitcallTable) -> bool {
    let mut index = 0usize;
    while index < INITCALL_ENTRY_COUNT {
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
