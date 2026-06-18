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
 * DriverCoreBase 表示 driver_init() 中 platform_bus_init() 的必要前置
 * registry 基础。当前只把 devices_init() 与 buses_init() 升级为 formal
 * 事实：device_register(&platform_bus) 依赖 devices_kset，bus_register()
 * 依赖 bus_kset。bdi/devtmpfs/classes/firmware/hypervisor/of_core 等
 * platform_bus_init() 不直接依赖的前置调用只保留 deferred 位置。
 */
object DriverCoreBase: DeviceObject {
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
                    driver_core_device_registry_ready(DriverCoreBase);
                    driver_core_bus_registry_ready(DriverCoreBase);
                    driver_core_pre_platform_deferred();
                    driver_core_pre_platform_order_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            driver_core_device_registry_ready(DriverCoreBase);
            driver_core_bus_registry_ready(DriverCoreBase);
            driver_core_pre_platform_deferred();
            driver_core_pre_platform_order_preserved();
        }
    }
}

/*
 * PlatformBusRootDevice 表示 platform_bus_init() 的第一段：
 * early_platform_cleanup(); device_register(&platform_bus)。
 * 当前 RISC-V 路径下 early platform cleanup 不展开；platform_bus 是
 * 静态 struct device，注册成功后成为 /sys/devices 下的 root device。
 */
object PlatformBusRootDevice: DeviceType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DriverCoreBase.state == State::Ready;
                    StaticObjects.state == State::Online;
                }

                ensures {
                    early_platform_cleanup_deferred();
                    platform_bus_static_device_registered(PlatformBusRootDevice);
                    platform_bus_device_name_bound(PlatformBusRootDevice);
                    platform_bus_device_register_return_zero(PlatformBusRootDevice);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            early_platform_cleanup_deferred();
            platform_bus_static_device_registered(PlatformBusRootDevice);
            platform_bus_device_name_bound(PlatformBusRootDevice);
            platform_bus_device_register_return_zero(PlatformBusRootDevice);
        }
    }
}

/*
 * PlatformBusSubsysPrivate 是 bus_register(&platform_bus_type) 分配并挂到
 * platform_bus_type.p 的 struct subsys_private。它承载后续 platform
 * devices/drivers 两条 klist 以及 probe 控制入口。
 */
object PlatformBusSubsysPrivate: BusSubsysPrivate {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    PlatformBus.state == State::Prepared;
                }

                ensures {
                    bus_subsys_private_allocated(PlatformBusSubsysPrivate);
                    bus_subsys_private_bound_to_bus(PlatformBusSubsysPrivate, PlatformBus);
                    bus_subsys_notifier_ready(PlatformBusSubsysPrivate);
                    bus_subsys_autoprobe_enabled(PlatformBusSubsysPrivate);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            bus_subsys_private_allocated(PlatformBusSubsysPrivate);
            bus_subsys_private_bound_to_bus(PlatformBusSubsysPrivate, PlatformBus);
            bus_subsys_notifier_ready(PlatformBusSubsysPrivate);
            bus_subsys_autoprobe_enabled(PlatformBusSubsysPrivate);
        }

        events {
            on Event::Setup -> State::Ready {
                ensures {
                    bus_subsys_kobject_named(PlatformBusSubsysPrivate);
                    bus_subsys_kobject_attached_to_bus_kset(PlatformBusSubsysPrivate);
                    bus_subsys_registered(PlatformBusSubsysPrivate);
                    bus_subsys_uevent_file_ready(PlatformBusSubsysPrivate);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            bus_subsys_private_allocated(PlatformBusSubsysPrivate);
            bus_subsys_private_bound_to_bus(PlatformBusSubsysPrivate, PlatformBus);
            bus_subsys_notifier_ready(PlatformBusSubsysPrivate);
            bus_subsys_autoprobe_enabled(PlatformBusSubsysPrivate);
            bus_subsys_kobject_named(PlatformBusSubsysPrivate);
            bus_subsys_kobject_attached_to_bus_kset(PlatformBusSubsysPrivate);
            bus_subsys_registered(PlatformBusSubsysPrivate);
            bus_subsys_uevent_file_ready(PlatformBusSubsysPrivate);
        }

        events {
            on Event::Enable -> State::Online {
                ensures {
                    bus_subsys_devices_kset_ready(PlatformBusSubsysPrivate);
                    bus_subsys_drivers_kset_ready(PlatformBusSubsysPrivate);
                    bus_subsys_interfaces_ready(PlatformBusSubsysPrivate);
                    bus_subsys_mutex_ready(PlatformBusSubsysPrivate);
                    bus_subsys_klist_devices_ready(PlatformBusSubsysPrivate);
                    bus_subsys_device_ref_set_bound(PlatformBusSubsysPrivate, PlatformBusSubsysPrivate.klist_devices);
                    device_ref_set_ready(PlatformBusSubsysPrivate.klist_devices);
                    bus_subsys_klist_drivers_ready(PlatformBusSubsysPrivate);
                    bus_subsys_driver_ref_set_bound(PlatformBusSubsysPrivate, PlatformBusSubsysPrivate.klist_drivers);
                    device_driver_ref_set_ready(PlatformBusSubsysPrivate.klist_drivers);
                    bus_subsys_probe_files_ready(PlatformBusSubsysPrivate);
                    bus_subsys_groups_ready(PlatformBusSubsysPrivate);
                }
            }
        }
    }

    state State::Online {
        invariant {
            bus_subsys_private_allocated(PlatformBusSubsysPrivate);
            bus_subsys_private_bound_to_bus(PlatformBusSubsysPrivate, PlatformBus);
            bus_subsys_notifier_ready(PlatformBusSubsysPrivate);
            bus_subsys_autoprobe_enabled(PlatformBusSubsysPrivate);
            bus_subsys_kobject_named(PlatformBusSubsysPrivate);
            bus_subsys_kobject_attached_to_bus_kset(PlatformBusSubsysPrivate);
            bus_subsys_registered(PlatformBusSubsysPrivate);
            bus_subsys_uevent_file_ready(PlatformBusSubsysPrivate);
            bus_subsys_devices_kset_ready(PlatformBusSubsysPrivate);
            bus_subsys_drivers_kset_ready(PlatformBusSubsysPrivate);
            bus_subsys_interfaces_ready(PlatformBusSubsysPrivate);
            bus_subsys_mutex_ready(PlatformBusSubsysPrivate);
            bus_subsys_klist_devices_ready(PlatformBusSubsysPrivate);
            bus_subsys_device_ref_set_bound(PlatformBusSubsysPrivate, PlatformBusSubsysPrivate.klist_devices);
            device_ref_set_ready(PlatformBusSubsysPrivate.klist_devices);
            bus_subsys_klist_drivers_ready(PlatformBusSubsysPrivate);
            bus_subsys_driver_ref_set_bound(PlatformBusSubsysPrivate, PlatformBusSubsysPrivate.klist_drivers);
            device_driver_ref_set_ready(PlatformBusSubsysPrivate.klist_drivers);
            bus_subsys_probe_files_ready(PlatformBusSubsysPrivate);
            bus_subsys_groups_ready(PlatformBusSubsysPrivate);
        }
    }
}

/*
 * PlatformBus 是 PlatformBusType 的 singleton 实例。Preset 对应静态
 * struct bus_type platform_bus_type 的 name/ops/dev_groups 绑定，并声明
 * arch_initcall_sync(of_platform_default_populate_init) entry；Setup 对应
 * platform_bus_init() 中的 bus_register(&platform_bus_type)。
 * 真正的 platform device/driver 枚举与 probe 留给后续 initcall。
 */
object PlatformBus: PlatformBusType {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    StaticObjects.state == State::Online;
                }

                drives {
                    InitcallTable.Action::Register(
                        level: InitcallLevel::ArchSync,
                        entry: InitcallEntry::OfPlatformDefaultPopulate
                    );
                }

                ensures {
                    bus_type_descriptor_bound(PlatformBus);
                    bus_type_name_bound(PlatformBus);
                    bus_type_ops_bound(PlatformBus);
                    platform_bus_type_ops_bound(PlatformBus);
                    initcall_entry_declared(InitcallEntry::OfPlatformDefaultPopulate);
                    initcall_entry_has_prototype(InitcallEntry::OfPlatformDefaultPopulate, InitcallEntryPrototype);
                    initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
                    initcall_entry_prototype_returns_result(InitcallEntryPrototype);
                    initcall_entry_owner_bound(InitcallEntry::OfPlatformDefaultPopulate, PlatformBus);
                    initcall_entry_operation_bound(InitcallEntry::OfPlatformDefaultPopulate, PlatformBus.Action::OfPlatformDefaultPopulateInit);
                    initcall_table_registration_committed(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
                    initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::OfPlatformDefaultPopulate, PlatformBus);
                    initcall_table_entry_registered(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            bus_type_descriptor_bound(PlatformBus);
            bus_type_name_bound(PlatformBus);
            bus_type_ops_bound(PlatformBus);
            platform_bus_type_ops_bound(PlatformBus);
            initcall_entry_declared(InitcallEntry::OfPlatformDefaultPopulate);
            initcall_entry_has_prototype(InitcallEntry::OfPlatformDefaultPopulate, InitcallEntryPrototype);
            initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
            initcall_entry_prototype_returns_result(InitcallEntryPrototype);
            initcall_entry_owner_bound(InitcallEntry::OfPlatformDefaultPopulate, PlatformBus);
            initcall_entry_operation_bound(InitcallEntry::OfPlatformDefaultPopulate, PlatformBus.Action::OfPlatformDefaultPopulateInit);
            initcall_table_registration_committed(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
            initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::OfPlatformDefaultPopulate, PlatformBus);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    DriverCoreBase.state == State::Ready;
                    PlatformBusRootDevice.state == State::Ready;
                }

                drives {
                    PlatformBusSubsysPrivate.Event::Preset;
                    PlatformBusSubsysPrivate.Event::Setup;
                    PlatformBusSubsysPrivate.Event::Enable;
                }

                ensures {
                    bus_type_subsys_private_ready(PlatformBus, PlatformBusSubsysPrivate);
                    bus_type_subsys_private_online(PlatformBus, PlatformBusSubsysPrivate);
                    bus_type_registered(PlatformBus);
                    bus_type_devices_kset_ready(PlatformBus);
                    bus_type_drivers_kset_ready(PlatformBus);
                    bus_type_autoprobe_enabled(PlatformBus);
                    bus_type_register_return_zero(PlatformBus);
                    platform_bus_type_registered(PlatformBus);
                    platform_bus_type_devices_kset_ready(PlatformBus);
                    platform_bus_type_drivers_kset_ready(PlatformBus);
                    platform_bus_type_autoprobe_enabled(PlatformBus);
                    platform_bus_register_return_zero(PlatformBus);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            bus_type_descriptor_bound(PlatformBus);
            bus_type_name_bound(PlatformBus);
            bus_type_ops_bound(PlatformBus);
            bus_type_subsys_private_ready(PlatformBus, PlatformBusSubsysPrivate);
            bus_type_subsys_private_online(PlatformBus, PlatformBusSubsysPrivate);
            bus_type_registered(PlatformBus);
            bus_type_devices_kset_ready(PlatformBus);
            bus_type_drivers_kset_ready(PlatformBus);
            bus_type_autoprobe_enabled(PlatformBus);
            bus_type_register_return_zero(PlatformBus);
            platform_bus_type_ops_bound(PlatformBus);
            platform_bus_type_registered(PlatformBus);
            platform_bus_type_devices_kset_ready(PlatformBus);
            platform_bus_type_drivers_kset_ready(PlatformBus);
            platform_bus_type_autoprobe_enabled(PlatformBus);
            platform_bus_register_return_zero(PlatformBus);
            initcall_entry_declared(InitcallEntry::OfPlatformDefaultPopulate);
            initcall_entry_has_prototype(InitcallEntry::OfPlatformDefaultPopulate, InitcallEntryPrototype);
            initcall_entry_prototype_no_payload_args(InitcallEntryPrototype);
            initcall_entry_prototype_returns_result(InitcallEntryPrototype);
            initcall_entry_owner_bound(InitcallEntry::OfPlatformDefaultPopulate, PlatformBus);
            initcall_entry_operation_bound(InitcallEntry::OfPlatformDefaultPopulate, PlatformBus.Action::OfPlatformDefaultPopulateInit);
            initcall_table_registration_committed(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
            initcall_table_registration_owner_bound(InitcallTable, InitcallEntry::OfPlatformDefaultPopulate, PlatformBus);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
        }

    }
}

/*
 * DriverCoreDeferred 现在只表示 platform_bus_init() 之后仍未展开的
 * driver_init() 尾部：auxiliary_bus_init(), memory_dev_init(),
 * node_dev_init(), cpu_dev_init(), container_dev_init()。这些不是当前
 * platform_bus_init() 完成条件。
 */
object DriverCoreDeferred: DeviceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PlatformBus.state == State::Ready;
                }

                ensures {
                    driver_core_post_platform_deferred();
                    driver_model_entry_position_preserved();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            driver_core_post_platform_deferred();
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
                    DriverCoreBase.state == State::Ready;
                    PlatformBusRootDevice.state == State::Ready;
                    PlatformBus.state == State::Ready;
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
 * InitcallTable 是全量 initcall 表实例。Preset 基于各 owner 对象在
 * 自己 Preset 中声明的 entries 建立表视图；Setup 才对应 do_initcalls()
 * 的表执行。model 层不限制注册关系必须来自 LDS，当前 coding 层采用
 * Linux-like linker section 方式：section range 本身就是 InitcallEntry
 * 数组。
 */
object InitcallTable: InitcallTableType {
    initial_state: State::Base;
    parent: StaticObjects;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
                depends_on {
                    CtorTable.state == State::Ready;
                    StaticObjects.state == State::Online;
                    PlatformBus.state == State::Ready;
                    Ns16550aPlatformDriver.state == State::Prepared;
                }

                ensures {
                    initcall_table_static_ranges_ready(InitcallTable, StaticObjects);
                    initcall_table_level_count_ready(InitcallTable);
                    initcall_table_entries_recorded_as_properties(InitcallTable);
                    initcall_table_registered_entries_collected(InitcallTable);
                    initcall_table_level_mapping_ready(InitcallTable);
                    initcall_table_run_levels_ready(InitcallTable);
                    initcall_table_entry_operation_bindings_ready(InitcallTable);
                    initcall_table_entry_registered(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
                    initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            initcall_table_static_ranges_ready(InitcallTable, StaticObjects);
            initcall_table_level_count_ready(InitcallTable);
            initcall_table_entries_recorded_as_properties(InitcallTable);
            initcall_table_registered_entries_collected(InitcallTable);
            initcall_table_level_mapping_ready(InitcallTable);
            initcall_table_run_levels_ready(InitcallTable);
            initcall_table_entry_operation_bindings_ready(InitcallTable);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
        }

        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SavedCommandLine.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    PageAllocator.state == State::Ready;
                }

                ensures {
                    initcall_table_static_ranges_ready(InitcallTable, StaticObjects);
                    initcall_table_level_count_ready(InitcallTable);
                    initcall_table_entries_recorded_as_properties(InitcallTable);
                    initcall_table_registered_entries_collected(InitcallTable);
                    initcall_table_level_mapping_ready(InitcallTable);
                    initcall_table_run_levels_ready(InitcallTable);
                    initcall_table_entry_operation_bindings_ready(InitcallTable);
                    initcall_table_all_levels_ran(InitcallTable);
                    PlatformBus.Action::OfPlatformDefaultPopulateInit;
                    initcall_command_line_scratch_reused_per_level(InitcallTable, SavedCommandLine);
                    initcall_param_parser_applied(InitcallTable);
                    initcall_filter_applied(InitcallTable);
                    initcall_run_context_checked(InitcallTable);
                    initcall_entry_invoked(InitcallEntry::OfPlatformDefaultPopulate);
                    initcall_entry_return_recorded(InitcallEntry::OfPlatformDefaultPopulate);
                    initcall_entry_skipped_recorded(InitcallEntry::OfPlatformDefaultPopulate);
                    initcall_entry_run_context_checked(InitcallEntry::OfPlatformDefaultPopulate);
                    of_platform_default_populate_source_tree_ready(PlatformBus, DeviceTree);
                    of_platform_default_populate_root_children_scanned(PlatformBus, DeviceTree);
                    of_platform_default_populate_strict_compatible_required(PlatformBus);
                    of_platform_default_populate_default_bus_match_table_used(PlatformBus);
                    of_platform_default_populate_bus_nodes_recurse(PlatformBus);
                    of_platform_default_populate_candidates_identified(PlatformBus);
                    of_platform_default_populate_candidates_are_available(PlatformBus);
                    of_platform_default_populate_candidate_names_printed(PlatformBus);
                    of_platform_default_populate_candidate_compatibles_printed(PlatformBus);
                    of_platform_default_populate_scan_complete_checkpoint(PlatformBus);
                    of_platform_default_populate_device_node_ids_bound(PlatformBus);
                    of_platform_default_populate_node_refs_bound(PlatformBus);
                    of_platform_default_populate_platform_devices_created(PlatformBus);
                    of_platform_default_populate_platform_devices_owned(PlatformBus, PlatformBus.platform_devices);
                    of_platform_default_populate_device_refs_bound(PlatformBus);
                    of_platform_default_populate_devices_added_to_bus(PlatformBus);
                    platform_bus_platform_device_set_bound(PlatformBus, PlatformBus.platform_devices);
                    platform_bus_platform_device_owner_ready(PlatformBus, PlatformBus.platform_devices);
                    platform_bus_devices_added_from_platform_device_set(PlatformBus, PlatformBus.platform_devices);
                    platform_device_set_ready(PlatformBus.platform_devices);
                    platform_device_set_nonempty(PlatformBus.platform_devices);
                    bus_type_devices_klist_nonempty(PlatformBus);
                    device_ref_set_nonempty(PlatformBusSubsysPrivate.klist_devices);
                    PlatformBus.Action::RegisterNs16550aPlatformDriver;
                    initcall_entry_invoked(InitcallEntry::Ns16550aPlatformDriver);
                    initcall_entry_return_recorded(InitcallEntry::Ns16550aPlatformDriver);
                    initcall_entry_skipped_recorded(InitcallEntry::Ns16550aPlatformDriver);
                    initcall_entry_run_context_checked(InitcallEntry::Ns16550aPlatformDriver);
                    platform_bus_driver_registered(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver);
                    platform_bus_device_discovered(PlatformBus, DeviceRef::Ns16550aSerial);
                    platform_bus_match_attempted(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_probe_called(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_device_bound(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_ns16550a_driver_registered(PlatformBus);
                    platform_bus_ns16550a_driver_match_table_ready(PlatformBus);
                    platform_bus_ns16550a_device_matched(PlatformBus);
                    platform_bus_ns16550a_driver_probe_called(PlatformBus);
                    platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
                    platform_bus_ns16550a_device_bound(PlatformBus);
                    bus_type_drivers_klist_nonempty(PlatformBus);
                    device_driver_ref_set_nonempty(PlatformBusSubsysPrivate.klist_drivers);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            initcall_table_static_ranges_ready(InitcallTable, StaticObjects);
            initcall_table_level_count_ready(InitcallTable);
            initcall_table_entries_recorded_as_properties(InitcallTable);
            initcall_table_registered_entries_collected(InitcallTable);
            initcall_table_level_mapping_ready(InitcallTable);
            initcall_table_run_levels_ready(InitcallTable);
            initcall_table_entry_operation_bindings_ready(InitcallTable);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::ArchSync, InitcallEntry::OfPlatformDefaultPopulate);
            initcall_table_entry_registered(InitcallTable, InitcallLevel::Device, InitcallEntry::Ns16550aPlatformDriver);
            initcall_table_all_levels_ran(InitcallTable);
            initcall_entry_invoked(InitcallEntry::OfPlatformDefaultPopulate);
            initcall_command_line_scratch_reused_per_level(InitcallTable, SavedCommandLine);
            initcall_param_parser_applied(InitcallTable);
            initcall_filter_applied(InitcallTable);
            initcall_run_context_checked(InitcallTable);
            initcall_entry_return_recorded(InitcallEntry::OfPlatformDefaultPopulate);
            initcall_entry_skipped_recorded(InitcallEntry::OfPlatformDefaultPopulate);
            initcall_entry_run_context_checked(InitcallEntry::OfPlatformDefaultPopulate);
            of_platform_default_populate_source_tree_ready(PlatformBus, DeviceTree);
            of_platform_default_populate_root_children_scanned(PlatformBus, DeviceTree);
            of_platform_default_populate_strict_compatible_required(PlatformBus);
            of_platform_default_populate_default_bus_match_table_used(PlatformBus);
            of_platform_default_populate_bus_nodes_recurse(PlatformBus);
            of_platform_default_populate_candidates_identified(PlatformBus);
            of_platform_default_populate_candidates_are_available(PlatformBus);
            of_platform_default_populate_candidate_names_printed(PlatformBus);
            of_platform_default_populate_candidate_compatibles_printed(PlatformBus);
            of_platform_default_populate_scan_complete_checkpoint(PlatformBus);
            of_platform_default_populate_device_node_ids_bound(PlatformBus);
            of_platform_default_populate_node_refs_bound(PlatformBus);
            of_platform_default_populate_platform_devices_created(PlatformBus);
            of_platform_default_populate_platform_devices_owned(PlatformBus, PlatformBus.platform_devices);
            of_platform_default_populate_device_refs_bound(PlatformBus);
            of_platform_default_populate_devices_added_to_bus(PlatformBus);
            platform_bus_platform_device_set_bound(PlatformBus, PlatformBus.platform_devices);
            platform_bus_platform_device_owner_ready(PlatformBus, PlatformBus.platform_devices);
            platform_bus_devices_added_from_platform_device_set(PlatformBus, PlatformBus.platform_devices);
            platform_device_set_ready(PlatformBus.platform_devices);
            platform_device_set_nonempty(PlatformBus.platform_devices);
            bus_type_devices_klist_nonempty(PlatformBus);
            device_ref_set_nonempty(PlatformBusSubsysPrivate.klist_devices);
            initcall_entry_invoked(InitcallEntry::Ns16550aPlatformDriver);
            initcall_entry_return_recorded(InitcallEntry::Ns16550aPlatformDriver);
            initcall_entry_skipped_recorded(InitcallEntry::Ns16550aPlatformDriver);
            initcall_entry_run_context_checked(InitcallEntry::Ns16550aPlatformDriver);
            platform_bus_driver_registered(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver);
            platform_bus_device_discovered(PlatformBus, DeviceRef::Ns16550aSerial);
            platform_bus_match_attempted(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_probe_called(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_device_bound(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
            platform_bus_ns16550a_driver_registered(PlatformBus);
            platform_bus_ns16550a_driver_match_table_ready(PlatformBus);
            platform_bus_ns16550a_device_matched(PlatformBus);
            platform_bus_ns16550a_driver_probe_called(PlatformBus);
            platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
            platform_bus_ns16550a_device_bound(PlatformBus);
            bus_type_drivers_klist_nonempty(PlatformBus);
            device_driver_ref_set_nonempty(PlatformBusSubsysPrivate.klist_drivers);
        }
    }
}

/*
 * InitcallBoundary 聚合 do_basic_setup() 的完成边界，并把下一入口固定
 * 为 RootfsPhase 的 kunit_run_all_tests() 位置。
 */
object InitcallBoundary: KernelObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    CpusetSmpTrimmed.state == State::Ready;
                    DriverCoreBase.state == State::Ready;
                    PlatformBusRootDevice.state == State::Ready;
                    PlatformBus.state == State::Ready;
                    DriverCoreDeferred.state == State::Ready;
                    IrqProcViewDeferred.state == State::Ready;
                    CtorTable.state == State::Ready;
                    InitcallTable.state == State::Ready;
                    UartExternalIrqEnable.state == State::Ready;
                    UartInterruptChainProbe.state == State::Ready;
                    Serial8250Console.state == State::Online;
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
                    DriverCoreBase.Event::Setup;
                    PlatformBusRootDevice.Event::Setup;
                    PlatformBus.Event::Preset;
                    PlatformBus.Event::Setup;
                    DriverCoreDeferred.Event::Setup;
                    IrqProcViewDeferred.Event::Setup;
                    Ns16550aPlatformDriverStorage.Event::Setup;
                    Ns16550aPlatformDriver.Event::Preset;
                    CtorTable.Event::Setup;
                    InitcallTable.Event::Preset;
                    InitcallTable.Event::Setup;
                    Ns16550aPlatformDriver.Event::Setup;
                    UartExternalIrqEnable.Event::Setup;
                    UartInterruptChainProbe.Event::Setup;
                    Serial8250Console.Event::Enable;
                    TtyPort.Event::Setup;
                    Serial8250RuntimePort.Event::Setup;
                    TtyPort.Event::Enable;
                    Serial8250RuntimePort.Event::Enable;
                    Serial8250RxLoopbackProbe.Event::Setup;
                    Serial8250RxBatchLoopbackProbe.Event::Setup;
                    TtyXmitFifoProbe.Event::Setup;
                    InitcallBoundary.Event::Setup;
                }

                ensures {
                    initcall_phase_ready(InitcallPhase);
                    cpuset_smp_trimmed_noop();
                    driver_core_device_registry_ready(DriverCoreBase);
                    driver_core_bus_registry_ready(DriverCoreBase);
                    platform_bus_static_device_registered(PlatformBusRootDevice);
                    platform_bus_type_registered(PlatformBus);
                    driver_core_post_platform_deferred();
                    irq_proc_view_setup_deferred();
                    constructors_trimmed_or_empty(CtorTable);
                    initcall_table_registered_entries_collected(InitcallTable);
                    initcall_table_all_levels_ran(InitcallTable);
                    of_platform_default_populate_candidates_identified(PlatformBus);
                    of_platform_default_populate_candidate_names_printed(PlatformBus);
                    of_platform_default_populate_candidate_compatibles_printed(PlatformBus);
                    of_platform_default_populate_scan_complete_checkpoint(PlatformBus);
                    of_platform_default_populate_device_node_ids_bound(PlatformBus);
                    of_platform_default_populate_node_refs_bound(PlatformBus);
                    of_platform_default_populate_platform_devices_created(PlatformBus);
                    of_platform_default_populate_platform_devices_owned(PlatformBus, PlatformBus.platform_devices);
                    of_platform_default_populate_device_refs_bound(PlatformBus);
                    of_platform_default_populate_devices_added_to_bus(PlatformBus);
                    platform_bus_platform_device_set_bound(PlatformBus, PlatformBus.platform_devices);
                    platform_bus_platform_device_owner_ready(PlatformBus, PlatformBus.platform_devices);
                    platform_bus_devices_added_from_platform_device_set(PlatformBus, PlatformBus.platform_devices);
                    platform_device_set_ready(PlatformBus.platform_devices);
                    platform_device_set_nonempty(PlatformBus.platform_devices);
                    bus_type_devices_klist_nonempty(PlatformBus);
                    device_ref_set_nonempty(PlatformBusSubsysPrivate.klist_devices);
                    platform_bus_driver_registered(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver);
                    platform_bus_device_discovered(PlatformBus, DeviceRef::Ns16550aSerial);
                    platform_bus_match_attempted(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_driver_matched_device(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_probe_called(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_probe_return_zero(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_device_bound(PlatformBus, DeviceDriverRef::Ns16550aPlatformDriver, DeviceRef::Ns16550aSerial);
                    platform_bus_ns16550a_driver_registered(PlatformBus);
                    platform_bus_ns16550a_device_matched(PlatformBus);
                    platform_bus_ns16550a_driver_probe_called(PlatformBus);
                    platform_bus_ns16550a_driver_probe_return_zero(PlatformBus);
                    platform_bus_ns16550a_device_bound(PlatformBus);
                    uart_external_irq_enable_ready(UartExternalIrqEnable);
                    uart_external_irq_enable_opens_plic_source_gate(UartExternalIrqEnable, PlicIrqDomain, IrqGateRef::PlicUartSource);
                    uart_external_irq_enable_opens_root_input_gate(UartExternalIrqEnable, RiscvIntc, IrqGateRef::RootSupervisorExternalInput);
                    uart_external_irq_enable_keeps_uart_trigger_deferred(UartExternalIrqEnable, Plic);
                    uart_interrupt_chain_probe_ready(UartInterruptChainProbe);
                    uart_interrupt_chain_probe_observes_plic_claim(UartInterruptChainProbe, Plic);
                    uart_interrupt_chain_probe_observes_irq_dispatch(UartInterruptChainProbe, IrqHandlerRegistry);
                    uart_interrupt_chain_probe_observes_uart_handler(UartInterruptChainProbe, IrqAction);
                    uart_interrupt_chain_probe_observes_plic_complete(UartInterruptChainProbe, Plic);
                    uart_interrupt_chain_probe_observes_plic_loop_exit(UartInterruptChainProbe, Plic);
                    uart_interrupt_chain_probe_observes_irq_cycle_closure(UartInterruptChainProbe, Plic, IrqHandlerRegistry);
                    uart_interrupt_chain_probe_preserves_polling_console(UartInterruptChainProbe, Uart8250Port);
                    serial8250_console_interrupt_driven_ready(Serial8250Console);
                    tty_port_ready(TtyPort, Uart8250Port, TtyFlipBuffer, TtyXmitFifo);
                    tty_port_initialized(TtyPort);
                    tty_flip_buffer_ready(TtyFlipBuffer, TtyPort);
                    tty_xmit_fifo_ready(TtyXmitFifo, TtyPort);
                    serial8250_runtime_port_ready(Serial8250RuntimePort, Uart8250Port, IrqAction, TtyPort);
                    serial8250_runtime_port_online(Serial8250RuntimePort);
                    serial8250_runtime_port_rdi_enabled(Serial8250RuntimePort);
                    serial8250_runtime_port_rlsi_enabled(Serial8250RuntimePort);
                    serial8250_rx_loopback_probe_ready(Serial8250RxLoopbackProbe);
                    serial8250_rx_loopback_probe_single_byte_first_round(
                        Serial8250RxLoopbackProbe,
                        Serial8250RxByteRef::Uart0RxProbe
                    );
                    serial8250_rx_batch_loopback_probe_ready(Serial8250RxBatchLoopbackProbe);
                    serial8250_rx_batch_loopback_probe_bounded_drain_observed(
                        Serial8250RxBatchLoopbackProbe,
                        Serial8250RuntimePort
                    );
                    serial8250_rx_batch_loopback_probe_batch_count_matched(
                        Serial8250RxBatchLoopbackProbe,
                        TtyFlipBuffer
                    );
                    serial8250_rx_batch_loopback_probe_no_overflow(
                        Serial8250RxBatchLoopbackProbe,
                        TtyFlipBuffer
                    );
                    tty_xmit_fifo_probe_ready(TtyXmitFifoProbe);
                    tty_xmit_fifo_probe_enqueue_committed(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_dequeue_committed(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_byte_round_trip(
                        TtyXmitFifoProbe,
                        Serial8250TxByteRef::Uart0TxProbe
                    );
                    tty_xmit_fifo_probe_queue_empty_after_dequeue(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_distinct_from_printk_console_tx(
                        TtyXmitFifoProbe,
                        TtyXmitFifo,
                        Serial8250Console
                    );
                    tty_xmit_fifo_probe_keeps_runtime_tx_deferred(TtyXmitFifoProbe, TtyXmitFifo);
                    tty_xmit_fifo_probe_does_not_kick_uart_thri(TtyXmitFifoProbe, Uart8250Port);
                    tty_xmit_fifo_probe_does_not_mutate_printk_tx_queue(
                        TtyXmitFifoProbe,
                        Serial8250Console
                    );
                    tty_xmit_fifo_probe_no_overflow(TtyXmitFifoProbe, TtyXmitFifo);
                    bus_type_drivers_klist_nonempty(PlatformBus);
                    initcall_boundary_ready(InitcallBoundary);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            RuntimeCorePhase.state == State::Ready;
            CpusetSmpTrimmed.state == State::Ready;
            DriverCoreBase.state == State::Ready;
            PlatformBusRootDevice.state == State::Ready;
            PlatformBus.state == State::Ready;
            Ns16550aPlatformDriver.state == State::Ready;
            UartExternalIrqEnable.state == State::Ready;
            UartInterruptChainProbe.state == State::Ready;
            Serial8250Console.state == State::Online;
            TtyPort.state == State::Online;
            TtyFlipBuffer.state == State::Ready;
            TtyXmitFifo.state == State::Ready;
            Serial8250RuntimePort.state == State::Online;
            Serial8250RxLoopbackProbe.state == State::Ready;
            Serial8250RxBatchLoopbackProbe.state == State::Ready;
            TtyXmitFifoProbe.state == State::Ready;
            DriverCoreDeferred.state == State::Ready;
            IrqProcViewDeferred.state == State::Ready;
            CtorTable.state == State::Ready;
            InitcallTable.state == State::Ready;
            InitcallBoundary.state == State::Ready;
            initcall_phase_ready(InitcallPhase);
        }
    }
}
