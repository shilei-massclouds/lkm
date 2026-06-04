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
        events {
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
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
 * RootFsEnableDeferred represents the prepare_namespace() position. RootFS is
 * the target top-level filesystem view, but the actual device/fs probing,
 * devtmpfs mount, MS_MOVE and chroot details remain deferred in this round.
 */
object RootFsEnableDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RootfsConsoleDeferred.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                    KernelInitTask.state == State::Online;
                }

                ensures {
                    ramdisk_execute_command_eaccess_requires_prepare_namespace();
                    rootfs_enable_deferred();
                    rootfs_prepare_namespace_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ramdisk_execute_command_eaccess_requires_prepare_namespace();
            rootfs_enable_deferred();
            rootfs_prepare_namespace_position_preserved();
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RootFsEnableDeferred.state == State::Ready;
                }

                ensures {
                    integrity_keys_setup_deferred();
                    integrity_load_keys_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            integrity_keys_setup_deferred();
            integrity_load_keys_position_preserved();
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KUnitRuntimeTrimmed.state == State::Ready;
                    InitramfsSyncDeferred.state == State::Ready;
                    RootfsConsoleDeferred.state == State::Ready;
                    RootFsEnableDeferred.state == State::Ready;
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
 * part of kernel_init_freeable().
 */
object RootfsPhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    InitcallPhase.state == State::Ready;
                    InitcallBoundary.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    Workqueue.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                }

                drives {
                    KUnitRuntimeTrimmed.Event::Setup;
                    InitramfsSyncDeferred.Event::Setup;
                    RootfsConsoleDeferred.Event::Setup;
                    RootFsEnableDeferred.Event::Setup;
                    IntegrityKeysDeferred.Event::Setup;
                    RootfsBoundary.Event::Setup;
                }

                ensures {
                    rootfs_phase_ready(RootfsPhase);
                    kunit_runtime_trimmed_noop();
                    initramfs_sync_wait_deferred();
                    rootfs_console_setup_deferred();
                    ramdisk_execute_command_eaccess_requires_prepare_namespace();
                    rootfs_enable_deferred();
                    integrity_keys_setup_deferred();
                    rootfs_boundary_ready(RootfsBoundary);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            InitcallPhase.state == State::Ready;
            KUnitRuntimeTrimmed.state == State::Ready;
            InitramfsSyncDeferred.state == State::Ready;
            RootfsConsoleDeferred.state == State::Ready;
            RootFsEnableDeferred.state == State::Ready;
            IntegrityKeysDeferred.state == State::Ready;
            RootfsBoundary.state == State::Ready;
            rootfs_phase_ready(RootfsPhase);
        }
    }
}
