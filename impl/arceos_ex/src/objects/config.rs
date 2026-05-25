use super::{
    fix_map::FixMapSlot,
    state::{Lifecycle, State},
};

const PAGE_SIZE: usize = 4096;
const FDT_SLOT_SIZE: usize = 2 * 1024 * 1024;

pub struct Config {
    lifecycle: Lifecycle,
    page_size: usize,
    fixmap: FixMapConfig,
}

impl Config {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Online),
            page_size: PAGE_SIZE,
            fixmap: FixMapConfig::new(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn page_size(&self) -> usize {
        self.page_size
    }

    pub const fn fixmap(&self) -> &FixMapConfig {
        &self.fixmap
    }
}

pub struct FixMapConfig {
    fdt: FixMapSlot,
}

impl FixMapConfig {
    const fn new() -> Self {
        Self {
            fdt: FixMapSlot::new(PAGE_SIZE, FDT_SLOT_SIZE / PAGE_SIZE),
        }
    }

    pub const fn fdt(&self) -> FixMapSlot {
        self.fdt
    }
}
