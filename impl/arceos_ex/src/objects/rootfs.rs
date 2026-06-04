use super::{
    command_line::SavedCommandLine,
    initcall::InitcallBoundary,
    rest_init::{KernelInitTask, KERNEL_INIT_PID},
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

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

pub struct RootFsEnableDeferred {
    lifecycle: Lifecycle,
    ramdisk_eaccess_requires_prepare_namespace: bool,
    enable_deferred: bool,
    prepare_namespace_position_preserved: bool,
}

impl RootFsEnableDeferred {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            ramdisk_eaccess_requires_prepare_namespace: false,
            enable_deferred: false,
            prepare_namespace_position_preserved: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn ramdisk_eaccess_requires_prepare_namespace(&self) -> bool {
        self.ramdisk_eaccess_requires_prepare_namespace
    }

    pub const fn enable_deferred(&self) -> bool {
        self.enable_deferred
    }

    pub const fn prepare_namespace_position_preserved(&self) -> bool {
        self.prepare_namespace_position_preserved
    }

    pub fn setup(
        &mut self,
        rootfs_console: &RootfsConsoleDeferred,
        saved_command_line: &SavedCommandLine,
        kernel_init_task: &KernelInitTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rootfs_console.state() != State::Ready
            || !rootfs_console.setup_deferred()
            || saved_command_line.state() != State::Ready
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

        self.ramdisk_eaccess_requires_prepare_namespace = true;
        self.enable_deferred = true;
        self.prepare_namespace_position_preserved = true;
        crate::trace::checkpoint(Checkpoint::RamdiskExecuteCommandEaccessCheckpoint);
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RootFsEnableDeferredReady,
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

    pub fn setup(&mut self, rootfs_enable: &RootFsEnableDeferred) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rootfs_enable.state() != State::Ready
            || !rootfs_enable.enable_deferred()
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
        rootfs_enable: &RootFsEnableDeferred,
        integrity_keys: &IntegrityKeysDeferred,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kunit.state() != State::Ready
            || !kunit.trimmed_noop()
            || initramfs_sync.state() != State::Ready
            || !initramfs_sync.wait_deferred()
            || rootfs_console.state() != State::Ready
            || !rootfs_console.setup_deferred()
            || rootfs_enable.state() != State::Ready
            || !rootfs_enable.ramdisk_eaccess_requires_prepare_namespace()
            || !rootfs_enable.enable_deferred()
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
    rootfs_enable: &RootFsEnableDeferred,
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
        && rootfs_enable.state() == State::Ready
        && rootfs_enable.ramdisk_eaccess_requires_prepare_namespace()
        && rootfs_enable.enable_deferred()
        && rootfs_enable.prepare_namespace_position_preserved()
        && integrity_keys.state() == State::Ready
        && integrity_keys.setup_deferred()
        && integrity_keys.load_keys_position_preserved()
        && integrity_keys.config_integrity_enabled()
        && boundary.state() == State::Ready
        && boundary.finalize_next_boundary()
}
