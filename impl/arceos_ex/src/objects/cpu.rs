use super::{
    boot_args::BootArgs,
    exception_type::ExceptionType,
    interrupt_type::InterruptType,
    scheduler::Scheduler,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    trap_type::TrapType,
};
use crate::checkpoint::Checkpoint;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

pub(crate) const MAX_CPUS: usize = 16;
pub(crate) const BOOT_CPU_LOGICAL_ID: usize = 0;
pub(crate) const TRANSLATION_JOURNAL_CAPACITY: usize = 4;
pub(crate) const TRANSLATION_STATE_CONTROLLER_OFFSET: usize = 0;
pub(crate) const TRANSLATION_STATE_COMMITTED_COUNT_OFFSET: usize = 8;
pub(crate) const TRANSLATION_STATE_JOURNAL_OFFSET: usize = 16;
pub(crate) const TRANSLATION_RECEIPT_SIZE: usize = 24;
pub(crate) const TRANSLATION_RECEIPT_OLD_CONTROLLER_OFFSET: usize = 0;
pub(crate) const TRANSLATION_RECEIPT_NEW_CONTROLLER_OFFSET: usize = 1;
pub(crate) const TRANSLATION_RECEIPT_SYNC_COMPLETE_OFFSET: usize = 2;
pub(crate) const TRANSLATION_RECEIPT_KIND_OFFSET: usize = 3;
pub(crate) const TRANSLATION_RECEIPT_SATP_OFFSET: usize = 8;
pub(crate) const TRANSLATION_RECEIPT_SEQUENCE_OFFSET: usize = 16;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationController {
    PhysicalDirect = 1,
    TrampolineVm = 2,
    EarlyVm = 3,
    SwapperVm = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTranslationControllerEncoding {
    pub raw: u8,
}

impl TranslationController {
    const fn decode(value: u8) -> Result<Self, InvalidTranslationControllerEncoding> {
        match value {
            1 => Ok(Self::PhysicalDirect),
            2 => Ok(Self::TrampolineVm),
            3 => Ok(Self::EarlyVm),
            4 => Ok(Self::SwapperVm),
            raw => Err(InvalidTranslationControllerEncoding { raw }),
        }
    }

    const fn decode_optional(
        value: u8,
    ) -> Result<Option<Self>, InvalidTranslationControllerEncoding> {
        if value == 0 {
            Ok(None)
        } else {
            match Self::decode(value) {
                Ok(controller) => Ok(Some(controller)),
                Err(error) => Err(error),
            }
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationActivationKind {
    InitialActivation = 1,
    Handoff = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTranslationActivationKindEncoding {
    pub raw: u8,
}

impl TranslationActivationKind {
    const fn decode(value: u8) -> Result<Self, InvalidTranslationActivationKindEncoding> {
        match value {
            1 => Ok(Self::InitialActivation),
            2 => Ok(Self::Handoff),
            raw => Err(InvalidTranslationActivationKindEncoding { raw }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub struct TranslationActivationTrace {
    pub kind: TranslationActivationKind,
    pub old_controller: Option<TranslationController>,
    pub new_controller: TranslationController,
    pub satp: usize,
    pub synchronization_complete: bool,
    pub commit_sequence: usize,
}

impl TranslationActivationTrace {
    pub const fn completed(
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        satp: usize,
        commit_sequence: usize,
    ) -> Self {
        Self {
            kind,
            old_controller,
            new_controller,
            satp,
            synchronization_complete: true,
            commit_sequence,
        }
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct TranslationActivationJournal {
    pub committed_count: usize,
    pub receipts: [Option<TranslationActivationTrace>; TRANSLATION_JOURNAL_CAPACITY],
}

#[repr(C)]
struct TranslationActivationReceipt {
    old_controller: AtomicU8,
    new_controller: AtomicU8,
    synchronization_complete: AtomicU8,
    kind: AtomicU8,
    reserved: [u8; 4],
    satp: AtomicUsize,
    commit_sequence: AtomicUsize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidTranslationActivationReceiptEncoding {
    Controller(InvalidTranslationControllerEncoding),
    Kind(InvalidTranslationActivationKindEncoding),
    Synchronization(u8),
    Shape,
}

impl TranslationActivationReceipt {
    const fn new() -> Self {
        Self {
            old_controller: AtomicU8::new(0),
            new_controller: AtomicU8::new(0),
            synchronization_complete: AtomicU8::new(0),
            kind: AtomicU8::new(0),
            reserved: [0; 4],
            satp: AtomicUsize::new(0),
            commit_sequence: AtomicUsize::new(0),
        }
    }

    fn write(
        &self,
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        satp: usize,
        commit_sequence: usize,
    ) {
        self.old_controller.store(
            old_controller.map_or(0, |value| value as u8),
            Ordering::Relaxed,
        );
        self.new_controller
            .store(new_controller as u8, Ordering::Relaxed);
        self.synchronization_complete.store(1, Ordering::Relaxed);
        self.kind.store(kind as u8, Ordering::Relaxed);
        self.satp.store(satp, Ordering::Relaxed);
        self.commit_sequence
            .store(commit_sequence, Ordering::Relaxed);
    }

    fn read(
        &self,
    ) -> Result<TranslationActivationTrace, InvalidTranslationActivationReceiptEncoding> {
        let kind = TranslationActivationKind::decode(self.kind.load(Ordering::Relaxed))
            .map_err(InvalidTranslationActivationReceiptEncoding::Kind)?;
        let old_controller =
            TranslationController::decode_optional(self.old_controller.load(Ordering::Relaxed))
                .map_err(InvalidTranslationActivationReceiptEncoding::Controller)?;
        let new_controller =
            TranslationController::decode(self.new_controller.load(Ordering::Relaxed))
                .map_err(InvalidTranslationActivationReceiptEncoding::Controller)?;
        let synchronization = self.synchronization_complete.load(Ordering::Relaxed);
        if synchronization != 1 {
            return Err(
                InvalidTranslationActivationReceiptEncoding::Synchronization(synchronization),
            );
        }
        if (kind == TranslationActivationKind::InitialActivation && old_controller.is_some())
            || (kind == TranslationActivationKind::Handoff && old_controller.is_none())
        {
            return Err(InvalidTranslationActivationReceiptEncoding::Shape);
        }
        Ok(TranslationActivationTrace {
            kind,
            old_controller,
            new_controller,
            satp: self.satp.load(Ordering::Relaxed),
            synchronization_complete: true,
            commit_sequence: self.commit_sequence.load(Ordering::Relaxed),
        })
    }
}

#[repr(C, align(64))]
pub(crate) struct TranslationState {
    active_controller: AtomicU8,
    reserved: [u8; 7],
    committed_count: AtomicUsize,
    journal: [TranslationActivationReceipt; TRANSLATION_JOURNAL_CAPACITY],
}

const _: () = {
    assert!(
        core::mem::offset_of!(TranslationState, active_controller)
            == TRANSLATION_STATE_CONTROLLER_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationState, committed_count)
            == TRANSLATION_STATE_COMMITTED_COUNT_OFFSET
    );
    assert!(core::mem::offset_of!(TranslationState, journal) == TRANSLATION_STATE_JOURNAL_OFFSET);
    assert!(core::mem::size_of::<TranslationActivationReceipt>() == TRANSLATION_RECEIPT_SIZE);
    assert!(core::mem::align_of::<TranslationActivationReceipt>() == 8);
    assert!(
        core::mem::offset_of!(TranslationActivationReceipt, old_controller)
            == TRANSLATION_RECEIPT_OLD_CONTROLLER_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationActivationReceipt, new_controller)
            == TRANSLATION_RECEIPT_NEW_CONTROLLER_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationActivationReceipt, synchronization_complete)
            == TRANSLATION_RECEIPT_SYNC_COMPLETE_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationActivationReceipt, kind)
            == TRANSLATION_RECEIPT_KIND_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationActivationReceipt, satp)
            == TRANSLATION_RECEIPT_SATP_OFFSET
    );
    assert!(
        core::mem::offset_of!(TranslationActivationReceipt, commit_sequence)
            == TRANSLATION_RECEIPT_SEQUENCE_OFFSET
    );
};

impl TranslationState {
    pub(crate) const fn new() -> Self {
        Self {
            active_controller: AtomicU8::new(0),
            reserved: [0; 7],
            committed_count: AtomicUsize::new(0),
            journal: [const { TranslationActivationReceipt::new() }; TRANSLATION_JOURNAL_CAPACITY],
        }
    }

    pub(crate) fn active_controller(
        &self,
    ) -> Result<Option<TranslationController>, InvalidTranslationControllerEncoding> {
        TranslationController::decode_optional(self.active_controller.load(Ordering::Acquire))
    }

    pub(crate) fn committed_count(&self) -> usize {
        self.committed_count.load(Ordering::Acquire)
    }

    pub(crate) fn activation_preflight(
        &self,
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        expected_old_satp: usize,
        live_satp: usize,
    ) -> bool {
        let committed_count = self.committed_count();
        committed_count < TRANSLATION_JOURNAL_CAPACITY
            && self.active_controller().ok() == Some(old_controller)
            && live_satp == expected_old_satp
            && translation_activation_allowed(kind, old_controller, new_controller, committed_count)
    }

    pub(crate) fn commit_activation(
        &self,
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        target_satp: usize,
        live_satp: usize,
    ) -> bool {
        let committed_count = self.committed_count();
        if committed_count >= TRANSLATION_JOURNAL_CAPACITY
            || self.active_controller().ok() != Some(old_controller)
            || live_satp != target_satp
            || !translation_activation_allowed(
                kind,
                old_controller,
                new_controller,
                committed_count,
            )
        {
            return false;
        }

        let commit_sequence = committed_count + 1;
        self.journal[committed_count].write(
            kind,
            old_controller,
            new_controller,
            target_satp,
            commit_sequence,
        );
        self.active_controller
            .store(new_controller as u8, Ordering::Release);
        self.committed_count
            .store(commit_sequence, Ordering::Release);
        true
    }

    pub(crate) fn receipt(
        &self,
        index: usize,
    ) -> Result<Option<TranslationActivationTrace>, InvalidTranslationActivationReceiptEncoding>
    {
        if index < self.committed_count() && index < TRANSLATION_JOURNAL_CAPACITY {
            self.journal[index].read().map(Some)
        } else {
            Ok(None)
        }
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    fn journal(
        &self,
    ) -> Result<TranslationActivationJournal, InvalidTranslationActivationReceiptEncoding> {
        let committed_count = self.committed_count();
        let mut receipts = [None; TRANSLATION_JOURNAL_CAPACITY];
        let mut index = 0;
        while index < committed_count && index < TRANSLATION_JOURNAL_CAPACITY {
            receipts[index] = Some(self.journal[index].read()?);
            index += 1;
        }
        Ok(TranslationActivationJournal {
            committed_count,
            receipts,
        })
    }

    pub(crate) fn matches_chain(&self, expected: &[TranslationActivationTrace]) -> bool {
        if expected.is_empty()
            || expected.len() > TRANSLATION_JOURNAL_CAPACITY
            || self.committed_count() != expected.len()
            || self.active_controller().ok()
                != Some(Some(expected[expected.len() - 1].new_controller))
        {
            return false;
        }

        let mut index = 0;
        while index < expected.len() {
            if self.journal[index].read().ok() != Some(expected[index]) {
                return false;
            }
            index += 1;
        }
        true
    }

    fn verify_live_chain(&self, expected: &[TranslationActivationTrace], live_satp: usize) -> bool {
        self.matches_chain(expected)
            && expected
                .last()
                .is_some_and(|receipt| receipt.satp == live_satp)
    }

    #[cfg(app_smoke)]
    pub(crate) fn inject_unpublished_receipt_for_test(
        &self,
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        satp: usize,
    ) -> bool {
        let index = self.committed_count();
        if index >= TRANSLATION_JOURNAL_CAPACITY {
            return false;
        }
        self.journal[index].write(kind, old_controller, new_controller, satp, index + 1);
        true
    }

    #[cfg(app_smoke)]
    pub(crate) fn inject_active_controller_raw_for_test(&self, raw: u8) {
        self.active_controller.store(raw, Ordering::Release);
    }

    #[cfg(app_smoke)]
    pub(crate) fn inject_committed_receipt_raw_for_test(
        &self,
        old_controller: u8,
        new_controller: u8,
        kind: u8,
    ) {
        self.journal[0]
            .old_controller
            .store(old_controller, Ordering::Relaxed);
        self.journal[0]
            .new_controller
            .store(new_controller, Ordering::Relaxed);
        self.journal[0]
            .synchronization_complete
            .store(1, Ordering::Relaxed);
        self.journal[0].kind.store(kind, Ordering::Relaxed);
        self.journal[0].satp.store(0, Ordering::Relaxed);
        self.journal[0].commit_sequence.store(1, Ordering::Relaxed);
        self.active_controller.store(
            TranslationController::PhysicalDirect as u8,
            Ordering::Release,
        );
        self.committed_count.store(1, Ordering::Release);
    }
}

const fn translation_activation_allowed(
    kind: TranslationActivationKind,
    old_controller: Option<TranslationController>,
    new_controller: TranslationController,
    committed_count: usize,
) -> bool {
    matches!(
        (kind, old_controller, new_controller, committed_count),
        (
            TranslationActivationKind::InitialActivation,
            None,
            TranslationController::PhysicalDirect,
            0
        ) | (
            TranslationActivationKind::Handoff,
            Some(TranslationController::PhysicalDirect),
            TranslationController::TrampolineVm,
            1
        ) | (
            TranslationActivationKind::Handoff,
            Some(TranslationController::TrampolineVm),
            TranslationController::EarlyVm,
            2
        ) | (
            TranslationActivationKind::Handoff,
            Some(TranslationController::TrampolineVm),
            TranslationController::SwapperVm,
            2
        ) | (
            TranslationActivationKind::Handoff,
            Some(TranslationController::EarlyVm),
            TranslationController::SwapperVm,
            3
        )
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
    scheduler: Scheduler,
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
            scheduler: Scheduler::new(),
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
            scheduler: Scheduler::new(),
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

    pub fn active_translation_controller(
        &self,
    ) -> Result<Option<TranslationController>, InvalidTranslationControllerEncoding> {
        self.translation_state.active_controller()
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

    pub(crate) fn translation_activation_preflight(
        &self,
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        expected_old_satp: usize,
        live_satp: usize,
    ) -> bool {
        self.translation_state.activation_preflight(
            kind,
            old_controller,
            new_controller,
            expected_old_satp,
            live_satp,
        )
    }

    pub(crate) fn commit_translation_activation(
        &self,
        kind: TranslationActivationKind,
        old_controller: Option<TranslationController>,
        new_controller: TranslationController,
        target_satp: usize,
        live_satp: usize,
    ) -> bool {
        self.translation_state.commit_activation(
            kind,
            old_controller,
            new_controller,
            target_satp,
            live_satp,
        )
    }

    pub(crate) fn translation_receipt_matches(
        &self,
        index: usize,
        expected: TranslationActivationTrace,
    ) -> bool {
        self.translation_state.receipt(index).ok() == Some(Some(expected))
    }

    pub(crate) fn verify_translation_chain(
        &self,
        expected: &[TranslationActivationTrace],
        live_satp: usize,
    ) -> bool {
        self.translation_state
            .verify_live_chain(expected, live_satp)
    }

    pub fn translation_chain_matches(&self, expected: &[TranslationActivationTrace]) -> bool {
        self.translation_state.matches_chain(expected)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn translation_activation_journal(
        &self,
    ) -> Result<TranslationActivationJournal, InvalidTranslationActivationReceiptEncoding> {
        self.translation_state.journal()
    }

    #[allow(dead_code)]
    pub fn translation_activation_trace(
        &self,
    ) -> Result<Option<TranslationActivationTrace>, InvalidTranslationActivationReceiptEncoding>
    {
        self.translation_state
            .receipt(self.translation_state.committed_count().saturating_sub(1))
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

    pub const fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    pub fn scheduler_and_local_interrupt_mut(&mut self) -> (&mut Scheduler, &mut InterruptType) {
        (&mut self.scheduler, self.trap.interrupt_mut())
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
