use super::{
    bio::{self, BufferHead, BUFFER_HEAD_MAX_SIZE},
    block_device::{BlockDeviceProvider, BlockDeviceRegistry, DevT},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vfs::{DentryRef, MountRef, VfsCore},
};
use core::cmp::min;

pub const EXT2_SUPER_MAGIC: u16 = 0xef53;
pub const EXT2_ROOT_INO: u32 = 2;
pub const EXT2_MIN_BLOCK_SIZE: usize = 1024;
pub const EXT2_MAX_BLOCK_SIZE: usize = 4096;
pub const EXT2_SUPERBLOCK_OFFSET: usize = 1024;
const EXT2_SUPERBLOCK_PROBE_BLOCK: u64 = (EXT2_SUPERBLOCK_OFFSET / EXT2_MIN_BLOCK_SIZE) as u64;
pub const EXT2_N_BLOCKS: usize = 15;
pub const EXT2_NDIR_BLOCKS: usize = 12;
pub const EXT2_SINGLE_INDIRECT_INDEX: usize = EXT2_NDIR_BLOCKS;
pub const EXT2_ALPINE_RELEASE_PATH: &[u8] = b"/etc/alpine-release";
pub const EXT2_ALPINE_RELEASE_FILE_NAME: &[u8] = b"alpine-release";
pub const EXT2_ALPINE_RELEASE_FILE_CONTENT: &[u8] = b"3.24.1\n";
pub const EXT2_ALPINE_INSTALLED_DB_PATH: &[u8] = b"/lib/apk/db/installed";
pub const EXT2_ALPINE_INSTALLED_DB_FILE_NAME: &[u8] = b"installed";
pub const EXT2_SINGLE_INDIRECT_READ_MAX: usize =
    EXT2_MAX_BLOCK_SIZE * (EXT2_NDIR_BLOCKS + EXT2_MAX_BLOCK_SIZE / core::mem::size_of::<u32>());
pub const EXT2_ALPINE_INSTALLED_DB_MAX_SIZE: usize = EXT2_MAX_BLOCK_SIZE * EXT2_NDIR_BLOCKS;

const EXT2_NAME_MAX: usize = 32;
const EXT2_GOOD_OLD_INODE_SIZE: u16 = 128;
const EXT2_S_IFMT: u16 = 0xf000;
const EXT2_S_IFDIR: u16 = 0x4000;
const EXT2_S_IFREG: u16 = 0x8000;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Ext2Error {
    DriverNotReady,
    VolumeNotReady,
    FileSystemNotReady,
    InvalidState,
    DeviceMissing,
    Io,
    InvalidSuperblock,
    UnsupportedBlockSize,
    InvalidGroupDesc,
    InvalidInode,
    InvalidDirEntry,
    NotFound,
    NotDirectory,
    NotRegularFile,
    IndirectBlocksUnsupported,
    ShortBuffer,
}

impl From<bio::BlockIoError> for Ext2Error {
    fn from(error: bio::BlockIoError) -> Self {
        match error {
            bio::BlockIoError::DeviceMissing => Self::DeviceMissing,
            _ => Self::Io,
        }
    }
}

pub struct Ext2Driver {
    lifecycle: Lifecycle,
    registered: bool,
    read_only: bool,
    mount_callback_bound: bool,
    super_operations_bound: bool,
    inode_operations_bound: bool,
    file_operations_bound: bool,
}

#[allow(dead_code)]
impl Ext2Driver {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            registered: false,
            read_only: false,
            mount_callback_bound: false,
            super_operations_bound: false,
            inode_operations_bound: false,
            file_operations_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn registered(&self) -> bool {
        self.registered
    }

    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    pub const fn mount_callback_bound(&self) -> bool {
        self.mount_callback_bound
    }

    pub const fn super_operations_bound(&self) -> bool {
        self.super_operations_bound
    }

    pub const fn inode_operations_bound(&self) -> bool {
        self.inode_operations_bound
    }

    pub const fn file_operations_bound(&self) -> bool {
        self.file_operations_bound
    }

    pub fn setup(&mut self, registry: &BlockDeviceRegistry) -> EventResult {
        if self.lifecycle.state() != State::Base || registry.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.registered = true;
        self.read_only = true;
        self.mount_callback_bound = true;
        self.super_operations_bound = true;
        self.inode_operations_bound = true;
        self.file_operations_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct Ext2Volume {
    lifecycle: Lifecycle,
    devt: Option<DevT>,
    block_size: usize,
    inodes_count: u32,
    blocks_count: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    inode_size: u16,
    first_inode: u32,
    superblock_read: bool,
    magic_valid: bool,
    layout_valid: bool,
    block_size_supported: bool,
    not_found_nonfatal: bool,
}

#[allow(dead_code)]
impl Ext2Volume {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            devt: None,
            block_size: 0,
            inodes_count: 0,
            blocks_count: 0,
            blocks_per_group: 0,
            inodes_per_group: 0,
            inode_size: 0,
            first_inode: 0,
            superblock_read: false,
            magic_valid: false,
            layout_valid: false,
            block_size_supported: false,
            not_found_nonfatal: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn devt(&self) -> Option<DevT> {
        self.devt
    }

    pub const fn block_size(&self) -> usize {
        self.block_size
    }

    pub const fn inodes_count(&self) -> u32 {
        self.inodes_count
    }

    pub const fn blocks_count(&self) -> u32 {
        self.blocks_count
    }

    pub const fn blocks_per_group(&self) -> u32 {
        self.blocks_per_group
    }

    pub const fn inodes_per_group(&self) -> u32 {
        self.inodes_per_group
    }

    pub const fn inode_size(&self) -> u16 {
        self.inode_size
    }

    pub const fn first_inode(&self) -> u32 {
        self.first_inode
    }

    pub const fn superblock_read(&self) -> bool {
        self.superblock_read
    }

    pub const fn magic_valid(&self) -> bool {
        self.magic_valid
    }

    pub const fn layout_valid(&self) -> bool {
        self.layout_valid
    }

    pub const fn block_size_supported(&self) -> bool {
        self.block_size_supported
    }

    pub const fn not_found_nonfatal(&self) -> bool {
        self.not_found_nonfatal
    }

    pub fn preset_default<P: BlockDeviceProvider>(
        &mut self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
    ) -> Result<(), Ext2Error> {
        if self.lifecycle.state() != State::Base {
            return Err(Ext2Error::InvalidState);
        }
        let Some(default_entry) = registry.default_entry() else {
            return Err(Ext2Error::DeviceMissing);
        };
        let devt = default_entry.devt();

        let super_bh = read_superblock_probe(registry, provider, devt)?;
        let superblock = parse_superblock(super_bh.data())?;
        validate_superblock(&superblock)?;

        self.devt = Some(devt);
        self.block_size = superblock.block_size;
        self.inodes_count = superblock.inodes_count;
        self.blocks_count = superblock.blocks_count;
        self.blocks_per_group = superblock.blocks_per_group;
        self.inodes_per_group = superblock.inodes_per_group;
        self.inode_size = superblock.inode_size;
        self.first_inode = superblock.first_inode;
        self.superblock_read = true;
        self.magic_valid = true;
        self.layout_valid = true;
        self.block_size_supported = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Ready)
            .map_err(|_| Ext2Error::InvalidState)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Ext2FileType {
    Unknown,
    RegularFile,
    Directory,
    Other,
}

#[derive(Clone, Copy)]
pub struct Ext2InodeRecord {
    ino: u32,
    mode: u16,
    size: u32,
    direct_blocks: [u32; EXT2_NDIR_BLOCKS],
    single_indirect_block: u32,
    indirect_blocks_deferred: bool,
}

#[allow(dead_code)]
impl Ext2InodeRecord {
    pub const fn empty() -> Self {
        Self {
            ino: 0,
            mode: 0,
            size: 0,
            direct_blocks: [0; EXT2_NDIR_BLOCKS],
            single_indirect_block: 0,
            indirect_blocks_deferred: false,
        }
    }

    pub const fn ino(&self) -> u32 {
        self.ino
    }

    pub const fn mode(&self) -> u16 {
        self.mode
    }

    pub const fn size(&self) -> u32 {
        self.size
    }

    pub const fn direct_blocks(&self) -> &[u32; EXT2_NDIR_BLOCKS] {
        &self.direct_blocks
    }

    pub const fn single_indirect_block(&self) -> u32 {
        self.single_indirect_block
    }

    pub const fn is_root_dir(&self) -> bool {
        self.ino == EXT2_ROOT_INO && self.is_dir()
    }

    pub const fn is_dir(&self) -> bool {
        self.mode & EXT2_S_IFMT == EXT2_S_IFDIR
    }

    pub const fn is_regular_file(&self) -> bool {
        self.mode & EXT2_S_IFMT == EXT2_S_IFREG
    }

    pub const fn indirect_blocks_deferred(&self) -> bool {
        self.indirect_blocks_deferred
    }
}

#[derive(Clone, Copy)]
pub struct Ext2DirEntryRecord {
    inode: u32,
    rec_len: u16,
    name_len: u8,
    file_type: Ext2FileType,
    name: [u8; EXT2_NAME_MAX],
}

#[allow(dead_code)]
impl Ext2DirEntryRecord {
    pub const fn empty() -> Self {
        Self {
            inode: 0,
            rec_len: 0,
            name_len: 0,
            file_type: Ext2FileType::Unknown,
            name: [0; EXT2_NAME_MAX],
        }
    }

    pub const fn inode(&self) -> u32 {
        self.inode
    }

    pub const fn rec_len(&self) -> u16 {
        self.rec_len
    }

    pub const fn name_len(&self) -> u8 {
        self.name_len
    }

    pub const fn file_type(&self) -> Ext2FileType {
        self.file_type
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

pub struct Ext2FileSystem {
    lifecycle: Lifecycle,
    devt: Option<DevT>,
    block_size: usize,
    inodes_count: u32,
    blocks_count: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    inode_size: u16,
    first_inode: u32,
    group_inode_table_block: u32,
    superblock_read: bool,
    magic_valid: bool,
    block_size_supported: bool,
    group_desc_read: bool,
    ready: bool,
    mount_boundary_recorded: bool,
    vfs_mount_ref: Option<MountRef>,
    vfs_mount_point_ref: Option<DentryRef>,
    vfs_lookup_entry_bound: bool,
    vfs_read_entry_bound: bool,
    operations_bound: bool,
    root_dentry_bound: bool,
    page_cache_deferred: bool,
    write_paths_deferred: bool,
    root_inode: Ext2InodeRecord,
    lookup_dirent: Ext2DirEntryRecord,
    lookup_file_inode: Ext2InodeRecord,
    lookup_name_bound: bool,
    lookup_reads_dir: bool,
    lookup_direct_blocks_scanned: usize,
    lookup_multi_direct_block_supported: bool,
    lookup_not_found_nonfatal: bool,
    lookup_indirect_blocks_deferred: bool,
    lookup_dirent_valid: bool,
    lookup_returns_inode: bool,
    last_file_read_len: usize,
    file_read_uses_direct_block: bool,
    file_read_direct_blocks_scanned: usize,
    file_read_multi_direct_block_supported: bool,
    file_read_uses_buffer_head: bool,
    file_read_copies_to_caller: bool,
    file_read_len_matches_inode_size: bool,
    file_read_short_buffer_rejected: bool,
    file_read_indirect_blocks_deferred: bool,
    file_read_entered_from_vfs: bool,
}

#[allow(dead_code)]
impl Ext2FileSystem {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            devt: None,
            block_size: 0,
            inodes_count: 0,
            blocks_count: 0,
            blocks_per_group: 0,
            inodes_per_group: 0,
            inode_size: 0,
            first_inode: 0,
            group_inode_table_block: 0,
            superblock_read: false,
            magic_valid: false,
            block_size_supported: false,
            group_desc_read: false,
            ready: false,
            mount_boundary_recorded: false,
            vfs_mount_ref: None,
            vfs_mount_point_ref: None,
            vfs_lookup_entry_bound: false,
            vfs_read_entry_bound: false,
            operations_bound: false,
            root_dentry_bound: false,
            page_cache_deferred: true,
            write_paths_deferred: true,
            root_inode: Ext2InodeRecord::empty(),
            lookup_dirent: Ext2DirEntryRecord::empty(),
            lookup_file_inode: Ext2InodeRecord::empty(),
            lookup_name_bound: false,
            lookup_reads_dir: false,
            lookup_direct_blocks_scanned: 0,
            lookup_multi_direct_block_supported: false,
            lookup_not_found_nonfatal: true,
            lookup_indirect_blocks_deferred: true,
            lookup_dirent_valid: false,
            lookup_returns_inode: false,
            last_file_read_len: 0,
            file_read_uses_direct_block: false,
            file_read_direct_blocks_scanned: 0,
            file_read_multi_direct_block_supported: false,
            file_read_uses_buffer_head: false,
            file_read_copies_to_caller: false,
            file_read_len_matches_inode_size: false,
            file_read_short_buffer_rejected: false,
            file_read_indirect_blocks_deferred: true,
            file_read_entered_from_vfs: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn devt(&self) -> Option<DevT> {
        self.devt
    }

    pub const fn block_size(&self) -> usize {
        self.block_size
    }

    pub const fn inodes_count(&self) -> u32 {
        self.inodes_count
    }

    pub const fn blocks_count(&self) -> u32 {
        self.blocks_count
    }

    pub const fn blocks_per_group(&self) -> u32 {
        self.blocks_per_group
    }

    pub const fn inodes_per_group(&self) -> u32 {
        self.inodes_per_group
    }

    pub const fn inode_size(&self) -> u16 {
        self.inode_size
    }

    pub const fn first_inode(&self) -> u32 {
        self.first_inode
    }

    pub const fn group_inode_table_block(&self) -> u32 {
        self.group_inode_table_block
    }

    pub const fn superblock_read(&self) -> bool {
        self.superblock_read
    }

    pub const fn magic_valid(&self) -> bool {
        self.magic_valid
    }

    pub const fn block_size_supported(&self) -> bool {
        self.block_size_supported
    }

    pub const fn group_desc_read(&self) -> bool {
        self.group_desc_read
    }

    pub const fn ready(&self) -> bool {
        self.ready
    }

    pub const fn mount_boundary_recorded(&self) -> bool {
        self.mount_boundary_recorded
    }

    pub const fn vfs_mount_ref(&self) -> Option<MountRef> {
        self.vfs_mount_ref
    }

    pub const fn vfs_mount_point_ref(&self) -> Option<DentryRef> {
        self.vfs_mount_point_ref
    }

    pub const fn vfs_lookup_entry_bound(&self) -> bool {
        self.vfs_lookup_entry_bound
    }

    pub const fn vfs_read_entry_bound(&self) -> bool {
        self.vfs_read_entry_bound
    }

    pub const fn operations_bound(&self) -> bool {
        self.operations_bound
    }

    pub const fn root_dentry_bound(&self) -> bool {
        self.root_dentry_bound
    }

    pub const fn page_cache_deferred(&self) -> bool {
        self.page_cache_deferred
    }

    pub const fn write_paths_deferred(&self) -> bool {
        self.write_paths_deferred
    }

    pub const fn root_inode(&self) -> &Ext2InodeRecord {
        &self.root_inode
    }

    pub const fn lookup_dirent(&self) -> &Ext2DirEntryRecord {
        &self.lookup_dirent
    }

    pub const fn lookup_file_inode(&self) -> &Ext2InodeRecord {
        &self.lookup_file_inode
    }

    pub const fn lookup_name_bound(&self) -> bool {
        self.lookup_name_bound
    }

    pub const fn lookup_reads_dir(&self) -> bool {
        self.lookup_reads_dir
    }

    pub const fn lookup_direct_blocks_scanned(&self) -> usize {
        self.lookup_direct_blocks_scanned
    }

    pub const fn lookup_multi_direct_block_supported(&self) -> bool {
        self.lookup_multi_direct_block_supported
    }

    pub const fn lookup_not_found_nonfatal(&self) -> bool {
        self.lookup_not_found_nonfatal
    }

    pub const fn lookup_indirect_blocks_deferred(&self) -> bool {
        self.lookup_indirect_blocks_deferred
    }

    pub const fn lookup_dirent_valid(&self) -> bool {
        self.lookup_dirent_valid
    }

    pub const fn lookup_returns_inode(&self) -> bool {
        self.lookup_returns_inode
    }

    pub const fn last_file_read_len(&self) -> usize {
        self.last_file_read_len
    }

    pub const fn file_read_uses_direct_block(&self) -> bool {
        self.file_read_uses_direct_block
    }

    pub const fn file_read_direct_blocks_scanned(&self) -> usize {
        self.file_read_direct_blocks_scanned
    }

    pub const fn file_read_multi_direct_block_supported(&self) -> bool {
        self.file_read_multi_direct_block_supported
    }

    pub const fn file_read_uses_buffer_head(&self) -> bool {
        self.file_read_uses_buffer_head
    }

    pub const fn file_read_copies_to_caller(&self) -> bool {
        self.file_read_copies_to_caller
    }

    pub const fn file_read_len_matches_inode_size(&self) -> bool {
        self.file_read_len_matches_inode_size
    }

    pub const fn file_read_short_buffer_rejected(&self) -> bool {
        self.file_read_short_buffer_rejected
    }

    pub const fn file_read_indirect_blocks_deferred(&self) -> bool {
        self.file_read_indirect_blocks_deferred
    }

    pub const fn file_read_entered_from_vfs(&self) -> bool {
        self.file_read_entered_from_vfs
    }

    pub fn preset(&mut self, driver: &Ext2Driver, volume: &Ext2Volume) -> Result<(), Ext2Error> {
        if driver.state() != State::Ready || !driver.mount_callback_bound() {
            return Err(Ext2Error::DriverNotReady);
        }
        if volume.state() != State::Ready {
            return Err(Ext2Error::VolumeNotReady);
        }
        if self.lifecycle.state() != State::Base {
            return Err(Ext2Error::InvalidState);
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
            .map_err(|_| Ext2Error::InvalidState)
    }

    pub fn setup<P: BlockDeviceProvider>(
        &mut self,
        driver: &Ext2Driver,
        volume: &Ext2Volume,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
    ) -> Result<(), Ext2Error> {
        if driver.state() != State::Ready
            || !driver.super_operations_bound()
            || !driver.inode_operations_bound()
            || !driver.file_operations_bound()
        {
            return Err(Ext2Error::DriverNotReady);
        }
        if volume.state() != State::Ready {
            return Err(Ext2Error::VolumeNotReady);
        }
        if self.lifecycle.state() != State::Prepared {
            return Err(Ext2Error::InvalidState);
        }
        let Some(devt) = volume.devt() else {
            return Err(Ext2Error::DeviceMissing);
        };

        self.devt = Some(devt);
        self.block_size = volume.block_size();
        self.inodes_count = volume.inodes_count();
        self.blocks_count = volume.blocks_count();
        self.blocks_per_group = volume.blocks_per_group();
        self.inodes_per_group = volume.inodes_per_group();
        self.inode_size = volume.inode_size();
        self.first_inode = volume.first_inode();
        self.superblock_read = volume.superblock_read();
        self.magic_valid = volume.magic_valid();
        self.block_size_supported = volume.block_size_supported();
        self.operations_bound = true;

        let group_desc_block = group_descriptor_block(self.block_size);
        let group_bh = read_fs_block(registry, provider, devt, group_desc_block, self.block_size)?;
        let group_desc = parse_group_desc(group_bh.data())?;
        if group_desc.inode_table_block == 0 {
            return Err(Ext2Error::InvalidGroupDesc);
        }
        self.group_inode_table_block = group_desc.inode_table_block;
        self.group_desc_read = true;

        self.root_inode = self.read_inode(registry, provider, EXT2_ROOT_INO)?;
        if !self.root_inode.is_root_dir() {
            return Err(Ext2Error::NotDirectory);
        }
        self.root_dentry_bound = true;
        self.ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .map_err(|_| Ext2Error::InvalidState)
    }

    pub fn enable(
        &mut self,
        vfs_core: &mut VfsCore,
        mount_point_ref: DentryRef,
    ) -> Result<MountRef, Ext2Error> {
        if self.lifecycle.state() != State::Ready || !self.ready || !self.root_dentry_bound {
            return Err(Ext2Error::FileSystemNotReady);
        }
        if vfs_core.state() != State::Ready {
            return Err(Ext2Error::FileSystemNotReady);
        }

        let mount_ref = vfs_core
            .mount_ext2_at(self, mount_point_ref)
            .map_err(|_| Ext2Error::InvalidState)?;
        self.mount_boundary_recorded = true;
        self.vfs_mount_ref = Some(mount_ref);
        self.vfs_mount_point_ref = Some(mount_point_ref);
        self.vfs_lookup_entry_bound = true;
        self.vfs_read_entry_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
            .map_err(|_| Ext2Error::InvalidState)?;
        Ok(mount_ref)
    }

    pub fn lookup_root<P: BlockDeviceProvider>(
        &mut self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        name: &[u8],
    ) -> Result<Ext2DirEntryRecord, Ext2Error> {
        let root_inode = self.root_inode;
        self.lookup_child(registry, provider, &root_inode, name)
    }

    pub fn lookup_child<P: BlockDeviceProvider>(
        &mut self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        parent_inode: &Ext2InodeRecord,
        name: &[u8],
    ) -> Result<Ext2DirEntryRecord, Ext2Error> {
        if self.lifecycle.state() != State::Online || !self.ready || !self.mount_boundary_recorded {
            return Err(Ext2Error::FileSystemNotReady);
        }
        if !parent_inode.is_dir() {
            return Err(Ext2Error::NotDirectory);
        }
        if name.is_empty() || name.len() > EXT2_NAME_MAX {
            return Err(Ext2Error::InvalidDirEntry);
        }
        let Some(devt) = self.devt else {
            return Err(Ext2Error::DeviceMissing);
        };

        self.lookup_name_bound = false;
        self.lookup_reads_dir = true;
        self.lookup_direct_blocks_scanned = 0;
        self.lookup_multi_direct_block_supported = true;
        self.lookup_not_found_nonfatal = true;
        self.lookup_indirect_blocks_deferred = parent_inode.indirect_blocks_deferred();
        self.lookup_dirent_valid = false;
        self.lookup_returns_inode = false;
        let mut dir_bytes_remaining =
            usize::try_from(parent_inode.size()).map_err(|_| Ext2Error::InvalidDirEntry)?;
        for block in parent_inode.direct_blocks {
            if dir_bytes_remaining == 0 {
                break;
            }
            if block == 0 {
                continue;
            }
            let bh = read_fs_block(registry, provider, devt, block, self.block_size)?;
            self.lookup_direct_blocks_scanned += 1;
            let dir_block_len = min(dir_bytes_remaining, bh.data().len());
            if let Some(dirent) = find_dirent(&bh.data()[..dir_block_len], name)? {
                self.lookup_name_bound = true;
                self.lookup_dirent_valid = true;
                self.lookup_dirent = dirent;
                self.lookup_file_inode = self.read_inode(registry, provider, dirent.inode())?;
                self.lookup_returns_inode = true;
                if !self.lookup_file_inode.is_regular_file() && !self.lookup_file_inode.is_dir() {
                    return Err(Ext2Error::InvalidInode);
                }
                return Ok(dirent);
            }
            dir_bytes_remaining -= dir_block_len;
        }
        Err(Ext2Error::NotFound)
    }

    pub fn read_lookup_file<P: BlockDeviceProvider>(
        &mut self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        buffer: &mut [u8],
    ) -> Result<usize, Ext2Error> {
        let inode = self.lookup_file_inode;
        self.read_file_inode(registry, provider, &inode, buffer)
    }

    pub fn read_file_inode<P: BlockDeviceProvider>(
        &mut self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        inode: &Ext2InodeRecord,
        buffer: &mut [u8],
    ) -> Result<usize, Ext2Error> {
        if self.lifecycle.state() != State::Online || !self.ready || !self.mount_boundary_recorded {
            return Err(Ext2Error::FileSystemNotReady);
        }
        if !inode.is_regular_file() {
            return Err(Ext2Error::NotRegularFile);
        }
        let file_size = usize::try_from(inode.size()).map_err(|_| Ext2Error::InvalidInode)?;
        self.file_read_uses_direct_block = false;
        self.file_read_direct_blocks_scanned = 0;
        self.file_read_multi_direct_block_supported = true;
        self.file_read_uses_buffer_head = false;
        self.file_read_copies_to_caller = false;
        self.file_read_len_matches_inode_size = false;
        self.file_read_indirect_blocks_deferred = inode.indirect_blocks_deferred();
        if buffer.len() < file_size {
            self.file_read_short_buffer_rejected = true;
            return Err(Ext2Error::ShortBuffer);
        }
        let Some(devt) = self.devt else {
            return Err(Ext2Error::DeviceMissing);
        };

        let mut copied = 0usize;
        for block in inode.direct_blocks {
            if copied == file_size {
                break;
            }
            if block == 0 {
                return Err(Ext2Error::IndirectBlocksUnsupported);
            }
            let bh = read_fs_block(registry, provider, devt, block, self.block_size)?;
            self.file_read_direct_blocks_scanned += 1;
            let to_copy = min(file_size - copied, bh.data().len());
            buffer[copied..copied + to_copy].copy_from_slice(&bh.data()[..to_copy]);
            copied += to_copy;
            self.file_read_uses_buffer_head = true;
            self.file_read_uses_direct_block = true;
        }
        if copied < file_size && inode.single_indirect_block() != 0 {
            let indirect_bh = read_fs_block(
                registry,
                provider,
                devt,
                inode.single_indirect_block(),
                self.block_size,
            )?;
            self.file_read_uses_buffer_head = true;
            let pointer_count = indirect_bh.data().len() / core::mem::size_of::<u32>();
            let mut pointer_index = 0usize;
            while copied < file_size && pointer_index < pointer_count {
                let block = le_u32(
                    indirect_bh.data(),
                    pointer_index * core::mem::size_of::<u32>(),
                )?;
                if block == 0 {
                    break;
                }
                let bh = read_fs_block(registry, provider, devt, block, self.block_size)?;
                let to_copy = min(file_size - copied, bh.data().len());
                buffer[copied..copied + to_copy].copy_from_slice(&bh.data()[..to_copy]);
                copied += to_copy;
                self.file_read_uses_buffer_head = true;
                pointer_index += 1;
            }
        }
        if copied != file_size {
            return Err(Ext2Error::IndirectBlocksUnsupported);
        }

        self.last_file_read_len = copied;
        self.file_read_copies_to_caller = true;
        self.file_read_len_matches_inode_size = copied == file_size;
        Ok(copied)
    }

    pub fn read_vfs_file<P: BlockDeviceProvider>(
        &mut self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        ino: u32,
        buffer: &mut [u8],
    ) -> Result<usize, Ext2Error> {
        if !self.vfs_read_entry_bound {
            return Err(Ext2Error::FileSystemNotReady);
        }
        self.file_read_entered_from_vfs = true;
        let inode = self.read_inode(registry, provider, ino)?;
        self.read_file_inode(registry, provider, &inode, buffer)
    }

    pub fn read_inode_record<P: BlockDeviceProvider>(
        &self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        ino: u32,
    ) -> Result<Ext2InodeRecord, Ext2Error> {
        self.read_inode(registry, provider, ino)
    }

    fn read_inode<P: BlockDeviceProvider>(
        &self,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        ino: u32,
    ) -> Result<Ext2InodeRecord, Ext2Error> {
        if ino == 0 || self.inodes_per_group == 0 || self.inode_size == 0 {
            return Err(Ext2Error::InvalidInode);
        }
        let Some(devt) = self.devt else {
            return Err(Ext2Error::DeviceMissing);
        };
        let inode_index = ino - 1;
        let group = inode_index / self.inodes_per_group;
        if group != 0 {
            return Err(Ext2Error::InvalidInode);
        }
        let index_in_group = inode_index % self.inodes_per_group;
        let byte_offset = index_in_group
            .checked_mul(u32::from(self.inode_size))
            .ok_or(Ext2Error::InvalidInode)?;
        let block_offset = byte_offset / self.block_size as u32;
        let offset_in_block = byte_offset % self.block_size as u32;
        let inode_block = self
            .group_inode_table_block
            .checked_add(block_offset)
            .ok_or(Ext2Error::InvalidInode)?;
        let bh = read_fs_block(registry, provider, devt, inode_block, self.block_size)?;
        let offset = usize::try_from(offset_in_block).map_err(|_| Ext2Error::InvalidInode)?;
        parse_inode(ino, bh.data(), offset)
    }
}

#[derive(Clone, Copy)]
struct Ext2SuperBlockRecord {
    inodes_count: u32,
    blocks_count: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    magic: u16,
    block_size: usize,
    first_inode: u32,
    inode_size: u16,
}

#[derive(Clone, Copy)]
struct Ext2GroupDescRecord {
    inode_table_block: u32,
}

fn read_fs_block<P: BlockDeviceProvider>(
    registry: &mut BlockDeviceRegistry,
    provider: &mut P,
    devt: DevT,
    block: u32,
    block_size: usize,
) -> Result<BufferHead, Ext2Error> {
    if !supported_block_size(block_size) || block_size > BUFFER_HEAD_MAX_SIZE {
        return Err(Ext2Error::UnsupportedBlockSize);
    }
    bio::sb_bread_by_devt_block(registry, provider, devt, u64::from(block), block_size)
        .map_err(Ext2Error::from)
}

fn read_superblock_probe<P: BlockDeviceProvider>(
    registry: &mut BlockDeviceRegistry,
    provider: &mut P,
    devt: DevT,
) -> Result<BufferHead, Ext2Error> {
    bio::sb_bread_by_devt_block(
        registry,
        provider,
        devt,
        EXT2_SUPERBLOCK_PROBE_BLOCK,
        EXT2_MIN_BLOCK_SIZE,
    )
    .map_err(Ext2Error::from)
}

fn parse_superblock(data: &[u8]) -> Result<Ext2SuperBlockRecord, Ext2Error> {
    let inodes_count = le_u32(data, 0x00)?;
    let blocks_count = le_u32(data, 0x04)?;
    let log_block_size = le_u32(data, 0x18)?;
    let log_frag_size = le_u32(data, 0x1c)?;
    let blocks_per_group = le_u32(data, 0x20)?;
    let inodes_per_group = le_u32(data, 0x28)?;
    let magic = le_u16(data, 0x38)?;
    let rev_level = le_u32(data, 0x4c)?;
    let first_inode = if rev_level == 0 {
        11
    } else {
        le_u32(data, 0x54)?
    };
    let inode_size = if rev_level == 0 {
        EXT2_GOOD_OLD_INODE_SIZE
    } else {
        le_u16(data, 0x58)?
    };
    if log_block_size > 2 || log_frag_size != log_block_size {
        return Err(Ext2Error::UnsupportedBlockSize);
    }
    let block_size = EXT2_MIN_BLOCK_SIZE << log_block_size;
    if !supported_block_size(block_size) {
        return Err(Ext2Error::UnsupportedBlockSize);
    }

    Ok(Ext2SuperBlockRecord {
        inodes_count,
        blocks_count,
        blocks_per_group,
        inodes_per_group,
        magic,
        block_size,
        first_inode,
        inode_size,
    })
}

fn validate_superblock(superblock: &Ext2SuperBlockRecord) -> Result<(), Ext2Error> {
    if superblock.magic != EXT2_SUPER_MAGIC {
        return Err(Ext2Error::InvalidSuperblock);
    }
    if !supported_block_size(superblock.block_size) {
        return Err(Ext2Error::UnsupportedBlockSize);
    }
    if superblock.inodes_per_group == 0 || superblock.blocks_per_group == 0 {
        return Err(Ext2Error::InvalidSuperblock);
    }
    if superblock.inode_size < EXT2_GOOD_OLD_INODE_SIZE
        || usize::from(superblock.inode_size) > superblock.block_size
    {
        return Err(Ext2Error::InvalidSuperblock);
    }
    Ok(())
}

const fn supported_block_size(block_size: usize) -> bool {
    block_size == EXT2_MIN_BLOCK_SIZE || block_size == 2048 || block_size == EXT2_MAX_BLOCK_SIZE
}

const fn group_descriptor_block(block_size: usize) -> u32 {
    if block_size == EXT2_MIN_BLOCK_SIZE {
        2
    } else {
        1
    }
}

fn parse_group_desc(data: &[u8]) -> Result<Ext2GroupDescRecord, Ext2Error> {
    Ok(Ext2GroupDescRecord {
        inode_table_block: le_u32(data, 0x08)?,
    })
}

fn parse_inode(ino: u32, data: &[u8], offset: usize) -> Result<Ext2InodeRecord, Ext2Error> {
    let mode = le_u16(data, offset)?;
    let size = le_u32(data, offset + 0x04)?;
    let mut blocks = [0u32; EXT2_N_BLOCKS];
    for (index, block) in blocks.iter_mut().enumerate() {
        *block = le_u32(data, offset + 0x28 + index * 4)?;
    }
    if blocks[EXT2_SINGLE_INDIRECT_INDEX + 1..]
        .iter()
        .any(|block| *block != 0)
    {
        return Err(Ext2Error::IndirectBlocksUnsupported);
    }
    let mut direct_blocks = [0u32; EXT2_NDIR_BLOCKS];
    direct_blocks.copy_from_slice(&blocks[..EXT2_NDIR_BLOCKS]);
    Ok(Ext2InodeRecord {
        ino,
        mode,
        size,
        direct_blocks,
        single_indirect_block: blocks[EXT2_SINGLE_INDIRECT_INDEX],
        indirect_blocks_deferred: true,
    })
}

fn find_dirent(data: &[u8], needle: &[u8]) -> Result<Option<Ext2DirEntryRecord>, Ext2Error> {
    let limit = data.len();
    let mut offset = 0usize;
    while offset + 8 <= limit {
        let inode = le_u32(data, offset)?;
        let rec_len = le_u16(data, offset + 0x04)?;
        let name_len = *data.get(offset + 0x06).ok_or(Ext2Error::InvalidDirEntry)?;
        let file_type = ext2_file_type(*data.get(offset + 0x07).ok_or(Ext2Error::InvalidDirEntry)?);
        let rec_len_usize = usize::from(rec_len);
        let name_len_usize = usize::from(name_len);
        if rec_len_usize < 8
            || rec_len_usize % 4 != 0
            || name_len_usize > rec_len_usize.saturating_sub(8)
            || offset + rec_len_usize > limit
        {
            return Err(Ext2Error::InvalidDirEntry);
        }
        if inode != 0 && &data[offset + 8..offset + 8 + name_len_usize] == needle {
            let mut name = [0u8; EXT2_NAME_MAX];
            name[..name_len_usize].copy_from_slice(needle);
            return Ok(Some(Ext2DirEntryRecord {
                inode,
                rec_len,
                name_len,
                file_type,
                name,
            }));
        }
        offset += rec_len_usize;
    }
    Ok(None)
}

const fn ext2_file_type(file_type: u8) -> Ext2FileType {
    match file_type {
        1 => Ext2FileType::RegularFile,
        2 => Ext2FileType::Directory,
        0 => Ext2FileType::Unknown,
        _ => Ext2FileType::Other,
    }
}

fn le_u16(data: &[u8], offset: usize) -> Result<u16, Ext2Error> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or(Ext2Error::InvalidSuperblock)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn le_u32(data: &[u8], offset: usize) -> Result<u32, Ext2Error> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(Ext2Error::InvalidSuperblock)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
