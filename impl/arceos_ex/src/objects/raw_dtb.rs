use crate::trace::Checkpoint;

use super::{
    boot_args::BootArgs,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_TOTAL_SIZE_OFFSET: usize = 4;
const FDT_HEADER_SIZE: usize = 40;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PhysRange {
    start: usize,
    end: usize,
}

impl PhysRange {
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[allow(dead_code)]
    pub const fn start(self) -> usize {
        self.start
    }

    #[allow(dead_code)]
    pub const fn end(self) -> usize {
        self.end
    }

    pub const fn size(self) -> usize {
        self.end - self.start
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DtbHeader {
    magic: u32,
    total_size: usize,
}

impl DtbHeader {
    const fn empty() -> Self {
        Self {
            magic: 0,
            total_size: 0,
        }
    }

    const fn with_magic(magic: u32) -> Self {
        Self {
            magic,
            total_size: 0,
        }
    }

    const fn with_total_size(self, total_size: usize) -> Self {
        Self {
            magic: self.magic,
            total_size,
        }
    }
}

pub struct RawDtb {
    lifecycle: Lifecycle,
    header: DtbHeader,
    header_range: PhysRange,
    range: PhysRange,
}

impl RawDtb {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            header: DtbHeader::empty(),
            header_range: PhysRange::empty(),
            range: PhysRange::empty(),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn header_range(&self) -> PhysRange {
        self.header_range
    }

    #[allow(dead_code)]
    pub const fn range(&self) -> PhysRange {
        self.range
    }

    pub fn preset(&mut self, boot_args: &BootArgs) -> EventResult {
        let dtb_pa = boot_args.dtb_pa();
        let Some(header_end) = dtb_pa.checked_add(FDT_HEADER_SIZE) else {
            return self.failed_condition(LifecycleEvent::Preset, State::Base, State::Prepared);
        };
        if boot_args.state() != State::Online || dtb_pa == 0 {
            return self.failed_condition(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        let Some(magic) = read_be_u32(dtb_pa) else {
            return self.failed_condition(LifecycleEvent::Preset, State::Base, State::Prepared);
        };
        if magic != FDT_MAGIC {
            return self.failed_condition(LifecycleEvent::Preset, State::Base, State::Prepared);
        }

        self.header = DtbHeader::with_magic(magic);
        self.header_range = PhysRange::new(dtb_pa, header_end);
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::RawDtbPrepared,
        )
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared {
            return self.lifecycle.transition(
                LifecycleEvent::Setup,
                State::Prepared,
                State::Ready,
                Checkpoint::RawDtbReady,
            );
        }

        let Some(total_size) = read_be_u32(self.header_range.start + FDT_TOTAL_SIZE_OFFSET) else {
            return self.failed_condition(LifecycleEvent::Setup, State::Prepared, State::Ready);
        };
        let total_size = total_size as usize;
        let Some(range_end) = self.header_range.start.checked_add(total_size) else {
            return self.failed_condition(LifecycleEvent::Setup, State::Prepared, State::Ready);
        };
        if total_size < FDT_HEADER_SIZE || range_end <= self.header_range.start {
            return self.failed_condition(LifecycleEvent::Setup, State::Prepared, State::Ready);
        }

        self.header = self.header.with_total_size(total_size);
        self.range = PhysRange::new(self.header_range.start, range_end);
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::RawDtbReady,
        )
    }

    fn failed_condition(
        &self,
        event: LifecycleEvent,
        expected: State,
        target: State,
    ) -> EventResult {
        failed_condition(event, self.lifecycle.state(), expected, target)
    }
}

fn read_be_u32(pa: usize) -> Option<u32> {
    pa.checked_add(core::mem::size_of::<u32>())?;
    let ptr = pa as *const u8;
    // OpenSBI hands a physical DTB address to S-mode while MMU is still off.
    // RawDtb events are the first place where this source assumption is checked.
    let bytes = unsafe {
        [
            core::ptr::read_volatile(ptr),
            core::ptr::read_volatile(ptr.wrapping_add(1)),
            core::ptr::read_volatile(ptr.wrapping_add(2)),
            core::ptr::read_volatile(ptr.wrapping_add(3)),
        ]
    };
    Some(u32::from_be_bytes(bytes))
}
