#[cfg(checkpoint_sbi_char)]
pub fn checkpoint(checkpoint: Checkpoint) {
    crate::arch::riscv64::sbi::putchar(checkpoint.byte());
}

#[cfg(not(checkpoint_sbi_char))]
pub fn checkpoint(_checkpoint: Checkpoint) {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Checkpoint {
    EntryPreludePhaseStarted,
    #[allow(dead_code)]
    EntryPreludePhaseReady,
    EntryPreludeFoundationReady,
    #[allow(dead_code)]
    EntrySuccessorPhaseStarted,
    #[allow(dead_code)]
    EntrySuccessorPhaseReady,
    InterruptStreamPrepared,
    KernelImagePrepared,
    RootStreamPrepared,
    KernelImageReady,
    BootCpuPrepared,
    CpuGroupPrepared,
    InitTaskPrepared,
    InitStackPrepared,
    EventStreamPrepared,
    RawDtbPrepared,
    RawDtbReady,
    FixMapReady,
    EarlyVmPrepared,
    #[allow(dead_code)]
    PrintkBufferPrepared,
    #[allow(dead_code)]
    EarlyConPrepared,
    #[allow(dead_code)]
    EarlyConReady,
    #[allow(dead_code)]
    EarlyConOnline,
}

impl Checkpoint {
    #[allow(dead_code)]
    const fn byte(self) -> u8 {
        match self {
            Self::EntryPreludePhaseStarted => b'A',
            Self::EntryPreludePhaseReady => b'a',
            Self::EntryPreludeFoundationReady => b'F',
            Self::EntrySuccessorPhaseStarted => b'P',
            Self::EntrySuccessorPhaseReady => b'R',
            Self::InterruptStreamPrepared => b'I',
            Self::KernelImagePrepared => b'K',
            Self::RootStreamPrepared => b'O',
            Self::KernelImageReady => b'Z',
            Self::BootCpuPrepared => b'H',
            Self::CpuGroupPrepared => b'G',
            Self::InitTaskPrepared => b'T',
            Self::InitStackPrepared => b'S',
            Self::EventStreamPrepared => b'V',
            Self::RawDtbPrepared => b'Y',
            Self::RawDtbReady => b'W',
            Self::FixMapReady => b'M',
            Self::EarlyVmPrepared => b'N',
            Self::PrintkBufferPrepared => b'B',
            Self::EarlyConPrepared => b'C',
            Self::EarlyConReady => b'D',
            Self::EarlyConOnline => b'E',
        }
    }
}
