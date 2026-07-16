use super::{
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    user_boot::{UserAddressSpace, UserTrapFrame},
    user_stack::UserStack,
};

pub const ELF_HEADER_LEN: usize = 64;
pub const USER_INIT_EXPECTED_MESSAGE: &[u8] = b"user hello\n";
pub const USER_MAIN_PIE_LOAD_BIAS: usize = 0x1000_0000;
pub const USER_INTERPRETER_LOAD_BIAS: usize = 0x2000_0000;

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_TYPE_DYN: u16 = 3;
const ELF_MACHINE_RISCV: u16 = 243;
const ELF_PHDR_TYPE_LOAD: u32 = 1;
const ELF_PHDR_TYPE_INTERP: u32 = 3;
const ELF_PHDR_TYPE_PHDR: u32 = 6;
const ELF_PF_X: u32 = 1;
const ELF_PF_W: u32 = 2;
const ELF_PF_R: u32 = 4;
const ELF64_PHDR_SIZE: usize = 56;
const ELF_INTERP_PATH_MAX: usize = 128;
pub(crate) const MAX_LOAD_SEGMENTS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElfObjectRole {
    MainExecutable,
    Interpreter,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ElfType {
    Exec,
    Dyn,
}

impl ElfType {
    const fn index(self) -> usize {
        match self {
            Self::Exec => 2,
            Self::Dyn => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    TooManyMappings,
    TooManyMappingPages,
    MissingExecutableEntryMapping,
    InvalidStack,
    BackingAllocationFailed,
    PageTableAllocationFailed,
    PageTableInstallFailed,
    UserCopyOutOfRange,
}

// Stable error indices and names are consumed by optional user-boot diagnostics.
#[allow(dead_code)]
impl ElfError {
    pub const fn index(self) -> usize {
        match self {
            Self::InvalidState => 1,
            Self::ShortInput => 2,
            Self::BadMagic => 3,
            Self::UnsupportedClass => 4,
            Self::UnsupportedEndian => 5,
            Self::UnsupportedVersion => 6,
            Self::UnsupportedType => 7,
            Self::UnsupportedMachine => 8,
            Self::InvalidHeader => 9,
            Self::InvalidProgramHeader => 10,
            Self::TooManyLoadSegments => 11,
            Self::MissingLoadSegment => 12,
            Self::EntryOutsideExecutableSegment => 13,
            Self::TooManyMappings => 14,
            Self::TooManyMappingPages => 15,
            Self::MissingExecutableEntryMapping => 16,
            Self::InvalidStack => 17,
            Self::BackingAllocationFailed => 18,
            Self::PageTableAllocationFailed => 19,
            Self::PageTableInstallFailed => 20,
            Self::UserCopyOutOfRange => 21,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidState => "invalid_state",
            Self::ShortInput => "short_input",
            Self::BadMagic => "bad_magic",
            Self::UnsupportedClass => "unsupported_class",
            Self::UnsupportedEndian => "unsupported_endian",
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnsupportedType => "unsupported_type",
            Self::UnsupportedMachine => "unsupported_machine",
            Self::InvalidHeader => "invalid_header",
            Self::InvalidProgramHeader => "invalid_program_header",
            Self::TooManyLoadSegments => "too_many_load_segments",
            Self::MissingLoadSegment => "missing_load_segment",
            Self::EntryOutsideExecutableSegment => "entry_outside_executable_segment",
            Self::TooManyMappings => "too_many_mappings",
            Self::TooManyMappingPages => "too_many_mapping_pages",
            Self::MissingExecutableEntryMapping => "missing_executable_entry_mapping",
            Self::InvalidStack => "invalid_stack",
            Self::BackingAllocationFailed => "backing_allocation_failed",
            Self::PageTableAllocationFailed => "page_table_allocation_failed",
            Self::PageTableInstallFailed => "page_table_install_failed",
            Self::UserCopyOutOfRange => "user_copy_out_of_range",
        }
    }
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

pub struct ElfObject {
    lifecycle: Lifecycle,
    role: ElfObjectRole,
    elf_type: ElfType,
    input_len: usize,
    entry: usize,
    runtime_entry: usize,
    load_bias: usize,
    phdr_vaddr: usize,
    phentsize: usize,
    program_header_count: usize,
    load_segment_count: usize,
    load_segments: [ElfLoadSegment; MAX_LOAD_SEGMENTS],
    interpreter_path: [u8; ELF_INTERP_PATH_MAX],
    interpreter_path_len: usize,
    input_bound: bool,
    input_from_vfs: bool,
    magic_valid: bool,
    class_elf64: bool,
    little_endian: bool,
    machine_riscv: bool,
    type_supported: bool,
    static_executable: bool,
    dynamic_executable: bool,
    interpreter_required: bool,
    interpreter_path_bound: bool,
    et_dyn_pie_main_supported: bool,
    main_pie_load_bias_bound: bool,
    et_dyn_interpreter_supported: bool,
    et_dyn_loader_without_interp_deferred: bool,
    runtime_entry_bound: bool,
    auxv_exec_fields_bound: bool,
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
    user_entry_ready: bool,
}

#[allow(dead_code)]
impl ElfObject {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            role: ElfObjectRole::MainExecutable,
            elf_type: ElfType::Exec,
            input_len: 0,
            entry: 0,
            runtime_entry: 0,
            load_bias: 0,
            phdr_vaddr: 0,
            phentsize: 0,
            program_header_count: 0,
            load_segment_count: 0,
            load_segments: [ElfLoadSegment::empty(); MAX_LOAD_SEGMENTS],
            interpreter_path: [0; ELF_INTERP_PATH_MAX],
            interpreter_path_len: 0,
            input_bound: false,
            input_from_vfs: false,
            magic_valid: false,
            class_elf64: false,
            little_endian: false,
            machine_riscv: false,
            type_supported: false,
            static_executable: false,
            dynamic_executable: false,
            interpreter_required: false,
            interpreter_path_bound: false,
            et_dyn_pie_main_supported: false,
            main_pie_load_bias_bound: false,
            et_dyn_interpreter_supported: false,
            et_dyn_loader_without_interp_deferred: false,
            runtime_entry_bound: false,
            auxv_exec_fields_bound: false,
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
            user_entry_ready: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn input_len(&self) -> usize {
        self.input_len
    }

    pub const fn role(&self) -> ElfObjectRole {
        self.role
    }

    pub const fn elf_type_index(&self) -> usize {
        self.elf_type.index()
    }

    pub const fn entry(&self) -> usize {
        self.entry
    }

    pub const fn runtime_entry(&self) -> usize {
        self.runtime_entry
    }

    pub const fn load_bias(&self) -> usize {
        self.load_bias
    }

    pub const fn phdr_vaddr(&self) -> usize {
        self.phdr_vaddr
    }

    pub const fn phentsize(&self) -> usize {
        self.phentsize
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

    pub const fn dynamic_executable(&self) -> bool {
        self.dynamic_executable
    }

    pub const fn interpreter_required(&self) -> bool {
        self.interpreter_required
    }

    pub fn interpreter_path(&self) -> Option<&[u8]> {
        if self.interpreter_path_bound {
            Some(&self.interpreter_path[..self.interpreter_path_len])
        } else {
            None
        }
    }

    pub const fn interpreter_path_bound(&self) -> bool {
        self.interpreter_path_bound
    }

    pub const fn et_dyn_pie_main_supported(&self) -> bool {
        self.et_dyn_pie_main_supported
    }

    pub const fn main_pie_load_bias_bound(&self) -> bool {
        self.main_pie_load_bias_bound
    }

    pub const fn et_dyn_interpreter_supported(&self) -> bool {
        self.et_dyn_interpreter_supported
    }

    pub const fn et_dyn_loader_without_interp_deferred(&self) -> bool {
        self.et_dyn_loader_without_interp_deferred
    }

    pub const fn runtime_entry_bound(&self) -> bool {
        self.runtime_entry_bound
    }

    pub const fn auxv_exec_fields_bound(&self) -> bool {
        self.auxv_exec_fields_bound
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

    pub const fn user_entry_ready(&self) -> bool {
        self.user_entry_ready
    }

    pub fn preset_from_vfs(&mut self, input: &[u8]) -> Result<(), ElfError> {
        let header = parse_header(input)?;
        let load_bias = main_executable_load_bias(&header)?;
        self.preset_with_header(input, header, ElfObjectRole::MainExecutable, load_bias)
    }

    pub fn preset_interpreter_from_vfs(&mut self, input: &[u8]) -> Result<(), ElfError> {
        self.preset_with_role(
            input,
            ElfObjectRole::Interpreter,
            USER_INTERPRETER_LOAD_BIAS,
        )
    }

    fn preset_with_role(
        &mut self,
        input: &[u8],
        role: ElfObjectRole,
        load_bias: usize,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Base {
            return Err(ElfError::InvalidState);
        }
        let header = parse_header(input)?;
        self.preset_with_header(input, header, role, load_bias)
    }

    fn preset_with_header(
        &mut self,
        input: &[u8],
        header: ElfHeader,
        role: ElfObjectRole,
        load_bias: usize,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Base {
            return Err(ElfError::InvalidState);
        }
        if !elf_type_allowed_for_role(header.elf_type, role) {
            return Err(ElfError::UnsupportedType);
        }

        self.role = role;
        self.elf_type = header.elf_type;
        self.load_bias = load_bias;
        self.input_len = input.len();
        self.input_bound = true;
        self.input_from_vfs = true;
        self.magic_valid = true;
        self.class_elf64 = true;
        self.little_endian = true;
        self.machine_riscv = true;
        self.type_supported = true;
        self.static_executable = role == ElfObjectRole::MainExecutable
            && header.elf_type == ElfType::Exec
            && !header.interpreter_required;
        self.dynamic_executable = role == ElfObjectRole::MainExecutable
            && (header.elf_type == ElfType::Exec || header.elf_type == ElfType::Dyn)
            && header.interpreter_required;
        self.interpreter_required =
            role == ElfObjectRole::MainExecutable && header.interpreter_required;
        self.et_dyn_pie_main_supported = role == ElfObjectRole::MainExecutable
            && header.elf_type == ElfType::Dyn
            && header.interpreter_required;
        self.main_pie_load_bias_bound =
            self.et_dyn_pie_main_supported && self.load_bias == USER_MAIN_PIE_LOAD_BIAS;
        self.et_dyn_interpreter_supported =
            role == ElfObjectRole::Interpreter && header.elf_type == ElfType::Dyn;
        self.et_dyn_loader_without_interp_deferred = role == ElfObjectRole::MainExecutable
            && header.elf_type == ElfType::Dyn
            && !header.interpreter_required;
        self.no_separate_loader = !self.interpreter_required;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
            .map_err(|_| ElfError::InvalidState)
    }

    pub fn setup(&mut self, input: &[u8]) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Prepared {
            return Err(ElfError::InvalidState);
        }

        let header = parse_header(input)?;
        if !elf_type_allowed_for_role(header.elf_type, self.role) {
            return Err(ElfError::UnsupportedType);
        }
        if self.role == ElfObjectRole::MainExecutable
            && main_executable_load_bias(&header)? != self.load_bias
        {
            return Err(ElfError::InvalidState);
        }
        let parsed = parse_load_segments(input, &header, self.load_bias)?;
        if parsed.load_segment_count == 0 {
            return Err(ElfError::MissingLoadSegment);
        }
        let entry = self
            .load_bias
            .checked_add(header.entry)
            .ok_or(ElfError::InvalidHeader)?;
        if !entry_in_executable_segment(entry, &parsed.load_segments, parsed.load_segment_count) {
            return Err(ElfError::EntryOutsideExecutableSegment);
        }
        if self.role == ElfObjectRole::MainExecutable && header.interpreter_required {
            self.bind_interpreter_path(input, &header)?;
        }

        self.entry = entry;
        self.runtime_entry = entry;
        self.phdr_vaddr = phdr_vaddr(&header, &parsed, self.load_bias)?;
        self.phentsize = header.phentsize;
        self.program_header_count = header.phnum;
        self.load_segment_count = parsed.load_segment_count;
        self.load_segments = parsed.load_segments;
        self.program_headers_parsed = true;
        self.pt_load_segments_bound = true;
        self.segment_permissions_bound = parsed.segment_permissions_bound;
        self.load_plan_bound = true;
        self.entry_in_executable_segment = true;
        self.init_content_observed = self.role != ElfObjectRole::MainExecutable
            || loadable_content_contains(input, &parsed, USER_INIT_EXPECTED_MESSAGE);
        self.bss_zero_plan_bound = true;
        self.entry_bound = true;
        self.runtime_entry_bound = true;
        self.auxv_exec_fields_bound = self.role == ElfObjectRole::MainExecutable;
        self.load_merged_into_setup = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| ElfError::InvalidState)
    }

    fn bind_interpreter_path(&mut self, input: &[u8], header: &ElfHeader) -> Result<(), ElfError> {
        let Some((offset, len)) = header.interpreter else {
            return Err(ElfError::InvalidProgramHeader);
        };
        if len == 0 || len > ELF_INTERP_PATH_MAX {
            return Err(ElfError::InvalidProgramHeader);
        }
        let path = input
            .get(offset..offset + len)
            .ok_or(ElfError::InvalidProgramHeader)?;
        let path_len = if path[len - 1] == 0 { len - 1 } else { len };
        if path_len == 0 || path_len > ELF_INTERP_PATH_MAX {
            return Err(ElfError::InvalidProgramHeader);
        }
        self.interpreter_path = [0; ELF_INTERP_PATH_MAX];
        self.interpreter_path[..path_len].copy_from_slice(&path[..path_len]);
        self.interpreter_path_len = path_len;
        self.interpreter_path_bound = true;
        Ok(())
    }

    pub fn bind_runtime_interpreter(&mut self, interpreter: &ElfObject) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Ready
            || self.role != ElfObjectRole::MainExecutable
            || !self.interpreter_required
            || interpreter.state() != State::Ready
            || interpreter.role() != ElfObjectRole::Interpreter
        {
            return Err(ElfError::InvalidState);
        }
        self.runtime_entry = interpreter.entry();
        self.runtime_entry_bound = true;
        Ok(())
    }

    pub fn load_segments_fit_direct_read(&self, max_size: usize) -> bool {
        self.input_len <= max_size
    }

    pub fn enable(
        &mut self,
        address_space: &UserAddressSpace,
        stack: &UserStack,
        trap_frame: &UserTrapFrame,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || address_space.state() != State::Ready
            || stack.state() != State::Ready
            || trap_frame.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.user_entry_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}

struct ElfHeader {
    elf_type: ElfType,
    entry: usize,
    phoff: usize,
    phentsize: usize,
    phnum: usize,
    phdr_vaddr: usize,
    interpreter: Option<(usize, usize)>,
    interpreter_required: bool,
}

struct ParsedLoadSegments {
    load_segments: [ElfLoadSegment; MAX_LOAD_SEGMENTS],
    load_segment_count: usize,
    segment_permissions_bound: bool,
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
    let elf_type = match read_u16(input, 16)? {
        ELF_TYPE_EXEC => ElfType::Exec,
        ELF_TYPE_DYN => ElfType::Dyn,
        _ => return Err(ElfError::UnsupportedType),
    };
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

    let mut phdr_vaddr = 0usize;
    let mut interpreter = None;
    let mut index = 0usize;
    while index < phnum {
        let phdr = phoff + index * phentsize;
        let p_type = read_u32(input, phdr)?;
        if p_type == ELF_PHDR_TYPE_PHDR {
            phdr_vaddr = checked_usize(read_u64(input, phdr + 16)?)?;
        } else if p_type == ELF_PHDR_TYPE_INTERP {
            let offset = checked_usize(read_u64(input, phdr + 8)?)?;
            let filesz = checked_usize(read_u64(input, phdr + 32)?)?;
            if offset
                .checked_add(filesz)
                .filter(|end| *end <= input.len())
                .is_none()
            {
                return Err(ElfError::InvalidProgramHeader);
            }
            interpreter = Some((offset, filesz));
        }
        index += 1;
    }

    Ok(ElfHeader {
        elf_type,
        entry,
        phoff,
        phentsize,
        phnum,
        phdr_vaddr,
        interpreter_required: interpreter.is_some(),
        interpreter,
    })
}

fn elf_type_allowed_for_role(elf_type: ElfType, role: ElfObjectRole) -> bool {
    match role {
        ElfObjectRole::MainExecutable => elf_type == ElfType::Exec || elf_type == ElfType::Dyn,
        ElfObjectRole::Interpreter => elf_type == ElfType::Dyn,
    }
}

fn main_executable_load_bias(header: &ElfHeader) -> Result<usize, ElfError> {
    match header.elf_type {
        ElfType::Exec => Ok(0),
        ElfType::Dyn if header.interpreter_required => Ok(USER_MAIN_PIE_LOAD_BIAS),
        ElfType::Dyn => Err(ElfError::UnsupportedType),
    }
}

fn parse_load_segments(
    input: &[u8],
    header: &ElfHeader,
    load_bias: usize,
) -> Result<ParsedLoadSegments, ElfError> {
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
            let raw_vaddr = checked_usize(read_u64(input, phdr + 16)?)?;
            let vaddr = load_bias
                .checked_add(raw_vaddr)
                .ok_or(ElfError::InvalidProgramHeader)?;
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

fn phdr_vaddr(
    header: &ElfHeader,
    parsed: &ParsedLoadSegments,
    load_bias: usize,
) -> Result<usize, ElfError> {
    if header.phdr_vaddr != 0 {
        return header
            .phdr_vaddr
            .checked_add(load_bias)
            .ok_or(ElfError::InvalidHeader);
    }
    let phdr_size = header
        .phentsize
        .checked_mul(header.phnum)
        .ok_or(ElfError::InvalidHeader)?;
    let phdr_end = header
        .phoff
        .checked_add(phdr_size)
        .ok_or(ElfError::InvalidHeader)?;
    let mut index = 0usize;
    while index < parsed.load_segment_count {
        let segment = parsed.load_segments[index];
        if header.phoff >= segment.offset() && phdr_end <= segment.file_end() {
            return segment
                .vaddr()
                .checked_add(header.phoff - segment.offset())
                .ok_or(ElfError::InvalidHeader);
        }
        index += 1;
    }
    Err(ElfError::InvalidHeader)
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
