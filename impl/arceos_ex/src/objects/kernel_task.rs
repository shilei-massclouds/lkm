//! Bounded, explicitly placed tasks and the CPU-local inbound inbox.
//!
//! The creating CPU owns a slot only until `PUBLISHED` is released.  After
//! that boundary the target CPU is the sole mutable owner of the Task; remote
//! activation and wake paths communicate only through the inbox below.

#![cfg_attr(not(app_smoke), allow(dead_code))]

use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
};

use super::{
    cpu::{CpuRef, MAX_CPUS},
    current_task::CurrentTaskCandidate,
    state::{EventResult, State},
    task::{KERNEL_TASK_SLOT_COUNT, Task, TaskEntry, TaskKind, TaskRef, USER_TASK_SLOT_COUNT},
};

const KERNEL_TASK_STACK_WORDS: usize = 4096;
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

// Dynamic user/kernel Tasks occupy their generation-indexed slots. PID 1 is
// also a deliverable blocked-wake target even though its stable TaskRef does
// not use either dynamic slot encoding.
const PID1_INBOX_INDEX: usize = USER_TASK_SLOT_COUNT + KERNEL_TASK_SLOT_COUNT;
const INBOX_SLOT_COUNT: usize = PID1_INBOX_INDEX + 1;
const INBOX_EMPTY: u8 = 0;
const INBOX_RESERVED: u8 = 1;
const INBOX_PUBLISHED: u8 = 2;
const INBOX_CONSUMING: u8 = 3;

struct InboundSlot {
    state: AtomicU8,
    task_generation: AtomicUsize,
    target_cpu: AtomicUsize,
    kind: AtomicUsize,
    ordinal: AtomicU64,
}

impl InboundSlot {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(INBOX_EMPTY),
            task_generation: AtomicUsize::new(0),
            target_cpu: AtomicUsize::new(usize::MAX),
            kind: AtomicUsize::new(0),
            ordinal: AtomicU64::new(0),
        }
    }
}

struct CpuInbox {
    slots: [InboundSlot; INBOX_SLOT_COUNT],
    last_claimed_ordinal: AtomicU64,
    max_consumed_ordinal: AtomicU64,
    need_resched: AtomicBool,
    ipi_sent: AtomicU64,
    ipi_received: AtomicU64,
    consumed: AtomicU64,
}

impl CpuInbox {
    const fn new() -> Self {
        Self {
            slots: [const { InboundSlot::new() }; INBOX_SLOT_COUNT],
            last_claimed_ordinal: AtomicU64::new(0),
            max_consumed_ordinal: AtomicU64::new(0),
            need_resched: AtomicBool::new(false),
            ipi_sent: AtomicU64::new(0),
            ipi_received: AtomicU64::new(0),
            consumed: AtomicU64::new(0),
        }
    }
}

static INBOXES: [CpuInbox; MAX_CPUS] = [const { CpuInbox::new() }; MAX_CPUS];
static NEXT_AUTO_ORDINAL: AtomicU64 = AtomicU64::new(1);
static NONIDENTITY_SWITCHES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static IDENTITY_SCHEDULES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static IDLE_RESTORES: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

pub(crate) fn create(
    target_cpu: CpuRef,
    entry: extern "C" fn() -> !,
) -> Result<TaskRef, &'static str> {
    if !target_cpu.is_valid() || target_cpu.is_boot_cpu() {
        return Err("kernel-task-target");
    }
    let slot = PUBLISHED
        .iter()
        .position(|published| !published.load(Ordering::Acquire))
        .ok_or("kernel-task-slots-exhausted")?;

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
    if task_ref.same_identity(TaskRef::KERNEL_INIT) {
        return super::user_process_registry::global_registry()
            .inbound_target_valid(task_ref, target_cpu);
    }
    if task_ref.is_user() {
        return super::user_process_registry::global_registry()
            .inbound_target_valid(task_ref, target_cpu);
    }
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

fn validate_reservable_target(task_ref: TaskRef, target_cpu: CpuRef) -> bool {
    if task_ref.same_identity(TaskRef::KERNEL_INIT) {
        return super::user_process_registry::global_registry()
            .inbound_target_valid(task_ref, target_cpu);
    }
    if task_ref.is_user() {
        let registry = super::user_process_registry::global_registry();
        return registry.inbound_target_valid(task_ref, target_cpu)
            || registry.inbound_target_reservable(task_ref, target_cpu);
    }
    validate_published_target(task_ref, target_cpu)
}

fn inbox_index(task_ref: TaskRef) -> Option<usize> {
    if task_ref.same_identity(TaskRef::KERNEL_INIT) {
        Some(PID1_INBOX_INDEX)
    } else if let Some(slot) = task_ref.user_slot() {
        (slot < USER_TASK_SLOT_COUNT).then_some(slot)
    } else {
        let slot = task_ref.kernel_slot()?;
        (slot < KERNEL_TASK_SLOT_COUNT).then_some(USER_TASK_SLOT_COUNT + slot)
    }
}

fn task_ref_from_inbox_index(index: usize, generation: u32) -> Option<TaskRef> {
    if index == PID1_INBOX_INDEX {
        (generation == TaskRef::KERNEL_INIT.generation()).then_some(TaskRef::KERNEL_INIT)
    } else if index < USER_TASK_SLOT_COUNT {
        Some(TaskRef::user(index, generation))
    } else {
        let slot = index.checked_sub(USER_TASK_SLOT_COUNT)?;
        (slot < KERNEL_TASK_SLOT_COUNT).then_some(TaskRef::kernel(slot, generation))
    }
}

fn claim_ordinal(inbox: &CpuInbox, ordinal: u64) -> bool {
    let mut current = inbox.last_claimed_ordinal.load(Ordering::Acquire);
    loop {
        if ordinal <= current {
            return false;
        }
        match inbox.last_claimed_ordinal.compare_exchange_weak(
            current,
            ordinal,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return true,
            Err(observed) => current = observed,
        }
    }
}

pub(crate) struct InboxReservation {
    logical_id: usize,
    index: usize,
    task_ref: TaskRef,
    target_cpu: CpuRef,
    kind: InboundKind,
    ordinal: u64,
    coalesced: bool,
}

pub(crate) fn reserve_inbound(
    task_ref: TaskRef,
    target_cpu: CpuRef,
    kind: InboundKind,
    ordinal: u64,
) -> Result<InboxReservation, &'static str> {
    let logical_id = target_cpu.logical_id();
    if logical_id >= MAX_CPUS || ordinal == 0 {
        return Err("inbox-target-or-ordinal");
    }
    if !validate_reservable_target(task_ref, target_cpu) {
        return Err("inbox-task-ref-or-generation");
    }
    let index = inbox_index(task_ref).ok_or("inbox-task-slot")?;
    let inbox = &INBOXES[logical_id];
    let slot = &inbox.slots[index];

    let state = slot.state.load(Ordering::Acquire);
    if state == INBOX_PUBLISHED
        && slot.task_generation.load(Ordering::Relaxed) as u32 == task_ref.generation()
        && slot.target_cpu.load(Ordering::Relaxed) == logical_id
    {
        if !claim_ordinal(inbox, ordinal) {
            return Err("inbox-ordinal");
        }
        return Ok(InboxReservation {
            logical_id,
            index,
            task_ref,
            target_cpu,
            kind,
            ordinal,
            coalesced: true,
        });
    }
    if slot
        .state
        .compare_exchange(
            INBOX_EMPTY,
            INBOX_RESERVED,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        .is_err()
    {
        return Err("inbox-task-pending");
    }
    if !claim_ordinal(inbox, ordinal) {
        slot.state.store(INBOX_EMPTY, Ordering::Release);
        return Err("inbox-ordinal");
    }
    slot.task_generation
        .store(task_ref.generation() as usize, Ordering::Relaxed);
    slot.target_cpu.store(logical_id, Ordering::Relaxed);
    slot.kind.store(
        match kind {
            InboundKind::Activate => 1,
            InboundKind::Wake => 2,
        },
        Ordering::Relaxed,
    );
    slot.ordinal.store(ordinal, Ordering::Relaxed);
    Ok(InboxReservation {
        logical_id,
        index,
        task_ref,
        target_cpu,
        kind,
        ordinal,
        coalesced: false,
    })
}

pub(crate) fn next_inbound_ordinal() -> u64 {
    loop {
        let ordinal = NEXT_AUTO_ORDINAL.fetch_add(1, Ordering::Relaxed);
        if ordinal != 0 {
            return ordinal;
        }
    }
}

pub(crate) fn publish_reserved_and_signal(
    reservation: InboxReservation,
    target_hartid: usize,
) -> Result<(), &'static str> {
    publish_reserved_and_signal_from(reservation, target_hartid, CpuRef::new(0))
}

pub(crate) fn publish_reserved_and_signal_from(
    reservation: InboxReservation,
    target_hartid: usize,
    producer_cpu: CpuRef,
) -> Result<(), &'static str> {
    let logical_id = reservation.logical_id;
    let newly_published = publish_reserved(reservation)?;
    if newly_published && logical_id != producer_cpu.logical_id() {
        crate::arch::riscv64::sbi::send_ipi(target_hartid).map_err(|_| "sbi-send-ipi")?;
        mark_ipi_sent(logical_id);
    }
    Ok(())
}

pub(crate) fn publish_reserved(reservation: InboxReservation) -> Result<bool, &'static str> {
    if reservation.coalesced {
        return Ok(false);
    }
    let inbox = INBOXES
        .get(reservation.logical_id)
        .ok_or("inbox-reservation-cpu")?;
    let slot = inbox
        .slots
        .get(reservation.index)
        .ok_or("inbox-reservation-slot")?;
    if slot.task_generation.load(Ordering::Relaxed) as u32 != reservation.task_ref.generation()
        || slot.target_cpu.load(Ordering::Relaxed) != reservation.target_cpu.logical_id()
        || slot.ordinal.load(Ordering::Relaxed) != reservation.ordinal
        || slot.kind.load(Ordering::Relaxed)
            != match reservation.kind {
                InboundKind::Activate => 1,
                InboundKind::Wake => 2,
            }
        || slot
            .state
            .compare_exchange(
                INBOX_RESERVED,
                INBOX_PUBLISHED,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_err()
    {
        return Err("inbox-reservation-stale");
    }
    Ok(true)
}

pub(crate) fn rollback_reserved(reservation: InboxReservation) -> bool {
    if reservation.coalesced {
        return true;
    }
    INBOXES
        .get(reservation.logical_id)
        .and_then(|inbox| inbox.slots.get(reservation.index))
        .is_some_and(|slot| {
            slot.state
                .compare_exchange(
                    INBOX_RESERVED,
                    INBOX_EMPTY,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
        })
}

pub(crate) fn publish(
    task_ref: TaskRef,
    target_cpu: CpuRef,
    kind: InboundKind,
    ordinal: u64,
) -> Result<(), &'static str> {
    let reservation = reserve_inbound(task_ref, target_cpu, kind, ordinal)?;
    publish_reserved(reservation)?;
    Ok(())
}

pub(crate) fn publish_and_ipi(
    task_ref: TaskRef,
    target_cpu: CpuRef,
    target_hartid: usize,
    kind: InboundKind,
    ordinal: u64,
) -> Result<(), &'static str> {
    let reservation = reserve_inbound(task_ref, target_cpu, kind, ordinal)?;
    let newly_published = publish_reserved(reservation)?;
    if newly_published {
        crate::arch::riscv64::sbi::send_ipi(target_hartid).map_err(|_| "sbi-send-ipi")?;
        mark_ipi_sent(target_cpu.logical_id());
    }
    Ok(())
}

pub(crate) fn take_inbound(logical_id: usize) -> Option<InboundMessage> {
    let inbox = INBOXES.get(logical_id)?;
    let mut selected = None;
    let mut selected_ordinal = u64::MAX;
    for (index, slot) in inbox.slots.iter().enumerate() {
        if slot.state.load(Ordering::Acquire) == INBOX_PUBLISHED {
            let ordinal = slot.ordinal.load(Ordering::Relaxed);
            if ordinal < selected_ordinal {
                selected = Some(index);
                selected_ordinal = ordinal;
            }
        }
    }
    let index = selected?;
    let slot = &inbox.slots[index];
    if slot
        .state
        .compare_exchange(
            INBOX_PUBLISHED,
            INBOX_CONSUMING,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        .is_err()
    {
        return None;
    }
    let generation = slot.task_generation.load(Ordering::Relaxed) as u32;
    let target = slot.target_cpu.load(Ordering::Relaxed);
    let ordinal = slot.ordinal.load(Ordering::Relaxed);
    let kind = match slot.kind.load(Ordering::Relaxed) {
        1 => Some(InboundKind::Activate),
        2 => Some(InboundKind::Wake),
        _ => None,
    };
    // Exact consumption also clears malformed entries so they cannot poison
    // a task's one-pending-notification slot.
    slot.state.store(INBOX_EMPTY, Ordering::Release);
    let task_ref = task_ref_from_inbox_index(index, generation)?;
    let message = InboundMessage {
        task_ref,
        target_cpu: CpuRef::new(target),
        kind: kind?,
        ordinal,
    };
    // Ordinals are uniquely and monotonically claimed when each per-Task
    // slot is reserved, but independent reservations may be published in a
    // different order. The claimed slot state is therefore the exact-once
    // authority; a global consumed watermark cannot reject a lower, still
    // published ordinal after a higher ordinal was consumed first.
    let fresh = target == logical_id && validate_published_target(task_ref, message.target_cpu);
    if !fresh {
        return None;
    }
    inbox
        .max_consumed_ordinal
        .fetch_max(ordinal, Ordering::Release);
    inbox.consumed.fetch_add(1, Ordering::Relaxed);
    Some(message)
}

pub(crate) fn has_inbound(logical_id: usize) -> bool {
    INBOXES.get(logical_id).is_some_and(|inbox| {
        inbox
            .slots
            .iter()
            .any(|slot| slot.state.load(Ordering::Acquire) == INBOX_PUBLISHED)
    })
}

pub(crate) fn mark_ipi_sent(logical_id: usize) {
    if let Some(inbox) = INBOXES.get(logical_id) {
        inbox.ipi_sent.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn handle_reschedule_ipi(logical_id: usize) {
    if let Some(inbox) = INBOXES.get(logical_id) {
        inbox.need_resched.store(true, Ordering::Release);
        inbox.ipi_received.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn take_need_resched(logical_id: usize) -> bool {
    INBOXES
        .get(logical_id)
        .is_some_and(|inbox| inbox.need_resched.swap(false, Ordering::AcqRel))
}

pub(crate) fn need_resched_pending(logical_id: usize) -> bool {
    INBOXES
        .get(logical_id)
        .is_some_and(|inbox| inbox.need_resched.load(Ordering::Acquire))
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
    let inbox = INBOXES.get(logical_id)?;
    Some((
        inbox.ipi_sent.load(Ordering::Acquire),
        inbox.ipi_received.load(Ordering::Acquire),
        inbox.consumed.load(Ordering::Acquire),
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
