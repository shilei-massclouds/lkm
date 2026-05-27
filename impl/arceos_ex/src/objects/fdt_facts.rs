use super::raw_dtb::PhysRange;

pub const MAX_HARTS: usize = 16;
pub const MAX_PHYS_RANGES: usize = 16;
pub const MAX_CMDLINE: usize = 256;

#[derive(Clone, Copy)]
pub struct HartSet {
    harts: [usize; MAX_HARTS],
    count: usize,
}

impl HartSet {
    pub const fn empty() -> Self {
        Self {
            harts: [0; MAX_HARTS],
            count: 0,
        }
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub fn contains(&self, hartid: usize) -> bool {
        let mut index = 0;
        while index < self.count {
            if self.harts[index] == hartid {
                return true;
            }
            index += 1;
        }
        false
    }

    pub(super) fn push(&mut self, hartid: usize) -> bool {
        if self.contains(hartid) {
            return true;
        }
        if self.count >= MAX_HARTS {
            return false;
        }
        self.harts[self.count] = hartid;
        self.count += 1;
        true
    }
}

#[derive(Clone, Copy)]
pub struct PhysRangeSet {
    ranges: [PhysRange; MAX_PHYS_RANGES],
    count: usize,
}

impl PhysRangeSet {
    pub const fn empty() -> Self {
        Self {
            ranges: [PhysRange::empty(); MAX_PHYS_RANGES],
            count: 0,
        }
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub fn get(&self, index: usize) -> Option<PhysRange> {
        if index < self.count {
            Some(self.ranges[index])
        } else {
            None
        }
    }

    pub fn contains_range(&self, range: PhysRange) -> bool {
        let mut index = 0;
        while index < self.count {
            let item = self.ranges[index];
            if item.start() <= range.start() && item.end() >= range.end() {
                return true;
            }
            index += 1;
        }
        false
    }

    #[allow(dead_code)]
    pub fn overlaps_range(&self, range: PhysRange) -> bool {
        let mut index = 0;
        while index < self.count {
            let item = self.ranges[index];
            if item.start() < range.end() && range.start() < item.end() {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn push(&mut self, range: PhysRange) -> bool {
        if range.start() >= range.end() {
            return false;
        }
        if self.count >= MAX_PHYS_RANGES {
            return false;
        }
        self.ranges[self.count] = range;
        self.count += 1;
        true
    }
}

#[derive(Clone, Copy)]
pub struct BootCommandLine {
    bytes: [u8; MAX_CMDLINE],
    len: usize,
}

impl BootCommandLine {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; MAX_CMDLINE],
            len: 0,
        }
    }

    pub fn contains(&self, needle: &[u8]) -> bool {
        if needle.is_empty() || needle.len() > self.len {
            return false;
        }

        let mut index = 0;
        while index + needle.len() <= self.len {
            let mut matched = true;
            let mut sub = 0;
            while sub < needle.len() {
                if self.bytes[index + sub] != needle[sub] {
                    matched = false;
                    break;
                }
                sub += 1;
            }
            if matched {
                return true;
            }
            index += 1;
        }
        false
    }

    pub(super) fn copy_from(
        &mut self,
        addr: usize,
        len: usize,
        limit: usize,
        read_u8: fn(usize, usize) -> Option<u8>,
    ) -> Option<()> {
        let mut copied = 0;
        while copied < len && copied + 1 < MAX_CMDLINE {
            let byte = read_u8(addr.checked_add(copied)?, limit)?;
            if byte == 0 {
                break;
            }
            self.bytes[copied] = byte;
            copied += 1;
        }
        self.len = copied;
        Some(())
    }
}

#[derive(Clone, Copy)]
pub struct FdtFacts {
    pub harts: HartSet,
    pub memory: PhysRangeSet,
    pub reserved: PhysRangeSet,
    pub cmdline: BootCommandLine,
}

impl FdtFacts {
    pub(super) const fn empty() -> Self {
        Self {
            harts: HartSet::empty(),
            memory: PhysRangeSet::empty(),
            reserved: PhysRangeSet::empty(),
            cmdline: BootCommandLine::empty(),
        }
    }
}
