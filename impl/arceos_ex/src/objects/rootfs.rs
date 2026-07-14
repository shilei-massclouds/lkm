use super::{
    block_device::{BlockDeviceRef, BlockDeviceRegistry, DevT},
    command_line::SavedCommandLine,
    config::Config,
    devfs::DevFs,
    ext2::{Ext2Driver, Ext2FileSystem, Ext2Volume},
    initcall::{DriverCoreBase, InitcallBoundary},
    rest_init::{KERNEL_INIT_PID, KernelInitTask},
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    vfs::{DentryRef, FileSystemKind, FsStruct, MountRef, VfsCore},
    workqueue::Workqueue,
};
use crate::checkpoint::Checkpoint;

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

pub struct RootfsPrepareNamespacePaths {
    lifecycle: Lifecycle,
    root_delay_trimmed: bool,
    device_probe_wait_deferred: bool,
    device_probe_waitqueue_deferred: bool,
    device_probe_atomic_counter_deferred: bool,
    deferred_probe_work_flush_deferred: bool,
    md_run_setup_deferred: bool,
    saved_root_name_parse_deferred: bool,
    root_device_parse_deferred: bool,
    initrd_load_trimmed: bool,
    root_wait_trimmed: bool,
    root_wait_polling_deferred: bool,
    mount_root_block_formal: bool,
    nfs_root_deferred: bool,
    cifs_root_trimmed: bool,
    nodev_root_deferred: bool,
    ext4_for_ext2_linux_config_recorded: bool,
    arceos_ext2_driver_substitutes_linux_ext4_for_ext2: bool,
    devtmpfs_mount_deferred: bool,
    devfs_not_remounted_after_root_switch: bool,
}

impl RootfsPrepareNamespacePaths {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            root_delay_trimmed: false,
            device_probe_wait_deferred: false,
            device_probe_waitqueue_deferred: false,
            device_probe_atomic_counter_deferred: false,
            deferred_probe_work_flush_deferred: false,
            md_run_setup_deferred: false,
            saved_root_name_parse_deferred: false,
            root_device_parse_deferred: false,
            initrd_load_trimmed: false,
            root_wait_trimmed: false,
            root_wait_polling_deferred: false,
            mount_root_block_formal: false,
            nfs_root_deferred: false,
            cifs_root_trimmed: false,
            nodev_root_deferred: false,
            ext4_for_ext2_linux_config_recorded: false,
            arceos_ext2_driver_substitutes_linux_ext4_for_ext2: false,
            devtmpfs_mount_deferred: false,
            devfs_not_remounted_after_root_switch: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn root_delay_trimmed(&self) -> bool {
        self.root_delay_trimmed
    }

    pub const fn device_probe_wait_deferred(&self) -> bool {
        self.device_probe_wait_deferred
    }

    pub const fn device_probe_waitqueue_deferred(&self) -> bool {
        self.device_probe_waitqueue_deferred
    }

    pub const fn device_probe_atomic_counter_deferred(&self) -> bool {
        self.device_probe_atomic_counter_deferred
    }

    pub const fn deferred_probe_work_flush_deferred(&self) -> bool {
        self.deferred_probe_work_flush_deferred
    }

    pub const fn md_run_setup_deferred(&self) -> bool {
        self.md_run_setup_deferred
    }

    pub const fn saved_root_name_parse_deferred(&self) -> bool {
        self.saved_root_name_parse_deferred
    }

    pub const fn root_device_parse_deferred(&self) -> bool {
        self.root_device_parse_deferred
    }

    pub const fn initrd_load_trimmed(&self) -> bool {
        self.initrd_load_trimmed
    }

    pub const fn root_wait_trimmed(&self) -> bool {
        self.root_wait_trimmed
    }

    pub const fn root_wait_polling_deferred(&self) -> bool {
        self.root_wait_polling_deferred
    }

    pub const fn mount_root_block_formal(&self) -> bool {
        self.mount_root_block_formal
    }

    pub const fn nfs_root_deferred(&self) -> bool {
        self.nfs_root_deferred
    }

    pub const fn cifs_root_trimmed(&self) -> bool {
        self.cifs_root_trimmed
    }

    pub const fn nodev_root_deferred(&self) -> bool {
        self.nodev_root_deferred
    }

    pub const fn ext4_for_ext2_linux_config_recorded(&self) -> bool {
        self.ext4_for_ext2_linux_config_recorded
    }

    pub const fn arceos_ext2_driver_substitutes_linux_ext4_for_ext2(&self) -> bool {
        self.arceos_ext2_driver_substitutes_linux_ext4_for_ext2
    }

    pub const fn devtmpfs_mount_deferred(&self) -> bool {
        self.devtmpfs_mount_deferred
    }

    pub const fn devfs_not_remounted_after_root_switch(&self) -> bool {
        self.devfs_not_remounted_after_root_switch
    }

    pub fn setup(
        &mut self,
        rootfs_console: &RootfsConsoleDeferred,
        saved_command_line: &SavedCommandLine,
        driver_core: &DriverCoreBase,
        workqueue: &Workqueue,
        initcall_boundary: &InitcallBoundary,
        config: &Config,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rootfs_console.state() != State::Ready
            || !rootfs_console.setup_deferred()
            || saved_command_line.state() != State::Ready
            || driver_core.state() != State::Ready
            || workqueue.state() != State::Ready
            || initcall_boundary.state() != State::Ready
            || !initcall_boundary.kunit_next_boundary()
            || config.kunit_enabled()
            || config.blk_dev_initrd_enabled()
            || !config.md_enabled()
            || !config.root_nfs_enabled()
            || config.cifs_root_enabled()
            || config.ext2_fs_enabled()
            || !config.ext4_use_for_ext2_enabled()
            || !config.devtmpfs_enabled()
            || !config.devtmpfs_mount_enabled()
            || saved_command_line.has_token_prefix(b"rootdelay=")
            || saved_command_line.has_token(b"rootwait")
            || saved_command_line.has_token_prefix(b"rootwait=")
            || saved_command_line.root_value_is(b"/dev/nfs")
            || saved_command_line.root_value_is(b"/dev/cifs")
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.root_delay_trimmed = true;
        self.device_probe_wait_deferred = true;
        self.device_probe_waitqueue_deferred = true;
        self.device_probe_atomic_counter_deferred = true;
        self.deferred_probe_work_flush_deferred = true;
        self.md_run_setup_deferred = true;
        self.saved_root_name_parse_deferred = true;
        self.root_device_parse_deferred = true;
        self.initrd_load_trimmed = true;
        self.root_wait_trimmed = true;
        self.root_wait_polling_deferred = true;
        self.mount_root_block_formal = true;
        self.nfs_root_deferred = true;
        self.cifs_root_trimmed = true;
        self.nodev_root_deferred = true;
        self.ext4_for_ext2_linux_config_recorded = true;
        self.arceos_ext2_driver_substitutes_linux_ext4_for_ext2 = true;
        self.devtmpfs_mount_deferred = true;
        self.devfs_not_remounted_after_root_switch = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RootfsPrepareNamespacePathsReady,
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
        namespace_paths: &RootfsPrepareNamespacePaths,
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
            || namespace_paths.state() != State::Ready
            || !namespace_paths.mount_root_block_formal()
            || !namespace_paths.ext4_for_ext2_linux_config_recorded()
            || !namespace_paths.arceos_ext2_driver_substitutes_linux_ext4_for_ext2()
            || !namespace_paths.devfs_not_remounted_after_root_switch()
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
    ima_load_x509_deferred: bool,
    ima_load_x509_trimmed_because_config_ima_disabled: bool,
    evm_load_x509_deferred: bool,
    evm_load_x509_trimmed_because_config_evm_disabled: bool,
}

impl IntegrityKeysDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            setup_deferred: false,
            load_keys_position_preserved: false,
            config_integrity_enabled: false,
            ima_load_x509_deferred: false,
            ima_load_x509_trimmed_because_config_ima_disabled: false,
            evm_load_x509_deferred: false,
            evm_load_x509_trimmed_because_config_evm_disabled: false,
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

    pub const fn ima_load_x509_deferred(&self) -> bool {
        self.ima_load_x509_deferred
    }

    pub const fn ima_load_x509_trimmed_because_config_ima_disabled(&self) -> bool {
        self.ima_load_x509_trimmed_because_config_ima_disabled
    }

    pub const fn evm_load_x509_deferred(&self) -> bool {
        self.evm_load_x509_deferred
    }

    pub const fn evm_load_x509_trimmed_because_config_evm_disabled(&self) -> bool {
        self.evm_load_x509_trimmed_because_config_evm_disabled
    }

    pub fn setup(&mut self, rootfs: &RootFS, config: &Config) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rootfs.state() != State::Online
            || !rootfs.prepare_namespace_inputs_ready()
            || !rootfs.real_ext2_mount_created()
            || !rootfs.ms_move_done()
            || !config.integrity_enabled()
            || config.ima_enabled()
            || config.evm_enabled()
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
        self.ima_load_x509_deferred = true;
        self.ima_load_x509_trimmed_because_config_ima_disabled = true;
        self.evm_load_x509_deferred = true;
        self.evm_load_x509_trimmed_because_config_evm_disabled = true;
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
        namespace_paths: &RootfsPrepareNamespacePaths,
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
            || namespace_paths.state() != State::Ready
            || !namespace_paths.device_probe_wait_deferred()
            || !namespace_paths.md_run_setup_deferred()
            || !namespace_paths.initrd_load_trimmed()
            || !namespace_paths.devtmpfs_mount_deferred()
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
    namespace_paths: &RootfsPrepareNamespacePaths,
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
        && namespace_paths.state() == State::Ready
        && namespace_paths.root_delay_trimmed()
        && namespace_paths.device_probe_wait_deferred()
        && namespace_paths.device_probe_waitqueue_deferred()
        && namespace_paths.device_probe_atomic_counter_deferred()
        && namespace_paths.deferred_probe_work_flush_deferred()
        && namespace_paths.md_run_setup_deferred()
        && namespace_paths.saved_root_name_parse_deferred()
        && namespace_paths.root_device_parse_deferred()
        && namespace_paths.initrd_load_trimmed()
        && namespace_paths.root_wait_trimmed()
        && namespace_paths.root_wait_polling_deferred()
        && namespace_paths.mount_root_block_formal()
        && namespace_paths.nfs_root_deferred()
        && namespace_paths.cifs_root_trimmed()
        && namespace_paths.nodev_root_deferred()
        && namespace_paths.ext4_for_ext2_linux_config_recorded()
        && namespace_paths.arceos_ext2_driver_substitutes_linux_ext4_for_ext2()
        && namespace_paths.devtmpfs_mount_deferred()
        && namespace_paths.devfs_not_remounted_after_root_switch()
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
        && integrity_keys.ima_load_x509_deferred()
        && integrity_keys.ima_load_x509_trimmed_because_config_ima_disabled()
        && integrity_keys.evm_load_x509_deferred()
        && integrity_keys.evm_load_x509_trimmed_because_config_evm_disabled()
        && boundary.state() == State::Ready
        && boundary.finalize_next_boundary()
}
