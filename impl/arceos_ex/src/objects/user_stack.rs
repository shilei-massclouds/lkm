use alloc::vec::Vec;

use super::{
    config::UserStackConfig,
    elf_object::ElfObject,
    mm_core::{GfpFlags, PageAllocator, PageMetadataMap, PageRef},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    user_boot::UserAddressSpace,
};

pub const USER_PAGE_SIZE: usize = 4096;
pub const USER_STACK_SIZE: usize = 128 * 1024;
#[allow(dead_code)]
pub const USER_STACK_TOP: usize = 0x4000_0000;
#[allow(dead_code)]
pub const USER_STACK_RLIMIT: usize = 8 * 1024 * 1024;
#[allow(dead_code)]
pub const USER_STACK_GUARD_GAP: usize = 256 * USER_PAGE_SIZE;
pub const USER_STACK_RANDOM_BYTES: usize = 16;

const AT_NULL: usize = 0;
const AT_PHDR: usize = 3;
const AT_PHENT: usize = 4;
const AT_PHNUM: usize = 5;
const AT_PAGESZ: usize = 6;
const AT_BASE: usize = 7;
const AT_ENTRY: usize = 9;
pub const AT_RANDOM: usize = 25;
const USER_INITIAL_AUXV_WORDS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserStackGrowReject {
    None,
    InvalidState,
    WrongAddressSpace,
    UnsupportedAccess,
    Permission,
    Rlimit,
    GuardGap,
    MappingCollision,
    BackingAllocation,
    PageTableAllocation,
    PteInstall,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct UserStackPage {
    vaddr: usize,
    page: PageRef,
}

#[allow(dead_code)]
impl UserStackPage {
    pub const fn vaddr(self) -> usize {
        self.vaddr
    }

    pub const fn page(self) -> PageRef {
        self.page
    }
}

pub struct UserStack {
    lifecycle: Lifecycle,
    base: usize,
    top: usize,
    initial_sp: usize,
    arg0_ptr: usize,
    random_ptr: usize,
    config: UserStackConfig,
    pages: Vec<UserStackPage>,
    allocated: bool,
    mapped_into_address_space: bool,
    zeroed: bool,
    initial_sp_bound: bool,
    minimal_arg_env_bound: bool,
    last_fault_address: usize,
    last_old_base: usize,
    last_new_base: usize,
    last_allocated_page_count: usize,
    last_grow_rejection: UserStackGrowReject,
    last_tlb_flush: bool,
    #[cfg(app_smoke)]
    fail_next_backing_allocation: bool,
}

#[allow(dead_code)]
impl UserStack {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            base: 0,
            top: 0,
            initial_sp: 0,
            arg0_ptr: 0,
            random_ptr: 0,
            config: UserStackConfig::linux_default(),
            pages: Vec::new(),
            allocated: false,
            mapped_into_address_space: false,
            zeroed: false,
            initial_sp_bound: false,
            minimal_arg_env_bound: false,
            last_fault_address: 0,
            last_old_base: 0,
            last_new_base: 0,
            last_allocated_page_count: 0,
            last_grow_rejection: UserStackGrowReject::None,
            last_tlb_flush: false,
            #[cfg(app_smoke)]
            fail_next_backing_allocation: false,
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
        self.top.saturating_sub(self.base)
    }

    pub const fn rlimit_base(&self) -> usize {
        self.top.saturating_sub(self.config.rlimit_stack())
    }

    pub const fn initial_sp(&self) -> usize {
        self.initial_sp
    }

    pub const fn arg0_ptr(&self) -> usize {
        self.arg0_ptr
    }

    pub const fn random_ptr(&self) -> usize {
        self.random_ptr
    }

    pub const fn config(&self) -> UserStackConfig {
        self.config
    }

    pub const fn allocated(&self) -> bool {
        self.allocated
    }

    pub fn backing_page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn backing_page(&self, index: usize) -> Option<PageRef> {
        self.pages.get(index).map(|slot| slot.page)
    }

    pub fn backing_page_vaddr(&self, index: usize) -> Option<usize> {
        self.pages.get(index).map(|slot| slot.vaddr)
    }

    pub fn page_for_vaddr(&self, vaddr: usize) -> Option<PageRef> {
        let page_vaddr = align_down(vaddr, USER_PAGE_SIZE);
        self.pages
            .binary_search_by_key(&page_vaddr, |slot| slot.vaddr)
            .ok()
            .and_then(|index| self.pages.get(index))
            .map(|slot| slot.page)
    }

    pub const fn fixed_size_bound(&self) -> bool {
        false
    }

    pub const fn mapped_into_address_space(&self) -> bool {
        self.mapped_into_address_space
    }

    pub fn backing_pages_allocated(&self) -> bool {
        !self.pages.is_empty()
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

    pub const fn last_fault_address(&self) -> usize {
        self.last_fault_address
    }

    pub const fn last_old_base(&self) -> usize {
        self.last_old_base
    }

    pub const fn last_new_base(&self) -> usize {
        self.last_new_base
    }

    pub const fn last_allocated_page_count(&self) -> usize {
        self.last_allocated_page_count
    }

    pub const fn last_grow_rejection(&self) -> UserStackGrowReject {
        self.last_grow_rejection
    }

    pub const fn last_tlb_flush(&self) -> bool {
        self.last_tlb_flush
    }

    pub fn release_exec_backing(
        &mut self,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> usize {
        let released = self.pages.len();
        while let Some(slot) = self.pages.pop() {
            let _ = page_allocator.free_pages(slot.page, 0, page_metadata_map);
        }
        *self = Self::new();
        released
    }

    pub fn reset_staging_after_exec_commit(&mut self) {
        *self = Self::new();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn setup(
        &mut self,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        interpreter: Option<&ElfObject>,
        argv: &[&[u8]],
        config: UserStackConfig,
        random: &[u8; USER_STACK_RANDOM_BYTES],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> EventResult {
        self.setup_with_envp(
            address_space,
            elf,
            interpreter,
            argv,
            &[],
            config,
            random,
            page_allocator,
            page_metadata_map,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn setup_with_envp(
        &mut self,
        address_space: &UserAddressSpace,
        elf: &ElfObject,
        interpreter: Option<&ElfObject>,
        argv: &[&[u8]],
        envp: &[&[u8]],
        config: UserStackConfig,
        random: &[u8; USER_STACK_RANDOM_BYTES],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || address_space.state() != State::Prepared
            || page_allocator.state() != State::Ready
            || elf.state() != State::Ready
            || interpreter.is_some_and(|interp| interp.state() != State::Ready)
            || argv.is_empty()
            || config.random_bytes() != USER_STACK_RANDOM_BYTES
            || config.stack_top() <= config.rlimit_stack()
            || !config.stack_top().is_multiple_of(USER_PAGE_SIZE)
        {
            return setup_failed(self.lifecycle.state());
        }

        self.config = config;
        self.top = config.stack_top();
        self.base = self.top - config.rlimit_stack();
        self.pages.clear();
        let Some(initial_sp) = self.write_initial_arg_env(
            elf,
            interpreter,
            argv,
            envp,
            random,
            page_allocator,
            page_metadata_map,
        ) else {
            self.release_exec_backing(page_allocator, page_metadata_map);
            return setup_failed(State::Base);
        };
        let expanded = align_down(
            initial_sp.saturating_sub(config.initial_expand()),
            USER_PAGE_SIZE,
        );
        self.base = core::cmp::max(expanded, self.top - config.rlimit_stack());
        self.initial_sp = initial_sp;
        self.allocated = true;
        self.mapped_into_address_space = true;
        self.zeroed = true;
        self.initial_sp_bound = true;
        self.minimal_arg_env_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_initial_arg_env(
        &mut self,
        elf: &ElfObject,
        interpreter: Option<&ElfObject>,
        argv: &[&[u8]],
        envp: &[&[u8]],
        random: &[u8; USER_STACK_RANDOM_BYTES],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<usize> {
        let mut argv_ptrs = Vec::new();
        let mut envp_ptrs = Vec::new();
        argv_ptrs.try_reserve_exact(argv.len()).ok()?;
        envp_ptrs.try_reserve_exact(envp.len()).ok()?;
        argv_ptrs.resize(argv.len(), 0);
        envp_ptrs.resize(envp.len(), 0);

        let mut string_top = self.top;
        let mut index = argv.len();
        while index > 0 {
            index -= 1;
            let arg = argv[index];
            let arg_ptr = align_down(string_top.checked_sub(arg.len().checked_add(1)?)?, 8);
            self.write_bytes_allocating(arg_ptr, arg, page_allocator, page_metadata_map)?;
            self.write_bytes_allocating(
                arg_ptr + arg.len(),
                &[0],
                page_allocator,
                page_metadata_map,
            )?;
            argv_ptrs[index] = arg_ptr;
            string_top = arg_ptr;
        }
        let mut index = envp.len();
        while index > 0 {
            index -= 1;
            let env = envp[index];
            let env_ptr = align_down(string_top.checked_sub(env.len().checked_add(1)?)?, 8);
            self.write_bytes_allocating(env_ptr, env, page_allocator, page_metadata_map)?;
            self.write_bytes_allocating(
                env_ptr + env.len(),
                &[0],
                page_allocator,
                page_metadata_map,
            )?;
            envp_ptrs[index] = env_ptr;
            string_top = env_ptr;
        }

        self.random_ptr = align_down(string_top.checked_sub(random.len())?, 16);
        self.write_bytes_allocating(self.random_ptr, random, page_allocator, page_metadata_map)?;
        string_top = self.random_ptr;

        let word_count = 1usize
            .checked_add(argv.len())?
            .checked_add(1)?
            .checked_add(envp.len())?
            .checked_add(1)?
            .checked_add(USER_INITIAL_AUXV_WORDS)?;
        let initial_sp = align_down(
            string_top.checked_sub(word_count.checked_mul(core::mem::size_of::<usize>())?)?,
            16,
        );
        if initial_sp < self.rlimit_base() {
            return None;
        }

        let word = core::mem::size_of::<usize>();
        let mut word_index = 0usize;
        self.write_usize_allocating(initial_sp, argv.len(), page_allocator, page_metadata_map)?;
        word_index += 1;
        for ptr in &argv_ptrs {
            self.write_usize_allocating(
                initial_sp + word_index * word,
                *ptr,
                page_allocator,
                page_metadata_map,
            )?;
            word_index += 1;
        }
        self.write_usize_allocating(
            initial_sp + word_index * word,
            0,
            page_allocator,
            page_metadata_map,
        )?;
        word_index += 1;
        for ptr in &envp_ptrs {
            self.write_usize_allocating(
                initial_sp + word_index * word,
                *ptr,
                page_allocator,
                page_metadata_map,
            )?;
            word_index += 1;
        }
        self.write_usize_allocating(
            initial_sp + word_index * word,
            0,
            page_allocator,
            page_metadata_map,
        )?;
        word_index += 1;

        let auxv = [
            (AT_PHDR, elf.phdr_vaddr()),
            (AT_PHENT, elf.phentsize()),
            (AT_PHNUM, elf.program_header_count()),
            (AT_ENTRY, elf.entry()),
            (AT_BASE, interpreter.map_or(0, ElfObject::load_bias)),
            (AT_PAGESZ, USER_PAGE_SIZE),
            (AT_RANDOM, self.random_ptr),
            (AT_NULL, 0),
        ];
        for (key, value) in auxv {
            self.write_usize_allocating(
                initial_sp + word_index * word,
                key,
                page_allocator,
                page_metadata_map,
            )?;
            word_index += 1;
            self.write_usize_allocating(
                initial_sp + word_index * word,
                value,
                page_allocator,
                page_metadata_map,
            )?;
            word_index += 1;
        }
        self.arg0_ptr = argv_ptrs[0];
        Some(initial_sp)
    }

    fn write_usize_allocating(
        &mut self,
        addr: usize,
        value: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<()> {
        self.write_bytes_allocating(
            addr,
            &value.to_ne_bytes(),
            page_allocator,
            page_metadata_map,
        )
    }

    fn write_bytes_allocating(
        &mut self,
        addr: usize,
        bytes: &[u8],
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Option<()> {
        if addr < self.rlimit_base()
            || addr
                .checked_add(bytes.len())
                .filter(|end| *end <= self.top)
                .is_none()
        {
            return None;
        }
        let mut copied = 0usize;
        while copied < bytes.len() {
            let current = addr + copied;
            let page_vaddr = align_down(current, USER_PAGE_SIZE);
            let page = match self.page_for_vaddr(page_vaddr) {
                Some(page) => page,
                None => self
                    .allocate_page(page_vaddr, page_allocator, page_metadata_map)
                    .ok()?,
            };
            let offset = current - page_vaddr;
            let len = core::cmp::min(bytes.len() - copied, USER_PAGE_SIZE - offset);
            let linear = page_metadata_map.page_address(page)?;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    bytes.as_ptr().add(copied),
                    (linear + offset) as *mut u8,
                    len,
                );
            }
            copied += len;
        }
        Some(())
    }

    pub(crate) fn allocate_page(
        &mut self,
        page_vaddr: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) -> Result<PageRef, UserStackGrowReject> {
        if !page_vaddr.is_multiple_of(USER_PAGE_SIZE)
            || page_vaddr < self.rlimit_base()
            || page_vaddr >= self.top
        {
            return Err(UserStackGrowReject::Rlimit);
        }
        if self.page_for_vaddr(page_vaddr).is_some() {
            return Err(UserStackGrowReject::Permission);
        }
        #[cfg(app_smoke)]
        if self.fail_next_backing_allocation {
            self.fail_next_backing_allocation = false;
            return Err(UserStackGrowReject::BackingAllocation);
        }
        self.pages
            .try_reserve(1)
            .map_err(|_| UserStackGrowReject::BackingAllocation)?;
        let page = page_allocator
            .alloc_page(GfpFlags::kernel(), page_metadata_map)
            .ok_or(UserStackGrowReject::BackingAllocation)?;
        let Some(linear) = page_metadata_map.page_address(page) else {
            let _ = page_allocator.free_pages(page, 0, page_metadata_map);
            return Err(UserStackGrowReject::BackingAllocation);
        };
        unsafe { core::ptr::write_bytes(linear as *mut u8, 0, USER_PAGE_SIZE) };
        let index = self
            .pages
            .binary_search_by_key(&page_vaddr, |slot| slot.vaddr)
            .unwrap_or_else(|index| index);
        self.pages.insert(
            index,
            UserStackPage {
                vaddr: page_vaddr,
                page,
            },
        );
        Ok(page)
    }

    #[cfg(app_smoke)]
    pub fn smoke_fail_next_backing_allocation(&mut self) {
        self.fail_next_backing_allocation = true;
    }

    pub(crate) fn rollback_page(
        &mut self,
        page_vaddr: usize,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
    ) {
        if let Ok(index) = self
            .pages
            .binary_search_by_key(&page_vaddr, |slot| slot.vaddr)
        {
            let slot = self.pages.remove(index);
            let _ = page_allocator.free_pages(slot.page, 0, page_metadata_map);
        }
    }

    pub(crate) fn commit_vma_base(&mut self, base: usize) {
        self.base = base;
    }

    pub(crate) fn record_grow_complete(
        &mut self,
        fault: usize,
        old_base: usize,
        new_base: usize,
        tlb_flush: bool,
    ) {
        self.last_fault_address = fault;
        self.last_old_base = old_base;
        self.last_new_base = new_base;
        self.last_allocated_page_count = self.pages.len();
        self.last_grow_rejection = UserStackGrowReject::None;
        self.last_tlb_flush = tlb_flush;
    }

    pub(crate) fn record_grow_rejected(
        &mut self,
        fault: usize,
        old_base: usize,
        reason: UserStackGrowReject,
    ) {
        self.last_fault_address = fault;
        self.last_old_base = old_base;
        self.last_new_base = old_base;
        self.last_allocated_page_count = self.pages.len();
        self.last_grow_rejection = reason;
        self.last_tlb_flush = false;
    }

    pub fn read_byte(&self, page_metadata_map: &PageMetadataMap, addr: usize) -> Option<u8> {
        if addr < self.base || addr >= self.top {
            return None;
        }
        let page_vaddr = align_down(addr, USER_PAGE_SIZE);
        let page = self.page_for_vaddr(page_vaddr)?;
        let linear = page_metadata_map.page_address(page)?;
        Some(unsafe { *((linear + addr - page_vaddr) as *const u8) })
    }

    pub fn write_existing_byte(
        &self,
        page_metadata_map: &PageMetadataMap,
        addr: usize,
        value: u8,
    ) -> bool {
        if addr < self.base || addr >= self.top {
            return false;
        }
        let page_vaddr = align_down(addr, USER_PAGE_SIZE);
        let Some(page) = self.page_for_vaddr(page_vaddr) else {
            return value == 0;
        };
        let Some(linear) = page_metadata_map.page_address(page) else {
            return false;
        };
        unsafe { *((linear + addr - page_vaddr) as *mut u8) = value };
        true
    }
}

fn setup_failed(state: State) -> EventResult {
    failed_condition(LifecycleEvent::Setup, state, State::Base, State::Ready)
}

const fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}
