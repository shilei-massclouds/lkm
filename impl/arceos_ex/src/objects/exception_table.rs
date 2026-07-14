use super::{
    kernel_image::KernelImage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    vm::Vm,
};
use crate::checkpoint::Checkpoint;

unsafe extern "C" {
    fn __start___ex_table();
    fn __stop___ex_table();
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExceptionTableEntry {
    insn: i32,
    fixup: i32,
    kind: i16,
    data: i16,
}

pub struct ExceptionTable {
    lifecycle: Lifecycle,
    start: usize,
    count: usize,
    sorted: bool,
    lookup_ready: bool,
}

impl ExceptionTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start: 0,
            count: 0,
            sorted: false,
            lookup_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn count(&self) -> usize {
        self.count
    }

    #[allow(dead_code)]
    pub const fn is_sorted(&self) -> bool {
        self.sorted
    }

    #[allow(dead_code)]
    pub const fn lookup_ready(&self) -> bool {
        self.lookup_ready
    }

    #[allow(dead_code)]
    pub fn lookup(&self, instruction: usize) -> Option<&'static ExceptionTableEntry> {
        if self.lifecycle.state() != State::Ready || !self.lookup_ready {
            return None;
        }

        let entries = self.entries();
        let mut left = 0usize;
        let mut right = entries.len();

        while left < right {
            let mid = left + (right - left) / 2;
            let entry_instruction = entries[mid].instruction_addr();
            if instruction < entry_instruction {
                right = mid;
            } else if instruction > entry_instruction {
                left = mid + 1;
            } else {
                return Some(&entries[mid]);
            }
        }

        None
    }

    pub fn setup(&mut self, kernel_image: &KernelImage, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_image.state() != State::Online
            || vm.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        let start = __start___ex_table as usize;
        let stop = __stop___ex_table as usize;
        let Some(byte_len) = stop.checked_sub(start) else {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        };
        if !start.is_multiple_of(core::mem::align_of::<ExceptionTableEntry>())
            || !byte_len.is_multiple_of(core::mem::size_of::<ExceptionTableEntry>())
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.start = start;
        self.count = byte_len / core::mem::size_of::<ExceptionTableEntry>();
        self.sort_entries();
        self.sorted = self.entries_sorted();
        self.lookup_ready = self.sorted;
        if !self.lookup_ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::ExceptionTableReady,
        )
    }

    fn entries(&self) -> &'static [ExceptionTableEntry] {
        if self.count == 0 {
            &[]
        } else {
            unsafe {
                core::slice::from_raw_parts(self.start as *const ExceptionTableEntry, self.count)
            }
        }
    }

    fn entries_mut(&mut self) -> &'static mut [ExceptionTableEntry] {
        if self.count == 0 {
            &mut []
        } else {
            unsafe {
                core::slice::from_raw_parts_mut(self.start as *mut ExceptionTableEntry, self.count)
            }
        }
    }

    fn sort_entries(&mut self) {
        let entries = self.entries_mut();
        let mut index = 1usize;
        while index < entries.len() {
            let mut current = index;
            while current > 0
                && entries[current - 1].instruction_addr() > entries[current].instruction_addr()
            {
                swap_relative_entries(entries, current - 1, current);
                current -= 1;
            }
            index += 1;
        }
    }

    fn entries_sorted(&self) -> bool {
        let entries = self.entries();
        let mut index = 1usize;
        while index < entries.len() {
            if entries[index - 1].instruction_addr() > entries[index].instruction_addr() {
                return false;
            }
            index += 1;
        }
        true
    }
}

impl ExceptionTableEntry {
    fn instruction_addr(&self) -> usize {
        relative_addr(core::ptr::addr_of!(self.insn) as usize, self.insn)
    }

    #[allow(dead_code)]
    pub fn fixup_addr(&self) -> usize {
        relative_addr(core::ptr::addr_of!(self.fixup) as usize, self.fixup)
    }

    #[allow(dead_code)]
    pub const fn kind(&self) -> i16 {
        self.kind
    }

    #[allow(dead_code)]
    pub const fn data(&self) -> i16 {
        self.data
    }
}

fn relative_addr(field_addr: usize, offset: i32) -> usize {
    if offset >= 0 {
        field_addr.wrapping_add(offset as usize)
    } else {
        field_addr.wrapping_sub(offset.unsigned_abs() as usize)
    }
}

fn swap_relative_entries(entries: &mut [ExceptionTableEntry], left: usize, right: usize) {
    let left_addr = core::ptr::addr_of!(entries[left]) as isize;
    let right_addr = core::ptr::addr_of!(entries[right]) as isize;
    let delta = (right_addr - left_addr) as i32;
    let tmp = entries[left];

    entries[left].insn = entries[right].insn.wrapping_add(delta);
    entries[left].fixup = entries[right].fixup.wrapping_add(delta);
    entries[left].kind = entries[right].kind;
    entries[left].data = entries[right].data;

    entries[right].insn = tmp.insn.wrapping_sub(delta);
    entries[right].fixup = tmp.fixup.wrapping_sub(delta);
    entries[right].kind = tmp.kind;
    entries[right].data = tmp.data;
}
