use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        initcall::{InitcallLevelName, InitcallLevelState, InitcallTable, INITCALL_LEVEL_COUNT},
        printk,
        state::State,
    },
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::smp_runtime::initcall::is_ready() || !phases::smp_runtime::is_ready() {
        printk::write_str("initcall phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.cpuset_smp_trimmed.state() != State::Ready || !ctx.cpuset_smp_trimmed.trimmed_noop() {
        printk::write_str("cpuset SMP trimmed facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.driver_core_base.state() != State::Ready
        || !ctx.driver_core_base.device_registry_ready()
        || !ctx.driver_core_base.bus_registry_ready()
        || !ctx.driver_core_base.pre_platform_deferred()
        || !ctx.driver_core_base.pre_platform_order_preserved()
        || ctx.platform_bus_device.state() != State::Ready
        || !ctx.platform_bus_device.early_platform_cleanup_deferred()
        || !ctx.platform_bus_device.static_device_registered()
        || !ctx.platform_bus_device.device_name_bound()
        || !ctx.platform_bus_device.register_return_zero()
        || ctx.platform_bus_type.state() != State::Ready
        || !ctx.platform_bus_type.registered()
        || !ctx.platform_bus_type.devices_kset_ready()
        || !ctx.platform_bus_type.drivers_kset_ready()
        || !ctx.platform_bus_type.autoprobe_enabled()
        || !ctx.platform_bus_type.ops_bound()
        || !ctx.platform_bus_type.register_return_zero()
        || ctx.driver_core_deferred.state() != State::Ready
        || !ctx.driver_core_deferred.post_platform_deferred()
        || !ctx.driver_core_deferred.entry_position_preserved()
        || ctx.irq_proc_view_deferred.state() != State::Ready
        || !ctx.irq_proc_view_deferred.setup_deferred()
        || !ctx.irq_proc_view_deferred.proc_irq_export_deferred()
    {
        printk::write_str("initcall driver core facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.ctor_table.state() != State::Ready
        || !ctx.ctor_table.position_preserved()
        || !ctx.ctor_table.constructors_empty_or_trimmed()
    {
        printk::write_str("ctor table facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.initcall_table.state() != State::Ready
        || !ctx.initcall_table.static_ranges_ready()
        || !ctx.initcall_table.level_count_ready()
        || ctx.initcall_table.level_count() != INITCALL_LEVEL_COUNT
        || !ctx.initcall_table.all_levels_ran()
        || !ctx.initcall_table.entries_recorded_as_properties()
        || !ctx.initcall_table.registered_entries_collected()
        || !ctx.initcall_table.level_mapping_ready()
        || !ctx.initcall_table.run_levels_ready()
        || !ctx.initcall_table.entry_operation_bindings_ready()
        || !ctx.initcall_table.command_line_scratch_reused_per_level()
        || !ctx.initcall_table.param_parser_applied()
        || !ctx.initcall_table.filter_applied()
        || !ctx.initcall_table.run_context_checked()
        || ctx.initcall_table.entry_count() == 0
        || ctx.initcall_table.run_count() != ctx.initcall_table.entry_count()
        || !ctx.initcall_table.all_registered_entries_ran()
    {
        printk::write_str("initcall table summary facts invalid\n");
        return SmokeResult::Failed;
    }

    let mut index = 0usize;
    let mut level_entry_count = 0usize;
    while index < ctx.initcall_table.level_count() {
        let Some(level) = ctx.initcall_table.level(index) else {
            printk::write_str("initcall level missing\n");
            return SmokeResult::Failed;
        };
        if level.name() != expected_level_name(index) || level.state() != InitcallLevelState::Done {
            printk::write_str("initcall level facts invalid\n");
            return SmokeResult::Failed;
        }
        level_entry_count += level.entry_count();
        index += 1;
    }
    if level_entry_count != ctx.initcall_table.entry_count() {
        printk::write_str("initcall level entry count invalid\n");
        return SmokeResult::Failed;
    }

    if !check_entry_records(&ctx.initcall_table)
        || !check_static_section_entries(&ctx.initcall_table)
    {
        printk::write_str("initcall static section subset invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.initcall_boundary.state() != State::Ready || !ctx.initcall_boundary.kunit_next_boundary()
    {
        printk::write_str("initcall boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "initcall platform_bus=ready levels={} entries={} next=kunit\n",
        ctx.initcall_table.level_count(),
        ctx.initcall_table.entry_count()
    ));
    SmokeResult::Passed
}

fn check_entry_records(table: &InitcallTable) -> bool {
    let mut entry_index = 0usize;
    while entry_index < table.entry_count() {
        let Some(entry) = table.entry(entry_index) else {
            return false;
        };
        let Some(level) = table.run_order_level(entry_index) else {
            return false;
        };
        if entry.level() != level
            || entry.skipped()
            || entry.return_code() != 0
            || !entry.run_context_checked()
        {
            return false;
        }
        entry_index += 1;
    }
    true
}

fn check_static_section_entries(table: &InitcallTable) -> bool {
    let expected = [
        (InitcallLevelName::Pure, "pure_smoke_initcall"),
        (InitcallLevelName::Core, "core_smoke_initcall"),
        (InitcallLevelName::Postcore, "postcore_smoke_initcall"),
        (InitcallLevelName::Arch, "of_platform_default_populate_init"),
        (InitcallLevelName::Subsys, "subsys_smoke_initcall"),
        (InitcallLevelName::Fs, "fs_smoke_initcall"),
        (InitcallLevelName::Device, "device_smoke_initcall"),
        (InitcallLevelName::Late, "late_smoke_initcall"),
    ];
    let mut expected_index = 0usize;
    let mut entry_index = 0usize;

    while entry_index < table.entry_count() {
        let Some(entry) = table.entry(entry_index) else {
            return false;
        };
        if expected_index < expected.len()
            && entry.level() == expected[expected_index].0
            && entry.name() == expected[expected_index].1
            && !entry.skipped()
            && entry.return_code() == 0
            && entry.run_context_checked()
        {
            expected_index += 1;
        }
        entry_index += 1;
    }

    expected_index == expected.len()
}

const fn expected_level_name(index: usize) -> InitcallLevelName {
    match index {
        0 => InitcallLevelName::Pure,
        1 => InitcallLevelName::Core,
        2 => InitcallLevelName::Postcore,
        3 => InitcallLevelName::Arch,
        4 => InitcallLevelName::Subsys,
        5 => InitcallLevelName::Fs,
        6 => InitcallLevelName::Device,
        _ => InitcallLevelName::Late,
    }
}
