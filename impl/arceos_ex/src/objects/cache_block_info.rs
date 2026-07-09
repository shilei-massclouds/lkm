use super::{
    cpu_group::CpuGroup,
    device_tree::DeviceTree,
    fdt_reader::{read_be_u32, read_cells},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::checkpoint::Checkpoint;

pub struct CacheBlockInfo {
    lifecycle: Lifecycle,
    cbom_block_size: Option<u32>,
    cboz_block_size: Option<u32>,
    mismatch_count: usize,
}

impl CacheBlockInfo {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cbom_block_size: None,
            cboz_block_size: None,
            mismatch_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cbom_block_size(&self) -> Option<u32> {
        self.cbom_block_size
    }

    pub const fn cboz_block_size(&self) -> Option<u32> {
        self.cboz_block_size
    }

    pub const fn mismatch_count(&self) -> usize {
        self.mismatch_count
    }

    pub fn setup(&mut self, device_tree: &DeviceTree, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || cpu_group.state() != State::Ready
        {
            return self.failed_setup();
        }

        let Some(facts) = collect_cache_block_info(device_tree, cpu_group) else {
            return self.failed_setup();
        };
        self.cbom_block_size = facts.cbom.block_size;
        self.cboz_block_size = facts.cboz.block_size;
        self.mismatch_count = facts.cbom.mismatch_count + facts.cboz.mismatch_count;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::CacheBlockInfoReady,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

struct CacheBlockFacts {
    cbom: CacheBlockFact,
    cboz: CacheBlockFact,
}

struct CacheBlockFact {
    block_size: Option<u32>,
    first_hartid: usize,
    mismatch_count: usize,
}

impl CacheBlockFact {
    const fn empty() -> Self {
        Self {
            block_size: None,
            first_hartid: usize::MAX,
            mismatch_count: 0,
        }
    }

    fn update(&mut self, hartid: usize, value: u32) {
        if self.block_size.is_none() {
            self.block_size = Some(value);
            self.first_hartid = hartid;
        } else if self.block_size != Some(value) {
            self.mismatch_count += 1;
        }
    }
}

fn collect_cache_block_info(
    device_tree: &DeviceTree,
    cpu_group: &CpuGroup,
) -> Option<CacheBlockFacts> {
    let cpus = device_tree.find_node(b"/cpus")?;
    let address_cells = cpu_address_cells(cpus.property(b"#address-cells")?.raw_value())?;
    let mut facts = CacheBlockFacts {
        cbom: CacheBlockFact::empty(),
        cboz: CacheBlockFact::empty(),
    };

    for cpu in cpus.children() {
        let Some(reg) = cpu.property(b"reg") else {
            continue;
        };
        let Some(hartid) = cpu_hartid(reg.raw_value(), address_cells) else {
            continue;
        };
        if !cpu_group.has_hartid(hartid) {
            continue;
        }
        if let Some(value) = read_property_u32(cpu.property(b"riscv,cbom-block-size")) {
            facts.cbom.update(hartid, value);
        }
        if let Some(value) = read_property_u32(cpu.property(b"riscv,cboz-block-size")) {
            facts.cboz.update(hartid, value);
        }
    }

    Some(facts)
}

fn cpu_hartid(value: &[u8], address_cells: usize) -> Option<usize> {
    let (hartid, _) = read_cells(value.as_ptr() as usize, value.len(), address_cells)?;
    usize::try_from(hartid).ok()
}

fn cpu_address_cells(value: &[u8]) -> Option<usize> {
    let (cells, _) = read_cells(value.as_ptr() as usize, value.len(), 1)?;
    let cells = usize::try_from(cells).ok()?;
    if cells == 0 || cells > 2 {
        None
    } else {
        Some(cells)
    }
}

fn read_property_u32(property: Option<super::device_tree::DevicePropertyRef<'_>>) -> Option<u32> {
    let value = property?.raw_value();
    read_be_u32(
        value.as_ptr() as usize,
        (value.as_ptr() as usize).checked_add(value.len())?,
    )
}
