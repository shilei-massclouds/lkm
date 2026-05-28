use super::{
    config::Config,
    cpu_group::CpuGroup,
    cpu_id_map::CpuIdMap,
    lds::Lds,
    memblock::MemBlock,
    raw_dtb::PhysRange,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    vm::Vm,
};
use crate::trace::Checkpoint;

const MAX_PER_CPU_UNITS: usize = 16;
const DYNAMIC_RESERVE_SIZE: usize = 4096;
const RESERVED_SIZE: usize = 0;

#[used]
#[unsafe(link_section = ".data..percpu")]
static PER_CPU_STATIC_ANCHOR: usize = 0;

#[macro_export]
macro_rules! define_per_cpu {
    ($vis:vis static $name:ident: $ty:ty = $value:expr) => {
        #[used]
        #[unsafe(link_section = ".data..percpu")]
        $vis static $name: $ty = $value;
    };
}

#[derive(Clone, Copy)]
pub struct PerCpuSymbol<T> {
    template: *const T,
}

impl<T> PerCpuSymbol<T> {
    pub const fn new(template: &'static T) -> Self {
        Self { template }
    }

    fn template_addr(&self) -> usize {
        self.template as usize
    }
}

pub struct PerCpuStorage {
    lifecycle: Lifecycle,
    static_image: PerCpuStaticImage,
    first_chunk: PerCpuFirstChunk,
    offset_table: PerCpuOffsetTable,
}

impl PerCpuStorage {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            static_image: PerCpuStaticImage::new(),
            first_chunk: PerCpuFirstChunk::new(),
            offset_table: PerCpuOffsetTable::new(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn static_image(&self) -> &PerCpuStaticImage {
        &self.static_image
    }

    #[allow(dead_code)]
    pub const fn first_chunk(&self) -> &PerCpuFirstChunk {
        &self.first_chunk
    }

    #[allow(dead_code)]
    pub const fn offset_table(&self) -> &PerCpuOffsetTable {
        &self.offset_table
    }

    pub fn per_cpu_ptr<T>(&self, symbol: PerCpuSymbol<T>, logical_id: usize) -> Option<*mut T> {
        let template_addr = symbol.template_addr();
        if self.lifecycle.state() != State::Ready
            || !self
                .static_image
                .contains(template_addr, core::mem::size_of::<T>())
        {
            return None;
        }

        let offset = self.offset_table.offset(logical_id)?;
        let addr = template_addr.wrapping_add(offset);
        Some(addr as *mut T)
    }

    pub fn read_per_cpu<T: Copy>(&self, symbol: PerCpuSymbol<T>, logical_id: usize) -> Option<T> {
        let ptr = self.per_cpu_ptr(symbol, logical_id)?;
        Some(unsafe { core::ptr::read_volatile(ptr) })
    }

    pub fn write_per_cpu<T: Copy>(
        &self,
        symbol: PerCpuSymbol<T>,
        logical_id: usize,
        value: T,
    ) -> Option<()> {
        let ptr = self.per_cpu_ptr(symbol, logical_id)?;
        unsafe {
            core::ptr::write_volatile(ptr, value);
        }
        Some(())
    }

    pub fn setup(
        &mut self,
        lds: &Lds,
        static_objects: &StaticObjects,
        memblock: &mut MemBlock,
        vm: &Vm,
        config: &Config,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || lds.state() != State::Online
            || static_objects.state() != State::Online
            || memblock.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || config.state() != State::Online
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.static_image.setup(lds, static_objects)?;
        self.first_chunk.setup(
            &self.static_image,
            memblock,
            vm,
            config,
            cpu_group,
            cpu_id_map,
        )?;
        self.offset_table
            .setup(&self.static_image, &self.first_chunk, cpu_group, cpu_id_map)?;

        if !self.ready_facts_hold(cpu_group) {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PerCpuStorageReady,
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

    fn ready_facts_hold(&self, cpu_group: &CpuGroup) -> bool {
        self.static_image.state() == State::Ready
            && self.first_chunk.state() == State::Ready
            && self.offset_table.state() == State::Ready
            && self.first_chunk.unit_count() == cpu_group.possible_cpu_count()
            && self.offset_table.count() == self.first_chunk.unit_count()
            && self.first_chunk.static_size() == self.static_image.size()
            && self.first_chunk.dynamic_size() != 0
    }
}

pub struct PerCpuStaticImage {
    lifecycle: Lifecycle,
    start: usize,
    end: usize,
    load: usize,
    size: usize,
}

impl PerCpuStaticImage {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            start: 0,
            end: 0,
            load: 0,
            size: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn start(&self) -> usize {
        self.start
    }

    #[allow(dead_code)]
    pub const fn end(&self) -> usize {
        self.end
    }

    pub const fn load(&self) -> usize {
        self.load
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    fn contains(&self, addr: usize, size: usize) -> bool {
        if size == 0 {
            return addr >= self.start && addr <= self.end;
        }
        let Some(end) = addr.checked_add(size) else {
            return false;
        };
        addr >= self.start && end <= self.end
    }

    fn setup(&mut self, lds: &Lds, static_objects: &StaticObjects) -> EventResult {
        if self.lifecycle.state() != State::Base
            || lds.state() != State::Online
            || static_objects.state() != State::Online
            || !lds.per_cpu_static_image_layout_ready()
        {
            return self.failed_setup();
        }

        let start = lds.per_cpu_start();
        let end = lds.per_cpu_end();
        let size = end - start;
        let anchor = core::ptr::addr_of!(PER_CPU_STATIC_ANCHOR) as usize;
        if anchor < start || anchor >= end || size == 0 {
            return self.failed_setup();
        }

        self.start = start;
        self.end = end;
        self.load = lds.per_cpu_load();
        self.size = size;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PerCpuStaticImageReady,
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

pub struct PerCpuFirstChunk {
    lifecycle: Lifecycle,
    range: PhysRange,
    base_virt: usize,
    unit_size: usize,
    static_size: usize,
    reserved_size: usize,
    dynamic_size: usize,
    unit_count: usize,
    units: [PerCpuUnit; MAX_PER_CPU_UNITS],
}

impl PerCpuFirstChunk {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            range: PhysRange::empty(),
            base_virt: 0,
            unit_size: 0,
            static_size: 0,
            reserved_size: 0,
            dynamic_size: 0,
            unit_count: 0,
            units: [PerCpuUnit::empty(); MAX_PER_CPU_UNITS],
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn range(&self) -> PhysRange {
        self.range
    }

    #[allow(dead_code)]
    pub const fn base_virt(&self) -> usize {
        self.base_virt
    }

    #[allow(dead_code)]
    pub const fn unit_size(&self) -> usize {
        self.unit_size
    }

    pub const fn static_size(&self) -> usize {
        self.static_size
    }

    #[allow(dead_code)]
    pub const fn reserved_size(&self) -> usize {
        self.reserved_size
    }

    pub const fn dynamic_size(&self) -> usize {
        self.dynamic_size
    }

    pub const fn unit_count(&self) -> usize {
        self.unit_count
    }

    pub fn unit(&self, logical_id: usize) -> Option<PerCpuUnit> {
        if logical_id < self.unit_count {
            Some(self.units[logical_id])
        } else {
            None
        }
    }

    fn setup(
        &mut self,
        static_image: &PerCpuStaticImage,
        memblock: &mut MemBlock,
        vm: &Vm,
        config: &Config,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || static_image.state() != State::Ready
            || memblock.state() != State::Online
            || vm.state() != State::Online
            || !vm.entry_successor_ready()
            || config.state() != State::Online
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
            || cpu_id_map.count() == 0
            || cpu_id_map.count() != cpu_group.possible_cpu_count()
            || cpu_id_map.count() > MAX_PER_CPU_UNITS
        {
            return self.failed_setup();
        }

        let Some(dynamic_offset) = static_image.size().checked_add(RESERVED_SIZE) else {
            return self.failed_setup();
        };
        let Some(unit_payload_size) = dynamic_offset.checked_add(DYNAMIC_RESERVE_SIZE) else {
            return self.failed_setup();
        };
        let Some(unit_size) = round_up(unit_payload_size, config.page_size()) else {
            return self.failed_setup();
        };
        let Some(total_size) = unit_size.checked_mul(cpu_id_map.count()) else {
            return self.failed_setup();
        };
        let Some(range) = memblock.alloc_phys(total_size, config.page_size()) else {
            return self.failed_setup();
        };
        let Some(base_virt) = config.phys_to_linear(range.start()) else {
            return self.failed_setup();
        };
        if base_virt.checked_add(total_size).is_none() {
            return self.failed_setup();
        }

        let mut units = [PerCpuUnit::empty(); MAX_PER_CPU_UNITS];
        let mut logical_id = 0usize;
        while logical_id < cpu_id_map.count() {
            let Some(entry) = cpu_id_map.entry(logical_id) else {
                return self.failed_setup();
            };
            let Some(unit_offset) = unit_size.checked_mul(logical_id) else {
                return self.failed_setup();
            };
            let Some(unit_base) = base_virt.checked_add(unit_offset) else {
                return self.failed_setup();
            };
            let Some(static_end) = unit_base.checked_add(static_image.size()) else {
                return self.failed_setup();
            };
            let Some(dynamic_start) = unit_base.checked_add(dynamic_offset) else {
                return self.failed_setup();
            };
            let Some(dynamic_end) = dynamic_start.checked_add(DYNAMIC_RESERVE_SIZE) else {
                return self.failed_setup();
            };
            let Some(unit_end) = unit_base.checked_add(unit_size) else {
                return self.failed_setup();
            };
            if static_end > dynamic_start || dynamic_start > dynamic_end || dynamic_end > unit_end {
                return self.failed_setup();
            }
            if !copy_static_image(static_image, unit_base) {
                return self.failed_setup();
            }

            units[logical_id] = PerCpuUnit {
                logical_id,
                hartid: entry.hartid(),
                base: unit_base,
                static_start: unit_base,
                dynamic_start,
                end: unit_end,
            };
            logical_id += 1;
        }

        self.range = range;
        self.base_virt = base_virt;
        self.unit_size = unit_size;
        self.static_size = static_image.size();
        self.reserved_size = RESERVED_SIZE;
        self.dynamic_size = DYNAMIC_RESERVE_SIZE;
        self.unit_count = cpu_id_map.count();
        self.units = units;

        if !self.ready_facts_hold(static_image, cpu_id_map) {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PerCpuFirstChunkReady,
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

    fn ready_facts_hold(&self, static_image: &PerCpuStaticImage, cpu_id_map: &CpuIdMap) -> bool {
        if self.range.start() >= self.range.end()
            || self.range.size() != self.unit_size * self.unit_count
            || self.unit_count != cpu_id_map.count()
            || self.static_size != static_image.size()
            || self.dynamic_size == 0
        {
            return false;
        }

        let mut logical_id = 0usize;
        while logical_id < self.unit_count {
            let Some(entry) = cpu_id_map.entry(logical_id) else {
                return false;
            };
            let unit = self.units[logical_id];
            let Some(unit_offset) = self.unit_size.checked_mul(logical_id) else {
                return false;
            };
            let Some(expected_base) = self.base_virt.checked_add(unit_offset) else {
                return false;
            };
            let Some(static_end) = unit.static_start.checked_add(self.static_size) else {
                return false;
            };
            let Some(reserved_end) = static_end.checked_add(self.reserved_size) else {
                return false;
            };
            if unit.logical_id != logical_id
                || unit.hartid != entry.hartid()
                || unit.base != expected_base
                || unit.static_start != unit.base
                || unit.dynamic_start < reserved_end
                || unit.end != unit.base + self.unit_size
            {
                return false;
            }
            logical_id += 1;
        }

        true
    }
}

#[derive(Clone, Copy)]
pub struct PerCpuUnit {
    logical_id: usize,
    hartid: usize,
    base: usize,
    static_start: usize,
    dynamic_start: usize,
    end: usize,
}

impl PerCpuUnit {
    const fn empty() -> Self {
        Self {
            logical_id: usize::MAX,
            hartid: usize::MAX,
            base: 0,
            static_start: 0,
            dynamic_start: 0,
            end: 0,
        }
    }

    #[allow(dead_code)]
    pub const fn logical_id(self) -> usize {
        self.logical_id
    }

    #[allow(dead_code)]
    pub const fn hartid(self) -> usize {
        self.hartid
    }

    pub const fn base(self) -> usize {
        self.base
    }

    #[allow(dead_code)]
    pub const fn static_start(self) -> usize {
        self.static_start
    }

    #[allow(dead_code)]
    pub const fn dynamic_start(self) -> usize {
        self.dynamic_start
    }

    #[allow(dead_code)]
    pub const fn end(self) -> usize {
        self.end
    }
}

pub struct PerCpuOffsetTable {
    lifecycle: Lifecycle,
    offsets: [usize; MAX_PER_CPU_UNITS],
    count: usize,
}

impl PerCpuOffsetTable {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            offsets: [0; MAX_PER_CPU_UNITS],
            count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    #[allow(dead_code)]
    pub fn offset(&self, logical_id: usize) -> Option<usize> {
        if logical_id < self.count {
            Some(self.offsets[logical_id])
        } else {
            None
        }
    }

    fn setup(
        &mut self,
        static_image: &PerCpuStaticImage,
        first_chunk: &PerCpuFirstChunk,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || static_image.state() != State::Ready
            || first_chunk.state() != State::Ready
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
            || first_chunk.unit_count() != cpu_group.possible_cpu_count()
            || first_chunk.unit_count() != cpu_id_map.count()
        {
            return self.failed_setup();
        }

        let mut offsets = [0usize; MAX_PER_CPU_UNITS];
        let mut logical_id = 0usize;
        while logical_id < cpu_id_map.count() {
            let Some(unit) = first_chunk.unit(logical_id) else {
                return self.failed_setup();
            };
            offsets[logical_id] = unit.base().wrapping_sub(static_image.start());
            logical_id += 1;
        }

        self.offsets = offsets;
        self.count = cpu_id_map.count();

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::PerCpuOffsetTableReady,
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

fn copy_static_image(static_image: &PerCpuStaticImage, destination: usize) -> bool {
    if static_image.size() == 0
        || destination.checked_add(static_image.size()).is_none()
        || static_image
            .load()
            .checked_add(static_image.size())
            .is_none()
    {
        return false;
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            static_image.load() as *const u8,
            destination as *mut u8,
            static_image.size(),
        );
    }
    true
}

fn round_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    Some(value.checked_add(align - 1)? & !(align - 1))
}
