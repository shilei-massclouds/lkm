//! Bounded, explicitly placed kernel tasks and the CPU-local inbound lane.
//!
//! The creating CPU owns a slot only until `PUBLISHED` is released.  After
//! that boundary the target CPU is the sole mutable owner of the Task; remote
//! activation and wake paths communicate only through the mailbox below.

#![cfg_attr(not(app_smoke), allow(dead_code))]

use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

use super::{
    cpu::{CpuRef, MAX_CPUS},
    current_task::CurrentTaskCandidate,
    state::{EventResult, State},
    task::{KERNEL_TASK_SLOT_COUNT, Task, TaskEntry, TaskKind, TaskRef},
};

const KERNEL_TASK_STACK_WORDS: usize = 1024;
const KERNEL_TASK_PID_BASE: usize = 2000;

#[repr(C, align(16))]
struct KernelTaskStack {
    words: [usize; KERNEL_TASK_STACK_WORDS],
}

impl KernelTaskStack {
    const fn new() -> Self {
        Self {
            words: [0; KERNEL_TASK_STACK_WORDS],
        }
    }
}

struct KernelTaskRecord {
    task: Task,
    target_cpu: CpuRef,
    entry: Option<extern "C" fn() -> !>,
}

impl KernelTaskRecord {
    const fn new(slot: usize) -> Self {
        Self {
            task: Task::new_kernel(TaskRef::kernel(slot, 1), 1),
            target_cpu: CpuRef::invalid(),
            entry: None,
        }
    }
}

#[repr(transparent)]
struct KernelTaskCell(UnsafeCell<KernelTaskRecord>);

// Access is split by the release publication boundary described above.  The
// target CPU is the only mutable accessor after publication.
unsafe impl Sync for KernelTaskCell {}

impl KernelTaskCell {
    const fn new(slot: usize) -> Self {
        Self(UnsafeCell::new(KernelTaskRecord::new(slot)))
    }
}

static KERNEL_TASKS: [KernelTaskCell; KERNEL_TASK_SLOT_COUNT] = [
    KernelTaskCell::new(0),
    KernelTaskCell::new(1),
    KernelTaskCell::new(2),
    KernelTaskCell::new(3),
    KernelTaskCell::new(4),
    KernelTaskCell::new(5),
    KernelTaskCell::new(6),
    KernelTaskCell::new(7),
];
static mut KERNEL_TASK_STACKS: [KernelTaskStack; KERNEL_TASK_SLOT_COUNT] =
    [const { KernelTaskStack::new() }; KERNEL_TASK_SLOT_COUNT];
static PUBLISHED: [AtomicBool; KERNEL_TASK_SLOT_COUNT] =
    [const { AtomicBool::new(false) }; KERNEL_TASK_SLOT_COUNT];

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum InboundKind {
    Activate,
    Wake,
}

#[derive(Clone, Copy)]
pub(crate) struct InboundMessage {
    pub task_ref: TaskRef,
    pub target_cpu: CpuRef,
    pub kind: InboundKind,
    pub ordinal: u64,
}

struct CpuMailbox {
    writer: AtomicBool,
    occupied: AtomicBool,
    task_slot: AtomicUsize,
    task_generation: AtomicUsize,
    target_cpu: AtomicUsize,
    kind: AtomicUsize,
    ordinal: AtomicU64,
    last_published: AtomicU64,
    last_consumed: AtomicU64,
    need_resched: AtomicBool,
    ipi_sent: AtomicU64,
    ipi_received: AtomicU64,
    consumed: AtomicU64,
}

impl CpuMailbox {
    const fn new() -> Self {
        Self {
            writer: AtomicBool::new(false),
            occupied: AtomicBool::new(false),
            task_slot: AtomicUsize::new(0),
            task_generation: AtomicUsize::new(0),
            target_cpu: AtomicUsize::new(usize::MAX),
            kind: AtomicUsize::new(0),
            ordinal: AtomicU64::new(0),
            last_published: AtomicU64::new(0),
            last_consumed: AtomicU64::new(0),
            need_resched: AtomicBool::new(false),
            ipi_sent: AtomicU64::new(0),
            ipi_received: AtomicU64::new(0),
            consumed: AtomicU64::new(0),
        }
    }
}

static MAILBOXES: [CpuMailbox; MAX_CPUS] = [const { CpuMailbox::new() }; MAX_CPUS];
static NONIDENTITY_SWITCHES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static IDENTITY_SCHEDULES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static IDLE_RESTORES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

pub(crate) fn create(
    target_cpu: CpuRef,
    entry: extern "C" fn() -> !,
) -> Result<TaskRef, &'static str> {
    let slot = target_cpu.logical_id();
    if !target_cpu.is_valid() || target_cpu.is_boot_cpu() || slot >= KERNEL_TASK_SLOT_COUNT {
        return Err("kernel-task-target");
    }
    if PUBLISHED[slot].load(Ordering::Acquire) {
        return Err("kernel-task-slot-already-published");
    }

    // SAFETY: before the release store this unpublished slot is owned only by
    // the creating CPU. No target-CPU accessor accepts it yet.
    let record = unsafe { &mut *KERNEL_TASKS[slot].0.get() };
    let task_ref = TaskRef::kernel(slot, 1);
    if record.task.state() != State::Base
        || !record.task.task_ref().same_identity(task_ref)
        || record
            .task
            .set_identity_metadata(
                KERNEL_TASK_PID_BASE + slot,
                TaskEntry::KernelTask,
                TaskKind::KernelThread,
            )
            .is_err()
        || record.task.adopt_preset().is_err()
        || record.task.declare_and_bind_embedded_flow().is_err()
    {
        return Err("kernel-task-preset");
    }

    let stack_base = unsafe { core::ptr::addr_of!(KERNEL_TASK_STACKS[slot]) as usize };
    let stack_top = stack_base + core::mem::size_of::<KernelTaskStack>();
    record
        .task
        .init_switch_context(kernel_task_entry, stack_base, stack_top);
    if record.task.adopt_setup().is_err()
        || !record.task.bind_flow_cpu_ref(target_cpu)
        || record.task.publish_embedded_flow().is_err()
    {
        return Err("kernel-task-setup");
    }
    record.target_cpu = target_cpu;
    record.entry = Some(entry);
    PUBLISHED[slot].store(true, Ordering::Release);
    Ok(task_ref)
}

fn validate_published_target(task_ref: TaskRef, target_cpu: CpuRef) -> bool {
    let Some(slot) = task_ref.kernel_slot() else {
        return false;
    };
    if slot >= KERNEL_TASK_SLOT_COUNT || !PUBLISHED[slot].load(Ordering::Acquire) {
        return false;
    }
    // SAFETY: publication freezes target/ref/entry; only Task runtime fields
    // are subsequently mutated by the target CPU.
    let record = unsafe { &*KERNEL_TASKS[slot].0.get() };
    record.task.task_ref().same_identity(task_ref) && record.target_cpu == target_cpu
}

pub(crate) fn publish(
    task_ref: TaskRef,
    target_cpu: CpuRef,
    kind: InboundKind,
    ordinal: u64,
) -> Result<(), &'static str> {
    let cpu = target_cpu.logical_id();
    if cpu == 0 || cpu >= MAX_CPUS || ordinal == 0 {
        return Err("mailbox-target-or-ordinal");
    }
    if !validate_published_target(task_ref, target_cpu) {
        return Err("mailbox-task-ref-or-generation");
    }
    let mailbox = &MAILBOXES[cpu];
    if mailbox
        .writer
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err("mailbox-producer-busy");
    }
    if mailbox.occupied.load(Ordering::Acquire) {
        mailbox.writer.store(false, Ordering::Release);
        return Err("mailbox-full");
    }
    if ordinal <= mailbox.last_published.load(Ordering::Acquire) {
        mailbox.writer.store(false, Ordering::Release);
        return Err("mailbox-ordinal");
    }
    // The lane carries the bounded registry index, not a concrete TaskRef
    // encoding.  Reconstruction therefore remains owned by TaskRef.
    mailbox.task_slot.store(
        task_ref.kernel_slot().ok_or("mailbox-task-slot")?,
        Ordering::Relaxed,
    );
    mailbox
        .task_generation
        .store(task_ref.generation() as usize, Ordering::Relaxed);
    mailbox.target_cpu.store(cpu, Ordering::Relaxed);
    mailbox.kind.store(
        match kind {
            InboundKind::Activate => 1,
            InboundKind::Wake => 2,
        },
        Ordering::Relaxed,
    );
    mailbox.ordinal.store(ordinal, Ordering::Relaxed);
    mailbox.last_published.store(ordinal, Ordering::Relaxed);
    // This is the publication boundary paired with the target's acquire.
    mailbox.occupied.store(true, Ordering::Release);
    mailbox.writer.store(false, Ordering::Release);
    Ok(())
}

pub(crate) fn publish_and_ipi(
    task_ref: TaskRef,
    target_cpu: CpuRef,
    target_hartid: usize,
    kind: InboundKind,
    ordinal: u64,
) -> Result<(), &'static str> {
    publish(task_ref, target_cpu, kind, ordinal)?;
    crate::arch::riscv64::sbi::send_ipi(target_hartid).map_err(|_| "sbi-send-ipi")?;
    mark_ipi_sent(target_cpu.logical_id());
    Ok(())
}

pub(crate) fn take_inbound(logical_id: usize) -> Option<InboundMessage> {
    let mailbox = MAILBOXES.get(logical_id)?;
    if !mailbox.occupied.load(Ordering::Acquire) {
        return None;
    }
    let slot = mailbox.task_slot.load(Ordering::Relaxed);
    let generation = mailbox.task_generation.load(Ordering::Relaxed) as u32;
    let target = mailbox.target_cpu.load(Ordering::Relaxed);
    let ordinal = mailbox.ordinal.load(Ordering::Relaxed);
    let kind = match mailbox.kind.load(Ordering::Relaxed) {
        1 => Some(InboundKind::Activate),
        2 => Some(InboundKind::Wake),
        _ => None,
    };
    // Consumption of the one physical slot is exact even when validation
    // rejects its immutable payload; a malformed slot cannot poison the lane.
    mailbox.occupied.store(false, Ordering::Release);
    let task_ref = TaskRef::kernel(slot, generation);
    let message = InboundMessage {
        task_ref,
        target_cpu: CpuRef::new(target),
        kind: kind?,
        ordinal,
    };
    let fresh = target == logical_id
        && validate_published_target(task_ref, message.target_cpu)
        && ordinal > mailbox.last_consumed.load(Ordering::Relaxed);
    if !fresh {
        return None;
    }
    mailbox.last_consumed.store(ordinal, Ordering::Release);
    mailbox.consumed.fetch_add(1, Ordering::Relaxed);
    Some(message)
}

pub(crate) fn has_inbound(logical_id: usize) -> bool {
    MAILBOXES
        .get(logical_id)
        .is_some_and(|mailbox| mailbox.occupied.load(Ordering::Acquire))
}

pub(crate) fn mark_ipi_sent(logical_id: usize) {
    if let Some(mailbox) = MAILBOXES.get(logical_id) {
        mailbox.ipi_sent.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn handle_reschedule_ipi(logical_id: usize) {
    if let Some(mailbox) = MAILBOXES.get(logical_id) {
        mailbox.need_resched.store(true, Ordering::Release);
        mailbox.ipi_received.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn take_need_resched(logical_id: usize) -> bool {
    MAILBOXES
        .get(logical_id)
        .is_some_and(|mailbox| mailbox.need_resched.swap(false, Ordering::AcqRel))
}

pub(crate) fn need_resched_pending(logical_id: usize) -> bool {
    MAILBOXES
        .get(logical_id)
        .is_some_and(|mailbox| mailbox.need_resched.load(Ordering::Acquire))
}

pub(crate) fn record_nonidentity_switch(logical_id: usize) {
    if let Some(count) = NONIDENTITY_SWITCHES.get(logical_id) {
        count.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn record_identity_schedule(logical_id: usize) {
    if let Some(count) = IDENTITY_SCHEDULES.get(logical_id) {
        count.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn record_idle_restore(logical_id: usize) {
    if let Some(count) = IDLE_RESTORES.get(logical_id) {
        count.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn task_candidate_by_ref(task_ref: TaskRef) -> Option<CurrentTaskCandidate<'static>> {
    let task = task_by_ref(task_ref)?;
    Some(CurrentTaskCandidate {
        task,
        flow: task.embedded_flow(),
    })
}

pub(crate) fn task_candidate_by_identity(identity: usize) -> Option<CurrentTaskCandidate<'static>> {
    let mut slot = 0usize;
    while slot < KERNEL_TASK_SLOT_COUNT {
        if PUBLISHED[slot].load(Ordering::Acquire) {
            // SAFETY: immutable identity/Flow metadata was release-published.
            let record = unsafe { &*KERNEL_TASKS[slot].0.get() };
            if core::ptr::addr_of!(record.task) as usize == identity {
                return Some(CurrentTaskCandidate {
                    task: &record.task,
                    flow: record.task.embedded_flow(),
                });
            }
        }
        slot += 1;
    }
    None
}

pub(crate) fn task_by_ref(task_ref: TaskRef) -> Option<&'static Task> {
    let slot = task_ref.kernel_slot()?;
    if slot >= KERNEL_TASK_SLOT_COUNT || !PUBLISHED[slot].load(Ordering::Acquire) {
        return None;
    }
    // SAFETY: callers use this only for owner-CPU scheduling observations.
    let record = unsafe { &*KERNEL_TASKS[slot].0.get() };
    record
        .task
        .task_ref()
        .same_identity(task_ref)
        .then_some(&record.task)
}

pub(crate) fn task_mut_by_ref_on_cpu(
    task_ref: TaskRef,
    logical_id: usize,
) -> Option<&'static mut Task> {
    let slot = task_ref.kernel_slot()?;
    if slot >= KERNEL_TASK_SLOT_COUNT || !PUBLISHED[slot].load(Ordering::Acquire) {
        return None;
    }
    // SAFETY: the target CPU is the only post-publication mutable owner.
    let record = unsafe { &mut *KERNEL_TASKS[slot].0.get() };
    if record.target_cpu.logical_id() != logical_id
        || !record.task.task_ref().same_identity(task_ref)
    {
        return None;
    }
    Some(&mut record.task)
}

pub(crate) fn activate_for_enqueue(task_ref: TaskRef, logical_id: usize) -> EventResult {
    let task = task_mut_by_ref_on_cpu(task_ref, logical_id).ok_or_else(|| {
        super::state::EventError::failed(
            super::state::EventErrorCode::ConditionFailed,
            super::state::LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            State::Online,
        )
    })?;
    task.adopt_enable()?;
    task.pin_to_cpu(logical_id)
}

pub(crate) fn wake_for_enqueue(task_ref: TaskRef, logical_id: usize) -> EventResult {
    task_mut_by_ref_on_cpu(task_ref, logical_id)
        .ok_or_else(|| {
            super::state::EventError::failed(
                super::state::EventErrorCode::ConditionFailed,
                super::state::LifecycleEvent::Dispatch,
                State::Online,
                State::Online,
                State::Online,
            )
        })?
        .wake_for_scheduler_enqueue()
}

extern "C" fn kernel_task_entry() -> ! {
    let identity = crate::arch::riscv64::csr::read_tp();
    let Some(candidate) = task_candidate_by_identity(identity) else {
        fail_stop("kernel-task-identity");
    };
    let task_ref = candidate.task.task_ref();
    let Some(slot) = task_ref.kernel_slot() else {
        fail_stop("kernel-task-slot");
    };
    let logical_id = candidate.flow.cpu_id();
    if crate::context::finish_secondary_task_switch(logical_id, task_ref).is_err() {
        fail_stop("kernel-task-finish-switch");
    }
    // SAFETY: entry is immutable after release publication.
    let entry = unsafe { (&*KERNEL_TASKS[slot].0.get()).entry };
    match entry {
        Some(entry) => entry(),
        None => fail_stop("kernel-task-entry"),
    }
}

fn fail_stop(reason: &'static str) -> ! {
    crate::arch::riscv64::sbi::putstr("kernel task failure: ");
    crate::arch::riscv64::sbi::putstr(reason);
    crate::arch::riscv64::sbi::putchar(b'\n');
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}

#[cfg(app_smoke)]
pub(crate) fn counters(logical_id: usize) -> Option<(u64, u64, u64)> {
    let mailbox = MAILBOXES.get(logical_id)?;
    Some((
        mailbox.ipi_sent.load(Ordering::Acquire),
        mailbox.ipi_received.load(Ordering::Acquire),
        mailbox.consumed.load(Ordering::Acquire),
    ))
}

#[cfg(app_smoke)]
pub(crate) fn switch_counters(logical_id: usize) -> Option<(u64, u64, u64)> {
    Some((
        NONIDENTITY_SWITCHES
            .get(logical_id)?
            .load(Ordering::Acquire),
        IDENTITY_SCHEDULES.get(logical_id)?.load(Ordering::Acquire),
        IDLE_RESTORES.get(logical_id)?.load(Ordering::Acquire),
    ))
}
