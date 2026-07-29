use super::{
    boot_args::BootArgs,
    exception_type::ExceptionType,
    interrupt_type::InterruptType,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    trap_type::TrapType,
};
use crate::checkpoint::Checkpoint;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

pub(crate) const MAX_CPUS: usize = 16;
pub(crate) const BOOT_CPU_LOGICAL_ID: usize = 0;
pub(crate) const TRANSLATION_JOURNAL_CAPACITY: usize = 4;
pub(crate) const TRANSLATION_STATE_OWNER_OFFSET: usize = 0;
pub(crate) const TRANSLATION_STATE_COMMITTED_COUNT_OFFSET: usize = 8;
pub(crate) const TRANSLATION_STATE_JOURNAL_OFFSET: usize = 16;
pub(crate) const TRANSLATION_RECEIPT_SIZE: usize = 24;
pub(crate) const TRANSLATION_RECEIPT_OLD_OWNER_OFFSET: usize = 0;
pub(crate) const TRANSLATION_RECEIPT_NEW_OWNER_OFFSET: usize = 1;
pub(crate) const TRANSLATION_RECEIPT_SYNC_COMPLETE_OFFSET: usize = 2;
pub(crate) const TRANSLATION_RECEIPT_SATP_OFFSET: usize = 8;
pub(crate) const TRANSLATION_RECEIPT_SEQUENCE_OFFSET: usize = 16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationOwner {
    None = 0,
    PhysicalDirect = 1,
    TrampolineVm = 2,
    EarlyVm = 3,
    SwapperVm = 4,
}

impl TranslationOwner {
    const fn decode(value: u8) -> Self {
        match value {
            1 => Self::PhysicalDirect,
            2 => Self::TrampolineVm,
            3 => Self::EarlyVm,
            4 => Self::SwapperVm,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub struct TranslationTakeoverTrace {
    pub old_owner: TranslationOwner,
    pub new_owner: TranslationOwner,
    pub satp: usize,
    pub synchronization_complete: bool,
    pub commit_sequence: usize,
}

impl TranslationTakeoverTrace {
    pub const fn completed(
        old_owner: TranslationOwner,
        new_owner: TranslationOwner,
        satp: usize,
        commit_sequence: usize,
    ) -> Self {
        Self {
            old_owner,
            new_owner,
            satp,
            synchronization_complete: true,
            commit_sequence,
        }
    }

    const fn empty() -> Self {
        Self {
            old_owner: TranslationOwner::None,
            new_owner: TranslationOwner::None,
            satp: 0,
            synchronization_complete: false,
            commit_sequence: 0,
        }
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct TranslationTakeoverJournal {
    pub committed_count: usize,
    pub receipts: [TranslationTakeoverTrace; TRANSLATION_JOURNAL_CAPACITY],
}

#[repr(C)]
struct TranslationTakeoverReceipt {
    old_owner: AtomicU8,
    new_owner: AtomicU8,
    synchronization_complete: AtomicU8,
    reserved: [u8; 5],
    satp: AtomicUsize,
    commit_sequence: AtomicUsize,
}

impl TranslationTakeoverReceipt {
    const fn new() -> Self {
        Self {
            old_owner: AtomicU8::new(TranslationOwner::None as u8),
            new_owner: AtomicU8::new(TranslationOwner::None as u8),
            synchronization_complete: AtomicU8::new(0),
            reserved: [0; 5],
            satp: AtomicUsize::new(0),
            commit_sequence: AtomicUsize::new(0),
        }
    }

    fn write(
        &self,
        old_owner: TranslationOwner,
        new_owner: TranslationOwner,
        satp: usize,
        commit_sequence: usize,
    ) {
        self.old_owner.store(old_owner as u8, Ordering::Relaxed);
        self.new_owner.store(new_owner as u8, Ordering::Relaxed);
        self.synchronization_complete.store(1, Ordering::Relaxed);
        self.satp.store(satp, Ordering::Relaxed);
        self.commit_sequence
            .store(commit_sequence, Ordering::Relaxed);
    }

    fn read(&self) -> TranslationTakeoverTrace {
        TranslationTakeoverTrace {
            old_owner: TranslationOwner::decode(self.old_owner.load(Ordering::Relaxed)),
            new_owner: TranslationOwner::decode(self.new_owner.load(Ordering::Relaxed)),
            satp: self.satp.load(Ordering::Relaxed),
            synchronization_complete: self.synchronization_complete.load(Ordering::Relaxed) == 1,
            commit_sequence: self.commit_sequence.load(Ordering::Relaxed),
        }
    }
}

#[repr(C, align(64))]
pub(crate) struct TranslationState {
    active_owner: AtomicU8,
    reserved: [u8; 7],
    committed_count: AtomicUsize,
    journal: [TranslationTakeoverReceipt; TRANSLATION_JOURNAL_CAPACITY],
}

const _: () = {
    assert!(
        core::mem::offset_of!(TranslationState, active_owner) == TRANSLATION_STATE_OWNER_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationState, committed_count)
            == TRANSLATION_STATE_COMMITTED_COUNT_OFFSET
    );
    assert!(core::mem::offset_of!(TranslationState, journal) == TRANSLATION_STATE_JOURNAL_OFFSET);
    assert!(core::mem::size_of::<TranslationTakeoverReceipt>() == TRANSLATION_RECEIPT_SIZE);
    assert!(
        core::mem::offset_of!(TranslationTakeoverReceipt, old_owner)
            == TRANSLATION_RECEIPT_OLD_OWNER_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationTakeoverReceipt, new_owner)
            == TRANSLATION_RECEIPT_NEW_OWNER_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationTakeoverReceipt, synchronization_complete)
            == TRANSLATION_RECEIPT_SYNC_COMPLETE_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationTakeoverReceipt, satp) == TRANSLATION_RECEIPT_SATP_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationTakeoverReceipt, commit_sequence)
            == TRANSLATION_RECEIPT_SEQUENCE_OFFSET
    );
};

impl TranslationState {
    pub(crate) const fn new() -> Self {
        Self {
            active_owner: AtomicU8::new(TranslationOwner::None as u8),
            reserved: [0; 7],
            committed_count: AtomicUsize::new(0),
            journal: [const { TranslationTakeoverReceipt::new() }; TRANSLATION_JOURNAL_CAPACITY],
        }
    }

    pub(crate) fn owner(&self) -> TranslationOwner {
        TranslationOwner::decode(self.active_owner.load(Ordering::Acquire))
    }

    pub(crate) fn committed_count(&self) -> usize {
        self.committed_count.load(Ordering::Acquire)
    }

    pub(crate) fn commit_take_over(
        &self,
        old_owner: TranslationOwner,
        new_owner: TranslationOwner,
        target_satp: usize,
        live_satp: usize,
    ) -> bool {
        let committed_count = self.committed_count();
        if committed_count >= TRANSLATION_JOURNAL_CAPACITY
            || self.active_owner.load(Ordering::Acquire) != old_owner as u8
            || live_satp != target_satp
            || !translation_transition_allowed(old_owner, new_owner)
        {
            return false;
        }

        let commit_sequence = committed_count + 1;
        self.journal[committed_count].write(old_owner, new_owner, target_satp, commit_sequence);
        self.active_owner.store(new_owner as u8, Ordering::Release);
        self.committed_count
            .store(commit_sequence, Ordering::Release);
        true
    }

    pub(crate) fn receipt(&self, index: usize) -> Option<TranslationTakeoverTrace> {
        (index < self.committed_count() && index < TRANSLATION_JOURNAL_CAPACITY)
            .then(|| self.journal[index].read())
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    fn journal(&self) -> TranslationTakeoverJournal {
        let committed_count = self.committed_count();
        let mut receipts = [TranslationTakeoverTrace::empty(); TRANSLATION_JOURNAL_CAPACITY];
        let mut index = 0;
        while index < committed_count && index < TRANSLATION_JOURNAL_CAPACITY {
            receipts[index] = self.journal[index].read();
            index += 1;
        }
        TranslationTakeoverJournal {
            committed_count,
            receipts,
        }
    }

    pub(crate) fn matches_chain(&self, expected: &[TranslationTakeoverTrace]) -> bool {
        if expected.is_empty()
            || expected.len() > TRANSLATION_JOURNAL_CAPACITY
            || self.committed_count() != expected.len()
            || self.owner() != expected[expected.len() - 1].new_owner
        {
            return false;
        }

        let mut index = 0;
        while index < expected.len() {
            if self.journal[index].read() != expected[index] {
                return false;
            }
            index += 1;
        }
        true
    }

    fn verify_live_chain(&self, expected: &[TranslationTakeoverTrace], live_satp: usize) -> bool {
        self.matches_chain(expected)
            && expected
                .last()
                .is_some_and(|receipt| receipt.satp == live_satp)
    }

    #[cfg(app_smoke)]
    pub(crate) fn inject_unpublished_receipt_for_test(
        &self,
        old_owner: TranslationOwner,
        new_owner: TranslationOwner,
        satp: usize,
    ) -> bool {
        let index = self.committed_count();
        if index >= TRANSLATION_JOURNAL_CAPACITY {
            return false;
        }
        self.journal[index].write(old_owner, new_owner, satp, index + 1);
        true
    }
}

const fn translation_transition_allowed(
    old_owner: TranslationOwner,
    new_owner: TranslationOwner,
) -> bool {
    matches!(
        (old_owner, new_owner),
        (TranslationOwner::None, TranslationOwner::PhysicalDirect)
            | (
                TranslationOwner::PhysicalDirect,
                TranslationOwner::TrampolineVm
            )
            | (TranslationOwner::TrampolineVm, TranslationOwner::EarlyVm)
            | (TranslationOwner::TrampolineVm, TranslationOwner::SwapperVm)
            | (TranslationOwner::EarlyVm, TranslationOwner::SwapperVm)
    )
}

unsafe extern "C" {
    static head_boot_hartid: usize;
}

/// The compact key used by the CpuGroup-owned indexed collection.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct LogicId(u16);

impl LogicId {
    pub const INVALID: Self = Self(u16::MAX);

    pub const fn new(value: usize) -> Option<Self> {
        if value < MAX_CPUS {
            Some(Self(value as u16))
        } else {
            None
        }
    }

    pub const fn get(self) -> usize {
        self.0 as usize
    }

    pub const fn is_valid(self) -> bool {
        self.0 != u16::MAX && (self.0 as usize) < MAX_CPUS
    }
}

/// Stable typed reference to `CpuGroup.cpus[logic_id]`.
///
/// Construction validates only the compact key range. Dereference through
/// CpuGroup additionally verifies that the indexed element is published.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CpuRef {
    logical_id: LogicId,
}

impl CpuRef {
    pub const fn invalid() -> Self {
        Self {
            logical_id: LogicId::INVALID,
        }
    }

    pub const fn new(logical_id: usize) -> Self {
        match LogicId::new(logical_id) {
            Some(logical_id) => Self { logical_id },
            None => Self::invalid(),
        }
    }

    pub const fn logic_id(self) -> LogicId {
        self.logical_id
    }

    pub const fn logical_id(self) -> usize {
        if self.logical_id.is_valid() {
            self.logical_id.get()
        } else {
            usize::MAX
        }
    }

    pub const fn is_valid(self) -> bool {
        self.logical_id.is_valid()
    }

    pub const fn is_boot_cpu(self) -> bool {
        self.logical_id.get() == BOOT_CPU_LOGICAL_ID
    }
}

pub struct Cpu {
    lifecycle: Lifecycle,
    logical_id: LogicId,
    hartid: usize,
    role: CpuRole,
    possible: bool,
    present: bool,
    active: bool,
    online: bool,
    translation_state: TranslationState,
    trap: TrapType,
}

impl Cpu {
    pub const fn boot() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            logical_id: LogicId(BOOT_CPU_LOGICAL_ID as u16),
            hartid: usize::MAX,
            role: CpuRole::Boot,
            possible: false,
            present: false,
            active: false,
            online: false,
            translation_state: TranslationState::new(),
            trap: TrapType::new(),
        }
    }

    pub const fn secondary(logical_id: LogicId, hartid: usize) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Ready),
            logical_id,
            hartid,
            role: CpuRole::Secondary,
            possible: true,
            present: true,
            active: false,
            online: false,
            translation_state: TranslationState::new(),
            trap: TrapType::new(),
        }
    }

    pub fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        if self.role != CpuRole::Boot || self.logical_id.get() != BOOT_CPU_LOGICAL_ID {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }
        let head_hartid = unsafe { core::ptr::addr_of!(head_boot_hartid).read_volatile() };
        if head_hartid != boot_args.boot_hartid() {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.hartid = head_hartid;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup_boot(&mut self, boot_hartid_valid: bool) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !boot_hartid_valid {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.possible = true;
        self.present = true;
        self.active = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootCpuReady,
        )
    }

    pub fn enable_boot(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.role != CpuRole::Boot
            || !self.possible
            || !self.present
            || !self.active
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.online = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::BootCpuOnline,
        )
    }

    pub fn mark_online(&mut self) {
        self.active = true;
        self.online = true;
    }

    pub const fn cpu_ref(&self) -> CpuRef {
        CpuRef {
            logical_id: self.logical_id,
        }
    }

    pub const fn logical_id(&self) -> usize {
        self.logical_id.get()
    }

    pub const fn hartid(&self) -> usize {
        self.hartid
    }

    pub const fn role(&self) -> CpuRole {
        self.role
    }

    pub fn active_translation_owner(&self) -> TranslationOwner {
        self.translation_state.owner()
    }

    pub(crate) fn translation_state_storage(&self) -> *mut TranslationState {
        core::ptr::from_ref(&self.translation_state).cast_mut()
    }

    pub(crate) fn translation_state_storage_range(&self) -> Option<(usize, usize)> {
        let start = self.translation_state_storage() as usize;
        start
            .checked_add(core::mem::size_of::<TranslationState>())
            .map(|end| (start, end))
    }

    pub(crate) fn commit_translation_takeover(
        &self,
        old_owner: TranslationOwner,
        new_owner: TranslationOwner,
        target_satp: usize,
        live_satp: usize,
    ) -> bool {
        self.translation_state
            .commit_take_over(old_owner, new_owner, target_satp, live_satp)
    }

    pub(crate) fn translation_receipt_matches(
        &self,
        index: usize,
        expected: TranslationTakeoverTrace,
    ) -> bool {
        self.translation_state.receipt(index) == Some(expected)
    }

    pub(crate) fn verify_translation_chain(
        &self,
        expected: &[TranslationTakeoverTrace],
        live_satp: usize,
    ) -> bool {
        self.translation_state
            .verify_live_chain(expected, live_satp)
    }

    pub fn translation_chain_matches(&self, expected: &[TranslationTakeoverTrace]) -> bool {
        self.translation_state.matches_chain(expected)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn translation_takeover_journal(&self) -> TranslationTakeoverJournal {
        self.translation_state.journal()
    }

    #[allow(dead_code)]
    pub fn translation_takeover_trace(&self) -> TranslationTakeoverTrace {
        self.translation_state
            .receipt(self.translation_state.committed_count().saturating_sub(1))
            .unwrap_or(TranslationTakeoverTrace::empty())
    }

    pub const fn is_possible(&self) -> bool {
        self.possible
    }

    pub const fn is_present(&self) -> bool {
        self.present
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub const fn is_online(&self) -> bool {
        self.online
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn trap(&self) -> &TrapType {
        &self.trap
    }

    pub fn trap_mut(&mut self) -> &mut TrapType {
        &mut self.trap
    }

    pub const fn interrupt(&self) -> &InterruptType {
        self.trap.interrupt()
    }

    pub fn interrupt_mut(&mut self) -> &mut InterruptType {
        self.trap.interrupt_mut()
    }

    pub const fn exception(&self) -> &ExceptionType {
        self.trap.exception()
    }

    #[cfg_attr(not(any(app_smoke, app_user_boot)), allow(dead_code))]
    pub fn exception_mut(&mut self) -> &mut ExceptionType {
        self.trap.exception_mut()
    }

    pub const fn local_interrupt(&self) -> &InterruptType {
        self.interrupt()
    }

    pub fn local_interrupt_mut(&mut self) -> &mut InterruptType {
        self.interrupt_mut()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CpuRole {
    Boot,
    Secondary,
}
