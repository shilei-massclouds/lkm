use super::{
    block_device::{BlockDeviceProvider, BlockDeviceRegistry},
    ext2::{Ext2DirEntryRecord, Ext2Error, Ext2FileSystem, Ext2InodeRecord},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use alloc::vec::Vec;

pub const VFS_NAME_MAX: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FileSystemKind {
    RamFs,
    DevFs,
    Ext2,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VfsInodeKind {
    Directory,
    RegularFile,
    DeviceNode,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MountRef {
    index: usize,
}

impl MountRef {
    const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SuperBlockRef {
    index: usize,
}

impl SuperBlockRef {
    const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct InodeRef {
    index: usize,
}

impl InodeRef {
    const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DentryRef {
    index: usize,
}

impl DentryRef {
    const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct FileRef {
    index: usize,
}

impl FileRef {
    const fn new(index: usize) -> Self {
        Self { index }
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VfsError {
    CoreNotReady,
    FsTypeNotReady,
    FsTypeAlreadyRegistered,
    FsTypeMissing,
    MountMissing,
    AlreadyMounted,
    InvalidRef,
    InvalidName,
    NameTooLong,
    NotDirectory,
    NotFile,
    AlreadyExists,
    NotFound,
    DirectoryNotEmpty,
    ReadOnly,
    ShortBuffer,
    Backend,
}

impl From<Ext2Error> for VfsError {
    fn from(error: Ext2Error) -> Self {
        match error {
            Ext2Error::NotFound => Self::NotFound,
            Ext2Error::NotDirectory => Self::NotDirectory,
            Ext2Error::NotRegularFile => Self::NotFile,
            Ext2Error::ShortBuffer => Self::ShortBuffer,
            _ => Self::Backend,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Ext2InodeBinding {
    ino: u32,
}

impl Ext2InodeBinding {
    const fn new(ino: u32) -> Self {
        Self { ino }
    }

    pub const fn ino(&self) -> u32 {
        self.ino
    }
}

pub struct FileSystemType {
    lifecycle: Lifecycle,
    kind: FileSystemKind,
    name: [u8; VFS_NAME_MAX],
    name_len: usize,
    mount_callback_bound: bool,
}

impl FileSystemType {
    pub const fn new_ramfs() -> Self {
        let mut name = [0u8; VFS_NAME_MAX];
        name[0] = b'r';
        name[1] = b'a';
        name[2] = b'm';
        name[3] = b'f';
        name[4] = b's';
        Self {
            lifecycle: Lifecycle::new(State::Base),
            kind: FileSystemKind::RamFs,
            name,
            name_len: 5,
            mount_callback_bound: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kind(&self) -> FileSystemKind {
        self.kind
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub const fn name_bound(&self) -> bool {
        self.name_len != 0
    }

    pub const fn mount_callback_bound(&self) -> bool {
        self.mount_callback_bound
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || !self.name_bound() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.mount_callback_bound = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }
}

pub struct RamFsType {
    base: FileSystemType,
    memory_backed: bool,
}

impl RamFsType {
    pub const fn new() -> Self {
        Self {
            base: FileSystemType::new_ramfs(),
            memory_backed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.base.state()
    }

    pub fn name(&self) -> &[u8] {
        self.base.name()
    }

    pub const fn name_bound(&self) -> bool {
        self.base.name_bound()
    }

    pub const fn mount_callback_bound(&self) -> bool {
        self.base.mount_callback_bound()
    }

    pub const fn memory_backed(&self) -> bool {
        self.memory_backed
    }

    pub fn setup(&mut self) -> EventResult {
        self.base.setup()?;
        self.memory_backed = true;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct Mount {
    mount_ref: MountRef,
    fs_kind: FileSystemKind,
    superblock_ref: SuperBlockRef,
    root_dentry_ref: DentryRef,
    mount_point_ref: Option<DentryRef>,
    mounted: bool,
}

impl Mount {
    const fn new(
        mount_ref: MountRef,
        fs_kind: FileSystemKind,
        superblock_ref: SuperBlockRef,
        root_dentry_ref: DentryRef,
        mount_point_ref: Option<DentryRef>,
    ) -> Self {
        Self {
            mount_ref,
            fs_kind,
            superblock_ref,
            root_dentry_ref,
            mount_point_ref,
            mounted: true,
        }
    }

    pub const fn mount_ref(&self) -> MountRef {
        self.mount_ref
    }

    pub const fn fs_kind(&self) -> FileSystemKind {
        self.fs_kind
    }

    pub const fn superblock_ref(&self) -> SuperBlockRef {
        self.superblock_ref
    }

    pub const fn root_dentry_ref(&self) -> DentryRef {
        self.root_dentry_ref
    }

    pub const fn mount_point_ref(&self) -> Option<DentryRef> {
        self.mount_point_ref
    }

    pub const fn mounted(&self) -> bool {
        self.mounted
    }
}

pub struct SuperBlock {
    superblock_ref: SuperBlockRef,
    fs_kind: FileSystemKind,
    root_inode_ref: Option<InodeRef>,
    root_dentry_ref: Option<DentryRef>,
    ramfs_private_bound: bool,
    devfs_private_bound: bool,
    ext2_private_bound: bool,
}

impl SuperBlock {
    fn new(superblock_ref: SuperBlockRef, fs_kind: FileSystemKind) -> Self {
        Self {
            superblock_ref,
            fs_kind,
            root_inode_ref: None,
            root_dentry_ref: None,
            ramfs_private_bound: matches!(fs_kind, FileSystemKind::RamFs),
            devfs_private_bound: matches!(fs_kind, FileSystemKind::DevFs),
            ext2_private_bound: matches!(fs_kind, FileSystemKind::Ext2),
        }
    }

    pub const fn superblock_ref(&self) -> SuperBlockRef {
        self.superblock_ref
    }

    pub const fn fs_kind(&self) -> FileSystemKind {
        self.fs_kind
    }

    pub const fn root_inode_ref(&self) -> Option<InodeRef> {
        self.root_inode_ref
    }

    pub const fn root_dentry_ref(&self) -> Option<DentryRef> {
        self.root_dentry_ref
    }

    pub const fn ramfs_private_bound(&self) -> bool {
        self.ramfs_private_bound
    }

    pub const fn devfs_private_bound(&self) -> bool {
        self.devfs_private_bound
    }

    pub const fn ext2_private_bound(&self) -> bool {
        self.ext2_private_bound
    }

    fn bind_root(&mut self, root_inode_ref: InodeRef, root_dentry_ref: DentryRef) {
        self.root_inode_ref = Some(root_inode_ref);
        self.root_dentry_ref = Some(root_dentry_ref);
    }
}

pub struct Inode {
    inode_ref: InodeRef,
    superblock_ref: SuperBlockRef,
    kind: VfsInodeKind,
    size: usize,
    children: Vec<DentryRef>,
    data: Vec<u8>,
    ext2_binding: Option<Ext2InodeBinding>,
    read_only_backed: bool,
    removed: bool,
}

impl Inode {
    fn new(inode_ref: InodeRef, superblock_ref: SuperBlockRef, kind: VfsInodeKind) -> Self {
        Self {
            inode_ref,
            superblock_ref,
            kind,
            size: 0,
            children: Vec::new(),
            data: Vec::new(),
            ext2_binding: None,
            read_only_backed: false,
            removed: false,
        }
    }

    pub const fn inode_ref(&self) -> InodeRef {
        self.inode_ref
    }

    pub const fn superblock_ref(&self) -> SuperBlockRef {
        self.superblock_ref
    }

    pub const fn kind(&self) -> VfsInodeKind {
        self.kind
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn removed(&self) -> bool {
        self.removed
    }

    pub const fn ext2_binding(&self) -> Option<Ext2InodeBinding> {
        self.ext2_binding
    }

    pub const fn read_only_backed(&self) -> bool {
        self.read_only_backed
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    fn is_directory(&self) -> bool {
        self.kind == VfsInodeKind::Directory
    }

    fn is_file(&self) -> bool {
        self.kind == VfsInodeKind::RegularFile
    }

    fn bind_ext2_inode(&mut self, inode: &Ext2InodeRecord) {
        self.ext2_binding = Some(Ext2InodeBinding::new(inode.ino()));
        self.read_only_backed = true;
        self.size = inode.size() as usize;
    }
}

pub struct Dentry {
    dentry_ref: DentryRef,
    superblock_ref: SuperBlockRef,
    parent: Option<DentryRef>,
    inode_ref: InodeRef,
    name: [u8; VFS_NAME_MAX],
    name_len: usize,
    positive: bool,
    removed: bool,
    mounted_root: Option<DentryRef>,
}

impl Dentry {
    const fn new(
        dentry_ref: DentryRef,
        superblock_ref: SuperBlockRef,
        parent: Option<DentryRef>,
        inode_ref: InodeRef,
        name: [u8; VFS_NAME_MAX],
        name_len: usize,
    ) -> Self {
        Self {
            dentry_ref,
            superblock_ref,
            parent,
            inode_ref,
            name,
            name_len,
            positive: true,
            removed: false,
            mounted_root: None,
        }
    }

    pub const fn dentry_ref(&self) -> DentryRef {
        self.dentry_ref
    }

    pub const fn superblock_ref(&self) -> SuperBlockRef {
        self.superblock_ref
    }

    pub const fn parent(&self) -> Option<DentryRef> {
        self.parent
    }

    pub const fn inode_ref(&self) -> InodeRef {
        self.inode_ref
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub const fn positive(&self) -> bool {
        self.positive
    }

    pub const fn removed(&self) -> bool {
        self.removed
    }

    pub const fn mounted_root(&self) -> Option<DentryRef> {
        self.mounted_root
    }

    fn name_eq(&self, name: &[u8]) -> bool {
        self.positive && !self.removed && self.name() == name
    }

    fn bind_mount_root(&mut self, root_dentry_ref: DentryRef) {
        self.mounted_root = Some(root_dentry_ref);
    }

    fn mark_removed(&mut self) {
        self.positive = false;
        self.removed = true;
    }
}

pub struct File {
    file_ref: FileRef,
    dentry_ref: DentryRef,
    inode_ref: InodeRef,
    position: usize,
    write_committed: bool,
    read_returns_written_data: bool,
    read_returns_backend_data: bool,
    last_write_len: usize,
    last_read_len: usize,
}

impl File {
    const fn new(file_ref: FileRef, dentry_ref: DentryRef, inode_ref: InodeRef) -> Self {
        Self {
            file_ref,
            dentry_ref,
            inode_ref,
            position: 0,
            write_committed: false,
            read_returns_written_data: false,
            read_returns_backend_data: false,
            last_write_len: 0,
            last_read_len: 0,
        }
    }

    pub const fn file_ref(&self) -> FileRef {
        self.file_ref
    }

    pub const fn dentry_ref(&self) -> DentryRef {
        self.dentry_ref
    }

    pub const fn inode_ref(&self) -> InodeRef {
        self.inode_ref
    }

    pub const fn position(&self) -> usize {
        self.position
    }

    pub const fn write_committed(&self) -> bool {
        self.write_committed
    }

    pub const fn read_returns_written_data(&self) -> bool {
        self.read_returns_written_data
    }

    pub const fn read_returns_backend_data(&self) -> bool {
        self.read_returns_backend_data
    }

    pub const fn last_write_len(&self) -> usize {
        self.last_write_len
    }

    pub const fn last_read_len(&self) -> usize {
        self.last_read_len
    }
}

#[derive(Clone, Copy)]
pub struct DirEntry {
    dentry_ref: DentryRef,
    inode_ref: InodeRef,
    kind: VfsInodeKind,
    name: [u8; VFS_NAME_MAX],
    name_len: usize,
}

impl DirEntry {
    fn from_dentry(dentry: &Dentry, inode: &Inode) -> Self {
        Self {
            dentry_ref: dentry.dentry_ref,
            inode_ref: inode.inode_ref,
            kind: inode.kind,
            name: dentry.name,
            name_len: dentry.name_len,
        }
    }

    pub const fn dentry_ref(&self) -> DentryRef {
        self.dentry_ref
    }

    pub const fn inode_ref(&self) -> InodeRef {
        self.inode_ref
    }

    pub const fn kind(&self) -> VfsInodeKind {
        self.kind
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub fn name_eq(&self, name: &[u8]) -> bool {
        self.name() == name
    }
}

pub struct VfsCore {
    lifecycle: Lifecycle,
    fs_type_registry_ready: bool,
    mount_table_ready: bool,
    dentry_cache_ready: bool,
    inode_table_ready: bool,
    file_table_ready: bool,
    page_cache_deferred: bool,
    permissions_deferred: bool,
    mount_namespace_deferred: bool,
    ramfs_registered: bool,
    mounts: Vec<Mount>,
    superblocks: Vec<SuperBlock>,
    inodes: Vec<Inode>,
    dentries: Vec<Dentry>,
    files: Vec<File>,
    current_root_mount: Option<MountRef>,
    current_root_dentry: Option<DentryRef>,
    lookup_count: usize,
    insert_count: usize,
    remove_count: usize,
    readdir_count: usize,
    write_count: usize,
    read_count: usize,
    lookup_returned: bool,
    readdir_lists: bool,
    file_write_committed: bool,
    file_read_returns_written_data: bool,
    ext2_mount_created: bool,
    ext2_lookup_dispatched: bool,
    ext2_read_dispatched: bool,
    file_read_returns_backend_data: bool,
}

impl VfsCore {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            fs_type_registry_ready: false,
            mount_table_ready: false,
            dentry_cache_ready: false,
            inode_table_ready: false,
            file_table_ready: false,
            page_cache_deferred: false,
            permissions_deferred: false,
            mount_namespace_deferred: false,
            ramfs_registered: false,
            mounts: Vec::new(),
            superblocks: Vec::new(),
            inodes: Vec::new(),
            dentries: Vec::new(),
            files: Vec::new(),
            current_root_mount: None,
            current_root_dentry: None,
            lookup_count: 0,
            insert_count: 0,
            remove_count: 0,
            readdir_count: 0,
            write_count: 0,
            read_count: 0,
            lookup_returned: false,
            readdir_lists: false,
            file_write_committed: false,
            file_read_returns_written_data: false,
            ext2_mount_created: false,
            ext2_lookup_dispatched: false,
            ext2_read_dispatched: false,
            file_read_returns_backend_data: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn fs_type_registry_ready(&self) -> bool {
        self.fs_type_registry_ready
    }

    pub const fn mount_table_ready(&self) -> bool {
        self.mount_table_ready
    }

    pub const fn dentry_cache_ready(&self) -> bool {
        self.dentry_cache_ready
    }

    pub const fn inode_table_ready(&self) -> bool {
        self.inode_table_ready
    }

    pub const fn file_table_ready(&self) -> bool {
        self.file_table_ready
    }

    pub const fn page_cache_deferred(&self) -> bool {
        self.page_cache_deferred
    }

    pub const fn permissions_deferred(&self) -> bool {
        self.permissions_deferred
    }

    pub const fn mount_namespace_deferred(&self) -> bool {
        self.mount_namespace_deferred
    }

    pub const fn ramfs_registered(&self) -> bool {
        self.ramfs_registered
    }

    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }

    pub fn superblock_count(&self) -> usize {
        self.superblocks.len()
    }

    pub fn inode_count(&self) -> usize {
        self.inodes.len()
    }

    pub fn dentry_count(&self) -> usize {
        self.dentries.len()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub const fn current_root_mount(&self) -> Option<MountRef> {
        self.current_root_mount
    }

    pub const fn current_root_dentry(&self) -> Option<DentryRef> {
        self.current_root_dentry
    }

    pub const fn lookup_count(&self) -> usize {
        self.lookup_count
    }

    pub const fn insert_count(&self) -> usize {
        self.insert_count
    }

    pub const fn remove_count(&self) -> usize {
        self.remove_count
    }

    pub const fn readdir_count(&self) -> usize {
        self.readdir_count
    }

    pub const fn write_count(&self) -> usize {
        self.write_count
    }

    pub const fn read_count(&self) -> usize {
        self.read_count
    }

    pub const fn lookup_returned(&self) -> bool {
        self.lookup_returned
    }

    pub const fn readdir_lists(&self) -> bool {
        self.readdir_lists
    }

    pub const fn file_write_committed(&self) -> bool {
        self.file_write_committed
    }

    pub const fn file_read_returns_written_data(&self) -> bool {
        self.file_read_returns_written_data
    }

    pub const fn ext2_mount_created(&self) -> bool {
        self.ext2_mount_created
    }

    pub const fn ext2_lookup_dispatched(&self) -> bool {
        self.ext2_lookup_dispatched
    }

    pub const fn ext2_read_dispatched(&self) -> bool {
        self.ext2_read_dispatched
    }

    pub const fn file_read_returns_backend_data(&self) -> bool {
        self.file_read_returns_backend_data
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

        self.fs_type_registry_ready = true;
        self.mount_table_ready = true;
        self.dentry_cache_ready = true;
        self.inode_table_ready = true;
        self.file_table_ready = true;
        self.page_cache_deferred = true;
        self.permissions_deferred = true;
        self.mount_namespace_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn register_ramfs_type(&mut self, fs_type: &RamFsType) -> Result<(), VfsError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VfsError::CoreNotReady);
        }
        if fs_type.state() != State::Ready
            || !fs_type.name_bound()
            || !fs_type.mount_callback_bound()
            || !fs_type.memory_backed()
        {
            return Err(VfsError::FsTypeNotReady);
        }
        if self.ramfs_registered {
            return Err(VfsError::FsTypeAlreadyRegistered);
        }

        self.ramfs_registered = true;
        Ok(())
    }

    pub fn mount_initial_ramfs_root(&mut self, fs_type: &RamFsType) -> Result<MountRef, VfsError> {
        let mount_ref = self.create_ramfs_mount(fs_type, None)?;
        self.set_root(mount_ref)?;
        Ok(mount_ref)
    }

    pub fn mount_ramfs_at(
        &mut self,
        fs_type: &RamFsType,
        mount_point_ref: DentryRef,
    ) -> Result<MountRef, VfsError> {
        self.ensure_mount_point(mount_point_ref)?;

        let mount_ref = self.create_ramfs_mount(fs_type, Some(mount_point_ref))?;
        let root_dentry_ref = self
            .mount(mount_ref)
            .ok_or(VfsError::InvalidRef)?
            .root_dentry_ref();
        let mount_point = self
            .dentry_mut(mount_point_ref)
            .ok_or(VfsError::InvalidRef)?;
        mount_point.bind_mount_root(root_dentry_ref);
        Ok(mount_ref)
    }

    pub fn mount_devfs_at(&mut self, mount_point_ref: DentryRef) -> Result<MountRef, VfsError> {
        self.ensure_mount_point(mount_point_ref)?;

        let mount_ref = self.create_mount(FileSystemKind::DevFs, Some(mount_point_ref))?;
        let root_dentry_ref = self
            .mount(mount_ref)
            .ok_or(VfsError::InvalidRef)?
            .root_dentry_ref();
        let mount_point = self
            .dentry_mut(mount_point_ref)
            .ok_or(VfsError::InvalidRef)?;
        mount_point.bind_mount_root(root_dentry_ref);
        Ok(mount_ref)
    }

    pub fn mount_ext2_at(
        &mut self,
        fs: &Ext2FileSystem,
        mount_point_ref: DentryRef,
    ) -> Result<MountRef, VfsError> {
        self.ensure_mount_point(mount_point_ref)?;
        if fs.state() != State::Ready || !fs.ready() || !fs.root_dentry_bound() {
            return Err(VfsError::FsTypeNotReady);
        }

        let mount_ref = self.create_mount(FileSystemKind::Ext2, Some(mount_point_ref))?;
        let root_dentry_ref = self
            .mount(mount_ref)
            .ok_or(VfsError::InvalidRef)?
            .root_dentry_ref();
        let root_inode_ref = self
            .dentry(root_dentry_ref)
            .ok_or(VfsError::InvalidRef)?
            .inode_ref();
        let root_inode = self.inode_mut(root_inode_ref).ok_or(VfsError::InvalidRef)?;
        root_inode.bind_ext2_inode(fs.root_inode());

        let mount_point = self
            .dentry_mut(mount_point_ref)
            .ok_or(VfsError::InvalidRef)?;
        mount_point.bind_mount_root(root_dentry_ref);
        self.ext2_mount_created = true;
        Ok(mount_ref)
    }

    fn create_ramfs_mount(
        &mut self,
        fs_type: &RamFsType,
        mount_point_ref: Option<DentryRef>,
    ) -> Result<MountRef, VfsError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VfsError::CoreNotReady);
        }
        if !self.ramfs_registered {
            return Err(VfsError::FsTypeMissing);
        }
        if fs_type.state() != State::Ready || fs_type.base.kind() != FileSystemKind::RamFs {
            return Err(VfsError::FsTypeNotReady);
        }

        self.create_mount(FileSystemKind::RamFs, mount_point_ref)
    }

    fn create_mount(
        &mut self,
        fs_kind: FileSystemKind,
        mount_point_ref: Option<DentryRef>,
    ) -> Result<MountRef, VfsError> {
        if self.lifecycle.state() != State::Ready {
            return Err(VfsError::CoreNotReady);
        }

        let superblock_ref = SuperBlockRef::new(self.superblocks.len());
        self.superblocks
            .push(SuperBlock::new(superblock_ref, fs_kind));

        let root_inode_ref = InodeRef::new(self.inodes.len());
        self.inodes.push(Inode::new(
            root_inode_ref,
            superblock_ref,
            VfsInodeKind::Directory,
        ));

        let (root_name, root_name_len) = copy_name(b"/")?;
        let root_dentry_ref = DentryRef::new(self.dentries.len());
        self.dentries.push(Dentry::new(
            root_dentry_ref,
            superblock_ref,
            None,
            root_inode_ref,
            root_name,
            root_name_len,
        ));

        let Some(superblock) = self.superblock_mut(superblock_ref) else {
            return Err(VfsError::InvalidRef);
        };
        superblock.bind_root(root_inode_ref, root_dentry_ref);

        let mount_ref = MountRef::new(self.mounts.len());
        self.mounts.push(Mount::new(
            mount_ref,
            fs_kind,
            superblock_ref,
            root_dentry_ref,
            mount_point_ref,
        ));
        Ok(mount_ref)
    }

    pub const fn rootfs_mount_created(&self) -> bool {
        self.current_root_mount.is_some() && self.current_root_dentry.is_some()
    }

    pub fn set_root(&mut self, mount_ref: MountRef) -> Result<(), VfsError> {
        let Some(mount) = self.mount(mount_ref) else {
            return Err(VfsError::MountMissing);
        };
        if !mount.mounted() {
            return Err(VfsError::MountMissing);
        }
        let root_dentry_ref = mount.root_dentry_ref();
        self.current_root_mount = Some(mount_ref);
        self.current_root_dentry = Some(root_dentry_ref);
        Ok(())
    }

    pub fn mount(&self, mount_ref: MountRef) -> Option<&Mount> {
        self.mounts.get(mount_ref.index())
    }

    pub fn superblock(&self, superblock_ref: SuperBlockRef) -> Option<&SuperBlock> {
        self.superblocks.get(superblock_ref.index())
    }

    pub fn inode(&self, inode_ref: InodeRef) -> Option<&Inode> {
        self.inodes.get(inode_ref.index())
    }

    pub fn dentry(&self, dentry_ref: DentryRef) -> Option<&Dentry> {
        self.dentries.get(dentry_ref.index())
    }

    pub fn file(&self, file_ref: FileRef) -> Option<&File> {
        self.files.get(file_ref.index())
    }

    pub fn lookup_child(
        &mut self,
        parent_ref: DentryRef,
        name: &[u8],
    ) -> Result<DentryRef, VfsError> {
        self.lookup_count = self.lookup_count.saturating_add(1);
        let child_ref = self.find_child(self.follow_mount(parent_ref)?, name)?;
        self.lookup_returned = true;
        Ok(child_ref)
    }

    pub fn lookup_ext2_child<P: BlockDeviceProvider>(
        &mut self,
        fs: &mut Ext2FileSystem,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        parent_ref: DentryRef,
        name: &[u8],
    ) -> Result<DentryRef, VfsError> {
        self.lookup_count = self.lookup_count.saturating_add(1);
        let parent_ref = self.follow_mount(parent_ref)?;
        let parent = self.positive_dentry(parent_ref)?;
        let parent_inode = self.inode(parent.inode_ref()).ok_or(VfsError::InvalidRef)?;
        if parent_inode.superblock_ref() != parent.superblock_ref()
            || parent_inode.kind() != VfsInodeKind::Directory
            || !parent_inode.read_only_backed()
        {
            return Err(VfsError::NotDirectory);
        }
        let superblock = self
            .superblock(parent.superblock_ref())
            .ok_or(VfsError::InvalidRef)?;
        if superblock.fs_kind() != FileSystemKind::Ext2 || !superblock.ext2_private_bound() {
            return Err(VfsError::FsTypeMissing);
        }
        if let Ok(existing) = self.find_child(parent_ref, name) {
            self.lookup_returned = true;
            return Ok(existing);
        }

        let dirent = fs.lookup_root(registry, provider, name)?;
        let file_inode = *fs.lookup_file_inode();
        let child_ref = self.insert_ext2_lookup_child(parent_ref, &dirent, &file_inode)?;
        self.lookup_returned = true;
        self.ext2_lookup_dispatched = true;
        Ok(child_ref)
    }

    pub fn create_dir(
        &mut self,
        parent_ref: DentryRef,
        name: &[u8],
    ) -> Result<DentryRef, VfsError> {
        self.create_child(parent_ref, name, VfsInodeKind::Directory)
    }

    pub fn create_file(
        &mut self,
        parent_ref: DentryRef,
        name: &[u8],
    ) -> Result<DentryRef, VfsError> {
        self.create_child(parent_ref, name, VfsInodeKind::RegularFile)
    }

    pub fn create_device_node(
        &mut self,
        parent_ref: DentryRef,
        name: &[u8],
    ) -> Result<DentryRef, VfsError> {
        self.create_child(parent_ref, name, VfsInodeKind::DeviceNode)
    }

    pub fn open_file(&mut self, dentry_ref: DentryRef) -> Result<FileRef, VfsError> {
        let dentry_ref = self.follow_mount(dentry_ref)?;
        let inode_ref = self.positive_dentry(dentry_ref)?.inode_ref();
        if !self.inode(inode_ref).ok_or(VfsError::InvalidRef)?.is_file() {
            return Err(VfsError::NotFile);
        }

        let file_ref = FileRef::new(self.files.len());
        self.files.push(File::new(file_ref, dentry_ref, inode_ref));
        Ok(file_ref)
    }

    pub fn write_file(
        &mut self,
        file_ref: FileRef,
        offset: usize,
        data: &[u8],
    ) -> Result<usize, VfsError> {
        let inode_ref = self.file(file_ref).ok_or(VfsError::InvalidRef)?.inode_ref();
        let end = offset.saturating_add(data.len());
        let inode = self.inode_mut(inode_ref).ok_or(VfsError::InvalidRef)?;
        if inode.read_only_backed() {
            return Err(VfsError::ReadOnly);
        }
        if !inode.is_file() || inode.removed() {
            return Err(VfsError::NotFile);
        }
        if inode.data.len() < end {
            inode.data.resize(end, 0);
        }
        inode.data[offset..end].copy_from_slice(data);
        inode.size = inode.data.len();

        let file = self.file_mut(file_ref).ok_or(VfsError::InvalidRef)?;
        file.position = end;
        file.write_committed = true;
        file.last_write_len = data.len();
        self.write_count = self.write_count.saturating_add(1);
        self.file_write_committed = true;
        Ok(data.len())
    }

    pub fn read_file(
        &mut self,
        file_ref: FileRef,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, VfsError> {
        let inode_ref = self.file(file_ref).ok_or(VfsError::InvalidRef)?.inode_ref();
        let inode = self.inode(inode_ref).ok_or(VfsError::InvalidRef)?;
        if !inode.is_file() || inode.removed() {
            return Err(VfsError::NotFile);
        }
        if offset >= inode.data.len() {
            let file = self.file_mut(file_ref).ok_or(VfsError::InvalidRef)?;
            file.position = offset;
            file.last_read_len = 0;
            self.read_count = self.read_count.saturating_add(1);
            return Ok(0);
        }

        let available = inode.data.len() - offset;
        let len = core::cmp::min(buffer.len(), available);
        buffer[..len].copy_from_slice(&inode.data[offset..offset + len]);

        let read_returns_written_data = {
            let file = self.file_mut(file_ref).ok_or(VfsError::InvalidRef)?;
            file.position = offset + len;
            file.last_read_len = len;
            file.read_returns_written_data = file.write_committed && len != 0;
            file.read_returns_written_data
        };
        self.read_count = self.read_count.saturating_add(1);
        self.file_read_returns_written_data |= read_returns_written_data;
        Ok(len)
    }

    pub fn read_ext2_file<P: BlockDeviceProvider>(
        &mut self,
        fs: &mut Ext2FileSystem,
        registry: &mut BlockDeviceRegistry,
        provider: &mut P,
        file_ref: FileRef,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<usize, VfsError> {
        if offset != 0 {
            return Err(VfsError::Backend);
        }
        let inode_ref = self.file(file_ref).ok_or(VfsError::InvalidRef)?.inode_ref();
        let inode = self.inode(inode_ref).ok_or(VfsError::InvalidRef)?;
        if !inode.is_file() || inode.removed() {
            return Err(VfsError::NotFile);
        }
        if !inode.read_only_backed() || inode.ext2_binding().is_none() {
            return Err(VfsError::FsTypeMissing);
        }

        let len = fs.read_vfs_file(registry, provider, buffer)?;
        let file = self.file_mut(file_ref).ok_or(VfsError::InvalidRef)?;
        file.position = len;
        file.last_read_len = len;
        file.read_returns_backend_data = true;
        self.read_count = self.read_count.saturating_add(1);
        self.ext2_read_dispatched = true;
        self.file_read_returns_backend_data = true;
        Ok(len)
    }

    pub fn read_dir(&mut self, dir_ref: DentryRef) -> Result<Vec<DirEntry>, VfsError> {
        let dir_ref = self.follow_mount(dir_ref)?;
        let inode_ref = self.positive_dentry(dir_ref)?.inode_ref();
        let inode = self.inode(inode_ref).ok_or(VfsError::InvalidRef)?;
        if !inode.is_directory() {
            return Err(VfsError::NotDirectory);
        }

        let mut entries = Vec::new();
        for child_ref in inode.children.iter().copied() {
            let Some(dentry) = self.dentry(child_ref) else {
                return Err(VfsError::InvalidRef);
            };
            if !dentry.positive() || dentry.removed() {
                continue;
            }
            let Some(child_inode) = self.inode(dentry.inode_ref()) else {
                return Err(VfsError::InvalidRef);
            };
            if child_inode.removed() {
                continue;
            }
            entries.push(DirEntry::from_dentry(dentry, child_inode));
        }

        self.readdir_count = self.readdir_count.saturating_add(1);
        self.readdir_lists |= !entries.is_empty();
        Ok(entries)
    }

    pub fn remove_child(
        &mut self,
        parent_ref: DentryRef,
        name: &[u8],
    ) -> Result<DentryRef, VfsError> {
        let parent_ref = self.follow_mount(parent_ref)?;
        let child_ref = self.find_child(parent_ref, name)?;
        let child_inode_ref = self
            .dentry(child_ref)
            .ok_or(VfsError::InvalidRef)?
            .inode_ref();
        let child_is_nonempty_dir = {
            let child_inode = self.inode(child_inode_ref).ok_or(VfsError::InvalidRef)?;
            child_inode.is_directory() && self.live_child_count(child_inode_ref)? != 0
        };
        if child_is_nonempty_dir {
            return Err(VfsError::DirectoryNotEmpty);
        }

        let parent_inode_ref = self.positive_dentry(parent_ref)?.inode_ref();
        let parent_inode = self
            .inode_mut(parent_inode_ref)
            .ok_or(VfsError::InvalidRef)?;
        parent_inode.children.retain(|entry| *entry != child_ref);

        let child_inode = self
            .inode_mut(child_inode_ref)
            .ok_or(VfsError::InvalidRef)?;
        child_inode.removed = true;
        let child_dentry = self.dentry_mut(child_ref).ok_or(VfsError::InvalidRef)?;
        child_dentry.mark_removed();

        self.remove_count = self.remove_count.saturating_add(1);
        Ok(child_ref)
    }

    fn create_child(
        &mut self,
        parent_ref: DentryRef,
        name: &[u8],
        kind: VfsInodeKind,
    ) -> Result<DentryRef, VfsError> {
        let (name_buf, name_len) = copy_name(name)?;
        let parent_ref = self.follow_mount(parent_ref)?;
        if self.find_child(parent_ref, name).is_ok() {
            return Err(VfsError::AlreadyExists);
        }

        let parent = self.positive_dentry(parent_ref)?;
        let superblock_ref = parent.superblock_ref();
        let parent_inode_ref = parent.inode_ref();
        if !self
            .inode(parent_inode_ref)
            .ok_or(VfsError::InvalidRef)?
            .is_directory()
        {
            return Err(VfsError::NotDirectory);
        }

        let inode_ref = InodeRef::new(self.inodes.len());
        self.inodes
            .push(Inode::new(inode_ref, superblock_ref, kind));

        let dentry_ref = DentryRef::new(self.dentries.len());
        self.dentries.push(Dentry::new(
            dentry_ref,
            superblock_ref,
            Some(parent_ref),
            inode_ref,
            name_buf,
            name_len,
        ));

        let parent_inode = self
            .inode_mut(parent_inode_ref)
            .ok_or(VfsError::InvalidRef)?;
        parent_inode.children.push(dentry_ref);
        self.insert_count = self.insert_count.saturating_add(1);
        Ok(dentry_ref)
    }

    fn insert_ext2_lookup_child(
        &mut self,
        parent_ref: DentryRef,
        dirent: &Ext2DirEntryRecord,
        inode: &Ext2InodeRecord,
    ) -> Result<DentryRef, VfsError> {
        let (name_buf, name_len) = copy_name(dirent.name())?;
        let parent = self.positive_dentry(parent_ref)?;
        let superblock_ref = parent.superblock_ref();
        let parent_inode_ref = parent.inode_ref();
        if !self
            .inode(parent_inode_ref)
            .ok_or(VfsError::InvalidRef)?
            .is_directory()
        {
            return Err(VfsError::NotDirectory);
        }

        let inode_ref = InodeRef::new(self.inodes.len());
        let mut vfs_inode = Inode::new(inode_ref, superblock_ref, VfsInodeKind::RegularFile);
        vfs_inode.bind_ext2_inode(inode);
        self.inodes.push(vfs_inode);

        let dentry_ref = DentryRef::new(self.dentries.len());
        self.dentries.push(Dentry::new(
            dentry_ref,
            superblock_ref,
            Some(parent_ref),
            inode_ref,
            name_buf,
            name_len,
        ));

        let parent_inode = self
            .inode_mut(parent_inode_ref)
            .ok_or(VfsError::InvalidRef)?;
        parent_inode.children.push(dentry_ref);
        self.insert_count = self.insert_count.saturating_add(1);
        Ok(dentry_ref)
    }

    fn ensure_mount_point(&self, mount_point_ref: DentryRef) -> Result<(), VfsError> {
        let mount_point = self.positive_dentry(mount_point_ref)?;
        if mount_point.mounted_root().is_some() {
            return Err(VfsError::AlreadyMounted);
        }
        let mount_point_inode = self
            .inode(mount_point.inode_ref())
            .ok_or(VfsError::InvalidRef)?;
        if !mount_point_inode.is_directory() {
            return Err(VfsError::NotDirectory);
        }
        Ok(())
    }

    fn find_child(&self, parent_ref: DentryRef, name: &[u8]) -> Result<DentryRef, VfsError> {
        validate_name(name)?;
        let parent_ref = self.follow_mount(parent_ref)?;
        let parent = self.positive_dentry(parent_ref)?;
        let parent_inode = self.inode(parent.inode_ref()).ok_or(VfsError::InvalidRef)?;
        if !parent_inode.is_directory() {
            return Err(VfsError::NotDirectory);
        }

        for child_ref in parent_inode.children.iter().copied() {
            let Some(child) = self.dentry(child_ref) else {
                return Err(VfsError::InvalidRef);
            };
            if child.name_eq(name) {
                return Ok(child_ref);
            }
        }
        Err(VfsError::NotFound)
    }

    fn live_child_count(&self, inode_ref: InodeRef) -> Result<usize, VfsError> {
        let inode = self.inode(inode_ref).ok_or(VfsError::InvalidRef)?;
        let mut count = 0usize;
        for child_ref in inode.children.iter().copied() {
            let child = self.dentry(child_ref).ok_or(VfsError::InvalidRef)?;
            if child.positive() && !child.removed() {
                count = count.saturating_add(1);
            }
        }
        Ok(count)
    }

    fn follow_mount(&self, dentry_ref: DentryRef) -> Result<DentryRef, VfsError> {
        let dentry = self.positive_dentry(dentry_ref)?;
        Ok(dentry.mounted_root().unwrap_or(dentry_ref))
    }

    fn positive_dentry(&self, dentry_ref: DentryRef) -> Result<&Dentry, VfsError> {
        let dentry = self.dentry(dentry_ref).ok_or(VfsError::InvalidRef)?;
        if !dentry.positive() || dentry.removed() {
            return Err(VfsError::NotFound);
        }
        Ok(dentry)
    }

    fn superblock_mut(&mut self, superblock_ref: SuperBlockRef) -> Option<&mut SuperBlock> {
        self.superblocks.get_mut(superblock_ref.index())
    }

    fn inode_mut(&mut self, inode_ref: InodeRef) -> Option<&mut Inode> {
        self.inodes.get_mut(inode_ref.index())
    }

    fn dentry_mut(&mut self, dentry_ref: DentryRef) -> Option<&mut Dentry> {
        self.dentries.get_mut(dentry_ref.index())
    }

    fn file_mut(&mut self, file_ref: FileRef) -> Option<&mut File> {
        self.files.get_mut(file_ref.index())
    }
}

fn validate_name(name: &[u8]) -> Result<(), VfsError> {
    if name.is_empty() || name.iter().any(|byte| *byte == 0 || *byte == b'/') {
        return Err(VfsError::InvalidName);
    }
    if name.len() > VFS_NAME_MAX {
        return Err(VfsError::NameTooLong);
    }
    Ok(())
}

fn copy_name(name: &[u8]) -> Result<([u8; VFS_NAME_MAX], usize), VfsError> {
    if name == b"/" {
        let mut out = [0u8; VFS_NAME_MAX];
        out[0] = b'/';
        return Ok((out, 1));
    }
    validate_name(name)?;
    let mut out = [0u8; VFS_NAME_MAX];
    out[..name.len()].copy_from_slice(name);
    Ok((out, name.len()))
}
