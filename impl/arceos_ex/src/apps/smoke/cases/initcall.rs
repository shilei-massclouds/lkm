use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        initcall::{
            InitcallLevelName, InitcallLevelState, INITCALL_ENTRY_COUNT, INITCALL_LEVEL_COUNT,
        },
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
        || !ctx.initcall_table.command_line_scratch_reused_per_level()
        || !ctx.initcall_table.param_parser_applied()
        || !ctx.initcall_table.filter_applied()
        || !ctx.initcall_table.run_context_checked()
        || ctx.initcall_table.entry_count() != INITCALL_ENTRY_COUNT
    {
        printk::write_str("initcall table summary facts invalid\n");
        return SmokeResult::Failed;
    }

    let mut index = 0usize;
    while index < ctx.initcall_table.level_count() {
        let Some(level) = ctx.initcall_table.level(index) else {
            printk::write_str("initcall level missing\n");
            return SmokeResult::Failed;
        };
        if level.name() != expected_level_name(index)
            || level.state() != InitcallLevelState::Done
            || level.entry_count() == 0
        {
            printk::write_str("initcall level facts invalid\n");
            return SmokeResult::Failed;
        }
        index += 1;
    }

    let mut entry_index = 0usize;
    while entry_index < ctx.initcall_table.entry_count() {
        let Some(entry) = ctx.initcall_table.entry(entry_index) else {
            printk::write_str("initcall entry missing\n");
            return SmokeResult::Failed;
        };
        if entry.level() != expected_level_name(entry_index)
            || entry.skipped()
            || entry.return_code() != 0
            || !entry.run_context_checked()
        {
            printk::write_str("initcall entry facts invalid\n");
            return SmokeResult::Failed;
        }
        entry_index += 1;
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
