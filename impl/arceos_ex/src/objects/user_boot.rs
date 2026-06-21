use super::{
    mm_core::{KernelGlobalAllocator, PageAllocator},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    swapper_vm::SwapperVm,
};

pub const USER_INIT_PATH: &[u8] = b"/init";
pub const USER_INIT_EXPECTED_MESSAGE: &[u8] = b"user hello\n";

pub const ELF_HEADER_LEN: usize = 64;
pub const USER_STACK_SIZE: usize = 16 * 1024;
pub const USER_STACK_TOP: usize = 0x4000_0000;
pub const USER_PAGE_SIZE: usize = 4096;
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_MACHINE_RISCV: u16 = 243;
const ELF_PHDR_TYPE_LOAD: u32 = 1;
const ELF_PF_X: u32 = 1;
const ELF_PF_W: u32 = 2;
const ELF_PF_R: u32 = 4;
const ELF64_PHDR_SIZE: usize = 56;
const MAX_LOAD_SEGMENTS: usize = 8;
const MAX_USER_MAPPINGS: usize = MAX_LOAD_SEGMENTS + 1;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ElfError {
    InvalidState,
    ShortInput,
    BadMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedVersion,
    UnsupportedType,
    UnsupportedMachine,
    InvalidHeader,
    InvalidProgramHeader,
    TooManyLoadSegments,
    MissingLoadSegment,
    EntryOutsideExecutableSegment,
    MissingExpectedContent,
    TooManyMappings,
    MissingExecutableEntryMapping,
    InvalidStack,
}

#[derive(Clone, Copy)]
pub struct ElfLoadSegment {
    offset: usize,
    vaddr: usize,
    filesz: usize,
    memsz: usize,
    flags: u32,
    align: usize,
}

#[allow(dead_code)]
impl ElfLoadSegment {
    const fn empty() -> Self {
        Self {
            offset: 0,
            vaddr: 0,
            filesz: 0,
            memsz: 0,
            flags: 0,
            align: 0,
        }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn vaddr(&self) -> usize {
        self.vaddr
    }

    pub const fn filesz(&self) -> usize {
        self.filesz
    }

    pub const fn memsz(&self) -> usize {
        self.memsz
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn readable(&self) -> bool {
        self.flags & ELF_PF_R != 0
    }

    pub const fn writable(&self) -> bool {
        self.flags & ELF_PF_W != 0
    }

    pub const fn executable(&self) -> bool {
        self.flags & ELF_PF_X != 0
    }

    pub const fn align(&self) -> usize {
        self.align
    }

    pub const fn file_end(&self) -> usize {
        self.offset + self.filesz
    }

    pub const fn vaddr_end(&self) -> usize {
        self.vaddr + self.memsz
    }

    const fn contains_vaddr(&self, addr: usize) -> bool {
        self.vaddr <= addr && addr < self.vaddr_end()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserMappingKind {
    Empty,
    ElfSegment,
    Stack,
}

#[derive(Clone, Copy)]
pub struct UserMapping {
    kind: UserMappingKind,
    vaddr: usize,
    memsz: usize,
    file_offset: usize,
    filesz: usize,
    readable: bool,
    writable: bool,
    executable: bool,
    user_accessible: bool,
    bss_zero_bytes: usize,
}

impl UserMapping {
    const fn empty() -> Self {
        Self {
            kind: UserMappingKind::Empty,
            vaddr: 0,
            memsz: 0,
            file_offset: 0,
            filesz: 0,
            readable: false,
            writable: false,
            executable: false,
            user_accessible: false,
            bss_zero_bytes: 0,
        }
    }

    const fn from_segment(segment: ElfLoadSegment) -> Self {
        Self {
            kind: UserMappingKind::ElfSegment,
            vaddr: segment.vaddr,
            memsz: segment.memsz,
            file_offset: segment.offset,
            filesz: segment.filesz,
            readable: segment.readable(),
            writable: segment.writable(),
            executable: segment.executable(),
            user_accessible: true,
            bss_zero_bytes: segment.memsz - segment.filesz,
        }
    }

    const fn from_stack(stack: &UserStack) -> Self {
        Self {
            kind: UserMappingKind::Stack,
            vaddr: stack.base,
            memsz: stack.size,
            file_offset: 0,
            filesz: 0,
            readable: true,
            writable: true,
            executable: false,
            user_accessible: true,
            bss_zero_bytes: stack.size,
        }
    }

    pub const fn kind(&self) -> UserMappingKind {
        self.kind
    }

    pub const fn vaddr(&self) -> usize {
        self.vaddr
    }

    pub const fn memsz(&self) -> usize {
        self.memsz
    }

    pub const fn file_offset(&self) -> usize {
        self.file_offset
    }

    pub const fn filesz(&self) -> usize {
        self.filesz
    }

    pub const fn readable(&self) -> bool {
        self.readable
    }

    pub const fn writable(&self) -> bool {
        self.writable
    }

    pub const fn executable(&self) -> bool {
        self.executable
    }

    pub const fn user_accessible(&self) -> bool {
        self.user_accessible
    }

    pub const fn bss_zero_bytes(&self) -> usize {
        self.bss_zero_bytes
    }

    pub const fn end_vaddr(&self) -> usize {
        self.vaddr + self.memsz
    }

    const fn contains_vaddr(&self, addr: usize) -> bool {
        self.vaddr <= addr && addr < self.end_vaddr()
    }
}

pub struct UserStack {
    lifecycle: Lifecycle,
    base: usize,
    top: usize,
    size: usize,
    allocated: bool,
    fixed_size_bound: bool,
    mapped_into_address_space: bool,
    initial_sp_bound: bool,
    minimal_arg_env_bound: bool,
}

#[allow(dead_code)]
impl UserStack {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            base: 0,
            top: 0,
            size: 0,
            allocated: false,
            fixed_size_bound: false,
            mapped_into_address_space: false,
            initial_sp_bound: false,
            minimal_arg_env_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn base(&self) -> usize {
        self.base
    }

    pub const fn top(&self) -> usize {
        self.top
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn initial_sp(&self) -> usize {
        self.top
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn fixed_size_bound(&self) -> bool {
        self.fixed_size_bound
    }

    pub const fn mapped_into_address_space(&self) -> bool {
        self.mapped_into_address_space
    }

    pub const fn initial_sp_bound(&self) -> bool {
        self.initial_sp_bound
    }

    pub const fn minimal_arg_env_bound(&self) -> bool {
        self.minimal_arg_env_bound
    }

    pub fn setup(
        &mut self,
        address_space: &UserAddressSpace,
        page_allocator: &PageAllocator,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || address_space.state() != State::Prepared
            || page_allocator.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.size = USER_STACK_SIZE;
        self.top = USER_STACK_TOP;
        self.base = USER_STACK_TOP - USER_STACK_SIZE;
        self.allocated = true;
        self.fixed_size_bound = true;
        self.mapped_into_address_space = true;
        self.initial_sp_bound = true;
        self.minimal_arg_env_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct UserAddressSpace {
    lifecycle: Lifecycle,
    allocated: bool,
    low_half_private: bool,
    high_half_shares_swapper: bool,
    kernel_pages_u_disabled: bool,
    user_pages_u_enabled: bool,
    elf_load_plan_consumed: bool,
    segment_mappings_bound: bool,
    entry_mapping_executable: bool,
    bss_zero_plan_consumed: bool,
    elf_segments_mapped: bool,
    stack_mapped: bool,
    elf_mapped: bool,
    elf_bss_zeroed: bool,
    runtime_ready: bool,
    mappings: [UserMapping; MAX_USER_MAPPINGS],
    mapping_count: usize,
    segment_mapping_count: usize,
    stack_mapping_index: usize,
}

#[allow(dead_code)]
impl UserAddressSpace {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            low_half_private: false,
            high_half_shares_swapper: false,
            kernel_pages_u_disabled: false,
            user_pages_u_enabled: false,
            elf_load_plan_consumed: false,
            segment_mappings_bound: false,
            entry_mapping_executable: false,
            bss_zero_plan_consumed: false,
            elf_segments_mapped: false,
            stack_mapped: false,
            elf_mapped: false,
            elf_bss_zeroed: false,
            runtime_ready: false,
            mappings: [UserMapping::empty(); MAX_USER_MAPPINGS],
            mapping_count: 0,
            segment_mapping_count: 0,
            stack_mapping_index: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn low_half_private(&self) -> bool {
        self.low_half_private
    }

    pub const fn high_half_shares_swapper(&self) -> bool {
        self.high_half_shares_swapper
    }

    pub const fn kernel_pages_u_disabled(&self) -> bool {
        self.kernel_pages_u_disabled
    }

    pub const fn user_pages_u_enabled(&self) -> bool {
        self.user_pages_u_enabled
    }

    pub const fn elf_load_plan_consumed(&self) -> bool {
        self.elf_load_plan_consumed
    }

    pub const fn segment_mappings_bound(&self) -> bool {
        self.segment_mappings_bound
    }

    pub const fn entry_mapping_executable(&self) -> bool {
        self.entry_mapping_executable
    }

    pub const fn bss_zero_plan_consumed(&self) -> bool {
        self.bss_zero_plan_consumed
    }

    pub const fn elf_segments_mapped(&self) -> bool {
        self.elf_segments_mapped
    }

    pub const fn stack_mapped(&self) -> bool {
        self.stack_mapped
    }

    pub const fn elf_mapped(&self) -> bool {
        self.elf_mapped
    }

    pub const fn elf_bss_zeroed(&self) -> bool {
        self.elf_bss_zeroed
    }

    pub const fn runtime_ready(&self) -> bool {
        self.runtime_ready
    }

    pub const fn mapping_count(&self) -> usize {
        self.mapping_count
    }

    pub const fn segment_mapping_count(&self) -> usize {
        self.segment_mapping_count
    }

    pub const fn mapping(&self, index: usize) -> Option<UserMapping> {
        if index < self.mapping_count {
            Some(self.mappings[index])
        } else {
            None
        }
    }

    pub const fn stack_mapping(&self) -> Option<UserMapping> {
        if self.stack_mapped && self.stack_mapping_index < self.mapping_count {
            Some(self.mappings[self.stack_mapping_index])
        } else {
            None
        }
    }

    pub fn preset(
        &mut self,
        swapper_vm: &SwapperVm,
        page_allocator: &PageAllocator,
        global_allocator: &KernelGlobalAllocator,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || swapper_vm.state() != State::Online
            || page_allocator.state() != State::Ready
            || global_allocator.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.allocated = true;
        self.low_half_private = true;
        self.high_half_shares_swapper = true;
        self.kernel_pages_u_disabled = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self, elf: &ElfObject, stack: &UserStack) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Prepared
            || elf.state() != State::Ready
            || stack.state() != State::Ready
        {
            return Err(ElfError::InvalidState);
        }
        if !stack.mapped_into_address_space() || stack.size() == 0 {
            return Err(ElfError::InvalidStack);
        }

        self.mappings = [UserMapping::empty(); MAX_USER_MAPPINGS];
        self.mapping_count = 0;
        self.segment_mapping_count = 0;

        let mut index = 0usize;
        while index < elf.load_segment_count() {
            if self.mapping_count >= MAX_USER_MAPPINGS {
                return Err(ElfError::TooManyMappings);
            }
            let Some(segment) = elf.load_segment(index) else {
                return Err(ElfError::InvalidProgramHeader);
            };
            let mapping = UserMapping::from_segment(segment);
            if mapping.memsz() == 0
                || !mapping.user_accessible()
                || mapping
                    .file_offset()
                    .checked_add(mapping.filesz())
                    .is_none()
                || mapping.vaddr().checked_add(mapping.memsz()).is_none()
            {
                return Err(ElfError::InvalidProgramHeader);
            }
            self.mappings[self.mapping_count] = mapping;
            self.mapping_count += 1;
            self.segment_mapping_count += 1;
            index += 1;
        }

        if !entry_mapping_is_executable(&self.mappings, self.mapping_count, elf.entry()) {
            return Err(ElfError::MissingExecutableEntryMapping);
        }

        if self.mapping_count >= MAX_USER_MAPPINGS {
            return Err(ElfError::TooManyMappings);
        }
        self.stack_mapping_index = self.mapping_count;
        self.mappings[self.mapping_count] = UserMapping::from_stack(stack);
        self.mapping_count += 1;

        self.user_pages_u_enabled = true;
        self.elf_load_plan_consumed = true;
        self.segment_mappings_bound = self.segment_mapping_count == elf.load_segment_count();
        self.entry_mapping_executable = true;
        self.bss_zero_plan_consumed = elf.bss_zero_plan_bound();
        self.elf_segments_mapped = true;
        self.stack_mapped = true;
        self.elf_mapped = true;
        self.elf_bss_zeroed = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| ElfError::InvalidState)
    }
}

pub struct ElfObject {
    lifecycle: Lifecycle,
    input_len: usize,
    entry: usize,
    program_header_count: usize,
    load_segment_count: usize,
    load_segments: [ElfLoadSegment; MAX_LOAD_SEGMENTS],
    input_bound: bool,
    input_from_vfs: bool,
    magic_valid: bool,
    class_elf64: bool,
    little_endian: bool,
    machine_riscv: bool,
    type_supported: bool,
    static_executable: bool,
    no_separate_loader: bool,
    program_headers_parsed: bool,
    pt_load_segments_bound: bool,
    segment_permissions_bound: bool,
    load_plan_bound: bool,
    entry_in_executable_segment: bool,
    init_content_observed: bool,
    bss_zero_plan_bound: bool,
    entry_bound: bool,
    load_merged_into_setup: bool,
}

#[allow(dead_code)]
impl ElfObject {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            input_len: 0,
            entry: 0,
            program_header_count: 0,
            load_segment_count: 0,
            load_segments: [ElfLoadSegment::empty(); MAX_LOAD_SEGMENTS],
            input_bound: false,
            input_from_vfs: false,
            magic_valid: false,
            class_elf64: false,
            little_endian: false,
            machine_riscv: false,
            type_supported: false,
            static_executable: false,
            no_separate_loader: false,
            program_headers_parsed: false,
            pt_load_segments_bound: false,
            segment_permissions_bound: false,
            load_plan_bound: false,
            entry_in_executable_segment: false,
            init_content_observed: false,
            bss_zero_plan_bound: false,
            entry_bound: false,
            load_merged_into_setup: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn input_len(&self) -> usize {
        self.input_len
    }

    pub const fn entry(&self) -> usize {
        self.entry
    }

    pub const fn program_header_count(&self) -> usize {
        self.program_header_count
    }

    pub const fn load_segment_count(&self) -> usize {
        self.load_segment_count
    }

    pub const fn load_segment(&self, index: usize) -> Option<ElfLoadSegment> {
        if index < self.load_segment_count {
            Some(self.load_segments[index])
        } else {
            None
        }
    }

    pub const fn input_bound(&self) -> bool {
        self.input_bound
    }

    pub const fn input_from_vfs(&self) -> bool {
        self.input_from_vfs
    }

    pub const fn magic_valid(&self) -> bool {
        self.magic_valid
    }

    pub const fn class_elf64(&self) -> bool {
        self.class_elf64
    }

    pub const fn little_endian(&self) -> bool {
        self.little_endian
    }

    pub const fn machine_riscv(&self) -> bool {
        self.machine_riscv
    }

    pub const fn type_supported(&self) -> bool {
        self.type_supported
    }

    pub const fn static_executable(&self) -> bool {
        self.static_executable
    }

    pub const fn no_separate_loader(&self) -> bool {
        self.no_separate_loader
    }

    pub const fn program_headers_parsed(&self) -> bool {
        self.program_headers_parsed
    }

    pub const fn pt_load_segments_bound(&self) -> bool {
        self.pt_load_segments_bound
    }

    pub const fn segment_permissions_bound(&self) -> bool {
        self.segment_permissions_bound
    }

    pub const fn load_plan_bound(&self) -> bool {
        self.load_plan_bound
    }

    pub const fn entry_in_executable_segment(&self) -> bool {
        self.entry_in_executable_segment
    }

    pub const fn init_content_observed(&self) -> bool {
        self.init_content_observed
    }

    pub const fn bss_zero_plan_bound(&self) -> bool {
        self.bss_zero_plan_bound
    }

    pub const fn entry_bound(&self) -> bool {
        self.entry_bound
    }

    pub const fn load_merged_into_setup(&self) -> bool {
        self.load_merged_into_setup
    }

    pub fn preset_from_vfs(&mut self, input: &[u8]) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Base {
            return Err(ElfError::InvalidState);
        }
        validate_header(input)?;

        self.input_len = input.len();
        self.input_bound = true;
        self.input_from_vfs = true;
        self.magic_valid = true;
        self.class_elf64 = true;
        self.little_endian = true;
        self.machine_riscv = true;
        self.type_supported = true;
        self.static_executable = true;
        self.no_separate_loader = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
            .map_err(|_| ElfError::InvalidState)
    }

    pub fn setup(&mut self, input: &[u8]) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Prepared {
            return Err(ElfError::InvalidState);
        }

        let header = parse_header(input)?;
        let parsed = parse_load_segments(input, &header)?;
        if parsed.load_segment_count == 0 {
            return Err(ElfError::MissingLoadSegment);
        }
        if !entry_in_executable_segment(
            header.entry,
            &parsed.load_segments,
            parsed.load_segment_count,
        ) {
            return Err(ElfError::EntryOutsideExecutableSegment);
        }
        if !loadable_content_contains(input, &parsed, USER_INIT_EXPECTED_MESSAGE) {
            return Err(ElfError::MissingExpectedContent);
        }

        self.entry = header.entry;
        self.program_header_count = header.phnum;
        self.load_segment_count = parsed.load_segment_count;
        self.load_segments = parsed.load_segments;
        self.program_headers_parsed = true;
        self.pt_load_segments_bound = true;
        self.segment_permissions_bound = parsed.segment_permissions_bound;
        self.load_plan_bound = true;
        self.entry_in_executable_segment = true;
        self.init_content_observed = true;
        self.bss_zero_plan_bound = true;
        self.entry_bound = true;
        self.load_merged_into_setup = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| ElfError::InvalidState)
    }

    pub fn load_segments_fit_direct_read(&self, max_size: usize) -> bool {
        self.input_len <= max_size
    }
}

pub struct UserBootPayload {
    lifecycle: Lifecycle,
    selected: bool,
    candidates_bound: bool,
    default_init_path_bound: bool,
    uses_current_fs_struct: bool,
    no_partition_dependency: bool,
    partition_objects_deferred: bool,
    try_candidate_bound: bool,
    selected_path_bound: bool,
    reads_init_from_vfs: bool,
}

#[allow(dead_code)]
impl UserBootPayload {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            selected: false,
            candidates_bound: false,
            default_init_path_bound: false,
            uses_current_fs_struct: false,
            no_partition_dependency: false,
            partition_objects_deferred: false,
            try_candidate_bound: false,
            selected_path_bound: false,
            reads_init_from_vfs: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn selected(&self) -> bool {
        self.selected
    }

    pub const fn candidates_bound(&self) -> bool {
        self.candidates_bound
    }

    pub const fn default_init_path_bound(&self) -> bool {
        self.default_init_path_bound
    }

    pub const fn uses_current_fs_struct(&self) -> bool {
        self.uses_current_fs_struct
    }

    pub const fn no_partition_dependency(&self) -> bool {
        self.no_partition_dependency
    }

    pub const fn partition_objects_deferred(&self) -> bool {
        self.partition_objects_deferred
    }

    pub const fn try_candidate_bound(&self) -> bool {
        self.try_candidate_bound
    }

    pub const fn selected_path_bound(&self) -> bool {
        self.selected_path_bound
    }

    pub const fn reads_init_from_vfs(&self) -> bool {
        self.reads_init_from_vfs
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.selected = true;
        self.candidates_bound = true;
        self.default_init_path_bound = true;
        self.uses_current_fs_struct = true;
        self.no_partition_dependency = true;
        self.partition_objects_deferred = true;
        self.try_candidate_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn try_candidate(&mut self, elf: &ElfObject) -> EventResult {
        if self.lifecycle.state() != State::Ready || elf.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.selected_path_bound = true;
        self.reads_init_from_vfs = true;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct ElfHeader {
    entry: usize,
    phoff: usize,
    phentsize: usize,
    phnum: usize,
}

struct ParsedLoadSegments {
    load_segments: [ElfLoadSegment; MAX_LOAD_SEGMENTS],
    load_segment_count: usize,
    segment_permissions_bound: bool,
}

fn validate_header(input: &[u8]) -> Result<(), ElfError> {
    let _ = parse_header(input)?;
    Ok(())
}

fn parse_header(input: &[u8]) -> Result<ElfHeader, ElfError> {
    if input.len() < ELF_HEADER_LEN {
        return Err(ElfError::ShortInput);
    }
    if &input[0..4] != ELF_MAGIC {
        return Err(ElfError::BadMagic);
    }
    if input[4] != ELF_CLASS_64 {
        return Err(ElfError::UnsupportedClass);
    }
    if input[5] != ELF_DATA_LSB {
        return Err(ElfError::UnsupportedEndian);
    }
    if input[6] != ELF_VERSION_CURRENT {
        return Err(ElfError::UnsupportedVersion);
    }
    if read_u16(input, 16)? != ELF_TYPE_EXEC {
        return Err(ElfError::UnsupportedType);
    }
    if read_u16(input, 18)? != ELF_MACHINE_RISCV {
        return Err(ElfError::UnsupportedMachine);
    }
    if read_u32(input, 20)? != ELF_VERSION_CURRENT as u32 {
        return Err(ElfError::UnsupportedVersion);
    }

    let entry = checked_usize(read_u64(input, 24)?)?;
    let phoff = checked_usize(read_u64(input, 32)?)?;
    let ehsize = read_u16(input, 52)? as usize;
    let phentsize = read_u16(input, 54)? as usize;
    let phnum = read_u16(input, 56)? as usize;
    if ehsize != ELF_HEADER_LEN || phentsize != ELF64_PHDR_SIZE || phnum == 0 {
        return Err(ElfError::InvalidHeader);
    }
    if phoff
        .checked_add(
            phentsize
                .checked_mul(phnum)
                .ok_or(ElfError::InvalidHeader)?,
        )
        .filter(|end| *end <= input.len())
        .is_none()
    {
        return Err(ElfError::InvalidHeader);
    }

    Ok(ElfHeader {
        entry,
        phoff,
        phentsize,
        phnum,
    })
}

fn parse_load_segments(input: &[u8], header: &ElfHeader) -> Result<ParsedLoadSegments, ElfError> {
    let mut segments = [ElfLoadSegment::empty(); MAX_LOAD_SEGMENTS];
    let mut count = 0usize;
    let mut permissions_bound = true;
    let mut index = 0usize;
    while index < header.phnum {
        let phdr = header.phoff + index * header.phentsize;
        let p_type = read_u32(input, phdr)?;
        if p_type == ELF_PHDR_TYPE_LOAD {
            if count >= MAX_LOAD_SEGMENTS {
                return Err(ElfError::TooManyLoadSegments);
            }
            let flags = read_u32(input, phdr + 4)?;
            let offset = checked_usize(read_u64(input, phdr + 8)?)?;
            let vaddr = checked_usize(read_u64(input, phdr + 16)?)?;
            let filesz = checked_usize(read_u64(input, phdr + 32)?)?;
            let memsz = checked_usize(read_u64(input, phdr + 40)?)?;
            let align = checked_usize(read_u64(input, phdr + 48)?)?;
            if filesz > memsz
                || offset
                    .checked_add(filesz)
                    .filter(|end| *end <= input.len())
                    .is_none()
                || vaddr.checked_add(memsz).is_none()
                || align == 0
            {
                return Err(ElfError::InvalidProgramHeader);
            }
            if flags & (ELF_PF_R | ELF_PF_W | ELF_PF_X) == 0 {
                permissions_bound = false;
            }
            segments[count] = ElfLoadSegment {
                offset,
                vaddr,
                filesz,
                memsz,
                flags,
                align,
            };
            count += 1;
        }
        index += 1;
    }

    Ok(ParsedLoadSegments {
        load_segments: segments,
        load_segment_count: count,
        segment_permissions_bound: permissions_bound,
    })
}

fn entry_in_executable_segment(
    entry: usize,
    segments: &[ElfLoadSegment; MAX_LOAD_SEGMENTS],
    count: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        let segment = segments[index];
        if segment.executable() && segment.contains_vaddr(entry) {
            return true;
        }
        index += 1;
    }
    false
}

fn entry_mapping_is_executable(
    mappings: &[UserMapping; MAX_USER_MAPPINGS],
    count: usize,
    entry: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        let mapping = mappings[index];
        if mapping.kind() == UserMappingKind::ElfSegment
            && mapping.executable()
            && mapping.contains_vaddr(entry)
        {
            return true;
        }
        index += 1;
    }
    false
}

fn loadable_content_contains(input: &[u8], parsed: &ParsedLoadSegments, needle: &[u8]) -> bool {
    let mut index = 0usize;
    while index < parsed.load_segment_count {
        let segment = parsed.load_segments[index];
        let haystack = &input[segment.offset..segment.file_end()];
        if contains_bytes(haystack, needle) {
            return true;
        }
        index += 1;
    }
    false
}

pub fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }

    let mut index = 0usize;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            return true;
        }
        index += 1;
    }
    false
}

fn checked_usize(value: u64) -> Result<usize, ElfError> {
    if value > usize::MAX as u64 {
        return Err(ElfError::InvalidHeader);
    }
    Ok(value as usize)
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16, ElfError> {
    let bytes = read_bytes::<2>(input, offset)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32, ElfError> {
    let bytes = read_bytes::<4>(input, offset)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(input: &[u8], offset: usize) -> Result<u64, ElfError> {
    let bytes = read_bytes::<8>(input, offset)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_bytes<const N: usize>(input: &[u8], offset: usize) -> Result<[u8; N], ElfError> {
    let end = offset.checked_add(N).ok_or(ElfError::InvalidHeader)?;
    let slice = input.get(offset..end).ok_or(ElfError::ShortInput)?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    Ok(bytes)
}
