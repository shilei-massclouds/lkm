use super::{
    cache_block_info::CacheBlockInfo,
    cpu_group::CpuGroup,
    device_tree::{DevicePropertyRef, DeviceTree},
    fdt_reader::read_cells,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

#[derive(Clone, Copy)]
pub struct IsaFacts {
    pub i: bool,
    pub m: bool,
    pub a: bool,
    pub f: bool,
    pub d: bool,
    pub c: bool,
    pub v: bool,
    pub zicbom: bool,
    pub zicboz: bool,
}

impl IsaFacts {
    const fn empty() -> Self {
        Self {
            i: false,
            m: false,
            a: false,
            f: false,
            d: false,
            c: false,
            v: false,
            zicbom: false,
            zicboz: false,
        }
    }

    fn and(self, other: Self) -> Self {
        Self {
            i: self.i && other.i,
            m: self.m && other.m,
            a: self.a && other.a,
            f: self.f && other.f,
            d: self.d && other.d,
            c: self.c && other.c,
            v: self.v && other.v,
            zicbom: self.zicbom && other.zicbom,
            zicboz: self.zicboz && other.zicboz,
        }
    }

    pub const fn elf_hwcap(self) -> usize {
        self.a as usize
            | (self.c as usize) << (b'C' - b'A')
            | (self.d as usize) << (b'D' - b'A')
            | (self.f as usize) << (b'F' - b'A')
            | (self.i as usize) << (b'I' - b'A')
            | (self.m as usize) << (b'M' - b'A')
            | (self.v as usize) << (b'V' - b'A')
    }
}

pub struct CpuCapabilities {
    lifecycle: Lifecycle,
    hart_count: usize,
    common_isa: IsaFacts,
    fallback_isa_used: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl CpuCapabilities {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hart_count: 0,
            common_isa: IsaFacts::empty(),
            fallback_isa_used: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn hart_count(&self) -> usize {
        self.hart_count
    }

    pub const fn common_isa(&self) -> IsaFacts {
        self.common_isa
    }

    pub const fn fallback_isa_used(&self) -> bool {
        self.fallback_isa_used
    }

    pub const fn fpu_supported(&self) -> bool {
        self.common_isa.f && self.common_isa.d
    }

    pub const fn vector_supported(&self) -> bool {
        self.common_isa.v
    }

    pub const fn elf_hwcap(&self) -> usize {
        self.common_isa.elf_hwcap()
    }

    pub fn setup(
        &mut self,
        device_tree: &DeviceTree,
        cpu_group: &CpuGroup,
        cache_block_info: &CacheBlockInfo,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || device_tree.state() != State::Ready
            || cpu_group.state() != State::Ready
            || cache_block_info.state() != State::Ready
        {
            return self.failed_setup();
        }

        let Some(facts) = collect_cpu_capabilities(device_tree, cpu_group, cache_block_info) else {
            return self.failed_setup();
        };
        self.hart_count = facts.hart_count;
        self.common_isa = facts.common_isa;
        self.fallback_isa_used = facts.fallback_isa_used;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::CpuCapabilitiesReady,
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

struct CpuCapabilityFacts {
    hart_count: usize,
    common_isa: IsaFacts,
    fallback_isa_used: bool,
}

fn collect_cpu_capabilities(
    device_tree: &DeviceTree,
    cpu_group: &CpuGroup,
    cache_block_info: &CacheBlockInfo,
) -> Option<CpuCapabilityFacts> {
    let cpus = device_tree.find_node(b"/cpus")?;
    let address_cells = cpu_address_cells(cpus.property(b"#address-cells")?.raw_value())?;
    let mut hart_count = 0usize;
    let mut common_isa = IsaFacts::empty();
    let mut fallback_isa_used = false;

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

        let Some((mut isa, used_fallback)) = read_cpu_isa(
            cpu.property(b"riscv,isa-extensions"),
            cpu.property(b"riscv,isa"),
        ) else {
            continue;
        };
        validate_cache_block_extensions(&mut isa, cache_block_info);
        if isa.f && !isa.d {
            isa.f = false;
        }

        if hart_count == 0 {
            common_isa = isa;
        } else {
            common_isa = common_isa.and(isa);
        }
        fallback_isa_used = fallback_isa_used || used_fallback;
        hart_count += 1;
    }

    if hart_count == 0 {
        None
    } else {
        Some(CpuCapabilityFacts {
            hart_count,
            common_isa,
            fallback_isa_used,
        })
    }
}

fn validate_cache_block_extensions(isa: &mut IsaFacts, cache_block_info: &CacheBlockInfo) {
    if isa.zicbom && !valid_block_size(cache_block_info.cbom_block_size()) {
        isa.zicbom = false;
    }
    if isa.zicboz && !valid_block_size(cache_block_info.cboz_block_size()) {
        isa.zicboz = false;
    }
}

fn valid_block_size(block_size: Option<u32>) -> bool {
    block_size
        .filter(|value| *value != 0 && value.is_power_of_two())
        .is_some()
}

fn read_cpu_isa(
    extensions: Option<DevicePropertyRef<'_>>,
    fallback: Option<DevicePropertyRef<'_>>,
) -> Option<(IsaFacts, bool)> {
    if let Some(extensions) = extensions {
        return Some((read_isa_extensions(extensions.raw_value()), false));
    }
    let fallback = fallback?;
    Some((read_isa_string(fallback.raw_value())?, true))
}

fn read_isa_extensions(value: &[u8]) -> IsaFacts {
    let mut facts = IsaFacts::empty();
    for item in StringListIter::new(value) {
        apply_isa_extension(&mut facts, item);
    }
    facts
}

fn read_isa_string(value: &[u8]) -> Option<IsaFacts> {
    let mut facts = IsaFacts::empty();
    let len = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    let value = &value[..len];
    if !value.starts_with(b"rv32") && !value.starts_with(b"rv64") {
        return None;
    }

    let mut index = 4usize;
    while index < value.len() {
        let byte = to_lower(value[index]);
        if byte == b'_' {
            index += 1;
            continue;
        }
        match byte {
            b'i' => facts.i = true,
            b'm' => facts.m = true,
            b'a' => facts.a = true,
            b'f' => facts.f = true,
            b'd' => facts.d = true,
            b'c' => facts.c = true,
            b'v' => facts.v = true,
            b'z' => {
                let start = index;
                index += 1;
                while index < value.len() && value[index] != b'_' {
                    index += 1;
                }
                apply_isa_extension(&mut facts, &value[start..index]);
                continue;
            }
            _ => {}
        }
        index += 1;
    }

    Some(facts)
}

fn apply_isa_extension(facts: &mut IsaFacts, extension: &[u8]) {
    if extension == b"i" {
        facts.i = true;
    } else if extension == b"m" {
        facts.m = true;
    } else if extension == b"a" {
        facts.a = true;
    } else if extension == b"f" {
        facts.f = true;
    } else if extension == b"d" {
        facts.d = true;
    } else if extension == b"c" {
        facts.c = true;
    } else if extension == b"v" {
        facts.v = true;
    } else if eq_ignore_ascii_case(extension, b"zicbom") {
        facts.zicbom = true;
    } else if eq_ignore_ascii_case(extension, b"zicboz") {
        facts.zicboz = true;
    }
}

struct StringListIter<'a> {
    value: &'a [u8],
    cursor: usize,
}

impl<'a> StringListIter<'a> {
    const fn new(value: &'a [u8]) -> Self {
        Self { value, cursor: 0 }
    }
}

impl<'a> Iterator for StringListIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.value.len() && self.value[self.cursor] == 0 {
            self.cursor += 1;
        }
        if self.cursor >= self.value.len() {
            return None;
        }

        let start = self.cursor;
        while self.cursor < self.value.len() && self.value[self.cursor] != 0 {
            self.cursor += 1;
        }
        Some(&self.value[start..self.cursor])
    }
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

fn eq_ignore_ascii_case(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0usize;
    while index < left.len() {
        if to_lower(left[index]) != to_lower(right[index]) {
            return false;
        }
        index += 1;
    }
    true
}

fn to_lower(byte: u8) -> u8 {
    if byte.is_ascii_uppercase() {
        byte + 32
    } else {
        byte
    }
}
