/*
 * Rootfs Phase Specification
 *
 * This is SMP Runtime Phase subphase 4. It covers kernel_init_freeable()
 * from kunit_run_all_tests() through integrity_load_keys(), before the final
 * cleanup path that returns toward the payload entry.
 */

/*
 * KUnitRuntimeTrimmed preserves the position of kunit_run_all_tests().
 * CONFIG_KUNIT=n in the current Linux-like configuration, so this call is a
 * trimmed no-op rather than a separate phase.
 */
object KUnitRuntimeTrimmed: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    InitcallBoundary.state == State::Ready;
                }

                ensures {
                    kunit_runtime_trimmed_noop();
                    kunit_runtime_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kunit_runtime_trimmed_noop();
            kunit_runtime_position_preserved();
        }
    }
}

/*
 * InitramfsSyncDeferred preserves wait_for_initramfs(). The async
 * domain/cookie protocol is not expanded in this round.
 */
object InitramfsSyncDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KUnitRuntimeTrimmed.state == State::Ready;
                    Workqueue.state == State::Ready;
                }

                ensures {
                    initramfs_sync_wait_deferred();
                    initramfs_async_cookie_boundary_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            initramfs_sync_wait_deferred();
            initramfs_async_cookie_boundary_preserved();
        }
    }
}

/*
 * RootfsConsoleDeferred preserves console_on_rootfs(). The /dev/console open
 * and stdin/stdout/stderr duplication details are kept for a later VFS/files
 * round.
 */
object RootfsConsoleDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    InitramfsSyncDeferred.state == State::Ready;
                    KernelInitTask.state == State::Online;
                }

                ensures {
                    rootfs_console_setup_deferred();
                    pid1_console_fd_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            rootfs_console_setup_deferred();
            pid1_console_fd_position_preserved();
        }
    }
}

/*
 * RootfsPrepareNamespacePaths records the Linux prepare_namespace() branches
 * that are not expanded by the current RootFS enable path. It does not split
 * RootfsPhase: RootFS.Transition::Enable remains the formal prepare_namespace
 * event, while this object preserves the classification and synchronization
 * obligations of the surrounding Linux calls.
 * For Linux paired-diff ordering, the `RootfsPhase.Started` checkpoint is not
 * emitted at the Rust phase function entry. Its exact Linux anchor is the
 * prepare_namespace() branch after init_eaccess(ramdisk_execute_command), so
 * `RamdiskExecuteCommand.EaccessCheckpoint` must precede it and this
 * classification object follows it.
 */
object RootfsPrepareNamespacePaths: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    RootfsConsoleDeferred.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                    DriverCoreBase.state == State::Ready;
                    Workqueue.state == State::Ready;
                    InitcallBoundary.state == State::Ready;
                }

                ensures {
                    rootfs_root_delay_trimmed_noop();
                    rootfs_root_delay_trimmed_because_cmdline_absent();
                    rootfs_device_probe_wait_deferred();
                    rootfs_device_probe_waitqueue_deferred();
                    rootfs_device_probe_atomic_counter_deferred();
                    rootfs_deferred_probe_work_flush_deferred();
                    rootfs_md_run_setup_deferred();
                    rootfs_md_run_setup_deferred_because_config_md_enabled();
                    rootfs_saved_root_name_parse_deferred();
                    rootfs_root_device_parse_deferred();
                    rootfs_initrd_load_trimmed_noop();
                    rootfs_initrd_load_trimmed_because_config_blk_dev_initrd_disabled();
                    rootfs_root_wait_trimmed_noop();
                    rootfs_root_wait_trimmed_because_cmdline_absent();
                    rootfs_root_wait_polling_deferred();
                    rootfs_mount_root_block_formal();
                    rootfs_nfs_root_trimmed_by_reference_input();
                    rootfs_cifs_root_trimmed_noop();
                    rootfs_nodev_root_deferred();
                    rootfs_ext4_for_ext2_linux_config_recorded();
                    rootfs_arceos_ext2_driver_substitutes_linux_ext4_for_ext2();
                    rootfs_devtmpfs_mount_deferred();
                    rootfs_devtmpfs_mount_deferred_because_config_devtmpfs_enabled();
                    rootfs_devfs_currently_not_remounted_after_root_switch();
                }

                deferred rootfs.001 {
                    category: DeferredCategory::Protocol;
                    summary: "Complete wait_for_device_probe synchronization over deferred work, probe_count and probe_waitqueue.";
                    evidence {
                        rootfs_device_probe_wait_deferred();
                        rootfs_device_probe_waitqueue_deferred();
                        rootfs_device_probe_atomic_counter_deferred();
                        rootfs_deferred_probe_work_flush_deferred();
                    }
                    close_when: "Probe work flush, atomic accounting and wait/wake tests pass under concurrent probing.";
                }
                deferred rootfs.002 {
                    category: DeferredCategory::Feature;
                    summary: "Implement MD autodetection and assembly during root setup.";
                    evidence { rootfs_md_run_setup_deferred(); }
                    close_when: "MD discovery, assembly, failure and root-selection tests pass.";
                }
                deferred rootfs.003 {
                    category: DeferredCategory::ModelDetail;
                    summary: "Model saved_root_name and ROOT_DEV ownership.";
                    evidence { rootfs_saved_root_name_parse_deferred(); }
                    close_when: "Root-name storage, parsing lifetime and override tests pass.";
                }
                deferred rootfs.004 {
                    category: DeferredCategory::AlternatePath;
                    summary: "Parse root= device variants beyond the default block-device candidate.";
                    evidence {
                        rootfs_root_device_parse_deferred();
                        rootfs_nodev_root_deferred();
                    }
                    close_when: "Supported MTD/UBI/NFS/CIFS/ram/block root variants have Linux-compatible selection and errno tests.";
                }
                trimmed rootfs.005 {
                    category: TrimmedCategory::ReferenceInput;
                    summary: "root_wait/wait_for_root is unreachable because the fixed command line has no rootwait option.";
                    evidence {
                        rootfs_root_wait_trimmed_noop();
                        rootfs_root_wait_trimmed_because_cmdline_absent();
                    }
                    revisit_when: "The reference boot arguments include rootwait or rootwait=.";
                }
                trimmed rootfs.006 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "initrd_load is absent because CONFIG_BLK_DEV_INITRD=n.";
                    evidence {
                        rootfs_initrd_load_trimmed_noop();
                        rootfs_initrd_load_trimmed_because_config_blk_dev_initrd_disabled();
                    }
                    revisit_when: "The reference configuration enables CONFIG_BLK_DEV_INITRD.";
                }
                trimmed rootfs.007 {
                    category: TrimmedCategory::ReferenceInput;
                    summary: "NFS root is unreachable because the fixed root device is not /dev/nfs.";
                    evidence { rootfs_nfs_root_trimmed_by_reference_input(); }
                    revisit_when: "The reference root device selects /dev/nfs.";
                }
                trimmed rootfs.008 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "CIFS root is absent because CONFIG_CIFS_ROOT=n.";
                    evidence { rootfs_cifs_root_trimmed_noop(); }
                    revisit_when: "The reference configuration enables CONFIG_CIFS_ROOT.";
                }
                deferred rootfs.009 {
                    category: DeferredCategory::Feature;
                    summary: "Mount devtmpfs during root namespace preparation.";
                    evidence { rootfs_devtmpfs_mount_deferred(); }
                    close_when: "devtmpfs mount ordering and root-switch tests pass.";
                }
                deferred rootfs.010 {
                    category: DeferredCategory::Feature;
                    summary: "Attach /dev to the new root after the ext2 root switch.";
                    evidence { rootfs_devfs_currently_not_remounted_after_root_switch(); }
                    close_when: "The new root exposes the intended /dev mount with namespace and task fs tests.";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            rootfs_root_delay_trimmed_noop();
            rootfs_root_delay_trimmed_because_cmdline_absent();
            rootfs_device_probe_wait_deferred();
            rootfs_device_probe_waitqueue_deferred();
            rootfs_device_probe_atomic_counter_deferred();
            rootfs_deferred_probe_work_flush_deferred();
            rootfs_md_run_setup_deferred();
            rootfs_md_run_setup_deferred_because_config_md_enabled();
            rootfs_saved_root_name_parse_deferred();
            rootfs_root_device_parse_deferred();
            rootfs_initrd_load_trimmed_noop();
            rootfs_initrd_load_trimmed_because_config_blk_dev_initrd_disabled();
            rootfs_root_wait_trimmed_noop();
            rootfs_root_wait_trimmed_because_cmdline_absent();
            rootfs_root_wait_polling_deferred();
            rootfs_mount_root_block_formal();
            rootfs_nfs_root_trimmed_by_reference_input();
            rootfs_cifs_root_trimmed_noop();
            rootfs_nodev_root_deferred();
            rootfs_ext4_for_ext2_linux_config_recorded();
            rootfs_arceos_ext2_driver_substitutes_linux_ext4_for_ext2();
            rootfs_devtmpfs_mount_deferred();
            rootfs_devtmpfs_mount_deferred_because_config_devtmpfs_enabled();
            rootfs_devfs_currently_not_remounted_after_root_switch();
        }
    }
}

/*
 * RootFS is the target top-level filesystem view. It is already Ready when
 * this phase starts because ProcessPreparePhase built the initial ramfs-backed
 * rootfs mount via vfs_caches_init()/mnt_init(). RootFS.Transition::Enable
 * represents the prepare_namespace() position in this phase. This slice
 * records that the supported Linux-like path enters prepare_namespace(), that
 * /dev is already mounted, and that the default block device is available as
 * the first root device candidate. It mounts the prepared ext2 filesystem at
 * Linux's temporary /root mount point, then performs the final MS_MOVE to /
 * and chroot(".") as two separate actions.
 */
predicate rootfs_prepare_namespace_inputs_ready<T>(rootfs: T) -> bool;
predicate rootfs_initial_ramfs_still_active<T>(rootfs: T) -> bool;
predicate rootfs_devfs_available<T, D>(rootfs: T, devfs: D) -> bool;
predicate rootfs_block_root_device_candidate_bound<T, B>(rootfs: T, block_registry: B) -> bool;
predicate rootfs_ext2_driver_ready<T, D>(rootfs: T, driver: D) -> bool;
predicate rootfs_ext2_volume_ready<T, V>(rootfs: T, volume: V) -> bool;
predicate rootfs_ext2_filesystem_ready<T, F>(rootfs: T, fs: F) -> bool;
predicate rootfs_real_mount_point_created<T, D>(rootfs: T, mount_point: D) -> bool;
predicate rootfs_real_ext2_mount_created<T, M>(rootfs: T, mount: M) -> bool;
predicate rootfs_ms_move_done<T, M>(rootfs: T, mount: M) -> bool;
predicate rootfs_chroot_dot_done<T, F>(rootfs: T, fs: F) -> bool;
predicate rootfs_current_root_is_real_ext2<T, F>(rootfs: T, fs: F) -> bool;

object RootFS: KernelObject {
    initial_state: State::Ready;

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    RootfsConsoleDeferred.state == State::Ready;
                    RootfsPrepareNamespacePaths.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    rootfs_mount_created(VfsCore);
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                    fs_struct_pwd_dentry_set(FsStruct, Dentry);
                    DevFs.state == State::Ready;
                    devfs_mount_created(VfsCore);
                    devfs_block_node_bound(DevFs, BlockDeviceRegistry);
                    BlockDeviceRegistry.state == State::Ready;
                    block_core_default_device_slot_ready(BlockDeviceRegistry);
                    Ext2Driver.state == State::Ready;
                    Ext2Volume.state == State::Ready;
                    Ext2FileSystem.state == State::Ready;
                    rootfs_mount_root_block_formal();
                    rootfs_ext4_for_ext2_linux_config_recorded();
                    rootfs_arceos_ext2_driver_substitutes_linux_ext4_for_ext2();
                    rootfs_devfs_currently_not_remounted_after_root_switch();
                }

                drives {
                    Ext2FileSystem.Transition::Enable;
                    VfsCore.Action::MountExt2At;
                    VfsCore.Action::MoveMountToRoot;
                    FsStruct.Action::Chdir;
                    FsStruct.Action::ChrootDot;
                }

                ensures {
                    ramdisk_execute_command_eaccess_requires_prepare_namespace();
                    rootfs_prepare_namespace_inputs_ready(RootFS);
                    rootfs_initial_ramfs_still_active(RootFS);
                    rootfs_devfs_available(RootFS, DevFs);
                    rootfs_block_root_device_candidate_bound(RootFS, BlockDeviceRegistry);
                    rootfs_ext2_driver_ready(RootFS, Ext2Driver);
                    rootfs_ext2_volume_ready(RootFS, Ext2Volume);
                    rootfs_ext2_filesystem_ready(RootFS, Ext2FileSystem);
                    rootfs_real_mount_point_created(RootFS, Dentry);
                    rootfs_real_ext2_mount_created(RootFS, Mount);
                    rootfs_prepare_namespace_position_preserved();
                    rootfs_ms_move_done(RootFS, Mount);
                    rootfs_chroot_dot_done(RootFS, FsStruct);
                    rootfs_current_root_is_real_ext2(RootFS, Ext2FileSystem);
                }
            }
        }
    }

    state State::Online {
        invariant {
            ramdisk_execute_command_eaccess_requires_prepare_namespace();
            rootfs_prepare_namespace_inputs_ready(RootFS);
            rootfs_initial_ramfs_still_active(RootFS);
            rootfs_devfs_available(RootFS, DevFs);
            rootfs_block_root_device_candidate_bound(RootFS, BlockDeviceRegistry);
            rootfs_ext2_driver_ready(RootFS, Ext2Driver);
            rootfs_ext2_volume_ready(RootFS, Ext2Volume);
            rootfs_ext2_filesystem_ready(RootFS, Ext2FileSystem);
            rootfs_real_ext2_mount_created(RootFS, Mount);
            rootfs_prepare_namespace_position_preserved();
            rootfs_ms_move_done(RootFS, Mount);
            rootfs_chroot_dot_done(RootFS, FsStruct);
            rootfs_current_root_is_real_ext2(RootFS, Ext2FileSystem);
        }
    }
}

/*
 * IntegrityKeysDeferred preserves integrity_load_keys(). CONFIG_INTEGRITY=y is
 * acknowledged, but IMA/EVM keyring and certificate loading details are left
 * for a security/integrity round.
 */
object IntegrityKeysDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    RootFS.state == State::Online;
                }

                ensures {
                    integrity_keys_setup_deferred();
                    integrity_load_keys_position_preserved();
                    integrity_config_enabled_recorded();
                    integrity_ima_load_x509_deferred();
                    integrity_ima_load_x509_trimmed_because_config_ima_disabled();
                    integrity_evm_load_x509_deferred();
                    integrity_evm_load_x509_trimmed_because_config_evm_disabled();
                }

                trimmed integrity_keys.001 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "IMA X.509 key loading is absent because CONFIG_IMA=n.";
                    evidence { integrity_ima_load_x509_trimmed_because_config_ima_disabled(); }
                    revisit_when: "The reference configuration enables CONFIG_IMA.";
                }
                trimmed integrity_keys.002 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "EVM X.509 key loading is absent because CONFIG_EVM=n.";
                    evidence { integrity_evm_load_x509_trimmed_because_config_evm_disabled(); }
                    revisit_when: "The reference configuration enables CONFIG_EVM.";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            integrity_keys_setup_deferred();
            integrity_load_keys_position_preserved();
            integrity_config_enabled_recorded();
            integrity_ima_load_x509_deferred();
            integrity_ima_load_x509_trimmed_because_config_ima_disabled();
            integrity_evm_load_x509_deferred();
            integrity_evm_load_x509_trimmed_because_config_evm_disabled();
        }
    }
}

/*
 * RootfsBoundary aggregates the rootfs preparation segment and fixes the next
 * entry as FinalizePhase.
 */
object RootfsBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KUnitRuntimeTrimmed.state == State::Ready;
                    InitramfsSyncDeferred.state == State::Ready;
                    RootfsConsoleDeferred.state == State::Ready;
                    RootfsPrepareNamespacePaths.state == State::Ready;
                    RootFS.state == State::Online;
                    IntegrityKeysDeferred.state == State::Ready;
                }

                ensures {
                    rootfs_boundary_ready(RootfsBoundary);
                    finalize_phase_next_boundary();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            rootfs_boundary_ready(RootfsBoundary);
            finalize_phase_next_boundary();
        }
    }
}

/*
 * RootfsPhase is the minimal object-level boundary for the rootfs preparation
 * part of kernel_init_freeable(). The phase object begins after
 * InitcallPhase.Ready, but the paired-diff `RootfsPhase.Started` checkpoint is
 * anchored to Linux's prepare_namespace() branch after the KUnit/initramfs/
 * console prelude and the ramdisk eaccess checkpoint.
 */
object RootfsPhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    InitcallPhase.state == State::Online;
                    InitcallBoundary.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    Workqueue.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                }

                drives {
                    KUnitRuntimeTrimmed.Transition::Setup;
                    InitramfsSyncDeferred.Transition::Setup;
                    RootfsConsoleDeferred.Transition::Setup;
                    RootfsPrepareNamespacePaths.Transition::Setup;
                    Bio.Transition::Setup;
                    BufferHead.Transition::Setup;
                    Ext2Driver.Transition::Setup;
                    Ext2Volume.Transition::Preset;
                    Ext2FileSystem.Transition::Preset;
                    Ext2FileSystem.Transition::Setup;
                    RootFS.Transition::Enable;
                    IntegrityKeysDeferred.Transition::Setup;
                    RootfsBoundary.Transition::Setup;
                }

                ensures {
                    rootfs_phase_ready(RootfsPhase);
                    kunit_runtime_trimmed_noop();
                    initramfs_sync_wait_deferred();
                    rootfs_console_setup_deferred();
                    rootfs_device_probe_wait_deferred();
                    rootfs_md_run_setup_deferred();
                    rootfs_initrd_load_trimmed_noop();
                    rootfs_devtmpfs_mount_deferred();
                    ramdisk_execute_command_eaccess_requires_prepare_namespace();
                    rootfs_prepare_namespace_inputs_ready(RootFS);
                    rootfs_devfs_available(RootFS, DevFs);
                    rootfs_block_root_device_candidate_bound(RootFS, BlockDeviceRegistry);
                    rootfs_real_ext2_mount_created(RootFS, Mount);
                    rootfs_ms_move_done(RootFS, Mount);
                    rootfs_chroot_dot_done(RootFS, FsStruct);
                    rootfs_current_root_is_real_ext2(RootFS, Ext2FileSystem);
                    integrity_keys_setup_deferred();
                    rootfs_boundary_ready(RootfsBoundary);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            InitcallPhase.state == State::Online;
            KUnitRuntimeTrimmed.state == State::Ready;
            InitramfsSyncDeferred.state == State::Ready;
            RootfsConsoleDeferred.state == State::Ready;
            RootfsPrepareNamespacePaths.state == State::Ready;
            RootFS.state == State::Online;
            IntegrityKeysDeferred.state == State::Ready;
            RootfsBoundary.state == State::Ready;
            rootfs_phase_ready(RootfsPhase);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    InitcallPhase.state == State::Online;
                    rootfs_phase_ready(RootfsPhase);
                    RootfsBoundary.state == State::Ready;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            InitcallPhase.state == State::Online;
            rootfs_phase_ready(RootfsPhase);
            RootfsBoundary.state == State::Ready;
        }

        transitions {
            on Transition::Enable -> State::Online {
                ensures {
                    rootfs_phase_ready(RootfsPhase);
                    RootfsBoundary.state == State::Ready;
                }
            }
        }
    }

    state State::Online {
        invariant {
            InitcallPhase.state == State::Online;
            rootfs_phase_ready(RootfsPhase);
            RootfsBoundary.state == State::Ready;
        }
    }
}
