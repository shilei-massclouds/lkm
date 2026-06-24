use super::{
    cpu_capabilities::CpuCapabilities,
    cpu_group::CpuGroup,
    exception_stream::ExceptionStream,
    init_task::InitTask,
    mm_core::{KmallocCaches, MmStructCache, SlubSubsystem},
    per_cpu_storage::PerCpuStorage,
    scheduler::Scheduler,
    state::{failed_condition, EventError, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::StaticBranch,
    task::TaskEntry,
};
use crate::trace::Checkpoint;

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

pub struct RootPidNamespace {
    lifecycle: Lifecycle,
    idr_ready: bool,
    pid_cache_level: usize,
    pid_max_min: usize,
    pid_max: usize,
    compiletime_limit_checked: bool,
}

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
            || PID_MAX_LIMIT >= PIDNS_ADDING
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
            || inputs.init_task.state() != State::Online
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
            || inputs.entry == TaskEntry::SmokeScheduler
            || inputs.src_task.state() != State::Online
            || inputs.root_pid_namespace.state() != State::Ready
            || inputs.credential_core.state() != State::Prepared
            || inputs.signal_core.state() != State::Prepared
            || inputs.task_file_context.state() != State::Prepared
            || inputs.security_core.state() != State::Ready
            || inputs.scheduler.state() != State::Online
            || inputs.scheduler.boot_runqueue().state() != State::Ready
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
            TaskEntry::None | TaskEntry::SmokeScheduler => {}
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
    pub src_task: &'a InitTask,
    pub root_pid_namespace: &'a RootPidNamespace,
    pub credential_core: &'a CredentialCore,
    pub signal_core: &'a SignalCore,
    pub task_file_context: &'a TaskFileContext,
    pub security_core: &'a SecurityCore,
    pub scheduler: &'a Scheduler,
    pub entry: TaskEntry,
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
    pub init_task: &'a InitTask,
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
