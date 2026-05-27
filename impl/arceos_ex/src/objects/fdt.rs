pub use super::fdt_facts::{BootCommandLine, FdtFacts, HartSet, PhysRangeSet};

use super::{fix_map::FixMap, raw_dtb::RawDtb};

pub fn parse(raw_dtb: &RawDtb, fix_map: &FixMap) -> Option<FdtFacts> {
    super::fdt_parser::parse(raw_dtb, fix_map)
}
