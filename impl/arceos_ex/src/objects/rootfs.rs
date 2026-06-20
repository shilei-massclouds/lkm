use super::{
    block_device::{BlockDeviceRef, BlockDeviceRegistry, DevT},
    command_line::SavedCommandLine,
    devfs::DevFs,
    ext2::{Ext2Driver, Ext2FileSystem, Ext2Volume},
    initcall::InitcallBoundary,
    rest_init::{KernelInitTask, KERNEL_INIT_PID},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vfs::{DentryRef, FileSystemKind, FsStruct, MountRef, VfsCore},
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

pub const ROOTFS_REAL_MOUNT_POINT_NAME: &[u8] = b"root";

pub struct KUnitRuntimeTrimmed {
    lifecycle: Lifecycle,
    trimmed_noop: bool,
    position_preserved: bool,
}

impl KUnitRuntimeTrimmed {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trimmed_noop: false,
            position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn trimmed_noop(&self) -> bool {
        self.trimmed_noop
    }

    pub const fn position_preserved(&self) -> bool {
        self.position_preserved
    }

    pub fn setup(&mut self, initcall_boundary: &InitcallBoundary) -> EventResult {
        if self.lifecycle.state() != State::Base
            || initcall_boundary.state() != State::Ready
            || !initcall_boundary.kunit_next_boundary()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.trimmed_noop = true;
        self.position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KUnitRuntimeTrimmedReady,
        )
    }
}

pub struct InitramfsSyncDeferred {
    lifecycle: Lifecycle,
    wait_deferred: bool,
    async_cookie_boundary_preserved: bool,
}

impl InitramfsSyncDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            wait_deferred: false,
            async_cookie_boundary_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn wait_deferred(&self) -> bool {
        self.wait_deferred
    }

    pub const fn async_cookie_boundary_preserved(&self) -> bool {
        self.async_cookie_boundary_preserved
    }

    pub fn setup(&mut self, kunit: &KUnitRuntimeTrimmed, workqueue: &Workqueue) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kunit.state() != State::Ready
            || !kunit.trimmed_noop()
            || workqueue.state() != State::Ready
            || !workqueue.topology_ready()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.wait_deferred = true;
        self.async_cookie_boundary_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::InitramfsSyncDeferredReady,
        )
    }
}

pub struct RootfsConsoleDeferred {
    lifecycle: Lifecycle,
    setup_deferred: bool,
    pid1_console_fd_position_preserved: bool,
}

impl RootfsConsoleDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            pid1_console_fd_position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn setup_deferred(&self) -> bool {
        self.setup_deferred
    }

    pub const fn pid1_console_fd_position_preserved(&self) -> bool {
        self.pid1_console_fd_position_preserved
    }

    pub fn setup(
        &mut self,
        initramfs_sync: &InitramfsSyncDeferred,
        kernel_init_task: &KernelInitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || initramfs_sync.state() != State::Ready
            || !initramfs_sync.wait_deferred()
            || kernel_init_task.state() != State::Online
            || kernel_init_task.pid() != KERNEL_INIT_PID
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.setup_deferred = true;
        self.pid1_console_fd_position_preserved = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RootfsConsoleDeferredReady,
        )
    }
}

pub struct RootFS {
    lifecycle: Lifecycle,
    ramdisk_eaccess_requires_prepare_namespace: bool,
    prepare_namespace_inputs_ready: bool,
    initial_ramfs_still_active: bool,
    devfs_available: bool,
    block_root_device_candidate_bound: bool,
    root_device_ref: Option<BlockDeviceRef>,
    root_device_devt: Option<DevT>,
    ext2_driver_ready: bool,
    ext2_volume_ready: bool,
    ext2_filesystem_ready: bool,
    real_mount_point_created: bool,
    real_ext2_mount_created: bool,
    real_mount_point_ref: Option<DentryRef>,
    real_mount_ref: Option<MountRef>,
    prepare_namespace_position_preserved: bool,
    ms_move_done: bool,
    chroot_dot_done: bool,
    current_root_is_real_ext2: bool,
}

impl RootFS {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Ready),
            ramdisk_eaccess_requires_prepare_namespace: false,
            prepare_namespace_inputs_ready: false,
            initial_ramfs_still_active: false,
            devfs_available: false,
            block_root_device_candidate_bound: false,
            root_device_ref: None,
            root_device_devt: None,
            ext2_driver_ready: false,
            ext2_volume_ready: false,
            ext2_filesystem_ready: false,
            real_mount_point_created: false,
            real_ext2_mount_created: false,
            real_mount_point_ref: None,
            real_mount_ref: None,
            prepare_namespace_position_preserved: false,
            ms_move_done: false,
            chroot_dot_done: false,
            current_root_is_real_ext2: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ramdisk_eaccess_requires_prepare_namespace(&self) -> bool {
        self.ramdisk_eaccess_requires_prepare_namespace
    }

    pub const fn prepare_namespace_inputs_ready(&self) -> bool {
        self.prepare_namespace_inputs_ready
    }

    pub const fn initial_ramfs_still_active(&self) -> bool {
        self.initial_ramfs_still_active
    }

    pub const fn devfs_available(&self) -> bool {
        self.devfs_available
    }

    pub const fn block_root_device_candidate_bound(&self) -> bool {
        self.block_root_device_candidate_bound
    }

    pub const fn root_device_ref(&self) -> Option<BlockDeviceRef> {
        self.root_device_ref
    }

    pub const fn root_device_devt(&self) -> Option<DevT> {
        self.root_device_devt
    }

    pub const fn ext2_driver_ready(&self) -> bool {
        self.ext2_driver_ready
    }

    pub const fn ext2_volume_ready(&self) -> bool {
        self.ext2_volume_ready
    }

    pub const fn ext2_filesystem_ready(&self) -> bool {
        self.ext2_filesystem_ready
    }

    pub const fn real_mount_point_created(&self) -> bool {
        self.real_mount_point_created
    }

    pub const fn real_ext2_mount_created(&self) -> bool {
        self.real_ext2_mount_created
    }

    pub const fn real_mount_point_ref(&self) -> Option<DentryRef> {
        self.real_mount_point_ref
    }

    pub const fn real_mount_ref(&self) -> Option<MountRef> {
        self.real_mount_ref
    }

    pub const fn prepare_namespace_position_preserved(&self) -> bool {
        self.prepare_namespace_position_preserved
    }

    pub const fn ms_move_done(&self) -> bool {
        self.ms_move_done
    }

    pub const fn chroot_dot_done(&self) -> bool {
        self.chroot_dot_done
    }

    pub const fn current_root_is_real_ext2(&self) -> bool {
        self.current_root_is_real_ext2
    }

    pub fn enable(
        &mut self,
        rootfs_console: &RootfsConsoleDeferred,
        saved_command_line: &SavedCommandLine,
        kernel_init_task: &KernelInitTask,
        vfs_core: &mut VfsCore,
        fs_struct: &mut FsStruct,
        devfs: &DevFs,
        block_registry: &BlockDeviceRegistry,
        ext2_driver: &Ext2Driver,
        ext2_volume: &Ext2Volume,
        ext2_filesystem: &mut Ext2FileSystem,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || rootfs_console.state() != State::Ready
            || !rootfs_console.setup_deferred()
            || saved_command_line.state() != State::Ready
            || kernel_init_task.state() != State::Online
            || kernel_init_task.pid() != KERNEL_INIT_PID
            || vfs_core.state() != State::Ready
            || !vfs_core.rootfs_mount_created()
            || fs_struct.state() != State::Ready
            || devfs.state() != State::Ready
            || !devfs.mounted()
            || !devfs.block_node_listed()
            || block_registry.state() != State::Ready
            || !block_registry.default_device_slot_ready()
            || ext2_driver.state() != State::Ready
            || ext2_volume.state() != State::Ready
            || ext2_filesystem.state() != State::Ready
            || !ext2_filesystem.ready()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let Some(root_mount_ref) = vfs_core.initial_root_mount() else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        let Some(root_mount) = vfs_core.mount(root_mount_ref) else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        if root_mount.fs_kind() != FileSystemKind::RamFs {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        let Some(root_dentry_ref) = fs_struct.root_dentry() else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        if root_dentry_ref != root_mount.root_dentry_ref()
            || fs_struct.pwd_dentry() != Some(root_dentry_ref)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let Some(default_entry) = block_registry.default_entry() else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        let Some(block_node) = devfs.block_node() else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        if block_node.block_device_ref() != Some(default_entry.device_ref())
            || block_node.devt() != Some(default_entry.devt())
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let mount_point_ref = match vfs_core
            .lookup_child(root_dentry_ref, ROOTFS_REAL_MOUNT_POINT_NAME)
        {
            Ok(dentry_ref) => dentry_ref,
            Err(_) => match vfs_core.create_dir(root_dentry_ref, ROOTFS_REAL_MOUNT_POINT_NAME) {
                Ok(dentry_ref) => dentry_ref,
                Err(_) => {
                    return failed_condition(
                        LifecycleEvent::Enable,
                        self.lifecycle.state(),
                        State::Ready,
                        State::Online,
                    );
                }
            },
        };
        let Ok(mount_ref) = ext2_filesystem.enable(vfs_core, mount_point_ref) else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        let Ok(real_root_ref) = vfs_core.move_mount_to_root(mount_ref, root_dentry_ref) else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        if fs_struct.chdir(vfs_core, real_root_ref).is_err()
            || fs_struct.chroot_dot(vfs_core).is_err()
            || !vfs_core.current_root_is_ext2(fs_struct)
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.ramdisk_eaccess_requires_prepare_namespace = true;
        self.prepare_namespace_inputs_ready = true;
        self.initial_ramfs_still_active = true;
        self.devfs_available = true;
        self.block_root_device_candidate_bound = true;
        self.root_device_ref = Some(default_entry.device_ref());
        self.root_device_devt = Some(default_entry.devt());
        self.ext2_driver_ready = true;
        self.ext2_volume_ready = true;
        self.ext2_filesystem_ready = true;
        self.real_mount_point_created = true;
        self.real_ext2_mount_created = true;
        self.real_mount_point_ref = Some(mount_point_ref);
        self.real_mount_ref = Some(mount_ref);
        self.prepare_namespace_position_preserved = true;
        self.ms_move_done = true;
        self.chroot_dot_done = true;
        self.current_root_is_real_ext2 = true;
        crate::trace::checkpoint(Checkpoint::RamdiskExecuteCommandEaccessCheckpoint);
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::RootFSOnline,
        )
    }
}

pub struct IntegrityKeysDeferred {
    lifecycle: Lifecycle,
    setup_deferred: bool,
    load_keys_position_preserved: bool,
    config_integrity_enabled: bool,
}

impl IntegrityKeysDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            load_keys_position_preserved: false,
            config_integrity_enabled: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn setup_deferred(&self) -> bool {
        self.setup_deferred
    }

    pub const fn load_keys_position_preserved(&self) -> bool {
        self.load_keys_position_preserved
    }

    pub const fn config_integrity_enabled(&self) -> bool {
        self.config_integrity_enabled
    }

    pub fn setup(&mut self, rootfs: &RootFS) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rootfs.state() != State::Online
            || !rootfs.prepare_namespace_inputs_ready()
            || !rootfs.real_ext2_mount_created()
            || !rootfs.ms_move_done()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.setup_deferred = true;
        self.load_keys_position_preserved = true;
        self.config_integrity_enabled = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::IntegrityKeysDeferredReady,
        )
    }
}

pub struct RootfsBoundary {
    lifecycle: Lifecycle,
    finalize_next_boundary: bool,
}

impl RootfsBoundary {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            finalize_next_boundary: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn finalize_next_boundary(&self) -> bool {
        self.finalize_next_boundary
    }

    pub fn setup(
        &mut self,
        kunit: &KUnitRuntimeTrimmed,
        initramfs_sync: &InitramfsSyncDeferred,
        rootfs_console: &RootfsConsoleDeferred,
        rootfs: &RootFS,
        integrity_keys: &IntegrityKeysDeferred,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kunit.state() != State::Ready
            || !kunit.trimmed_noop()
            || initramfs_sync.state() != State::Ready
            || !initramfs_sync.wait_deferred()
            || rootfs_console.state() != State::Ready
            || !rootfs_console.setup_deferred()
            || rootfs.state() != State::Online
            || !rootfs.ramdisk_eaccess_requires_prepare_namespace()
            || !rootfs.prepare_namespace_inputs_ready()
            || !rootfs.initial_ramfs_still_active()
            || !rootfs.devfs_available()
            || !rootfs.block_root_device_candidate_bound()
            || !rootfs.ext2_driver_ready()
            || !rootfs.ext2_volume_ready()
            || !rootfs.ext2_filesystem_ready()
            || !rootfs.real_mount_point_created()
            || !rootfs.real_ext2_mount_created()
            || !rootfs.ms_move_done()
            || !rootfs.chroot_dot_done()
            || !rootfs.current_root_is_real_ext2()
            || integrity_keys.state() != State::Ready
            || !integrity_keys.setup_deferred()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.finalize_next_boundary = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RootfsBoundaryReady,
        )
    }
}

pub fn rootfs_phase_ready(
    kunit: &KUnitRuntimeTrimmed,
    initramfs_sync: &InitramfsSyncDeferred,
    rootfs_console: &RootfsConsoleDeferred,
    rootfs: &RootFS,
    integrity_keys: &IntegrityKeysDeferred,
    boundary: &RootfsBoundary,
) -> bool {
    kunit.state() == State::Ready
        && kunit.trimmed_noop()
        && kunit.position_preserved()
        && initramfs_sync.state() == State::Ready
        && initramfs_sync.wait_deferred()
        && initramfs_sync.async_cookie_boundary_preserved()
        && rootfs_console.state() == State::Ready
        && rootfs_console.setup_deferred()
        && rootfs_console.pid1_console_fd_position_preserved()
        && rootfs.state() == State::Online
        && rootfs.ramdisk_eaccess_requires_prepare_namespace()
        && rootfs.prepare_namespace_inputs_ready()
        && rootfs.initial_ramfs_still_active()
        && rootfs.devfs_available()
        && rootfs.block_root_device_candidate_bound()
        && rootfs.root_device_ref().is_some()
        && rootfs.root_device_devt().is_some()
        && rootfs.ext2_driver_ready()
        && rootfs.ext2_volume_ready()
        && rootfs.ext2_filesystem_ready()
        && rootfs.real_mount_point_created()
        && rootfs.real_ext2_mount_created()
        && rootfs.real_mount_point_ref().is_some()
        && rootfs.real_mount_ref().is_some()
        && rootfs.prepare_namespace_position_preserved()
        && rootfs.ms_move_done()
        && rootfs.chroot_dot_done()
        && rootfs.current_root_is_real_ext2()
        && integrity_keys.state() == State::Ready
        && integrity_keys.setup_deferred()
        && integrity_keys.load_keys_position_preserved()
        && integrity_keys.config_integrity_enabled()
        && boundary.state() == State::Ready
        && boundary.finalize_next_boundary()
}
