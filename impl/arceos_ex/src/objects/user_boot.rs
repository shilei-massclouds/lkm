use core::sync::atomic::{AtomicU8, Ordering};

use crate::arch::riscv64::csr;

#[cfg(app_user_boot)]
use super::{
    block_device::BlockDeviceRegistry,
    ext2::{Ext2FileSystem, EXT2_MAX_BLOCK_SIZE, EXT2_NDIR_BLOCKS},
    vfs::VfsCore,
    virtio_blk,
};
use super::{
    exception_stream::{ExceptionStream, SyscallTable},
    kernel_image::KernelImage,
    mm_core::{GfpFlags, KernelGlobalAllocator, PageAllocator, PageMetadataMap, PageRef},
    page_table::{
        copy_high_half_root_entries, page_table_storage_ready, sv39_indices, table_pte_from_phys,
        user_leaf_pte_from_phys, PageTablePage,
    },
    rest_init::KernelInitTask,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_page_tables,
    swapper_vm::SwapperVm,
    vfs::FsStruct,
};

pub const USER_INIT_PATH: &[u8] = b"/sbin/init";
pub const USER_INIT_EXPECTED_MESSAGE: &[u8] = b"user hello\n";

pub const ELF_HEADER_LEN: usize = 64;
#[cfg(app_user_boot)]
pub const USER_BOOT_READ_MAX: usize = EXT2_MAX_BLOCK_SIZE * EXT2_NDIR_BLOCKS;
pub const USER_STACK_SIZE: usize = 16 * 1024;
pub const USER_STACK_TOP: usize = 0x4000_0000;
pub const USER_PAGE_SIZE: usize = 4096;
#[cfg(app_user_boot)]
pub const USER_KERNEL_TRAP_STACK_SIZE: usize = 4096;
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
const MAX_MAPPING_BACKING_PAGES: usize = 32;
const MAX_STACK_PAGES: usize = USER_STACK_SIZE / USER_PAGE_SIZE;
const MAX_USER_L0_TABLES: usize = MAX_USER_MAPPINGS + 1;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserInitPathRef {
    DefaultInit,
}

static USER_INIT_RUNTIME_ENTERED: AtomicU8 = AtomicU8::new(0);

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
    TooManyMappingPages,
    MissingExecutableEntryMapping,
    InvalidStack,
    BackingAllocationFailed,
    PageTableAllocationFailed,
    PageTableInstallFailed,
    UserCopyOutOfRange,
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
    page_offset: usize,
    file_offset: usize,
    filesz: usize,
    readable: bool,
    writable: bool,
    executable: bool,
    user_accessible: bool,
    bss_zero_bytes: usize,
    backing_pages: [Option<PageRef>; MAX_MAPPING_BACKING_PAGES],
    backing_page_count: usize,
    file_bytes_copied: usize,
    bss_bytes_zeroed: usize,
    page_table_entry_bound: bool,
}

impl UserMapping {
    const fn empty() -> Self {
        Self {
            kind: UserMappingKind::Empty,
            vaddr: 0,
            memsz: 0,
            page_offset: 0,
            file_offset: 0,
            filesz: 0,
            readable: false,
            writable: false,
            executable: false,
            user_accessible: false,
            bss_zero_bytes: 0,
            backing_pages: [None; MAX_MAPPING_BACKING_PAGES],
            backing_page_count: 0,
            file_bytes_copied: 0,
            bss_bytes_zeroed: 0,
            page_table_entry_bound: false,
        }
    }

    fn from_segment(segment: ElfLoadSegment) -> Self {
        Self {
            kind: UserMappingKind::ElfSegment,
            vaddr: segment.vaddr,
            memsz: segment.memsz,
            page_offset: segment.vaddr % USER_PAGE_SIZE,
            file_offset: segment.offset,
            filesz: segment.filesz,
            readable: segment.readable(),
            writable: segment.writable(),
            executable: segment.executable(),
            user_accessible: true,
            bss_zero_bytes: segment.memsz - segment.filesz,
            backing_pages: [None; MAX_MAPPING_BACKING_PAGES],
            backing_page_count: 0,
            file_bytes_copied: 0,
            bss_bytes_zeroed: 0,
            page_table_entry_bound: false,
        }
    }

    fn from_stack(stack: &UserStack) -> Self {
        let mut mapping = Self {
            kind: UserMappingKind::Stack,
            vaddr: stack.base,
            memsz: stack.size,
            page_offset: 0,
            file_offset: 0,
            filesz: 0,
            readable: true,
            writable: true,
            executable: false,
            user_accessible: true,
            bss_zero_bytes: stack.size,
            backing_pages: [None; MAX_MAPPING_BACKING_PAGES],
            backing_page_count: 0,
            file_bytes_copied: 0,
            bss_bytes_zeroed: stack.size,
            page_table_entry_bound: stack.backing_pages_allocated,
        };
        let mut index = 0usize;
        while index < stack.backing_page_count && index < MAX_MAPPING_BACKING_PAGES {
            mapping.backing_pages[index] = stack.backing_pages[index];
            index += 1;
        }
        mapping.backing_page_count = stack.backing_page_count;
        mapping
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

    pub const fn page_offset(&self) -> usize {
        self.page_offset
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

    pub const fn backing_page_count(&self) -> usize {
        self.backing_page_count
    }

    pub const fn backing_page(&self, index: usize) -> Option<PageRef> {
        if index < self.backing_page_count {
            self.backing_pages[index]
        } else {
            None
        }
    }

    pub const fn file_bytes_copied(&self) -> usize {
        self.file_bytes_copied
    }

    pub const fn bss_bytes_zeroed(&self) -> usize {
        self.bss_bytes_zeroed
    }

    pub const fn page_table_entry_bound(&self) -> bool {
        self.page_table_entry_bound
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
    backing_pages: [Option<PageRef>; MAX_STACK_PAGES],
    backing_page_count: usize,
    allocated: bool,
    fixed_size_bound: bool,
    mapped_into_address_space: bool,
    backing_pages_allocated: bool,
    zeroed: bool,
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
            backing_pages: [None; MAX_STACK_PAGES],
            backing_page_count: 0,
            allocated: false,
            fixed_size_bound: false,
            mapped_into_address_space: false,
            backing_pages_allocated: false,
            zeroed: false,
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

    pub const fn backing_page_count(&self) -> usize {
        self.backing_page_count
    }

    pub const fn backing_page(&self, index: usize) -> Option<PageRef> {
        if index < self.backing_page_count {
            self.backing_pages[index]
        } else {
            None
        }
    }

    pub const fn fixed_size_bound(&self) -> bool {
        self.fixed_size_bound
    }

    pub const fn mapped_into_address_space(&self) -> bool {
        self.mapped_into_address_space
    }

    pub const fn backing_pages_allocated(&self) -> bool {
        self.backing_pages_allocated
    }

    pub const fn zeroed(&self) -> bool {
        self.zeroed
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
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
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
        self.backing_pages = [None; MAX_STACK_PAGES];
        self.backing_page_count = 0;
        let mut index = 0usize;
        while index < MAX_STACK_PAGES {
            let Some(page) = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map)
            else {
                while self.backing_page_count > 0 {
                    self.backing_page_count -= 1;
                    if let Some(allocated) = self.backing_pages[self.backing_page_count] {
                        let _ = page_allocator.free_pages(allocated, 0, page_metadata_map);
                    }
                }
                return failed_condition(
                    LifecycleEvent::Setup,
                    self.lifecycle.state(),
                    State::Base,
                    State::Ready,
                );
            };
            let Some(linear) = page_metadata_map.page_address(page) else {
                let _ = page_allocator.free_pages(page, 0, page_metadata_map);
                return failed_condition(
                    LifecycleEvent::Setup,
                    self.lifecycle.state(),
                    State::Base,
                    State::Ready,
                );
            };
            unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };
            self.backing_pages[index] = Some(page);
            self.backing_page_count += 1;
            index += 1;
        }
        self.allocated = true;
        self.fixed_size_bound = true;
        self.mapped_into_address_space = true;
        self.backing_pages_allocated = true;
        self.zeroed = true;
        self.initial_sp_bound = true;
        self.minimal_arg_env_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct UserAddressSpace {
    lifecycle: Lifecycle,
    allocated: bool,
    first_instance: bool,
    bound_to_kernel_init_task: bool,
    low_half_private: bool,
    high_half_shares_swapper: bool,
    kernel_pages_u_disabled: bool,
    user_pages_u_enabled: bool,
    elf_load_plan_consumed: bool,
    segment_mappings_bound: bool,
    entry_mapping_executable: bool,
    bss_zero_plan_consumed: bool,
    backing_pages_allocated: bool,
    elf_file_bytes_copied: bool,
    bss_bytes_zeroed: bool,
    page_table_view_ready: bool,
    elf_segments_mapped: bool,
    stack_mapped: bool,
    elf_mapped: bool,
    elf_bss_zeroed: bool,
    runtime_ready: bool,
    real_page_table_allocated: bool,
    user_leaf_ptes_installed: bool,
    high_half_root_entries_shared: bool,
    satp_token_ready: bool,
    prepared_but_not_current: bool,
    satp_token: usize,
    user_leaf_pte_count: usize,
    page_table_root: Option<PageRef>,
    page_table_l1: Option<PageRef>,
    page_table_l0s: [UserL0TableSlot; MAX_USER_L0_TABLES],
    page_table_l0_count: usize,
    mappings: [UserMapping; MAX_USER_MAPPINGS],
    mapping_count: usize,
    segment_mapping_count: usize,
    stack_mapping_index: usize,
}

#[derive(Clone, Copy)]
struct UserL0TableSlot {
    vpn1: usize,
    page: Option<PageRef>,
}

impl UserL0TableSlot {
    const fn empty() -> Self {
        Self {
            vpn1: 0,
            page: None,
        }
    }
}

#[allow(dead_code)]
impl UserAddressSpace {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            allocated: false,
            first_instance: false,
            bound_to_kernel_init_task: false,
            low_half_private: false,
            high_half_shares_swapper: false,
            kernel_pages_u_disabled: false,
            user_pages_u_enabled: false,
            elf_load_plan_consumed: false,
            segment_mappings_bound: false,
            entry_mapping_executable: false,
            bss_zero_plan_consumed: false,
            backing_pages_allocated: false,
            elf_file_bytes_copied: false,
            bss_bytes_zeroed: false,
            page_table_view_ready: false,
            elf_segments_mapped: false,
            stack_mapped: false,
            elf_mapped: false,
            elf_bss_zeroed: false,
            runtime_ready: false,
            real_page_table_allocated: false,
            user_leaf_ptes_installed: false,
            high_half_root_entries_shared: false,
            satp_token_ready: false,
            prepared_but_not_current: false,
            satp_token: 0,
            user_leaf_pte_count: 0,
            page_table_root: None,
            page_table_l1: None,
            page_table_l0s: [UserL0TableSlot::empty(); MAX_USER_L0_TABLES],
            page_table_l0_count: 0,
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

    pub const fn first_instance(&self) -> bool {
        self.first_instance
    }

    pub const fn bound_to_kernel_init_task(&self) -> bool {
        self.bound_to_kernel_init_task
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

    pub const fn backing_pages_allocated(&self) -> bool {
        self.backing_pages_allocated
    }

    pub const fn elf_file_bytes_copied(&self) -> bool {
        self.elf_file_bytes_copied
    }

    pub const fn bss_bytes_zeroed(&self) -> bool {
        self.bss_bytes_zeroed
    }

    pub const fn page_table_view_ready(&self) -> bool {
        self.page_table_view_ready
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

    pub const fn real_page_table_allocated(&self) -> bool {
        self.real_page_table_allocated
    }

    pub const fn user_leaf_ptes_installed(&self) -> bool {
        self.user_leaf_ptes_installed
    }

    pub const fn high_half_root_entries_shared(&self) -> bool {
        self.high_half_root_entries_shared
    }

    pub const fn satp_token_ready(&self) -> bool {
        self.satp_token_ready
    }

    pub const fn prepared_but_not_current(&self) -> bool {
        self.prepared_but_not_current
    }

    pub const fn satp_token(&self) -> usize {
        self.satp_token
    }

    pub const fn user_leaf_pte_count(&self) -> usize {
        self.user_leaf_pte_count
    }

    pub const fn page_table_l0_count(&self) -> usize {
        self.page_table_l0_count
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
        kernel_init_task: &KernelInitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || swapper_vm.state() != State::Online
            || page_allocator.state() != State::Ready
            || global_allocator.state() != State::Ready
            || kernel_init_task.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.allocated = true;
        self.first_instance = true;
        self.bound_to_kernel_init_task = true;
        self.low_half_private = true;
        self.high_half_shares_swapper = true;
        self.kernel_pages_u_disabled = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(
        &mut self,
        elf: &ElfObject,
        stack: &UserStack,
        image: &[u8],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), ElfError> {
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
            let mut mapping = UserMapping::from_segment(segment);
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
            if let Err(error) =
                materialize_mapping(&mut mapping, image, page_allocator, page_metadata_map)
            {
                release_mappings(
                    &mut self.mappings,
                    self.mapping_count,
                    page_allocator,
                    page_metadata_map,
                );
                self.mapping_count = 0;
                self.segment_mapping_count = 0;
                return Err(error);
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
        self.backing_pages_allocated =
            mappings_have_backing_pages(&self.mappings, self.mapping_count);
        self.elf_file_bytes_copied =
            mapping_file_bytes_match(&self.mappings, self.segment_mapping_count);
        self.bss_bytes_zeroed = mapping_bss_bytes_match(&self.mappings, self.segment_mapping_count);
        self.page_table_view_ready =
            mappings_have_page_table_entries(&self.mappings, self.mapping_count);
        self.elf_segments_mapped = true;
        self.stack_mapped = true;
        self.elf_mapped = true;
        self.elf_bss_zeroed = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| ElfError::InvalidState)
    }

    pub fn enable(
        &mut self,
        trap_frame: &UserTrapFrame,
        swapper_vm: &SwapperVm,
        kernel_image: &KernelImage,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<(), ElfError> {
        if self.lifecycle.state() != State::Ready
            || trap_frame.state() != State::Ready
            || swapper_vm.state() != State::Online
            || page_allocator.state() != State::Ready
            || page_metadata_map.state() != State::Ready
            || !self.page_table_view_ready()
            || !self.entry_mapping_executable()
        {
            return Err(ElfError::InvalidState);
        }

        self.release_page_table_pages(page_allocator, page_metadata_map);
        if !self.allocate_base_page_tables(page_allocator, page_metadata_map) {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableAllocationFailed);
        }
        if !self.copy_swapper_high_half(kernel_image, page_metadata_map)
            || !self.install_all_user_leaf_ptes(page_allocator, page_metadata_map)
        {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableInstallFailed);
        }

        let Some(root) = self.page_table_root else {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableInstallFailed);
        };
        let root_phys = root.phys().value();
        if root_phys == 0 {
            self.release_page_table_pages(page_allocator, page_metadata_map);
            return Err(ElfError::PageTableInstallFailed);
        }

        self.satp_token = csr::SATP_MODE_SV39 | (root_phys >> 12);
        self.satp_token_ready = true;
        self.real_page_table_allocated = true;
        self.runtime_ready = true;
        self.prepared_but_not_current = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
            .map_err(|_| ElfError::InvalidState)
    }

    fn allocate_base_page_tables(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        let Some(root) = allocate_zeroed_page_table_page(page_allocator, page_metadata_map) else {
            return false;
        };
        let Some(l1) = allocate_zeroed_page_table_page(page_allocator, page_metadata_map) else {
            let _ = page_allocator.free_pages(root, 0, page_metadata_map);
            return false;
        };

        self.page_table_root = Some(root);
        self.page_table_l1 = Some(l1);
        self.page_table_l0s = [UserL0TableSlot::empty(); MAX_USER_L0_TABLES];
        self.page_table_l0_count = 0;
        self.user_leaf_pte_count = 0;
        true
    }

    fn copy_swapper_high_half(
        &mut self,
        kernel_image: &KernelImage,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        let Some(root_page) = self.page_table_root else {
            return false;
        };
        let Some(root_table) = page_table_page_mut(root_page, page_metadata_map) else {
            return false;
        };
        let copied = copy_high_half_root_entries(
            root_table,
            static_page_tables::swapper_pg_dir(kernel_image),
        );
        self.high_half_root_entries_shared = copied != 0;
        self.high_half_root_entries_shared
    }

    fn install_all_user_leaf_ptes(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        let mut index = 0usize;
        while index < self.mapping_count {
            let mapping = self.mappings[index];
            if !self.install_mapping_ptes(mapping, page_allocator, page_metadata_map) {
                return false;
            }
            index += 1;
        }
        self.user_leaf_ptes_installed = self.user_leaf_pte_count != 0
            && self.user_leaf_pte_count
                == total_mapping_page_count(&self.mappings, self.mapping_count);
        self.user_leaf_ptes_installed
    }

    fn install_mapping_ptes(
        &mut self,
        mapping: UserMapping,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if mapping.kind() == UserMappingKind::Empty
            || !mapping.user_accessible()
            || mapping.backing_page_count() == 0
            || mapping.vaddr().checked_sub(mapping.page_offset()).is_none()
        {
            return false;
        }

        let mut virt = mapping.vaddr() - mapping.page_offset();
        let mut page_index = 0usize;
        while page_index < mapping.backing_page_count() {
            let Some(page) = mapping.backing_page(page_index) else {
                return false;
            };
            if !self.install_user_leaf_pte(
                virt,
                page.phys().value(),
                mapping.readable(),
                mapping.writable(),
                mapping.executable(),
                page_allocator,
                page_metadata_map,
            ) {
                return false;
            }
            let Some(next_virt) = virt.checked_add(USER_PAGE_SIZE) else {
                return false;
            };
            virt = next_virt;
            page_index += 1;
        }
        true
    }

    fn install_user_leaf_pte(
        &mut self,
        virt: usize,
        phys: usize,
        readable: bool,
        writable: bool,
        executable: bool,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> bool {
        if virt % USER_PAGE_SIZE != 0 || phys % USER_PAGE_SIZE != 0 {
            return false;
        }
        let (vpn2, vpn1, vpn0) = sv39_indices(virt);
        if vpn2 != 0 {
            return false;
        }
        let Some(root_page) = self.page_table_root else {
            return false;
        };
        let Some(l1_page) = self.page_table_l1 else {
            return false;
        };
        let Some(l0_page) = self.l0_page_for_vpn1(vpn1, page_allocator, page_metadata_map) else {
            return false;
        };
        let Some(root_table) = page_table_page_mut(root_page, page_metadata_map) else {
            return false;
        };
        if !root_table.set_entry(vpn2, table_pte_from_phys(l1_page.phys().value())) {
            return false;
        }
        let Some(l1_table) = page_table_page_mut(l1_page, page_metadata_map) else {
            return false;
        };
        if !l1_table.set_entry(vpn1, table_pte_from_phys(l0_page.phys().value())) {
            return false;
        }
        let Some(leaf) = user_leaf_pte_from_phys(phys, readable, writable, executable) else {
            return false;
        };
        let Some(l0_table) = page_table_page_mut(l0_page, page_metadata_map) else {
            return false;
        };
        if !l0_table.set_entry(vpn0, leaf) {
            return false;
        }
        self.user_leaf_pte_count += 1;
        true
    }

    fn l0_page_for_vpn1(
        &mut self,
        vpn1: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<PageRef> {
        let mut index = 0usize;
        while index < self.page_table_l0_count {
            let slot = self.page_table_l0s[index];
            if slot.page.is_some() && slot.vpn1 == vpn1 {
                return slot.page;
            }
            index += 1;
        }

        if self.page_table_l0_count >= MAX_USER_L0_TABLES {
            return None;
        }
        let page = allocate_zeroed_page_table_page(page_allocator, page_metadata_map)?;
        self.page_table_l0s[self.page_table_l0_count] = UserL0TableSlot {
            vpn1,
            page: Some(page),
        };
        self.page_table_l0_count += 1;
        Some(page)
    }

    fn release_page_table_pages(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) {
        let mut index = 0usize;
        while index < self.page_table_l0_count {
            if let Some(page) = self.page_table_l0s[index].page.take() {
                let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            }
            index += 1;
        }
        if let Some(page) = self.page_table_l1.take() {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        }
        if let Some(page) = self.page_table_root.take() {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        }
        self.page_table_l0s = [UserL0TableSlot::empty(); MAX_USER_L0_TABLES];
        self.page_table_l0_count = 0;
        self.user_leaf_pte_count = 0;
        self.real_page_table_allocated = false;
        self.user_leaf_ptes_installed = false;
        self.high_half_root_entries_shared = false;
        self.satp_token_ready = false;
        self.prepared_but_not_current = false;
        self.satp_token = 0;
        self.runtime_ready = false;
    }
}

fn allocate_zeroed_page_table_page(
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) -> Option<PageRef> {
    let page = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map)?;
    let Some(table) = page_table_page_mut(page, page_metadata_map) else {
        let _ = page_allocator.free_pages(page, 0, page_metadata_map);
        return None;
    };
    table.clear();
    Some(page)
}

fn page_table_page_mut(
    page: PageRef,
    page_metadata_map: &PageMetadataMap,
) -> Option<&'static mut PageTablePage> {
    let linear = page_metadata_map.page_address(page)?;
    if !page_table_storage_ready(linear, USER_PAGE_SIZE) {
        return None;
    }
    Some(unsafe { &mut *(linear as *mut PageTablePage) })
}

fn total_mapping_page_count(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> usize {
    let mut pages = 0usize;
    let mut index = 0usize;
    while index < count {
        pages += mappings[index].backing_page_count();
        index += 1;
    }
    pages
}

pub const SSTATUS_SPP_USER_CLEAR: usize = 0;
pub const SSTATUS_SPIE_SET: usize = 1 << 5;

#[cfg(app_user_boot)]
#[repr(align(16))]
struct UserKernelTrapStack {
    bytes: [u8; USER_KERNEL_TRAP_STACK_SIZE],
}

#[cfg(app_user_boot)]
static mut USER_KERNEL_TRAP_STACK: UserKernelTrapStack = UserKernelTrapStack {
    bytes: [0; USER_KERNEL_TRAP_STACK_SIZE],
};
#[cfg(app_user_boot)]
static mut USER_BOOT_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];

pub struct UserTrapFrame {
    lifecycle: Lifecycle,
    entry: usize,
    sp: usize,
    sstatus: usize,
    allocated: bool,
    entry_bound: bool,
    sp_bound: bool,
    sstatus_user_mode: bool,
    sret_ready: bool,
    address_space_bound: bool,
    prepared_but_not_entered: bool,
}

#[allow(dead_code)]
impl UserTrapFrame {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            entry: 0,
            sp: 0,
            sstatus: 0,
            allocated: false,
            entry_bound: false,
            sp_bound: false,
            sstatus_user_mode: false,
            sret_ready: false,
            address_space_bound: false,
            prepared_but_not_entered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn entry(&self) -> usize {
        self.entry
    }

    pub const fn sp(&self) -> usize {
        self.sp
    }

    pub const fn sstatus(&self) -> usize {
        self.sstatus
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub const fn entry_bound(&self) -> bool {
        self.entry_bound
    }

    pub const fn sp_bound(&self) -> bool {
        self.sp_bound
    }

    pub const fn sstatus_user_mode(&self) -> bool {
        self.sstatus_user_mode
    }

    pub const fn sret_ready(&self) -> bool {
        self.sret_ready
    }

    pub const fn address_space_bound(&self) -> bool {
        self.address_space_bound
    }

    pub const fn prepared_but_not_entered(&self) -> bool {
        self.prepared_but_not_entered
    }

    pub fn setup(
        &mut self,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        stack: &UserStack,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || address_space.state() != State::Ready
            || elf.state() != State::Ready
            || stack.state() != State::Ready
            || !address_space.page_table_view_ready()
            || !address_space.entry_mapping_executable()
            || !address_space.bound_to_kernel_init_task()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.entry = elf.entry();
        self.sp = stack.initial_sp();
        self.sstatus = SSTATUS_SPIE_SET | SSTATUS_SPP_USER_CLEAR;
        self.allocated = true;
        self.entry_bound = true;
        self.sp_bound = true;
        self.sstatus_user_mode = true;
        self.sret_ready = true;
        self.address_space_bound = true;
        self.prepared_but_not_entered = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
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
    user_entry_ready: bool,
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
            user_entry_ready: false,
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

    pub const fn user_entry_ready(&self) -> bool {
        self.user_entry_ready
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

pub struct UserInitProcess {
    lifecycle: Lifecycle,
    reuses_kernel_init_task: bool,
    pid1_preserved: bool,
    exec_identity_handoff: bool,
    no_new_task_struct: bool,
    kernel_init_not_destroyed: bool,
    path: UserInitPathRef,
    path_bound: bool,
    address_space_bound: bool,
    fs_struct_inherited: bool,
    trap_frame_bound: bool,
    syscall_context_bound: bool,
    kernel_init_execve_to_user_init: bool,
    kernel_init_pid1_identity_preserved: bool,
    kernel_init_user_mm_attached: bool,
    kernel_init_user_trap_frame_attached: bool,
    user_entry_ready: bool,
    trap_return_bound: bool,
    syscall_dispatch_bound: bool,
    syscall_arguments_extracted: bool,
    runtime_entered: bool,
}

#[allow(dead_code)]
impl UserInitProcess {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            reuses_kernel_init_task: false,
            pid1_preserved: false,
            exec_identity_handoff: false,
            no_new_task_struct: false,
            kernel_init_not_destroyed: false,
            path: UserInitPathRef::DefaultInit,
            path_bound: false,
            address_space_bound: false,
            fs_struct_inherited: false,
            trap_frame_bound: false,
            syscall_context_bound: false,
            kernel_init_execve_to_user_init: false,
            kernel_init_pid1_identity_preserved: false,
            kernel_init_user_mm_attached: false,
            kernel_init_user_trap_frame_attached: false,
            user_entry_ready: false,
            trap_return_bound: false,
            syscall_dispatch_bound: false,
            syscall_arguments_extracted: false,
            runtime_entered: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn reuses_kernel_init_task(&self) -> bool {
        self.reuses_kernel_init_task
    }

    pub const fn pid1_preserved(&self) -> bool {
        self.pid1_preserved
    }

    pub const fn exec_identity_handoff(&self) -> bool {
        self.exec_identity_handoff
    }

    pub const fn no_new_task_struct(&self) -> bool {
        self.no_new_task_struct
    }

    pub const fn kernel_init_not_destroyed(&self) -> bool {
        self.kernel_init_not_destroyed
    }

    pub const fn path(&self) -> UserInitPathRef {
        self.path
    }

    pub const fn path_bound(&self) -> bool {
        self.path_bound
    }

    pub const fn address_space_bound(&self) -> bool {
        self.address_space_bound
    }

    pub const fn fs_struct_inherited(&self) -> bool {
        self.fs_struct_inherited
    }

    pub const fn trap_frame_bound(&self) -> bool {
        self.trap_frame_bound
    }

    pub const fn syscall_context_bound(&self) -> bool {
        self.syscall_context_bound
    }

    pub const fn kernel_init_execve_to_user_init(&self) -> bool {
        self.kernel_init_execve_to_user_init
    }

    pub const fn kernel_init_pid1_identity_preserved(&self) -> bool {
        self.kernel_init_pid1_identity_preserved
    }

    pub const fn kernel_init_user_mm_attached(&self) -> bool {
        self.kernel_init_user_mm_attached
    }

    pub const fn kernel_init_user_trap_frame_attached(&self) -> bool {
        self.kernel_init_user_trap_frame_attached
    }

    pub const fn user_entry_ready(&self) -> bool {
        self.user_entry_ready
    }

    pub const fn trap_return_bound(&self) -> bool {
        self.trap_return_bound
    }

    pub const fn syscall_dispatch_bound(&self) -> bool {
        self.syscall_dispatch_bound
    }

    pub const fn syscall_arguments_extracted(&self) -> bool {
        self.syscall_arguments_extracted
    }

    pub const fn runtime_entered(&self) -> bool {
        self.runtime_entered
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        trap_frame: &UserTrapFrame,
        fs_struct: &FsStruct,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || kernel_init_task.pid() != super::rest_init::KERNEL_INIT_PID
            || address_space.state() != State::Online
            || elf.state() != State::Online
            || trap_frame.state() != State::Ready
            || fs_struct.state() != State::Ready
            || !address_space.bound_to_kernel_init_task()
            || !trap_frame.address_space_bound()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.reuses_kernel_init_task = true;
        self.pid1_preserved = true;
        self.exec_identity_handoff = true;
        self.no_new_task_struct = true;
        self.kernel_init_not_destroyed = kernel_init_task.state() == State::Online;
        self.path = UserInitPathRef::DefaultInit;
        self.path_bound = true;
        self.address_space_bound = true;
        self.fs_struct_inherited = true;
        self.trap_frame_bound = true;
        self.kernel_init_execve_to_user_init = true;
        self.kernel_init_pid1_identity_preserved = true;
        self.kernel_init_user_mm_attached = true;
        self.kernel_init_user_trap_frame_attached = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn enable(
        &mut self,
        trap_frame: &UserTrapFrame,
        exception_stream: &ExceptionStream,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || trap_frame.state() != State::Ready
            || exception_stream.syscall_state() != State::Online
            || syscall_table.state() != State::Ready
            || !syscall_table.bound_to_exception()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.syscall_context_bound = true;
        self.trap_return_bound = true;
        self.syscall_dispatch_bound = true;
        self.syscall_arguments_extracted = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }

    pub fn enter_user_mode(&mut self, trap_frame: &UserTrapFrame) -> EventResult {
        if self.lifecycle.state() != State::Online
            || trap_frame.state() != State::Ready
            || !self.trap_return_bound
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.user_entry_ready = true;
        self.runtime_entered = true;
        USER_INIT_RUNTIME_ENTERED.store(1, Ordering::Release);
        Ok(())
    }

    pub fn refresh_runtime_observations(&mut self) {
        self.runtime_entered |= USER_INIT_RUNTIME_ENTERED.load(Ordering::Acquire) != 0;
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
    driven_by_kernel_init_task: bool,
    try_candidate_bound: bool,
    selected_path_bound: bool,
    reads_init_from_vfs: bool,
    enters_user_mode: bool,
    no_return_handoff: bool,
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
            driven_by_kernel_init_task: false,
            try_candidate_bound: false,
            selected_path_bound: false,
            reads_init_from_vfs: false,
            enters_user_mode: false,
            no_return_handoff: false,
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

    pub const fn driven_by_kernel_init_task(&self) -> bool {
        self.driven_by_kernel_init_task
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

    pub const fn enters_user_mode(&self) -> bool {
        self.enters_user_mode
    }

    pub const fn no_return_handoff(&self) -> bool {
        self.no_return_handoff
    }

    pub fn setup(&mut self, kernel_init_task: &KernelInitTask) -> EventResult {
        if self.lifecycle.state() != State::Base || kernel_init_task.state() != State::Online {
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
        self.driven_by_kernel_init_task = true;
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

    #[cfg(app_user_boot)]
    pub fn enable_for_user_entry(
        &mut self,
        elf: &ElfObject,
        address_space: &UserAddressSpace,
        trap_frame: &UserTrapFrame,
        exception_stream: &ExceptionStream,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || elf.state() != State::Online
            || address_space.state() != State::Online
            || trap_frame.state() != State::Ready
            || syscall_table.state() != State::Ready
            || exception_stream.syscall_state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.enters_user_mode = true;
        self.no_return_handoff = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
    }
}

#[cfg(app_user_boot)]
#[allow(clippy::too_many_arguments)]
pub fn run_first_user_init(
    payload: &mut UserBootPayload,
    elf: &mut ElfObject,
    address_space: &mut UserAddressSpace,
    stack: &mut UserStack,
    trap_frame: &mut UserTrapFrame,
    user_init_process: &mut UserInitProcess,
    vfs_core: &mut VfsCore,
    fs_struct: &FsStruct,
    ext2_filesystem: &mut Ext2FileSystem,
    block_device_registry: &mut BlockDeviceRegistry,
    kernel_init_task: &KernelInitTask,
    swapper_vm: &SwapperVm,
    kernel_image: &KernelImage,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
    kernel_global_allocator: &KernelGlobalAllocator,
    exception_stream: &mut ExceptionStream,
    syscall_table: &mut SyscallTable,
) -> ! {
    if payload.setup(kernel_init_task).is_err() {
        user_boot_panic("user payload setup failed\n");
    }

    let image = read_user_init_image(
        vfs_core,
        fs_struct,
        ext2_filesystem,
        block_device_registry,
        kernel_image,
    );
    if elf.preset_from_vfs(image).is_err() || elf.setup(image).is_err() {
        user_boot_panic("user init ELF setup failed\n");
    }
    if payload.try_candidate(elf).is_err() {
        user_boot_panic("user init candidate failed\n");
    }
    if address_space
        .preset(
            swapper_vm,
            page_allocator,
            kernel_global_allocator,
            kernel_init_task,
        )
        .is_err()
    {
        user_boot_panic("user address space preset failed\n");
    }
    if stack
        .setup(address_space, page_allocator, page_metadata_map)
        .is_err()
    {
        user_boot_panic("user stack setup failed\n");
    }
    if address_space
        .setup(elf, stack, image, page_allocator, page_metadata_map)
        .is_err()
    {
        user_boot_panic("user address space setup failed\n");
    }
    if trap_frame.setup(address_space, elf, stack).is_err() {
        user_boot_panic("user trap frame setup failed\n");
    }
    if elf.enable(address_space, stack, trap_frame).is_err() {
        user_boot_panic("user ELF enable failed\n");
    }
    if address_space
        .enable(
            trap_frame,
            swapper_vm,
            kernel_image,
            page_allocator,
            page_metadata_map,
        )
        .is_err()
    {
        user_boot_panic("user address space enable failed\n");
    }
    if exception_stream.syscall_setup(syscall_table).is_err() {
        user_boot_panic("user syscall setup failed\n");
    }
    if exception_stream.syscall_enable(syscall_table).is_err() {
        user_boot_panic("user syscall enable failed\n");
    }
    if user_init_process
        .setup(kernel_init_task, address_space, elf, trap_frame, fs_struct)
        .is_err()
    {
        user_boot_panic("user init process setup failed\n");
    }
    if user_init_process
        .enable(trap_frame, exception_stream, syscall_table)
        .is_err()
    {
        user_boot_panic("user init process enable failed\n");
    }
    if payload
        .enable_for_user_entry(
            elf,
            address_space,
            trap_frame,
            exception_stream,
            syscall_table,
        )
        .is_err()
    {
        user_boot_panic("user payload enable failed\n");
    }
    if user_init_process.enter_user_mode(trap_frame).is_err() {
        user_boot_panic("user init process enter failed\n");
    }

    crate::checkpoint::dispatch(
        crate::trace::Checkpoint::UserModeEntry,
        crate::context::context_ref(),
    );
    unsafe {
        crate::arch::riscv64::csr::enter_user_mode(
            address_space.satp_token(),
            trap_frame.entry(),
            trap_frame.sp(),
            trap_frame.sstatus(),
            user_kernel_trap_stack_top(),
        )
    }
}

#[cfg(app_user_boot)]
fn read_user_init_image(
    vfs_core: &mut VfsCore,
    fs_struct: &FsStruct,
    ext2_filesystem: &mut Ext2FileSystem,
    block_device_registry: &mut BlockDeviceRegistry,
    kernel_image: &KernelImage,
) -> &'static [u8] {
    let mut provider = virtio_blk::live_provider(kernel_image);
    let buffer = unsafe {
        let ptr = core::ptr::addr_of_mut!(USER_BOOT_READ_BUFFER);
        &mut *ptr
    };
    buffer.fill(0);
    let Ok(len) = vfs_core.read_path(
        fs_struct,
        ext2_filesystem,
        block_device_registry,
        &mut provider,
        USER_INIT_PATH,
        buffer,
    ) else {
        user_boot_panic("read /sbin/init failed\n");
    };
    &buffer[..len]
}

#[cfg(app_user_boot)]
fn user_kernel_trap_stack_top() -> usize {
    unsafe {
        let base = core::ptr::addr_of!(USER_KERNEL_TRAP_STACK.bytes) as usize;
        base + USER_KERNEL_TRAP_STACK_SIZE
    }
}

#[cfg(app_user_boot)]
fn user_boot_panic(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
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

fn materialize_mapping(
    mapping: &mut UserMapping,
    image: &[u8],
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) -> Result<(), ElfError> {
    let page_count = pages_for_range(mapping.page_offset, mapping.memsz)?;
    if page_count > MAX_MAPPING_BACKING_PAGES {
        return Err(ElfError::TooManyMappingPages);
    }
    if mapping
        .file_offset()
        .checked_add(mapping.filesz())
        .filter(|end| *end <= image.len())
        .is_none()
    {
        return Err(ElfError::InvalidProgramHeader);
    }

    let mut index = 0usize;
    while index < page_count {
        let Some(page) = page_allocator.alloc_page(GfpFlags::kernel(), page_metadata_map) else {
            release_mapping_pages(mapping, page_allocator, page_metadata_map);
            return Err(ElfError::BackingAllocationFailed);
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            release_mapping_pages(mapping, page_allocator, page_metadata_map);
            return Err(ElfError::BackingAllocationFailed);
        };
        unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };
        mapping.backing_pages[index] = Some(page);
        mapping.backing_page_count += 1;
        index += 1;
    }

    if let Err(error) = copy_mapping_bytes(
        mapping,
        image,
        page_metadata_map,
        0,
        mapping.file_offset(),
        mapping.filesz(),
    ) {
        release_mapping_pages(mapping, page_allocator, page_metadata_map);
        return Err(error);
    }
    mapping.file_bytes_copied = mapping.filesz();
    mapping.bss_bytes_zeroed = mapping.bss_zero_bytes();
    mapping.page_table_entry_bound = true;
    Ok(())
}

fn release_mapping_pages(
    mapping: &mut UserMapping,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) {
    while mapping.backing_page_count > 0 {
        mapping.backing_page_count -= 1;
        if let Some(page) = mapping.backing_pages[mapping.backing_page_count] {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            mapping.backing_pages[mapping.backing_page_count] = None;
        }
    }
}

fn release_mappings(
    mappings: &mut [UserMapping; MAX_USER_MAPPINGS],
    count: usize,
    page_allocator: &mut PageAllocator,
    page_metadata_map: &PageMetadataMap,
) {
    let mut index = 0usize;
    while index < count {
        release_mapping_pages(&mut mappings[index], page_allocator, page_metadata_map);
        mappings[index] = UserMapping::empty();
        index += 1;
    }
}

fn copy_mapping_bytes(
    mapping: &UserMapping,
    image: &[u8],
    page_metadata_map: &PageMetadataMap,
    user_offset: usize,
    file_offset: usize,
    len: usize,
) -> Result<(), ElfError> {
    if file_offset
        .checked_add(len)
        .filter(|end| *end <= image.len())
        .is_none()
        || user_offset
            .checked_add(len)
            .filter(|end| *end <= mapping.memsz())
            .is_none()
    {
        return Err(ElfError::UserCopyOutOfRange);
    }

    let mut remaining = len;
    let mut copied = 0usize;
    while remaining > 0 {
        let absolute = mapping.page_offset() + user_offset + copied;
        let page_index = absolute / USER_PAGE_SIZE;
        let page_offset = absolute % USER_PAGE_SIZE;
        let chunk = min_usize(remaining, USER_PAGE_SIZE - page_offset);
        let Some(page) = mapping.backing_page(page_index) else {
            return Err(ElfError::BackingAllocationFailed);
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            return Err(ElfError::BackingAllocationFailed);
        };
        unsafe {
            core::ptr::copy_nonoverlapping(
                image.as_ptr().add(file_offset + copied),
                (linear + page_offset) as *mut u8,
                chunk,
            );
        }
        copied += chunk;
        remaining -= chunk;
    }
    Ok(())
}

fn pages_for_range(offset: usize, len: usize) -> Result<usize, ElfError> {
    if offset >= USER_PAGE_SIZE {
        return Err(ElfError::InvalidProgramHeader);
    }
    if len == 0 {
        return Ok(0);
    }
    let end = offset
        .checked_add(len)
        .ok_or(ElfError::InvalidProgramHeader)?;
    Ok((end + USER_PAGE_SIZE - 1) / USER_PAGE_SIZE)
}

fn mappings_have_backing_pages(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if mappings[index].backing_page_count() == 0 {
            return false;
        }
        index += 1;
    }
    true
}

fn mapping_file_bytes_match(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if mappings[index].kind() == UserMappingKind::ElfSegment
            && mappings[index].file_bytes_copied() != mappings[index].filesz()
        {
            return false;
        }
        index += 1;
    }
    true
}

fn mapping_bss_bytes_match(mappings: &[UserMapping; MAX_USER_MAPPINGS], count: usize) -> bool {
    let mut index = 0usize;
    while index < count {
        if mappings[index].kind() == UserMappingKind::ElfSegment
            && mappings[index].bss_bytes_zeroed() != mappings[index].bss_zero_bytes()
        {
            return false;
        }
        index += 1;
    }
    true
}

fn mappings_have_page_table_entries(
    mappings: &[UserMapping; MAX_USER_MAPPINGS],
    count: usize,
) -> bool {
    let mut index = 0usize;
    while index < count {
        if !mappings[index].page_table_entry_bound() {
            return false;
        }
        index += 1;
    }
    true
}

const fn min_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
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
