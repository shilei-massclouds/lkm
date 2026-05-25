#[cfg(checkpoint_sbi_char)]
pub fn checkpoint(checkpoint: Checkpoint) {
    crate::arch::riscv64::sbi::putchar(checkpoint.byte());
}

#[cfg(not(checkpoint_sbi_char))]
pub fn checkpoint(_checkpoint: Checkpoint) {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Checkpoint {
    EntryPreludePhaseReady,
    EntrySuccessorPhaseStarted,
    EntrySuccessorPhaseReady,
    PrintkBufferPrepared,
    EarlyConPrepared,
    EarlyConReady,
    EarlyConOnline,
}

impl Checkpoint {
    #[allow(dead_code)]
    const fn byte(self) -> u8 {
        match self {
            Self::EntryPreludePhaseReady => b'A',
            Self::EntrySuccessorPhaseStarted => b'P',
            Self::EntrySuccessorPhaseReady => b'R',
            Self::PrintkBufferPrepared => b'B',
            Self::EarlyConPrepared => b'C',
            Self::EarlyConReady => b'D',
            Self::EarlyConOnline => b'E',
        }
    }
}
