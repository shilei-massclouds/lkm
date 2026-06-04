/*
 * Finalize Phase Specification
 *
 * This is SMP Runtime Phase subphase 5. It covers kernel_init() after
 * kernel_init_freeable() returns, from async_synchronize_full() through
 * do_sysctl_args(), before PayloadPhase starts selecting the first payload.
 */

/*
 * AsyncFullSyncDeferred preserves async_synchronize_full(). AsyncCore itself
 * is still deferred in the current object-level prototype.
 */
object AsyncFullSyncDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RootfsBoundary.state == State::Ready;
                    AsyncCoreDeferred.state == State::Ready;
                }

                ensures {
                    async_synchronize_full_deferred();
                    async_init_work_drain_boundary_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            async_synchronize_full_deferred();
            async_init_work_drain_boundary_preserved();
        }
    }
}

/*
 * InitMemoryCleanupDeferred preserves the init-only memory cleanup sequence:
 * kprobe/ftrace/kgdb/bootconfig cleanup and free_initmem(). Config-disabled
 * paths are trimmed; ftrace and free_initmem details remain deferred.
 */
object InitMemoryCleanupDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    AsyncFullSyncDeferred.state == State::Ready;
                    SystemState.state == State::Ready;
                }

                ensures {
                    system_state_freeing_initmem_window_entered(SystemState);
                    kprobe_initmem_trimmed_noop();
                    ftrace_initmem_cleanup_deferred();
                    kgdb_initmem_trimmed_noop();
                    bootconfig_exit_trimmed_noop();
                    init_memory_free_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            system_state_freeing_initmem_window_entered(SystemState);
            kprobe_initmem_trimmed_noop();
            ftrace_initmem_cleanup_deferred();
            kgdb_initmem_trimmed_noop();
            bootconfig_exit_trimmed_noop();
            init_memory_free_deferred();
        }
    }
}

/*
 * KernelMappingProtectionDeferred preserves mark_readonly(). The strict RWX
 * and rodata details are kept for a later mapping-protection round.
 */
object KernelMappingProtectionDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    InitMemoryCleanupDeferred.state == State::Ready;
                }

                ensures {
                    kernel_mapping_protection_deferred();
                    strict_kernel_rwx_position_preserved();
                    rodata_debug_test_trimmed_or_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            kernel_mapping_protection_deferred();
            strict_kernel_rwx_position_preserved();
            rodata_debug_test_trimmed_or_deferred();
        }
    }
}

/*
 * PtiFinalizeTrimmed preserves pti_finalize(). RISC-V currently uses the
 * empty inline implementation.
 */
object PtiFinalizeTrimmed: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelMappingProtectionDeferred.state == State::Ready;
                }

                ensures {
                    pti_finalize_trimmed_noop();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            pti_finalize_trimmed_noop();
        }
    }
}

/*
 * RcuBootEnd records rcu_end_inkernel_boot(). It is an action on RcuCore
 * rather than a new RCU lifecycle state.
 */
object RcuBootEnd: KernelObject {
    initial_state: State::Base;
    parent: RcuCore;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PtiFinalizeTrimmed.state == State::Ready;
                    SystemState.state == State::Online;
                    RcuCore.state == State::Ready;
                }

                ensures {
                    system_state_running(SystemState);
                    rcu_inkernel_boot_ended(RcuCore);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            system_state_running(SystemState);
            rcu_inkernel_boot_ended(RcuCore);
        }
    }
}

/*
 * SysctlArgsDeferred preserves do_sysctl_args(). The sysctl.* parser and
 * temporary proc mount details remain deferred in this round.
 */
object SysctlArgsDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RcuBootEnd.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                }

                ensures {
                    sysctl_args_apply_deferred();
                    sysctl_command_line_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            sysctl_args_apply_deferred();
            sysctl_command_line_position_preserved();
        }
    }
}

/*
 * FinalizeBoundary closes kernel-side startup orchestration and fixes the next
 * entry as PayloadPhase.
 */
object FinalizeBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    AsyncFullSyncDeferred.state == State::Ready;
                    InitMemoryCleanupDeferred.state == State::Ready;
                    KernelMappingProtectionDeferred.state == State::Ready;
                    PtiFinalizeTrimmed.state == State::Ready;
                    RcuBootEnd.state == State::Ready;
                    SysctlArgsDeferred.state == State::Ready;
                }

                ensures {
                    finalize_boundary_ready(FinalizeBoundary);
                    payload_phase_next_boundary();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            finalize_boundary_ready(FinalizeBoundary);
            payload_phase_next_boundary();
        }
    }
}

/*
 * FinalizePhase is the minimal object-level boundary for the kernel-side
 * startup finalization after rootfs preparation.
 */
object FinalizePhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RootfsPhase.state == State::Ready;
                    RootfsBoundary.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    SystemState.state == State::Ready;
                    RcuCore.state == State::Ready;
                    SavedCommandLine.state == State::Ready;
                }

                drives {
                    AsyncFullSyncDeferred.Event::Setup;
                    InitMemoryCleanupDeferred.Event::Setup;
                    KernelMappingProtectionDeferred.Event::Setup;
                    PtiFinalizeTrimmed.Event::Setup;
                    SystemState.Event::Enable;
                    RcuBootEnd.Event::Setup;
                    SysctlArgsDeferred.Event::Setup;
                    FinalizeBoundary.Event::Setup;
                }

                ensures {
                    finalize_phase_ready(FinalizePhase);
                    async_synchronize_full_deferred();
                    system_state_running(SystemState);
                    init_memory_free_deferred();
                    kernel_mapping_protection_deferred();
                    pti_finalize_trimmed_noop();
                    rcu_inkernel_boot_ended(RcuCore);
                    sysctl_args_apply_deferred();
                    finalize_boundary_ready(FinalizeBoundary);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            RootfsPhase.state == State::Ready;
            AsyncFullSyncDeferred.state == State::Ready;
            InitMemoryCleanupDeferred.state == State::Ready;
            KernelMappingProtectionDeferred.state == State::Ready;
            PtiFinalizeTrimmed.state == State::Ready;
            SystemState.state == State::Online;
            RcuBootEnd.state == State::Ready;
            SysctlArgsDeferred.state == State::Ready;
            FinalizeBoundary.state == State::Ready;
            finalize_phase_ready(FinalizePhase);
        }
    }
}
