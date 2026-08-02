use crate::{
    checkpoint::{Checkpoint, dispatch},
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::{AtomicU8, Ordering};

#[unsafe(link_section = ".data.boot_init_flow")]
static BOOT_INIT_SETUP_FACTS: AtomicU8 = AtomicU8::new(0);

const TRIMMED_VMLINUX_BUILD_ID: u8 = 1 << 0;
const TRIMMED_PAGE_ADDRESS_INIT: u8 = 1 << 1;
const START_KERNEL_POSITION_PRESERVED: u8 = 1 << 2;
const REQUIRED_FACTS: u8 =
    TRIMMED_VMLINUX_BUILD_ID | TRIMMED_PAGE_ADDRESS_INIT | START_KERNEL_POSITION_PRESERVED;

pub(super) fn run(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        direct_setup_start(ctx),
        "arceos_ex boot init direct setup start failed\n",
    );
    record_start_kernel_facts();
    crate::phases::shutdown_on_error(
        direct_setup_objects(ctx).and_then(|()| setup_arch_return_ready(ctx)),
        "arceos_ex boot init direct setup failed\n",
    );
    crate::phases::boot::core_prepare::preset(ctx)
}

fn direct_setup_start(ctx: &Context) -> EventResult {
    if !super::is_prepared()
        || ctx.vm.state() != State::Ready
        || ctx.kernel_addr_space.state() != State::Ready
        || !ctx
            .cpu_group
            .boot_cpu()
            .is_some_and(|cpu| ctx.vm.entry_prelude_ready_for(cpu))
        || ctx.boot_task.state() != State::OnCpu
        || ctx.init_stack.state() != State::Ready
        || ctx.boot_cpu_interrupt().state() != State::Ready
        || ctx.raw_dtb.state() != State::Ready
        || ctx.fix_map.state() != State::Ready
        || ctx.kernel_image.state() != State::Online
    {
        return failed_condition(
            LifecycleEvent::Setup,
            ctx.boot_init_flow.state(),
            State::Prepared,
            State::Ready,
        );
    }
    Ok(())
}

fn direct_setup_objects(ctx: &mut Context) -> EventResult {
    let stack_base = ctx.lds.init_stack_start();
    let stack_top = ctx.lds.init_stack_end();
    ctx.boot_task
        .task_mut()
        .enable_stack_guard(stack_base, stack_top)?;
    dispatch(Checkpoint::BootTaskStackGuardEnabled, ctx);

    let Some(boot_hartid) = ctx.cpu_group.boot_hartid() else {
        return direct_setup_failure(ctx);
    };
    ctx.early_dtb.preset(
        &ctx.raw_dtb,
        &ctx.fix_map,
        boot_hartid,
        &mut ctx.platform_cpu_info,
        &mut ctx.physical_memory,
    )?;
    if !ctx.platform_cpu_info.contains(boot_hartid) {
        return direct_setup_failure(ctx);
    }
    ctx.cpu_group.enable_boot_cpu()?;
    printk::preset()?;
    printk::write_str("arceos_ex object kernel\n");

    ctx.early_dtb.setup(
        &ctx.raw_dtb,
        &mut ctx.memblock,
        &mut ctx.command_line,
        &mut ctx.kernel_cmdline,
        &ctx.physical_memory,
    )?;
    ctx.init_mm.setup(&ctx.lds)?;
    ctx.early_ioremap.setup(&ctx.fix_map)?;
    ctx.sbi.setup()?;
    ctx.params.preset(
        &ctx.command_line,
        &ctx.kernel_cmdline,
        &ctx.sbi,
        &mut ctx.early_param,
    )?;
    ctx.memblock.setup(
        &ctx.early_dtb,
        &ctx.kernel_image,
        &ctx.raw_dtb,
        &ctx.config,
        &ctx.lds,
        &ctx.physical_memory,
    )?;
    {
        let Context {
            vm,
            config,
            static_objects,
            lds,
            kernel_image,
            memblock,
            cpu_group,
            kernel_addr_space,
            ..
        } = ctx;
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return failed_condition(
                LifecycleEvent::Setup,
                State::Prepared,
                State::Prepared,
                State::Ready,
            );
        };
        vm.enable(
            config,
            static_objects,
            lds,
            kernel_image,
            memblock,
            boot_cpu,
            kernel_addr_space,
        )?;
    }
    ctx.memblock.enable(ctx.vm.state())?;
    dispatch(Checkpoint::MemBlockOnline, ctx);
    ctx.early_dtb.cleanup(&ctx.memblock, &ctx.early_param)?;

    ctx.device_tree
        .setup(&ctx.raw_dtb, &ctx.vm, &mut ctx.memblock, &ctx.config)?;
    ctx.zones.setup(&ctx.memblock, &ctx.vm)?;
    ctx.page_metadata_map
        .setup(&mut ctx.memblock, &ctx.zones, &ctx.vm, &ctx.config)?;
    ctx.resource_lock.preset_static()?;
    ctx.resource_lock.setup()?;
    ctx.resource_tree.setup(
        &ctx.memblock,
        &ctx.kernel_image,
        &ctx.lds,
        &mut ctx.resource_lock,
    )?;
    ctx.cpu_group.setup_smp(&ctx.device_tree, &ctx.sbi)?;
    ctx.cache_block_info
        .setup(&ctx.device_tree, &ctx.cpu_group)?;
    ctx.cpu_capabilities
        .setup(&ctx.device_tree, &ctx.cpu_group, &ctx.cache_block_info)?;
    ctx.dma_cache_policy
        .setup(&ctx.cpu_capabilities, &ctx.cache_block_info)
}

fn setup_arch_return_ready(ctx: &Context) -> EventResult {
    let ready = super::is_prepared()
        && ctx.init_stack.state() == State::Ready
        && ctx.boot_task.task().stack_guard_installed()
        && ctx.boot_task.task().stack_guard_intact()
        && ctx.cpu_group.boot_cpu_state() == State::Online
        && ctx
            .cpu_group
            .boot_cpu_local_interrupt()
            .map(|control| control.state() == State::Ready && control.disabled())
            .unwrap_or(false)
        && ctx.boot_cpu_interrupt().state() == State::Ready
        && ctx.boot_cpu_interrupt().early_boot_irqs_disabled()
        && ctx.vm.state() == State::Online
        && ctx.kernel_addr_space.state() == State::Online
        && ctx.kernel_addr_space.final_swapper_mappings_published()
        && ctx
            .cpu_group
            .boot_cpu()
            .is_some_and(|cpu| ctx.vm.boot_init_setup_ready_for(cpu))
        && printk::is_prepared()
        && ctx.early_dtb.state() == State::Destroyed
        && ctx.command_line.state() == State::Prepared
        && ctx.kernel_cmdline.state() == State::Ready
        && ctx.init_mm.state() == State::Ready
        && ctx.early_ioremap.state() == State::Ready
        && ctx.sbi.state() == State::Ready
        && ctx.params.state() == State::Prepared
        && ctx.early_param.state() == State::Ready
        && earlycon::is_online()
        && ctx.memblock.state() == State::Online
        && ctx.memblock.boot_init_setup_facts_ready()
        && ctx.device_tree.state() == State::Ready
        && ctx.zones.state() == State::Ready
        && ctx.page_metadata_map.state() == State::Ready
        && ctx.resource_lock.state() == State::Ready
        && ctx.resource_lock.ready()
        && ctx.resource_lock.boot_init_task_write_guard_completed()
        && ctx.resource_tree.state() == State::Ready
        && ctx.resource_tree.write_lock_guard_used()
        && ctx
            .resource_tree
            .resource_lock_write_guard_used_by(&ctx.resource_lock)
        && ctx.cpu_group.state() == State::Ready
        && !ctx.cpu_group.smp_concurrency_open()
        && ctx.cache_block_info.state() == State::Ready
        && ctx.cpu_capabilities.state() == State::Ready
        && ctx.dma_cache_policy.state() == State::Ready
        && start_kernel_facts_ready();
    if ready {
        Ok(())
    } else {
        direct_setup_failure(ctx)
    }
}

fn direct_setup_failure(ctx: &Context) -> EventResult {
    failed_condition(
        LifecycleEvent::Setup,
        ctx.boot_init_flow.state(),
        State::Prepared,
        State::Ready,
    )
}

fn record_start_kernel_facts() {
    BOOT_INIT_SETUP_FACTS.store(REQUIRED_FACTS, Ordering::Relaxed);
}

fn start_kernel_facts_ready() -> bool {
    BOOT_INIT_SETUP_FACTS.load(Ordering::Relaxed) == REQUIRED_FACTS
}
