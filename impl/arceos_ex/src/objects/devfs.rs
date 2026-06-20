use super::{
    block_device::{BlockDeviceRef, BlockDeviceRegistry, DevT},
    hwrng::{HwRngCore, HwRngDeviceRef},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vfs::{DentryRef, FsStruct, MountRef, VfsCore, VfsError, VFS_NAME_MAX},
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum DevFsNodeKind {
    HwRng,
    BlockDevice,
}

#[derive(Clone, Copy)]
pub struct DevFsNode {
    dentry_ref: DentryRef,
    kind: DevFsNodeKind,
    name: [u8; VFS_NAME_MAX],
    name_len: usize,
    hwrng_device_ref: Option<HwRngDeviceRef>,
    block_device_ref: Option<BlockDeviceRef>,
    devt: Option<DevT>,
}

impl DevFsNode {
    fn new_hwrng(dentry_ref: DentryRef, hwrng_device_ref: HwRngDeviceRef) -> Self {
        let (name, name_len) = copy_node_name(b"hwrng");
        Self {
            dentry_ref,
            kind: DevFsNodeKind::HwRng,
            name,
            name_len,
            hwrng_device_ref: Some(hwrng_device_ref),
            block_device_ref: None,
            devt: None,
        }
    }

    fn new_block(
        dentry_ref: DentryRef,
        name: &[u8],
        block_device_ref: BlockDeviceRef,
        devt: DevT,
    ) -> Self {
        let (name, name_len) = copy_node_name(name);
        Self {
            dentry_ref,
            kind: DevFsNodeKind::BlockDevice,
            name,
            name_len,
            hwrng_device_ref: None,
            block_device_ref: Some(block_device_ref),
            devt: Some(devt),
        }
    }

    pub const fn dentry_ref(&self) -> DentryRef {
        self.dentry_ref
    }

    pub const fn kind(&self) -> DevFsNodeKind {
        self.kind
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub const fn hwrng_device_ref(&self) -> Option<HwRngDeviceRef> {
        self.hwrng_device_ref
    }

    pub const fn block_device_ref(&self) -> Option<BlockDeviceRef> {
        self.block_device_ref
    }

    pub const fn devt(&self) -> Option<DevT> {
        self.devt
    }
}

pub struct DevFs {
    lifecycle: Lifecycle,
    initialized: bool,
    mounted: bool,
    mount_ref: Option<MountRef>,
    mount_point_ref: Option<DentryRef>,
    root_dentry_ref: Option<DentryRef>,
    hwrng_node: Option<DevFsNode>,
    block_node: Option<DevFsNode>,
    device_file_ops_deferred: bool,
    uevent_deferred: bool,
    sysfs_deferred: bool,
}

impl DevFs {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            initialized: false,
            mounted: false,
            mount_ref: None,
            mount_point_ref: None,
            root_dentry_ref: None,
            hwrng_node: None,
            block_node: None,
            device_file_ops_deferred: false,
            uevent_deferred: false,
            sysfs_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn initialized(&self) -> bool {
        self.initialized
    }

    pub const fn mounted(&self) -> bool {
        self.mounted
    }

    pub const fn mount_ref(&self) -> Option<MountRef> {
        self.mount_ref
    }

    pub const fn mount_point_ref(&self) -> Option<DentryRef> {
        self.mount_point_ref
    }

    pub const fn root_dentry_ref(&self) -> Option<DentryRef> {
        self.root_dentry_ref
    }

    pub const fn hwrng_node(&self) -> Option<DevFsNode> {
        self.hwrng_node
    }

    pub const fn block_node(&self) -> Option<DevFsNode> {
        self.block_node
    }

    pub const fn hwrng_node_listed(&self) -> bool {
        self.hwrng_node.is_some()
    }

    pub const fn block_node_listed(&self) -> bool {
        self.block_node.is_some()
    }

    pub const fn node_count(&self) -> usize {
        self.hwrng_node.is_some() as usize + self.block_node.is_some() as usize
    }

    pub const fn device_file_ops_deferred(&self) -> bool {
        self.device_file_ops_deferred
    }

    pub const fn uevent_deferred(&self) -> bool {
        self.uevent_deferred
    }

    pub const fn sysfs_deferred(&self) -> bool {
        self.sysfs_deferred
    }

    pub fn setup(
        &mut self,
        vfs: &mut VfsCore,
        fs_struct: &FsStruct,
        hwrng_core: &HwRngCore,
        block_registry: &BlockDeviceRegistry,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || vfs.state() != State::Ready
            || !vfs.rootfs_mount_created()
            || fs_struct.state() != State::Ready
            || hwrng_core.state() != State::Ready
            || !hwrng_core.current_slot_ready()
            || block_registry.state() != State::Ready
            || !block_registry.default_device_slot_ready()
        {
            return self.setup_failed();
        }

        let Some(hwrng_device_ref) = hwrng_core.current_device() else {
            return self.setup_failed();
        };
        let Some(hwrng_entry) = hwrng_core.device(hwrng_device_ref) else {
            return self.setup_failed();
        };
        if !hwrng_entry.registered() || !hwrng_entry.current() {
            return self.setup_failed();
        }

        let Some(block_device_ref) = block_registry.default_device() else {
            return self.setup_failed();
        };
        let Some(block_entry) = block_registry.device(block_device_ref) else {
            return self.setup_failed();
        };
        if !block_entry.registered() || !block_entry.default_device() {
            return self.setup_failed();
        }

        let Some(root_ref) = fs_struct.root_dentry() else {
            return self.setup_failed();
        };

        let mount_point_ref = match vfs.lookup_child(root_ref, b"dev") {
            Ok(dentry_ref) => dentry_ref,
            Err(VfsError::NotFound) => match vfs.create_dir(root_ref, b"dev") {
                Ok(dentry_ref) => dentry_ref,
                Err(_) => return self.setup_failed(),
            },
            Err(_) => return self.setup_failed(),
        };

        let mount_ref = match vfs.mount_devfs_at(mount_point_ref) {
            Ok(mount_ref) => mount_ref,
            Err(_) => return self.setup_failed(),
        };
        let Some(dev_root_ref) = vfs.mount(mount_ref).map(|mount| mount.root_dentry_ref()) else {
            return self.setup_failed();
        };

        let hwrng_dentry_ref = match vfs.create_device_node(dev_root_ref, b"hwrng") {
            Ok(dentry_ref) => dentry_ref,
            Err(_) => return self.setup_failed(),
        };
        let block_dentry_ref = match vfs.create_device_node(dev_root_ref, block_entry.name()) {
            Ok(dentry_ref) => dentry_ref,
            Err(_) => return self.setup_failed(),
        };

        self.initialized = true;
        self.mounted = true;
        self.mount_ref = Some(mount_ref);
        self.mount_point_ref = Some(mount_point_ref);
        self.root_dentry_ref = Some(dev_root_ref);
        self.hwrng_node = Some(DevFsNode::new_hwrng(hwrng_dentry_ref, hwrng_device_ref));
        self.block_node = Some(DevFsNode::new_block(
            block_dentry_ref,
            block_entry.name(),
            block_device_ref,
            block_entry.devt(),
        ));
        self.device_file_ops_deferred = true;
        self.uevent_deferred = true;
        self.sysfs_deferred = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn setup_failed(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

fn copy_node_name(name: &[u8]) -> ([u8; VFS_NAME_MAX], usize) {
    let mut out = [0u8; VFS_NAME_MAX];
    let len = core::cmp::min(name.len(), VFS_NAME_MAX);
    out[..len].copy_from_slice(&name[..len]);
    (out, len)
}
