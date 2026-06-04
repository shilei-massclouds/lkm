/*
 * Initcall Phase Specification
 *
 * This is SMP Runtime Phase subphase 3. It covers do_basic_setup(), from
 * cpuset_init_smp() through do_initcalls(), before kunit_run_all_tests().
 */

/*
 * CpusetSmpTrimmed 表示 cpuset_init_smp() 的当前位置。当前配置
 * CONFIG_CGROUPS=n，因此该路径为空实现。
 */
object CpusetSmpTrimmed: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RuntimeCorePhase.state == State::Ready;
                }

                ensures {
                    cpuset_smp_trimmed_noop();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            cpuset_smp_trimmed_noop();
        }
    }
}

/*
 * DriverCoreDeferred 保留 driver_init() 的 Linux 时序位置。驱动模型
 * 内部对象层次较大，当前轮次不展开 device/bus/class/firmware/platform
 * 等对象。
 */
object DriverCoreDeferred: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpusetSmpTrimmed.state == State::Ready;
                    PageAllocator.state == State::Ready;
                    Workqueue.state == State::Ready;
                }

                ensures {
                    driver_core_setup_deferred();
                    driver_model_entry_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            driver_core_setup_deferred();
            driver_model_entry_position_preserved();
        }
    }
}

/*
 * IrqProcViewDeferred 保留 init_irq_proc() 的位置。该路径服务 procfs
 * 下的 IRQ 观测/配置导出，不改变当前 IRQ dispatch 主线。
 */
object IrqProcViewDeferred: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DriverCoreDeferred.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                }

                ensures {
                    irq_proc_view_setup_deferred();
                    proc_irq_export_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            irq_proc_view_setup_deferred();
            proc_irq_export_deferred();
        }
    }
}

/*
 * CtorTable 表示 do_ctors() 的构造函数表位置。当前对象级实现只保留
 * 表边界；无构造函数条目时记录为 trimmed/empty。
 */
object CtorTable: KernelObject {
    initial_state: State::Base;
    parent: StaticObjects;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    IrqProcViewDeferred.state == State::Ready;
                    StaticObjects.state == State::Online;
                }

                ensures {
                    ctor_table_position_preserved(CtorTable, StaticObjects);
                    constructors_trimmed_or_empty(CtorTable);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ctor_table_position_preserved(CtorTable, StaticObjects);
            constructors_trimmed_or_empty(CtorTable);
        }
    }
}

/*
 * InitcallTable 表示 do_initcalls() 对 linker initcall table 的执行动作。
 * 当前不把每个 entry 升级为顶层对象，只记录表属性和运行摘要。
 */
object InitcallTable: KernelObject {
    initial_state: State::Base;
    parent: StaticObjects;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CtorTable.state == State::Ready;
                    StaticObjects.state == State::Online;
                    SavedCommandLine.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    PageAllocator.state == State::Ready;
                }

                ensures {
                    initcall_table_static_ranges_ready(InitcallTable, StaticObjects);
                    initcall_table_level_count_ready(InitcallTable);
                    initcall_table_all_levels_ran(InitcallTable);
                    initcall_table_entries_recorded_as_properties(InitcallTable);
                    initcall_command_line_scratch_reused_per_level(InitcallTable, SavedCommandLine);
                    initcall_param_parser_applied(InitcallTable);
                    initcall_filter_applied(InitcallTable);
                    initcall_run_context_checked(InitcallTable);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            initcall_table_static_ranges_ready(InitcallTable, StaticObjects);
            initcall_table_level_count_ready(InitcallTable);
            initcall_table_all_levels_ran(InitcallTable);
            initcall_table_entries_recorded_as_properties(InitcallTable);
            initcall_command_line_scratch_reused_per_level(InitcallTable, SavedCommandLine);
            initcall_param_parser_applied(InitcallTable);
            initcall_filter_applied(InitcallTable);
            initcall_run_context_checked(InitcallTable);
        }
    }
}

/*
 * InitcallBoundary 聚合 do_basic_setup() 的完成边界，并把下一入口固定
 * 为 kunit_run_all_tests()/RootfsPhase。
 */
object InitcallBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpusetSmpTrimmed.state == State::Ready;
                    DriverCoreDeferred.state == State::Ready;
                    IrqProcViewDeferred.state == State::Ready;
                    CtorTable.state == State::Ready;
                    InitcallTable.state == State::Ready;
                }

                ensures {
                    initcall_boundary_ready(InitcallBoundary);
                    kunit_run_all_tests_next_boundary();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            initcall_boundary_ready(InitcallBoundary);
            kunit_run_all_tests_next_boundary();
        }
    }
}

/*
 * InitcallPhase 表示 do_basic_setup() 的最小对象级边界。
 */
object InitcallPhase: PhaseObject {
    initial_state: State::Base;
    parent: SmpRuntimePhase;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    RuntimeCorePhase.state == State::Ready;
                    RuntimeCoreBoundary.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    StaticObjects.state == State::Online;
                    SavedCommandLine.state == State::Ready;
                    IrqDispatchTree.state == State::Ready;
                }

                drives {
                    CpusetSmpTrimmed.Event::Setup;
                    DriverCoreDeferred.Event::Setup;
                    IrqProcViewDeferred.Event::Setup;
                    CtorTable.Event::Setup;
                    InitcallTable.Event::Setup;
                    InitcallBoundary.Event::Setup;
                }

                ensures {
                    initcall_phase_ready(InitcallPhase);
                    cpuset_smp_trimmed_noop();
                    driver_core_setup_deferred();
                    irq_proc_view_setup_deferred();
                    constructors_trimmed_or_empty(CtorTable);
                    initcall_table_all_levels_ran(InitcallTable);
                    initcall_boundary_ready(InitcallBoundary);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            RuntimeCorePhase.state == State::Ready;
            CpusetSmpTrimmed.state == State::Ready;
            DriverCoreDeferred.state == State::Ready;
            IrqProcViewDeferred.state == State::Ready;
            CtorTable.state == State::Ready;
            InitcallTable.state == State::Ready;
            InitcallBoundary.state == State::Ready;
            initcall_phase_ready(InitcallPhase);
        }
    }
}
