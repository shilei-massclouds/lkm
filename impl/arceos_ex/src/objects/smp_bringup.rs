use core::arch::global_asm;
use core::sync::atomic::Ordering;

use super::{
    cpu::{MAX_CPUS, SecondaryCpuStore},
    cpu_control::{LocalInterruptControl, RawSpinLock},
    cpu_group::CpuGroup,
    irq_time::SbiIpi,
    kernel_image::KernelImage,
    lds::Lds,
    mutex::{Mutex, MutexLockOutcome, MutexOwner},
    per_cpu_storage::PerCpuStorage,
    percpu_rw_semaphore::{
        PerCpuRwSemaphore, PerCpuRwSemaphoreOwner, PerCpuRwSemaphoreReadOutcome,
        PerCpuRwSemaphoreWriteOutcome,
    },
    pre_smp_init::PreSmpInitBoundary,
    rest_init::{BootIdleFlow, KernelInitTask, KthreaddTask},
    sbi::Sbi,
    scheduler::Scheduler,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
};
use crate::checkpoint::Checkpoint;

const AP_STACK_SIZE: usize = 16 * 1024;
const SSTATUS_FPU_VECTOR_MASK: usize = (0b11 << 9) | (0b11 << 13);
const AP_BOOT_DATA_TASK_PTR_OFFSET: usize = 16;
const AP_BOOT_DATA_STACK_PTR_OFFSET: usize = 24;
const AP_BOOT_DATA_SATP_OFFSET: usize = 32;
const AP_BOOT_DATA_GP_OFFSET: usize = 40;
const AP_BOOT_DATA_RUST_ENTRY_OFFSET: usize = 48;
const AP_BOOT_DATA_VIRT_OFFSET: usize = 56;
const AP_BOOT_DATA_KERNEL_VIRT_OFFSET_OFFSET: usize = 64;

#[repr(C, align(64))]
struct SbiHartBootData {
    logical_id: usize,
    hartid: usize,
    task_ptr: usize,
    stack_ptr: usize,
    satp: usize,
    gp: usize,
    rust_entry: usize,
    boot_data_virt: usize,
    kernel_virt_offset: usize,
}

impl SbiHartBootData {
    const fn empty() -> Self {
        Self {
            logical_id: usize::MAX,
            hartid: usize::MAX,
            task_ptr: 0,
            stack_ptr: 0,
            satp: 0,
            gp: 0,
            rust_entry: 0,
            boot_data_virt: 0,
            kernel_virt_offset: 0,
        }
    }
}

#[repr(C, align(64))]
struct ApIdleTaskRecord {
    logical_id: usize,
    hartid: usize,
}

impl ApIdleTaskRecord {
    const fn empty() -> Self {
        Self {
            logical_id: usize::MAX,
            hartid: usize::MAX,
        }
    }
}

#[repr(C, align(4096))]
struct ApStack {
    bytes: [u8; AP_STACK_SIZE],
}

impl ApStack {
    const fn new() -> Self {
        Self {
            bytes: [0; AP_STACK_SIZE],
        }
    }
}

#[unsafe(link_section = ".data.ap_boot")]
static mut AP_BOOT_DATA: [SbiHartBootData; MAX_CPUS] =
    [const { SbiHartBootData::empty() }; MAX_CPUS];

#[unsafe(link_section = ".data.ap_boot")]
static mut AP_IDLE_TASKS: [ApIdleTaskRecord; MAX_CPUS] =
    [const { ApIdleTaskRecord::empty() }; MAX_CPUS];

#[unsafe(link_section = ".bss.ap_stack")]
static mut AP_STACKS: [ApStack; MAX_CPUS] = [const { ApStack::new() }; MAX_CPUS];

global_asm!(
    r#"
    .section .head.text.ap, "ax"
    .align 2
    .globl arceos_ex_secondary_start_sbi
arceos_ex_secondary_start_sbi:
    /*
     * SBI HSM enters with a0 = hartid and a1 = opaque boot-data PA.
     * The AP consumes boot data while translation is off, switches to the
     * already-built swapper page table, then calls the Rust AP bringup entry
     * using virtual addresses.
     */
    csrw    sie, zero
    csrw    sip, zero
    li      t0, {sstatus_fpu_vector_mask}
    csrrc   zero, sstatus, t0

    mv      s0, a1
    ld      tp, {task_ptr_offset}(s0)
    ld      sp, {stack_ptr_offset}(s0)
    ld      t1, {satp_offset}(s0)
    ld      t2, {gp_offset}(s0)
    ld      t3, {rust_entry_offset}(s0)
    ld      t4, {boot_data_virt_offset}(s0)
    ld      t5, {kernel_virt_offset_offset}(s0)

    la      t0, 1f
    add     t0, t0, t5
    csrw    stvec, t0
    sfence.vma
    csrw    satp, t1
    .balign 4
1:
    mv      gp, t2
    mv      a0, t4
    jr      t3
"#,
    boot_data_virt_offset = const AP_BOOT_DATA_VIRT_OFFSET,
    gp_offset = const AP_BOOT_DATA_GP_OFFSET,
    kernel_virt_offset_offset = const AP_BOOT_DATA_KERNEL_VIRT_OFFSET_OFFSET,
    rust_entry_offset = const AP_BOOT_DATA_RUST_ENTRY_OFFSET,
    satp_offset = const AP_BOOT_DATA_SATP_OFFSET,
    sstatus_fpu_vector_mask = const SSTATUS_FPU_VECTOR_MASK,
    stack_ptr_offset = const AP_BOOT_DATA_STACK_PTR_OFFSET,
    task_ptr_offset = const AP_BOOT_DATA_TASK_PTR_OFFSET,
);

unsafe extern "C" {
    fn arceos_ex_secondary_start_sbi();
}

#[unsafe(no_mangle)]
extern "C" fn arceos_ex_secondary_entry_rust(boot_data: *const SbiHartBootData) -> ! {
    if boot_data.is_null() {
        crate::phases::smp_runtime::smp_bringup::ap_phase_fail_stop(
            "ApEntryPreludePhase",
            usize::MAX,
            State::Base,
            "boot-data-null",
        );
    }

    let observed_sp: usize;
    let observed_tp: usize;
    unsafe {
        core::arch::asm!(
            "mv {observed_sp}, sp",
            "mv {observed_tp}, tp",
            observed_sp = out(reg) observed_sp,
            observed_tp = out(reg) observed_tp,
            options(nomem, nostack),
        );
    }
    let Some(target_logical_id) = ap_boot_data_target_logical_id(boot_data as usize) else {
        crate::phases::smp_runtime::smp_bringup::ap_phase_fail_stop(
            "ApEntryPreludePhase",
            usize::MAX,
            State::Base,
            "boot-data-slot",
        );
    };
    let data = unsafe { &*boot_data };
    let adoption = ApEntryAdoption {
        target_logical_id,
        boot_data_logical_id: data.logical_id,
        boot_data_pointer: boot_data as usize,
        boot_data_self_pointer: data.boot_data_virt,
        boot_data_stack_pointer: data.stack_ptr,
        expected_stack_top: ap_stack_top_virt(target_logical_id).unwrap_or(0),
        observed_sp,
        boot_data_task_pointer: data.task_ptr,
        expected_task_pointer: ap_idle_task_virt(target_logical_id).unwrap_or(0),
        observed_tp,
    };
    crate::phases::smp_runtime::ap_entry_prelude::preset(adoption)
}

pub(crate) struct ApEntryAdoption {
    target_logical_id: usize,
    boot_data_logical_id: usize,
    boot_data_pointer: usize,
    boot_data_self_pointer: usize,
    boot_data_stack_pointer: usize,
    expected_stack_top: usize,
    observed_sp: usize,
    boot_data_task_pointer: usize,
    expected_task_pointer: usize,
    observed_tp: usize,
}

impl ApEntryAdoption {
    pub(crate) const fn logical_id(&self) -> usize {
        self.target_logical_id
    }

    pub(crate) fn boot_data_matches_target(&self) -> bool {
        self.target_logical_id != 0
            && self.target_logical_id < MAX_CPUS
            && self.boot_data_logical_id == self.target_logical_id
            && self.boot_data_pointer == self.boot_data_self_pointer
            && ap_boot_data_virt(self.target_logical_id) == Some(self.boot_data_pointer)
    }

    pub(crate) fn stack_matches_target(&self) -> bool {
        let Some(stack_base) = self.expected_stack_top.checked_sub(AP_STACK_SIZE) else {
            return false;
        };
        self.expected_stack_top != 0
            && self.boot_data_stack_pointer == self.expected_stack_top
            && self.observed_sp >= stack_base
            && self.observed_sp <= self.expected_stack_top
    }

    pub(crate) fn task_pointer_matches_target(&self) -> bool {
        self.expected_task_pointer != 0
            && self.boot_data_task_pointer == self.expected_task_pointer
            && self.observed_tp == self.expected_task_pointer
    }
}

fn ap_boot_data_virt(logical_id: usize) -> Option<usize> {
    if logical_id >= MAX_CPUS {
        return None;
    }
    Some(unsafe { core::ptr::addr_of!(AP_BOOT_DATA[logical_id]) as usize })
}

fn ap_boot_data_target_logical_id(pointer: usize) -> Option<usize> {
    let mut logical_id = 1usize;
    while logical_id < MAX_CPUS {
        if ap_boot_data_virt(logical_id) == Some(pointer) {
            return Some(logical_id);
        }
        logical_id += 1;
    }
    None
}

fn ap_idle_task_virt(logical_id: usize) -> Option<usize> {
    if logical_id >= MAX_CPUS {
        return None;
    }
    Some(unsafe { core::ptr::addr_of!(AP_IDLE_TASKS[logical_id]) as usize })
}

fn ap_stack_top_virt(logical_id: usize) -> Option<usize> {
    if logical_id >= MAX_CPUS {
        return None;
    }
    let stack = unsafe { core::ptr::addr_of!(AP_STACKS[logical_id]) as usize };
    stack.checked_add(AP_STACK_SIZE)
}

pub struct SecondaryIdleTaskSet {
    lifecycle: Lifecycle,
    prepared_count: usize,
    inactive: bool,
    per_secondary_idle_task: bool,
    dedicated_stack: bool,
    pt_regs_stack_pointer: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SecondaryIdleTaskSet {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            prepared_count: 0,
            inactive: true,
            per_secondary_idle_task: false,
            dedicated_stack: false,
            pt_regs_stack_pointer: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn prepared_count(&self) -> usize {
        self.prepared_count
    }

    pub const fn inactive(&self) -> bool {
        self.inactive
    }

    pub const fn per_secondary_idle_task(&self) -> bool {
        self.per_secondary_idle_task
    }

    pub const fn dedicated_stack(&self) -> bool {
        self.dedicated_stack
    }

    pub const fn pt_regs_stack_pointer(&self) -> bool {
        self.pt_regs_stack_pointer
    }

    pub fn preset(
        &mut self,
        boundary: &PreSmpInitBoundary,
        cpu_group: &CpuGroup,
        scheduler: &Scheduler,
        per_cpu_storage: &PerCpuStorage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || boundary.state() != State::Ready
            || !boundary.smp_init_not_called()
            || !boundary.secondary_cpus_present_not_online()
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_present_not_online()
            || scheduler.state() != State::Online
            || per_cpu_storage.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.prepared_count = cpu_group.secondary_count();
        self.inactive = true;
        self.per_secondary_idle_task = self.prepared_count != 0;
        self.dedicated_stack = self.prepared_count != 0;
        self.pt_regs_stack_pointer = self.prepared_count != 0;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SecondaryIdleTasksPrepared,
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

pub struct CpuHotplugSyncSet {
    lifecycle: Lifecycle,
    cpu_running_ready: bool,
    cpu_running_observed: bool,
    done_up_ready: bool,
    done_up_observed: bool,
    done_down_ready: bool,
    boot_cpu_hotplug_thread_online: bool,
    secondary_hotplug_threads_deferred: bool,
    cpus_read_guard_used: bool,
    smpboot_threads_mutex_guard_used: bool,
    cpu_running_wait_lock_guard_used: bool,
    done_up_wait_lock_guard_used: bool,
}

impl CpuHotplugSyncSet {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cpu_running_ready: false,
            cpu_running_observed: false,
            done_up_ready: false,
            done_up_observed: false,
            done_down_ready: false,
            boot_cpu_hotplug_thread_online: false,
            secondary_hotplug_threads_deferred: true,
            cpus_read_guard_used: false,
            smpboot_threads_mutex_guard_used: false,
            cpu_running_wait_lock_guard_used: false,
            done_up_wait_lock_guard_used: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cpu_running_ready(&self) -> bool {
        self.cpu_running_ready
    }

    pub const fn cpu_running_observed(&self) -> bool {
        self.cpu_running_observed
    }

    pub const fn done_up_ready(&self) -> bool {
        self.done_up_ready
    }

    pub const fn done_up_observed(&self) -> bool {
        self.done_up_observed
    }

    pub const fn done_down_ready(&self) -> bool {
        self.done_down_ready
    }

    pub const fn boot_cpu_hotplug_thread_online(&self) -> bool {
        self.boot_cpu_hotplug_thread_online
    }

    pub const fn secondary_hotplug_threads_deferred(&self) -> bool {
        self.secondary_hotplug_threads_deferred
    }

    pub const fn cpus_read_guard_used(&self) -> bool {
        self.cpus_read_guard_used
    }

    pub const fn smpboot_threads_mutex_guard_used(&self) -> bool {
        self.smpboot_threads_mutex_guard_used
    }

    pub const fn cpu_running_wait_lock_guard_used(&self) -> bool {
        self.cpu_running_wait_lock_guard_used
    }

    pub const fn done_up_wait_lock_guard_used(&self) -> bool {
        self.done_up_wait_lock_guard_used
    }

    pub fn preset(
        &mut self,
        cpu_group: &CpuGroup,
        boot_idle_flow: &BootIdleFlow,
        kthreadd_task: &KthreaddTask,
        cpu_hotplug_lock: &mut PerCpuRwSemaphore,
        smpboot_threads_lock: &mut Mutex,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
            || !cpu_group.secondary_cpus_present_not_online()
            || boot_idle_flow.state() != State::Ready
            || kthreadd_task.state() != State::Online
            || !cpu_hotplug_lock.ready()
            || !smpboot_threads_lock.ready()
        {
            return self.failed_preset();
        }

        if cpu_hotplug_lock.read_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, 0)?
            != PerCpuRwSemaphoreReadOutcome::AcquiredFast
        {
            return self.failed_preset();
        }
        crate::checkpoint::checkpoint(Checkpoint::CpuHotplugReadGuardUsed);

        if smpboot_threads_lock.lock_owner(MutexOwner::KernelInitTask)?
            != MutexLockOutcome::Acquired
        {
            return self.failed_preset();
        }
        crate::checkpoint::checkpoint(Checkpoint::SmpbootThreadsMutexGuardUsed);

        self.cpu_running_ready = true;
        self.cpu_running_observed = false;
        self.done_up_ready = true;
        self.done_up_observed = false;
        self.done_down_ready = true;
        self.boot_cpu_hotplug_thread_online = true;
        self.secondary_hotplug_threads_deferred = true;
        self.cpus_read_guard_used = true;
        self.smpboot_threads_mutex_guard_used = true;

        smpboot_threads_lock.unlock_owner(MutexOwner::KernelInitTask)?;
        cpu_hotplug_lock.read_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, 0)?;

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CpuHotplugSyncPrepared,
        )
    }

    pub fn observe_cpu_running(
        &mut self,
        wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.cpu_running_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        wait_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.cpu_running_observed = true;
        wait_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.cpu_running_wait_lock_guard_used = true;
        crate::checkpoint::checkpoint(Checkpoint::CpuRunningObserved);
        crate::checkpoint::checkpoint(Checkpoint::CpuRunningWaitLockGuardUsed);
        Ok(())
    }

    pub fn observe_done_up(
        &mut self,
        wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || !self.done_up_ready
            || !self.cpu_running_observed
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        wait_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.done_up_observed = true;
        wait_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        self.done_up_wait_lock_guard_used = true;
        crate::checkpoint::checkpoint(Checkpoint::CpuDoneUpObserved);
        crate::checkpoint::checkpoint(Checkpoint::CpuDoneUpWaitLockGuardUsed);
        Ok(())
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

pub struct CpuStartProvider {
    lifecycle: Lifecycle,
    start_requests_issued: bool,
    secondary_start_sbi_selected: bool,
    boot_data_selected: bool,
    hsm_start_requests_issued: bool,
    hsm_start_return_observed: bool,
    boot_data_per_secondary_cpu: bool,
    boot_data_task_ptr_is_idle_task: bool,
    boot_data_stack_ptr_is_pt_regs_stack: bool,
    cpu_add_remove_mutex_guard_used: bool,
    cpu_hotplug_write_guard_used: bool,
    sbi_boot_data_publish_barriers_observed: bool,
}

impl CpuStartProvider {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start_requests_issued: false,
            secondary_start_sbi_selected: false,
            boot_data_selected: false,
            hsm_start_requests_issued: false,
            hsm_start_return_observed: false,
            boot_data_per_secondary_cpu: false,
            boot_data_task_ptr_is_idle_task: false,
            boot_data_stack_ptr_is_pt_regs_stack: false,
            cpu_add_remove_mutex_guard_used: false,
            cpu_hotplug_write_guard_used: false,
            sbi_boot_data_publish_barriers_observed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn start_requests_issued(&self) -> bool {
        self.start_requests_issued
    }

    pub const fn secondary_start_sbi_selected(&self) -> bool {
        self.secondary_start_sbi_selected
    }

    pub const fn boot_data_selected(&self) -> bool {
        self.boot_data_selected
    }

    pub const fn hsm_start_requests_issued(&self) -> bool {
        self.hsm_start_requests_issued
    }

    pub const fn hsm_start_return_observed(&self) -> bool {
        self.hsm_start_return_observed
    }

    pub const fn boot_data_per_secondary_cpu(&self) -> bool {
        self.boot_data_per_secondary_cpu
    }

    pub const fn boot_data_task_ptr_is_idle_task(&self) -> bool {
        self.boot_data_task_ptr_is_idle_task
    }

    pub const fn boot_data_stack_ptr_is_pt_regs_stack(&self) -> bool {
        self.boot_data_stack_ptr_is_pt_regs_stack
    }

    pub const fn cpu_add_remove_mutex_guard_used(&self) -> bool {
        self.cpu_add_remove_mutex_guard_used
    }

    pub const fn cpu_hotplug_write_guard_used(&self) -> bool {
        self.cpu_hotplug_write_guard_used
    }

    pub const fn sbi_boot_data_publish_barriers_observed(&self) -> bool {
        self.sbi_boot_data_publish_barriers_observed
    }

    // CPU start setup retains the complete specified hotplug and SBI publication boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        idle_tasks: &SecondaryIdleTaskSet,
        sync: &CpuHotplugSyncSet,
        sbi: &Sbi,
        sbi_ipi: &SbiIpi,
        kernel_image: &KernelImage,
        static_objects: &StaticObjects,
        lds: &Lds,
        cpu_add_remove_lock: &mut Mutex,
        cpu_hotplug_lock: &mut PerCpuRwSemaphore,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_present_not_online()
            || idle_tasks.state() != State::Prepared
            || sync.state() != State::Prepared
            || !sync.cpu_running_ready()
            || !sync.done_up_ready()
            || !sync.done_down_ready()
            || !sync.cpus_read_guard_used()
            || !sync.smpboot_threads_mutex_guard_used()
            || sbi.state() != State::Ready
            || !sbi.hsm_available()
            || sbi_ipi.state() != State::Ready
            || kernel_image.state() != State::Online
            || static_objects.state() != State::Online
            || lds.state() != State::Online
            || !idle_tasks.per_secondary_idle_task()
            || !idle_tasks.dedicated_stack()
            || !idle_tasks.pt_regs_stack_pointer()
            || !cpu_add_remove_lock.ready()
            || !cpu_hotplug_lock.ready()
        {
            return self.failed_setup();
        }

        if cpu_add_remove_lock.lock_owner(MutexOwner::KernelInitTask)? != MutexLockOutcome::Acquired
        {
            return self.failed_setup();
        }

        if cpu_hotplug_lock.write_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask)?
            != PerCpuRwSemaphoreWriteOutcome::Acquired
        {
            return self.failed_setup();
        }

        if !self.prepare_and_start_secondary_cpus(cpu_group, kernel_image, static_objects, lds) {
            let _ = cpu_hotplug_lock.write_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask);
            let _ = cpu_add_remove_lock.unlock_owner(MutexOwner::KernelInitTask);
            return self.failed_setup();
        }

        self.cpu_add_remove_mutex_guard_used = true;
        self.cpu_hotplug_write_guard_used = true;
        crate::checkpoint::checkpoint(Checkpoint::CpuHotplugWriteGuardUsed);

        cpu_hotplug_lock.write_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask)?;
        cpu_add_remove_lock.unlock_owner(MutexOwner::KernelInitTask)?;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::CpuStartProviderReady,
        )
    }

    fn prepare_and_start_secondary_cpus(
        &mut self,
        cpu_group: &CpuGroup,
        kernel_image: &KernelImage,
        static_objects: &StaticObjects,
        lds: &Lds,
    ) -> bool {
        let Some(entry_pa) =
            kernel_image.runtime_to_phys(arceos_ex_secondary_start_sbi as *const () as usize)
        else {
            return false;
        };
        let Some(satp) = static_objects.swapper_satp(kernel_image) else {
            return false;
        };
        let gp = lds.global_pointer();
        let rust_entry = arceos_ex_secondary_entry_rust as *const () as usize;

        let mut logical_id = 1usize;
        while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
            let Some(cpu) = cpu_group.cpu(logical_id) else {
                return false;
            };
            let Some(task_ptr) = ap_idle_task_virt(logical_id) else {
                return false;
            };
            let Some(stack_ptr) = ap_stack_top_virt(logical_id) else {
                return false;
            };
            let Some(boot_data_virt) = ap_boot_data_virt(logical_id) else {
                return false;
            };
            let Some(boot_data_pa) = kernel_image.runtime_to_phys(boot_data_virt) else {
                return false;
            };
            unsafe {
                AP_IDLE_TASKS[logical_id] = ApIdleTaskRecord {
                    logical_id,
                    hartid: cpu.hartid(),
                };
                AP_BOOT_DATA[logical_id] = SbiHartBootData {
                    logical_id,
                    hartid: cpu.hartid(),
                    task_ptr,
                    stack_ptr,
                    satp,
                    gp,
                    rust_entry,
                    boot_data_virt,
                    kernel_virt_offset: kernel_image.virt_offset(),
                };
            }

            self.secondary_start_sbi_selected = true;
            self.boot_data_selected = true;
            self.boot_data_per_secondary_cpu = true;
            self.boot_data_task_ptr_is_idle_task = true;
            self.boot_data_stack_ptr_is_pt_regs_stack = true;
            crate::checkpoint::checkpoint(Checkpoint::CpuStartProviderBootDataSelected);
            core::sync::atomic::fence(Ordering::SeqCst);
            self.sbi_boot_data_publish_barriers_observed = true;
            crate::checkpoint::checkpoint(Checkpoint::CpuStartProviderBootDataPublished);

            self.start_requests_issued = true;
            self.hsm_start_requests_issued = true;
            crate::checkpoint::checkpoint(Checkpoint::CpuStartProviderHsmStartIssued);
            if crate::arch::riscv64::sbi::hart_start(cpu.hartid(), entry_pa, boot_data_pa).is_err()
            {
                return false;
            }
            self.hsm_start_return_observed = true;
            crate::checkpoint::checkpoint(Checkpoint::CpuStartProviderHsmStartReturned);

            logical_id += 1;
        }

        self.start_requests_issued
            && self.secondary_start_sbi_selected
            && self.boot_data_selected
            && self.hsm_start_requests_issued
            && self.hsm_start_return_observed
            && self.sbi_boot_data_publish_barriers_observed
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

pub struct SecondaryCpuStartupAck {
    lifecycle: Lifecycle,
    acknowledged: bool,
    ap_smp_callin_ack_matches_secondary_cpu: bool,
}

impl SecondaryCpuStartupAck {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            acknowledged: false,
            ap_smp_callin_ack_matches_secondary_cpu: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub const fn ap_smp_callin_ack_matches_secondary_cpu(&self) -> bool {
        self.ap_smp_callin_ack_matches_secondary_cpu
    }

    pub fn setup(
        &mut self,
        start_provider: &CpuStartProvider,
        cpu_group: &CpuGroup,
        sync: &mut CpuHotplugSyncSet,
        cpu_running_wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || start_provider.state() != State::Ready
            || !start_provider.start_requests_issued()
            || !crate::phases::smp_runtime::ap_smp_callin::all_online(cpu_group)
            || !crate::phases::smp_runtime::ap_smp_callin::all_callin_facts(cpu_group)
            || !start_provider.cpu_add_remove_mutex_guard_used()
            || !start_provider.cpu_hotplug_write_guard_used()
            || !start_provider.sbi_boot_data_publish_barriers_observed()
            || sync.state() != State::Prepared
        {
            return self.failed_setup();
        }

        sync.observe_cpu_running(cpu_running_wait_lock, local_interrupt, scheduler)?;
        self.acknowledged = true;
        self.ap_smp_callin_ack_matches_secondary_cpu = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SecondaryCpuStartupAckReady,
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

pub struct SecondaryCpuOnlineAck {
    lifecycle: Lifecycle,
    acknowledged: bool,
    online_after_ap_ack: bool,
    ap_idle_or_park_loop_entered: bool,
    ap_local_irq_enable_observed: bool,
    ap_cache_tlb_flush_summary_observed: bool,
    ap_ipi_enable_observed: bool,
    ap_hotplug_thread_mb_pair_deferred: bool,
}

impl SecondaryCpuOnlineAck {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            acknowledged: false,
            online_after_ap_ack: false,
            ap_idle_or_park_loop_entered: false,
            ap_local_irq_enable_observed: false,
            ap_cache_tlb_flush_summary_observed: false,
            ap_ipi_enable_observed: false,
            ap_hotplug_thread_mb_pair_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub const fn online_after_ap_ack(&self) -> bool {
        self.online_after_ap_ack
    }

    pub const fn ap_idle_or_park_loop_entered(&self) -> bool {
        self.ap_idle_or_park_loop_entered
    }

    pub const fn ap_local_irq_enable_observed(&self) -> bool {
        self.ap_local_irq_enable_observed
    }

    pub const fn ap_cache_tlb_flush_summary_observed(&self) -> bool {
        self.ap_cache_tlb_flush_summary_observed
    }

    pub const fn ap_ipi_enable_observed(&self) -> bool {
        self.ap_ipi_enable_observed
    }

    pub const fn ap_hotplug_thread_mb_pair_deferred(&self) -> bool {
        self.ap_hotplug_thread_mb_pair_deferred
    }

    // Online acknowledgement setup commits all specified AP and synchronization facts.
    #[allow(clippy::too_many_arguments)]
    pub fn setup(
        &mut self,
        startup_ack: &SecondaryCpuStartupAck,
        sync: &mut CpuHotplugSyncSet,
        cpu_group: &mut CpuGroup,
        secondary_cpus: &mut SecondaryCpuStore,
        sbi_ipi: &SbiIpi,
        done_up_wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || startup_ack.state() != State::Ready
            || !startup_ack.acknowledged()
            || !crate::phases::smp_runtime::ap_online_idle::all_online(cpu_group)
            || !crate::phases::smp_runtime::ap_online_idle::all_online_idle_facts(cpu_group)
            || !crate::phases::smp_runtime::ap_online_idle::all_park_loops_entered(cpu_group)
            || !crate::phases::smp_runtime::ap_smp_callin::all_online(cpu_group)
            || sync.state() != State::Prepared
            || sbi_ipi.state() != State::Ready
        {
            return self.failed_setup();
        }

        sync.observe_done_up(done_up_wait_lock, local_interrupt, scheduler)?;
        cpu_group.mark_secondary_cpus_online_after_ap_ack(secondary_cpus)?;
        self.acknowledged = true;
        self.online_after_ap_ack = true;
        self.ap_idle_or_park_loop_entered = true;
        self.ap_local_irq_enable_observed = true;
        self.ap_cache_tlb_flush_summary_observed = true;
        self.ap_ipi_enable_observed = true;
        self.ap_hotplug_thread_mb_pair_deferred = true;
        crate::checkpoint::checkpoint(Checkpoint::SecondaryCpuApLocalSyncSummary);
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SecondaryCpuOnlineAckReady,
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

pub struct SmpBringupBoundary {
    lifecycle: Lifecycle,
    smp_cpus_done_trimmed: bool,
    ap_hotplug_callbacks_deferred: bool,
}

impl SmpBringupBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            smp_cpus_done_trimmed: false,
            ap_hotplug_callbacks_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn smp_cpus_done_trimmed(&self) -> bool {
        self.smp_cpus_done_trimmed
    }

    pub const fn ap_hotplug_callbacks_deferred(&self) -> bool {
        self.ap_hotplug_callbacks_deferred
    }

    pub fn setup(
        &mut self,
        online_ack: &SecondaryCpuOnlineAck,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || online_ack.state() != State::Ready
            || !online_ack.acknowledged()
            || !cpu_group.secondary_cpus_online()
            || !cpu_group.smp_concurrency_open()
        {
            return self.failed_setup();
        }

        self.smp_cpus_done_trimmed = true;
        self.ap_hotplug_callbacks_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SmpBringupBoundaryReady,
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

// Runtime readiness is the specified conjunction across the full SMP handoff.
#[allow(clippy::too_many_arguments)]
pub fn smp_bringup_runtime_ready(
    kernel_init_task: &KernelInitTask,
    cpu_group: &CpuGroup,
    idle_tasks: &SecondaryIdleTaskSet,
    smpboot_threads_lock: &Mutex,
    sync: &CpuHotplugSyncSet,
    cpu_add_remove_lock: &Mutex,
    cpu_hotplug_lock: &PerCpuRwSemaphore,
    cpu_running_wait_lock: &RawSpinLock,
    done_up_wait_lock: &RawSpinLock,
    start_provider: &CpuStartProvider,
    startup_ack: &SecondaryCpuStartupAck,
    online_ack: &SecondaryCpuOnlineAck,
    boundary: &SmpBringupBoundary,
) -> bool {
    kernel_init_task.state() == State::Online
        && cpu_group.state() == State::Ready
        && cpu_group.secondary_cpus_online()
        && cpu_group.smp_concurrency_open()
        && idle_tasks.state() == State::Prepared
        && idle_tasks.inactive()
        && idle_tasks.per_secondary_idle_task()
        && idle_tasks.dedicated_stack()
        && idle_tasks.pt_regs_stack_pointer()
        && smpboot_threads_lock.state() == State::Ready
        && smpboot_threads_lock.ready()
        && sync.state() == State::Prepared
        && sync.cpu_running_ready()
        && sync.cpu_running_observed()
        && sync.done_up_ready()
        && sync.done_up_observed()
        && sync.done_down_ready()
        && sync.boot_cpu_hotplug_thread_online()
        && sync.secondary_hotplug_threads_deferred()
        && sync.cpus_read_guard_used()
        && sync.smpboot_threads_mutex_guard_used()
        && sync.cpu_running_wait_lock_guard_used()
        && sync.done_up_wait_lock_guard_used()
        && cpu_add_remove_lock.state() == State::Ready
        && cpu_add_remove_lock.ready()
        && cpu_hotplug_lock.state() == State::Ready
        && cpu_hotplug_lock.ready()
        && cpu_hotplug_lock.read_lock_count() != 0
        && cpu_hotplug_lock.read_unlock_count() == cpu_hotplug_lock.read_lock_count()
        && cpu_hotplug_lock.write_lock_count() != 0
        && cpu_hotplug_lock.write_unlock_count() == cpu_hotplug_lock.write_lock_count()
        && cpu_running_wait_lock.state() == State::Ready
        && cpu_running_wait_lock.irqsave_entered_count() != 0
        && cpu_running_wait_lock.irqrestore_exited_count()
            == cpu_running_wait_lock.irqsave_entered_count()
        && done_up_wait_lock.state() == State::Ready
        && done_up_wait_lock.irqsave_entered_count() != 0
        && done_up_wait_lock.irqrestore_exited_count() == done_up_wait_lock.irqsave_entered_count()
        && start_provider.state() == State::Ready
        && start_provider.secondary_start_sbi_selected()
        && start_provider.boot_data_selected()
        && start_provider.hsm_start_requests_issued()
        && start_provider.hsm_start_return_observed()
        && start_provider.boot_data_per_secondary_cpu()
        && start_provider.boot_data_task_ptr_is_idle_task()
        && start_provider.boot_data_stack_ptr_is_pt_regs_stack()
        && start_provider.cpu_add_remove_mutex_guard_used()
        && start_provider.cpu_hotplug_write_guard_used()
        && start_provider.sbi_boot_data_publish_barriers_observed()
        && crate::phases::smp_runtime::ap_entry_prelude::all_online(cpu_group)
        && crate::phases::smp_runtime::ap_entry_prelude::all_adoption_facts(cpu_group)
        && crate::phases::smp_runtime::ap_smp_callin::all_online(cpu_group)
        && crate::phases::smp_runtime::ap_smp_callin::all_callin_facts(cpu_group)
        && crate::phases::smp_runtime::ap_online_idle::all_online(cpu_group)
        && crate::phases::smp_runtime::ap_online_idle::all_online_idle_facts(cpu_group)
        && crate::phases::smp_runtime::ap_online_idle::all_park_loops_entered(cpu_group)
        && startup_ack.state() == State::Ready
        && startup_ack.ap_smp_callin_ack_matches_secondary_cpu()
        && online_ack.state() == State::Ready
        && online_ack.online_after_ap_ack()
        && online_ack.ap_idle_or_park_loop_entered()
        && online_ack.ap_local_irq_enable_observed()
        && online_ack.ap_cache_tlb_flush_summary_observed()
        && online_ack.ap_ipi_enable_observed()
        && online_ack.ap_hotplug_thread_mb_pair_deferred()
        && boundary.state() == State::Ready
        && boundary.smp_cpus_done_trimmed()
        && boundary.ap_hotplug_callbacks_deferred()
}
