use super::{
    boot_task::BootTask,
    config::Config,
    cpu_capabilities::CpuCapabilities,
    cpu_group::CpuGroup,
    exception_stream::ExceptionStream,
    files::FilesStruct,
    mm_core::{KmallocCaches, MmStructCache, SlubSubsystem},
    per_cpu_storage::PerCpuStorage,
    scheduler::Scheduler,
    state::{EventError, EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_branch::StaticBranch,
    task::TaskEntry,
    user_boot::{
        KernelInitTaskUserState, UserAddressSpace, UserCloneDeferredBoundaries, UserTaskSet,
        UserTrapFrame,
    },
    vfs::{FsStruct, VfsCore},
};
use crate::checkpoint::Checkpoint;

const PID_MAX_MIN_BASE: usize = 301;
const PID_MAX_DEFAULT: usize = 32_768;
const PID_MAX_LIMIT: usize = 4_194_304;
const PIDNS_ADDING: usize = usize::MAX;
const PID_CACHE_LEVEL0: usize = 0;
const THREAD_STACK_CACHE_BYTES: usize = 16 * 1024;
const TASK_STRUCT_CACHE_BYTES: usize = 4096;
const MAX_THREADS_PER_CPU: usize = 8192;
const BUILTIN_KEY_TYPE_COUNT: usize = 4;
const BUILTIN_LSM_COUNT: usize = 1;
const CAPABILITY_HOOK_COUNT: usize = 8;

const _: () = assert!(PID_MAX_LIMIT < PIDNS_ADDING);

pub struct RootPidNamespace {
    lifecycle: Lifecycle,
    idr_ready: bool,
    pid_cache_level: usize,
    pid_max_min: usize,
    pid_max: usize,
    compiletime_limit_checked: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl RootPidNamespace {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            idr_ready: false,
            pid_cache_level: usize::MAX,
            pid_max_min: 0,
            pid_max: 0,
            compiletime_limit_checked: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn idr_ready(&self) -> bool {
        self.idr_ready
    }

    pub const fn pid_cache_level(&self) -> usize {
        self.pid_cache_level
    }

    pub const fn pid_max_min(&self) -> usize {
        self.pid_max_min
    }

    pub const fn pid_max(&self) -> usize {
        self.pid_max
    }

    pub const fn compiletime_limit_checked(&self) -> bool {
        self.compiletime_limit_checked
    }

    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
            || cpu_group.possible_cpu_count() == 0
        {
            return self.failed_setup();
        }

        self.idr_ready = true;
        self.pid_cache_level = PID_CACHE_LEVEL0;
        self.pid_max_min = PID_MAX_MIN_BASE;
        self.pid_max = PID_MAX_DEFAULT.max(cpu_group.possible_cpu_count().saturating_mul(1024));
        self.compiletime_limit_checked = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RootPidNamespaceReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct AnonVmaCore {
    lifecycle: Lifecycle,
    anon_vma_cache_ready: bool,
    anon_vma_chain_cache_ready: bool,
    runtime_graph_deferred: bool,
}

impl AnonVmaCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            anon_vma_cache_ready: false,
            anon_vma_chain_cache_ready: false,
            runtime_graph_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn anon_vma_cache_ready(&self) -> bool {
        self.anon_vma_cache_ready
    }

    pub const fn anon_vma_chain_cache_ready(&self) -> bool {
        self.anon_vma_chain_cache_ready
    }

    pub const fn runtime_graph_deferred(&self) -> bool {
        self.runtime_graph_deferred
    }

    pub fn setup(
        &mut self,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.anon_vma_cache_ready = true;
        self.anon_vma_chain_cache_ready = true;
        self.runtime_graph_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::AnonVmaCoreReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct CredentialCore {
    lifecycle: Lifecycle,
    cred_cache_ready: bool,
    init_cred_static_root_not_created_here: bool,
    runtime_relations_deferred: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl CredentialCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cred_cache_ready: false,
            init_cred_static_root_not_created_here: true,
            runtime_relations_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cred_cache_ready(&self) -> bool {
        self.cred_cache_ready
    }

    pub const fn init_cred_static_root_not_created_here(&self) -> bool {
        self.init_cred_static_root_not_created_here
    }

    pub const fn runtime_relations_deferred(&self) -> bool {
        self.runtime_relations_deferred
    }

    pub fn preset(
        &mut self,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.cred_cache_ready = true;
        self.init_cred_static_root_not_created_here = true;
        self.runtime_relations_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CredentialCorePrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct VectorContext {
    lifecycle: Lifecycle,
    vector_supported: bool,
    user_cache_ready: bool,
    kernel_cache_ready: bool,
}

impl VectorContext {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            vector_supported: false,
            user_cache_ready: false,
            kernel_cache_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn vector_supported(&self) -> bool {
        self.vector_supported
    }

    pub const fn user_cache_ready(&self) -> bool {
        self.user_cache_ready
    }

    pub const fn kernel_cache_ready(&self) -> bool {
        self.kernel_cache_ready
    }

    fn preset(
        &mut self,
        cpu_capabilities: &CpuCapabilities,
        slub_subsystem: &SlubSubsystem,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_capabilities.state() != State::Ready
            || slub_subsystem.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.vector_supported = cpu_capabilities.vector_supported();
        self.user_cache_ready = self.vector_supported;
        self.kernel_cache_ready = self.vector_supported;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::VectorContextPrepared,
        )
    }
}

pub struct UprobeCore {
    lifecycle: Lifecycle,
    hash_mutex_ready: bool,
    die_notifier_registered: bool,
}

impl UprobeCore {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hash_mutex_ready: false,
            die_notifier_registered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn hash_mutex_ready(&self) -> bool {
        self.hash_mutex_ready
    }

    pub const fn die_notifier_registered(&self) -> bool {
        self.die_notifier_registered
    }

    fn setup(
        &mut self,
        exception_stream: &ExceptionStream,
        slub_subsystem: &SlubSubsystem,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || exception_stream.state() != State::Ready
            || slub_subsystem.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.hash_mutex_ready = true;
        self.die_notifier_registered = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::UprobeCoreReady,
        )
    }
}

pub struct TaskCreationCore {
    lifecycle: Lifecycle,
    vector_context: VectorContext,
    uprobe_core: UprobeCore,
    thread_stack_cache_ready: bool,
    thread_stack_cache_bytes: usize,
    vmap_stack_selected: bool,
    task_struct_cache_ready: bool,
    task_struct_cache_bytes: usize,
    max_threads: usize,
    init_task_rlimits_ready: bool,
    init_user_namespace_ucounts_ready: bool,
    fork_vm_stack_cpuhp_registered: bool,
    rest_init_inputs_ready: bool,
    entry_contract_ready: bool,
    kernel_init_created: bool,
    kthreadd_created: bool,
    user_child_created: bool,
    system_scheduling: bool,
}

impl TaskCreationCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            vector_context: VectorContext::new(),
            uprobe_core: UprobeCore::new(),
            thread_stack_cache_ready: false,
            thread_stack_cache_bytes: 0,
            vmap_stack_selected: false,
            task_struct_cache_ready: false,
            task_struct_cache_bytes: 0,
            max_threads: 0,
            init_task_rlimits_ready: false,
            init_user_namespace_ucounts_ready: false,
            fork_vm_stack_cpuhp_registered: false,
            rest_init_inputs_ready: false,
            entry_contract_ready: false,
            kernel_init_created: false,
            kthreadd_created: false,
            user_child_created: false,
            system_scheduling: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn vector_context(&self) -> &VectorContext {
        &self.vector_context
    }

    pub const fn uprobe_core(&self) -> &UprobeCore {
        &self.uprobe_core
    }

    pub const fn thread_stack_cache_ready(&self) -> bool {
        self.thread_stack_cache_ready
    }

    pub const fn thread_stack_cache_bytes(&self) -> usize {
        self.thread_stack_cache_bytes
    }

    pub const fn vmap_stack_selected(&self) -> bool {
        self.vmap_stack_selected
    }

    pub const fn task_struct_cache_ready(&self) -> bool {
        self.task_struct_cache_ready
    }

    pub const fn task_struct_cache_bytes(&self) -> usize {
        self.task_struct_cache_bytes
    }

    pub const fn max_threads(&self) -> usize {
        self.max_threads
    }

    pub const fn init_task_rlimits_ready(&self) -> bool {
        self.init_task_rlimits_ready
    }

    pub const fn init_user_namespace_ucounts_ready(&self) -> bool {
        self.init_user_namespace_ucounts_ready
    }

    pub const fn fork_vm_stack_cpuhp_registered(&self) -> bool {
        self.fork_vm_stack_cpuhp_registered
    }

    pub const fn rest_init_inputs_ready(&self) -> bool {
        self.rest_init_inputs_ready
    }

    pub const fn entry_contract_ready(&self) -> bool {
        self.entry_contract_ready
    }

    pub const fn kernel_init_created(&self) -> bool {
        self.kernel_init_created
    }

    pub const fn kthreadd_created(&self) -> bool {
        self.kthreadd_created
    }

    // Observed by the optional user-boot checkpoint handlers.
    #[allow(dead_code)]
    pub const fn user_child_created(&self) -> bool {
        self.user_child_created
    }

    pub const fn system_scheduling(&self) -> bool {
        self.system_scheduling
    }

    pub fn preset(
        &mut self,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.thread_stack_cache_ready = true;
        self.thread_stack_cache_bytes = THREAD_STACK_CACHE_BYTES;
        self.vmap_stack_selected = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TaskCreationCorePrepared,
        )
    }

    pub fn setup(&mut self, inputs: TaskCreationSetup<'_>) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || inputs.root_pid_namespace.state() != State::Ready
            || inputs.credential_core.state() != State::Prepared
            || inputs.cpu_group.state() != State::Ready
            || inputs.cpu_capabilities.state() != State::Ready
            || inputs.slub_subsystem.state() != State::Ready
            || inputs.boot_task.state() != State::Online
            || inputs.exception_stream.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.vector_context
            .preset(inputs.cpu_capabilities, inputs.slub_subsystem)?;
        self.uprobe_core
            .setup(inputs.exception_stream, inputs.slub_subsystem)?;

        self.task_struct_cache_ready = true;
        self.task_struct_cache_bytes = TASK_STRUCT_CACHE_BYTES;
        self.max_threads = inputs
            .cpu_group
            .possible_cpu_count()
            .saturating_mul(MAX_THREADS_PER_CPU)
            .max(MAX_THREADS_PER_CPU);
        self.init_task_rlimits_ready = true;
        self.init_user_namespace_ucounts_ready = true;
        self.fork_vm_stack_cpuhp_registered = self.vmap_stack_selected;
        self.rest_init_inputs_ready = true;
        self.entry_contract_ready = true;
        self.kernel_init_created = false;
        self.kthreadd_created = false;
        self.user_child_created = false;
        self.system_scheduling = false;

        if self.vector_context.state() != State::Prepared
            || self.uprobe_core.state() != State::Ready
            || self.max_threads == 0
            || !self.entry_contract_ready
            || !self.rest_init_inputs_ready
        {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::TaskCreationCoreReady,
        )
    }

    pub fn copy_process(
        &mut self,
        inputs: TaskCopyProcessInputs<'_>,
        dst_state: State,
        dst_entry: TaskEntry,
    ) -> Result<TaskCopyProcessResult, EventError> {
        if self.lifecycle.state() != State::Ready
            || !self.entry_contract_ready
            || !self.rest_init_inputs_ready
            || dst_state != State::Prepared
            || dst_entry != inputs.entry
            || inputs.entry == TaskEntry::None
            || inputs.entry == TaskEntry::UserChild
            || inputs.entry == TaskEntry::SmokeScheduler
            || inputs.src_task.state() != State::Online
            || inputs.root_pid_namespace.state() != State::Ready
            || inputs.credential_core.state() != State::Prepared
            || inputs.signal_core.state() != State::Prepared
            || inputs.task_file_context.state() != State::Prepared
            || inputs.security_core.state() != State::Ready
            || inputs.scheduler.state() != State::Online
            || inputs
                .scheduler
                .boot_cpu_owned_scheduler_view(inputs.cpu_group)
                .is_none()
        {
            return Err(EventError::failed(
                super::state::EventErrorCode::ConditionFailed,
                LifecycleEvent::Setup,
                dst_state,
                State::Prepared,
                State::Ready,
            ));
        }

        match inputs.entry {
            TaskEntry::KernelInit => self.kernel_init_created = true,
            TaskEntry::Kthreadd => self.kthreadd_created = true,
            TaskEntry::None | TaskEntry::UserChild | TaskEntry::SmokeScheduler => {}
        }

        Ok(TaskCopyProcessResult {
            entry: inputs.entry,
            task_struct_allocated: self.task_struct_cache_ready,
            thread_context_ready: self.thread_stack_cache_ready,
            sched_entity_ready: true,
            task_state_new: true,
            task_not_enqueued: true,
        })
    }

    pub fn copy_user_process(
        &mut self,
        inputs: TaskCopyUserProcessInputs<'_>,
        dst_state: State,
        dst_entry: TaskEntry,
    ) -> Result<TaskCopyProcessResult, EventError> {
        let dst_prepared_record = dst_state == State::Prepared
            && inputs.dst_process.active_task_state() == State::Prepared
            && inputs.dst_process.prepared();
        let dst_nested_vfork_record = inputs.allow_nested_vfork
            && dst_state == State::Ready
            && inputs.dst_process.nested_vfork_copy_ready();
        if self.lifecycle.state() != State::Ready
            || !self.entry_contract_ready
            || !self.rest_init_inputs_ready
            || dst_entry != TaskEntry::UserChild
            || inputs.entry != TaskEntry::UserChild
            || !inputs.src_process.active_user_flow_online()
            || !(dst_prepared_record || dst_nested_vfork_record)
            || inputs.root_pid_namespace.state() != State::Ready
            || inputs.scheduler.state() != State::Online
            || inputs.fs_struct.state() != State::Ready
            || inputs.files_struct.state() != State::Ready
            || inputs.address_space.state() != State::Online
            || inputs.trap_frame.state() != State::Ready
            || inputs.boundaries.state() != State::Ready
            || !inputs.boundaries.plain_fork_first_slice_bound()
            || !inputs
                .scheduler
                .boot_cpu_owned_scheduler_view(inputs.cpu_group)
                .is_some()
        {
            return Err(EventError::failed(
                super::state::EventErrorCode::ConditionFailed,
                LifecycleEvent::Setup,
                dst_state,
                State::Prepared,
                State::Ready,
            ));
        }

        self.user_child_created = true;
        Ok(TaskCopyProcessResult {
            entry: inputs.entry,
            task_struct_allocated: self.task_struct_cache_ready,
            thread_context_ready: self.thread_stack_cache_ready,
            sched_entity_ready: true,
            task_state_new: true,
            task_not_enqueued: true,
        })
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }
}

pub struct TaskCopyProcessInputs<'a> {
    pub src_task: &'a BootTask,
    pub root_pid_namespace: &'a RootPidNamespace,
    pub credential_core: &'a CredentialCore,
    pub signal_core: &'a SignalCore,
    pub task_file_context: &'a TaskFileContext,
    pub security_core: &'a SecurityCore,
    pub scheduler: &'a Scheduler,
    pub cpu_group: &'a CpuGroup,
    pub entry: TaskEntry,
}

pub struct TaskCopyUserProcessInputs<'a> {
    pub src_process: &'a KernelInitTaskUserState,
    pub dst_process: &'a UserTaskSet,
    pub root_pid_namespace: &'a RootPidNamespace,
    pub scheduler: &'a Scheduler,
    pub cpu_group: &'a CpuGroup,
    pub fs_struct: &'a FsStruct,
    pub files_struct: &'a FilesStruct,
    pub address_space: &'a UserAddressSpace,
    pub trap_frame: &'a UserTrapFrame,
    pub boundaries: &'a UserCloneDeferredBoundaries,
    pub entry: TaskEntry,
    pub allow_nested_vfork: bool,
}

pub struct TaskCopyProcessResult {
    entry: TaskEntry,
    task_struct_allocated: bool,
    thread_context_ready: bool,
    sched_entity_ready: bool,
    task_state_new: bool,
    task_not_enqueued: bool,
}

impl TaskCopyProcessResult {
    pub const fn entry(&self) -> TaskEntry {
        self.entry
    }

    pub const fn task_struct_allocated(&self) -> bool {
        self.task_struct_allocated
    }

    pub const fn thread_context_ready(&self) -> bool {
        self.thread_context_ready
    }

    pub const fn sched_entity_ready(&self) -> bool {
        self.sched_entity_ready
    }

    pub const fn task_state_new(&self) -> bool {
        self.task_state_new
    }

    pub const fn task_not_enqueued(&self) -> bool {
        self.task_not_enqueued
    }
}

pub struct TaskCreationSetup<'a> {
    pub root_pid_namespace: &'a RootPidNamespace,
    pub credential_core: &'a CredentialCore,
    pub cpu_group: &'a CpuGroup,
    pub cpu_capabilities: &'a CpuCapabilities,
    pub slub_subsystem: &'a SlubSubsystem,
    pub boot_task: &'a BootTask,
    pub exception_stream: &'a ExceptionStream,
}

pub struct SignalCore {
    lifecycle: Lifecycle,
    sighand_cache_ready: bool,
    signal_struct_cache_ready: bool,
    sigqueue_cache_deferred: bool,
}

impl SignalCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            sighand_cache_ready: false,
            signal_struct_cache_ready: false,
            sigqueue_cache_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn sighand_cache_ready(&self) -> bool {
        self.sighand_cache_ready
    }

    pub const fn signal_struct_cache_ready(&self) -> bool {
        self.signal_struct_cache_ready
    }

    pub const fn sigqueue_cache_deferred(&self) -> bool {
        self.sigqueue_cache_deferred
    }

    pub fn preset(
        &mut self,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.sighand_cache_ready = true;
        self.signal_struct_cache_ready = true;
        self.sigqueue_cache_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SignalCorePrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct TaskFileContext {
    lifecycle: Lifecycle,
    files_struct_cache_ready: bool,
    fs_struct_cache_ready: bool,
    vfs_runtime_deferred: bool,
}

impl TaskFileContext {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            files_struct_cache_ready: false,
            fs_struct_cache_ready: false,
            vfs_runtime_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn files_struct_cache_ready(&self) -> bool {
        self.files_struct_cache_ready
    }

    pub const fn fs_struct_cache_ready(&self) -> bool {
        self.fs_struct_cache_ready
    }

    pub const fn vfs_runtime_deferred(&self) -> bool {
        self.vfs_runtime_deferred
    }

    pub fn preset(
        &mut self,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.files_struct_cache_ready = true;
        self.fs_struct_cache_ready = true;
        self.vfs_runtime_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::TaskFileContextPrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct VmaCore {
    lifecycle: Lifecycle,
    vm_area_struct_cache_ready: bool,
    per_vma_lock_cache_ready: bool,
    vm_committed_as_counter_ready: bool,
    runtime_mapping_deferred: bool,
}

impl VmaCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            vm_area_struct_cache_ready: false,
            per_vma_lock_cache_ready: false,
            vm_committed_as_counter_ready: false,
            runtime_mapping_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn vm_area_struct_cache_ready(&self) -> bool {
        self.vm_area_struct_cache_ready
    }

    pub const fn per_vma_lock_cache_ready(&self) -> bool {
        self.per_vma_lock_cache_ready
    }

    pub const fn vm_committed_as_counter_ready(&self) -> bool {
        self.vm_committed_as_counter_ready
    }

    pub const fn runtime_mapping_deferred(&self) -> bool {
        self.runtime_mapping_deferred
    }

    pub fn preset(
        &mut self,
        mm_struct_cache: &MmStructCache,
        anon_vma_core: &AnonVmaCore,
        slub_subsystem: &SlubSubsystem,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || mm_struct_cache.state() != State::Ready
            || anon_vma_core.state() != State::Ready
            || slub_subsystem.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.vm_area_struct_cache_ready = true;
        self.per_vma_lock_cache_ready = true;
        self.vm_committed_as_counter_ready = true;
        self.runtime_mapping_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::VmaCorePrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct NsProxy {
    lifecycle: Lifecycle,
    nsproxy_cache_ready: bool,
    runtime_refs_deferred: bool,
}

impl NsProxy {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            nsproxy_cache_ready: false,
            runtime_refs_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn nsproxy_cache_ready(&self) -> bool {
        self.nsproxy_cache_ready
    }

    pub const fn runtime_refs_deferred(&self) -> bool {
        self.runtime_refs_deferred
    }

    pub fn preset(
        &mut self,
        slub_subsystem: &SlubSubsystem,
        kmalloc_caches: &KmallocCaches,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || slub_subsystem.state() != State::Ready
            || kmalloc_caches.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.nsproxy_cache_ready = true;
        self.runtime_refs_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::NsProxyPrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct UtsNamespace {
    lifecycle: Lifecycle,
    cache_ready: bool,
    static_root_not_created_here: bool,
    runtime_ops_deferred: bool,
}

impl UtsNamespace {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cache_ready: false,
            static_root_not_created_here: true,
            runtime_ops_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cache_ready(&self) -> bool {
        self.cache_ready
    }

    pub const fn static_root_not_created_here(&self) -> bool {
        self.static_root_not_created_here
    }

    pub const fn runtime_ops_deferred(&self) -> bool {
        self.runtime_ops_deferred
    }

    pub fn preset(&mut self, ns_proxy: &NsProxy, slub_subsystem: &SlubSubsystem) -> EventResult {
        if self.lifecycle.state() != State::Base
            || ns_proxy.state() != State::Prepared
            || slub_subsystem.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.cache_ready = true;
        self.static_root_not_created_here = true;
        self.runtime_ops_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::UtsNamespacePrepared,
        )
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }
}

pub struct KeyringCore {
    lifecycle: Lifecycle,
    key_cache_ready: bool,
    builtin_key_type_count: usize,
    root_key_user_tracking_ready: bool,
    persistent_keyrings_trimmed: bool,
}

impl KeyringCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            key_cache_ready: false,
            builtin_key_type_count: 0,
            root_key_user_tracking_ready: false,
            persistent_keyrings_trimmed: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn key_cache_ready(&self) -> bool {
        self.key_cache_ready
    }

    pub const fn builtin_key_type_count(&self) -> usize {
        self.builtin_key_type_count
    }

    pub const fn root_key_user_tracking_ready(&self) -> bool {
        self.root_key_user_tracking_ready
    }

    pub const fn persistent_keyrings_trimmed(&self) -> bool {
        self.persistent_keyrings_trimmed
    }

    pub fn setup(
        &mut self,
        credential_core: &CredentialCore,
        slub_subsystem: &SlubSubsystem,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || credential_core.state() != State::Prepared
            || slub_subsystem.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.key_cache_ready = true;
        self.builtin_key_type_count = BUILTIN_KEY_TYPE_COUNT;
        self.root_key_user_tracking_ready = true;
        self.persistent_keyrings_trimmed = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KeyringCoreReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct SecurityCore {
    lifecycle: Lifecycle,
    ordered_lsm_count: usize,
    blob_layout_ready: bool,
    hook_dispatcher_ready: bool,
    capability_hook_count: usize,
    optional_lsms_deferred: bool,
}

impl SecurityCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ordered_lsm_count: 0,
            blob_layout_ready: false,
            hook_dispatcher_ready: false,
            capability_hook_count: 0,
            optional_lsms_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ordered_lsm_count(&self) -> usize {
        self.ordered_lsm_count
    }

    pub const fn blob_layout_ready(&self) -> bool {
        self.blob_layout_ready
    }

    pub const fn hook_dispatcher_ready(&self) -> bool {
        self.hook_dispatcher_ready
    }

    pub const fn capability_hook_count(&self) -> usize {
        self.capability_hook_count
    }

    pub const fn optional_lsms_deferred(&self) -> bool {
        self.optional_lsms_deferred
    }

    pub fn setup(
        &mut self,
        credential_core: &CredentialCore,
        keyring_core: &KeyringCore,
        slub_subsystem: &SlubSubsystem,
        static_branch: &StaticBranch,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || credential_core.state() != State::Prepared
            || keyring_core.state() != State::Ready
            || slub_subsystem.state() != State::Ready
            || static_branch.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.ordered_lsm_count = BUILTIN_LSM_COUNT;
        self.blob_layout_ready = true;
        self.hook_dispatcher_ready = true;
        self.capability_hook_count = CAPABILITY_HOOK_COUNT;
        self.optional_lsms_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SecurityCoreReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct ProcessPrepareTrimmedPaths {
    lifecycle: Lifecycle,
    x86_efi_runtime_switch_trimmed_noop: bool,
    x86_efi_runtime_switch_trimmed_because_arch_riscv: bool,
    shadow_call_stack_init_trimmed_noop: bool,
    shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled: bool,
    lockdep_init_task_trimmed_noop: bool,
    lockdep_init_task_trimmed_because_config_lockdep_disabled: bool,
    dbg_late_init_trimmed_noop: bool,
    dbg_late_init_trimmed_because_config_kgdb_disabled: bool,
    net_namespace_deferred: bool,
    net_namespace_deferred_even_if_config_net_ns_enabled: bool,
    pagecache_deferred: bool,
    pagecache_waitqueue_table_deferred: bool,
    signal_core_setup_deferred: bool,
    seq_file_core_deferred: bool,
    procfs_deferred: bool,
    nsfs_deferred: bool,
    pidfs_deferred: bool,
    vfs_pseudo_filesystems_deferred: bool,
    bdev_chrdev_init_deferred: bool,
    cpuset_init_trimmed_noop: bool,
    cpuset_trimmed_because_config_cpusets_disabled: bool,
    cgroup_init_trimmed_noop: bool,
    cgroup_trimmed_because_config_cgroups_disabled: bool,
    taskstats_init_trimmed_noop: bool,
    taskstats_trimmed_because_config_taskstats_disabled: bool,
    delayacct_init_trimmed_noop: bool,
    delayacct_trimmed_because_config_task_delay_acct_disabled: bool,
    acpi_subsystem_init_trimmed_noop: bool,
    acpi_trimmed_because_config_acpi_disabled: bool,
    arch_post_acpi_subsys_init_trimmed_noop: bool,
    kcsan_init_trimmed_noop: bool,
    kcsan_trimmed_because_config_kcsan_disabled: bool,
    rcu_tasks_generic_out_of_scope: bool,
    rcu_tasks_generic_belongs_to_kernel_init_freeable: bool,
    position_preserved: bool,
}

impl ProcessPrepareTrimmedPaths {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            x86_efi_runtime_switch_trimmed_noop: false,
            x86_efi_runtime_switch_trimmed_because_arch_riscv: false,
            shadow_call_stack_init_trimmed_noop: false,
            shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled: false,
            lockdep_init_task_trimmed_noop: false,
            lockdep_init_task_trimmed_because_config_lockdep_disabled: false,
            dbg_late_init_trimmed_noop: false,
            dbg_late_init_trimmed_because_config_kgdb_disabled: false,
            net_namespace_deferred: false,
            net_namespace_deferred_even_if_config_net_ns_enabled: false,
            pagecache_deferred: false,
            pagecache_waitqueue_table_deferred: false,
            signal_core_setup_deferred: false,
            seq_file_core_deferred: false,
            procfs_deferred: false,
            nsfs_deferred: false,
            pidfs_deferred: false,
            vfs_pseudo_filesystems_deferred: false,
            bdev_chrdev_init_deferred: false,
            cpuset_init_trimmed_noop: false,
            cpuset_trimmed_because_config_cpusets_disabled: false,
            cgroup_init_trimmed_noop: false,
            cgroup_trimmed_because_config_cgroups_disabled: false,
            taskstats_init_trimmed_noop: false,
            taskstats_trimmed_because_config_taskstats_disabled: false,
            delayacct_init_trimmed_noop: false,
            delayacct_trimmed_because_config_task_delay_acct_disabled: false,
            acpi_subsystem_init_trimmed_noop: false,
            acpi_trimmed_because_config_acpi_disabled: false,
            arch_post_acpi_subsys_init_trimmed_noop: false,
            kcsan_init_trimmed_noop: false,
            kcsan_trimmed_because_config_kcsan_disabled: false,
            rcu_tasks_generic_out_of_scope: false,
            rcu_tasks_generic_belongs_to_kernel_init_freeable: false,
            position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn x86_efi_runtime_switch_trimmed_noop(&self) -> bool {
        self.x86_efi_runtime_switch_trimmed_noop
    }

    pub const fn x86_efi_runtime_switch_trimmed_because_arch_riscv(&self) -> bool {
        self.x86_efi_runtime_switch_trimmed_because_arch_riscv
    }

    pub const fn shadow_call_stack_init_trimmed_noop(&self) -> bool {
        self.shadow_call_stack_init_trimmed_noop
    }

    pub const fn shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled(
        &self,
    ) -> bool {
        self.shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled
    }

    pub const fn lockdep_init_task_trimmed_noop(&self) -> bool {
        self.lockdep_init_task_trimmed_noop
    }

    pub const fn lockdep_init_task_trimmed_because_config_lockdep_disabled(&self) -> bool {
        self.lockdep_init_task_trimmed_because_config_lockdep_disabled
    }

    pub const fn dbg_late_init_trimmed_noop(&self) -> bool {
        self.dbg_late_init_trimmed_noop
    }

    pub const fn dbg_late_init_trimmed_because_config_kgdb_disabled(&self) -> bool {
        self.dbg_late_init_trimmed_because_config_kgdb_disabled
    }

    pub const fn net_namespace_deferred(&self) -> bool {
        self.net_namespace_deferred
    }

    pub const fn net_namespace_deferred_even_if_config_net_ns_enabled(&self) -> bool {
        self.net_namespace_deferred_even_if_config_net_ns_enabled
    }

    pub const fn pagecache_deferred(&self) -> bool {
        self.pagecache_deferred
    }

    pub const fn pagecache_waitqueue_table_deferred(&self) -> bool {
        self.pagecache_waitqueue_table_deferred
    }

    pub const fn signal_core_setup_deferred(&self) -> bool {
        self.signal_core_setup_deferred
    }

    pub const fn seq_file_core_deferred(&self) -> bool {
        self.seq_file_core_deferred
    }

    pub const fn procfs_deferred(&self) -> bool {
        self.procfs_deferred
    }

    pub const fn nsfs_deferred(&self) -> bool {
        self.nsfs_deferred
    }

    pub const fn pidfs_deferred(&self) -> bool {
        self.pidfs_deferred
    }

    pub const fn vfs_pseudo_filesystems_deferred(&self) -> bool {
        self.vfs_pseudo_filesystems_deferred
    }

    pub const fn bdev_chrdev_init_deferred(&self) -> bool {
        self.bdev_chrdev_init_deferred
    }

    pub const fn cpuset_init_trimmed_noop(&self) -> bool {
        self.cpuset_init_trimmed_noop
    }

    pub const fn cpuset_trimmed_because_config_cpusets_disabled(&self) -> bool {
        self.cpuset_trimmed_because_config_cpusets_disabled
    }

    pub const fn cgroup_init_trimmed_noop(&self) -> bool {
        self.cgroup_init_trimmed_noop
    }

    pub const fn cgroup_trimmed_because_config_cgroups_disabled(&self) -> bool {
        self.cgroup_trimmed_because_config_cgroups_disabled
    }

    pub const fn taskstats_init_trimmed_noop(&self) -> bool {
        self.taskstats_init_trimmed_noop
    }

    pub const fn taskstats_trimmed_because_config_taskstats_disabled(&self) -> bool {
        self.taskstats_trimmed_because_config_taskstats_disabled
    }

    pub const fn delayacct_init_trimmed_noop(&self) -> bool {
        self.delayacct_init_trimmed_noop
    }

    pub const fn delayacct_trimmed_because_config_task_delay_acct_disabled(&self) -> bool {
        self.delayacct_trimmed_because_config_task_delay_acct_disabled
    }

    pub const fn acpi_subsystem_init_trimmed_noop(&self) -> bool {
        self.acpi_subsystem_init_trimmed_noop
    }

    pub const fn acpi_trimmed_because_config_acpi_disabled(&self) -> bool {
        self.acpi_trimmed_because_config_acpi_disabled
    }

    pub const fn arch_post_acpi_subsys_init_trimmed_noop(&self) -> bool {
        self.arch_post_acpi_subsys_init_trimmed_noop
    }

    pub const fn kcsan_init_trimmed_noop(&self) -> bool {
        self.kcsan_init_trimmed_noop
    }

    pub const fn kcsan_trimmed_because_config_kcsan_disabled(&self) -> bool {
        self.kcsan_trimmed_because_config_kcsan_disabled
    }

    pub const fn rcu_tasks_generic_out_of_scope(&self) -> bool {
        self.rcu_tasks_generic_out_of_scope
    }

    pub const fn rcu_tasks_generic_belongs_to_kernel_init_freeable(&self) -> bool {
        self.rcu_tasks_generic_belongs_to_kernel_init_freeable
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
    }

    pub fn preset(
        &mut self,
        config: &Config,
        task_creation_core: &TaskCreationCore,
        signal_core: &SignalCore,
        vfs_core: &VfsCore,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || config.state() != State::Online
            || config.x86_arch()
            || config.shadow_call_stack_enabled()
            || config.lockdep_enabled()
            || config.kgdb_enabled()
            || !config.net_ns_enabled()
            || !config.proc_fs_enabled()
            || !config.pid_ns_enabled()
            || config.cpusets_enabled()
            || config.cgroups_enabled()
            || config.taskstats_enabled()
            || config.task_delay_acct_enabled()
            || config.acpi_enabled()
            || config.kcsan_enabled()
            || task_creation_core.state() != State::Ready
            || signal_core.state() != State::Prepared
            || vfs_core.state() != State::Ready
            || !vfs_core.page_cache_deferred()
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.x86_efi_runtime_switch_trimmed_noop = true;
        self.x86_efi_runtime_switch_trimmed_because_arch_riscv = true;
        crate::checkpoint::checkpoint(Checkpoint::X86EfiRuntimeSwitchTrimmed);
        self.shadow_call_stack_init_trimmed_noop = true;
        self.shadow_call_stack_trimmed_because_config_shadow_call_stack_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::ShadowCallStackInitNoop);
        self.lockdep_init_task_trimmed_noop = true;
        self.lockdep_init_task_trimmed_because_config_lockdep_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::LockdepInitTaskNoop);
        self.dbg_late_init_trimmed_noop = true;
        self.dbg_late_init_trimmed_because_config_kgdb_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::DbgLateInitNoop);
        self.net_namespace_deferred = true;
        self.net_namespace_deferred_even_if_config_net_ns_enabled = true;
        crate::checkpoint::checkpoint(Checkpoint::NetNamespaceDeferred);
        self.pagecache_deferred = true;
        self.pagecache_waitqueue_table_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::PageCacheDeferred);
        self.signal_core_setup_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::SignalCoreSetupDeferred);
        self.seq_file_core_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::SeqFileCoreDeferred);
        self.procfs_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::ProcfsDeferred);
        self.nsfs_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::NsfsDeferred);
        self.pidfs_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::PidfsDeferred);
        self.vfs_pseudo_filesystems_deferred = true;
        self.bdev_chrdev_init_deferred = true;
        self.cpuset_init_trimmed_noop = true;
        self.cpuset_trimmed_because_config_cpusets_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::CpusetNoop);
        self.cgroup_init_trimmed_noop = true;
        self.cgroup_trimmed_because_config_cgroups_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::CgroupNoop);
        self.taskstats_init_trimmed_noop = true;
        self.taskstats_trimmed_because_config_taskstats_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::TaskstatsNoop);
        self.delayacct_init_trimmed_noop = true;
        self.delayacct_trimmed_because_config_task_delay_acct_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::DelayAccountingNoop);
        self.acpi_subsystem_init_trimmed_noop = true;
        self.acpi_trimmed_because_config_acpi_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::AcpiSubsystemNoop);
        self.arch_post_acpi_subsys_init_trimmed_noop = true;
        crate::checkpoint::checkpoint(Checkpoint::ArchPostAcpiNoop);
        self.kcsan_init_trimmed_noop = true;
        self.kcsan_trimmed_because_config_kcsan_disabled = true;
        crate::checkpoint::checkpoint(Checkpoint::KcsanNoop);
        self.rcu_tasks_generic_out_of_scope = true;
        self.rcu_tasks_generic_belongs_to_kernel_init_freeable = true;
        self.position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::ProcessPrepareTrimmedPathsPrepared,
        )
    }
}
