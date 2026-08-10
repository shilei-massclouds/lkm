use crate::{
    checkpoint::{Checkpoint, dispatch},
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventResult, FailureDiagnostic, LifecycleEvent, State, failed_condition},
        task_flow::task_flow_execution_guard_satisfied,
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

/// Starts BootInitFlow.Setup with its first direct leaf.
#[unsafe(no_mangle)]
pub extern "C" fn start_kernel() -> ! {
    let result = if start_kernel_entry_guard_satisfied() {
        Ok(())
    } else {
        super::phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex start_kernel guard failed\n");
    run(crate::context::context())
}

pub fn setup_after_core_prepare() -> ! {
    require_setup_leaf(
        crate::phases::boot::core_prepare::is_online(),
        "core prepare",
    );
    crate::phases::boot::mm_core_init::preset(crate::context::context())
}

pub fn setup_after_mm_core_init() -> ! {
    require_setup_leaf(
        crate::phases::boot::mm_core_init::is_online(),
        "mm core init",
    );
    crate::phases::boot::sched_init::preset(crate::context::context())
}

pub fn setup_after_sched_init() -> ! {
    require_setup_leaf(crate::phases::boot::sched_init::is_online(), "sched init");
    crate::phases::interrupt::irq_time_init::preset(crate::context::context())
}

pub fn setup_after_irq_time_init() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::irq_time_init::is_online(),
        "irq time init",
    );
    crate::phases::interrupt::local_irq_enable::preset(crate::context::context())
}

pub fn setup_after_local_irq_enable() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::local_irq_enable::is_online(),
        "local irq enable",
    );
    crate::phases::interrupt::irq_open_prepare::preset(crate::context::context())
}

pub fn setup_after_irq_open_prepare() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::irq_open_prepare::is_online(),
        "irq open prepare",
    );
    crate::phases::interrupt::process_prepare::preset(crate::context::context())
}

pub fn setup_after_process_prepare() -> ! {
    require_setup_leaf(
        crate::phases::interrupt::process_prepare::is_online(),
        "process prepare",
    );
    super::rest_init::preset(crate::context::context())
}

/// Completes BootInitFlow.Setup after the final direct leaf reaches Online.
pub fn setup_after_boot_init_rest_init() -> ! {
    let dependencies_ready = super::setup_leaves_online() && super::rest_init::is_online();
    let ctx = crate::context::context();
    let result = if dependencies_ready {
        ctx.boot_task
            .task_mut()
            .setup_embedded_flow(Checkpoint::BootInitFlowReady)
    } else {
        failed_condition(
            LifecycleEvent::Setup,
            ctx.boot_task.task().flow_state(),
            State::Prepared,
            State::Ready,
        )
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init setup failed\n");
    super::enable()
}

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
            ctx.boot_task.task().flow_state(),
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
    let Some(first_failed) = setup_arch_return_first_failed(ctx) else {
        return Ok(());
    };
    if first_failed == "boot_task.stack_guard_intact" {
        print_boot_task_stack_guard_diagnostic(ctx);
    }
    direct_setup_failure(ctx).map_err(|error| {
        error.with_diagnostic_if_absent(FailureDiagnostic::new(
            "BootInitFlow",
            "setup_arch_return",
            "BootInitFlow",
            "setup_arch_return_ready",
            first_failed,
        ))
    })
}

fn print_boot_task_stack_guard_diagnostic(ctx: &Context) {
    let task = ctx.boot_task.task();
    let task_ref = task.task_ref();
    let base = task.kernel_stack_base();
    let top = task.kernel_stack_top();
    let live_sp = crate::arch::riscv64::csr::read_sp();
    let actual = task.stack_guard_observed_value();
    let cpu = ctx
        .current_cpu()
        .map_or(usize::MAX, |current| current.logical_id());

    crate::arch::riscv64::sbi::putstr("stack_guard_diagnostic task_slot=");
    put_hex(task_ref.slot());
    crate::arch::riscv64::sbi::putstr(" generation=");
    put_hex(task_ref.generation() as usize);
    crate::arch::riscv64::sbi::putstr(" cpu=");
    put_hex(cpu);
    crate::arch::riscv64::sbi::putstr(" base=");
    put_hex(base);
    crate::arch::riscv64::sbi::putstr(" top=");
    put_hex(top);
    crate::arch::riscv64::sbi::putstr(" live_sp=");
    put_hex(live_sp);
    crate::arch::riscv64::sbi::putstr(" expected=");
    put_hex(crate::objects::task::TASK_STACK_GUARD_VALUE);
    crate::arch::riscv64::sbi::putstr(" actual=");
    if let Some(actual) = actual {
        put_hex(actual);
    } else {
        crate::arch::riscv64::sbi::putstr("unavailable");
    }
    crate::arch::riscv64::sbi::putchar(b'\n');
}

fn put_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    crate::arch::riscv64::sbi::putstr("0x");
    let mut shift = usize::BITS - 4;
    loop {
        crate::arch::riscv64::sbi::putchar(HEX[(value >> shift) & 0xf]);
        if shift == 0 {
            break;
        }
        shift -= 4;
    }
}

fn setup_arch_return_first_failed(ctx: &Context) -> Option<&'static str> {
    macro_rules! require {
        ($condition:expr, $name:literal) => {
            if !$condition {
                return Some($name);
            }
        };
    }

    require!(super::is_prepared(), "boot_init_flow.prepared");
    require!(ctx.init_stack.state() == State::Ready, "init_stack.ready");
    require!(
        ctx.boot_task.task().stack_guard_installed(),
        "boot_task.stack_guard_installed"
    );
    require!(
        ctx.boot_task.task().stack_guard_intact(),
        "boot_task.stack_guard_intact"
    );
    require!(
        ctx.cpu_group.boot_cpu_state() == State::Online,
        "boot_cpu.online"
    );
    let Some(local_interrupt) = ctx.cpu_group.boot_cpu_local_interrupt() else {
        return Some("boot_cpu.local_interrupt_present");
    };
    require!(
        local_interrupt.state() == State::Ready,
        "boot_cpu.local_interrupt_ready"
    );
    require!(
        local_interrupt.disabled(),
        "boot_cpu.local_interrupt_disabled"
    );
    require!(
        ctx.boot_cpu_interrupt().state() == State::Ready,
        "boot_cpu.interrupt_type_ready"
    );
    require!(
        ctx.boot_cpu_interrupt().early_boot_irqs_disabled(),
        "boot_cpu.early_boot_irqs_disabled"
    );
    require!(ctx.vm.state() == State::Online, "vm.online");
    require!(
        ctx.kernel_addr_space.state() == State::Online,
        "kernel_addr_space.online"
    );
    require!(
        ctx.kernel_addr_space.final_swapper_mappings_published(),
        "kernel_addr_space.final_swapper_mappings_published"
    );
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return Some("boot_cpu.present");
    };
    require!(
        ctx.vm.boot_init_setup_ready_for(boot_cpu),
        "vm.boot_init_setup_ready_for_boot_cpu"
    );
    require!(printk::is_prepared(), "printk.prepared");
    require!(
        ctx.early_dtb.state() == State::Destroyed,
        "early_dtb.destroyed"
    );
    require!(
        ctx.command_line.state() == State::Prepared,
        "command_line.prepared"
    );
    require!(
        ctx.kernel_cmdline.state() == State::Ready,
        "kernel_cmdline.ready"
    );
    require!(ctx.init_mm.state() == State::Ready, "init_mm.ready");
    require!(
        ctx.early_ioremap.state() == State::Ready,
        "early_ioremap.ready"
    );
    require!(ctx.sbi.state() == State::Ready, "sbi.ready");
    require!(ctx.params.state() == State::Prepared, "params.prepared");
    require!(ctx.early_param.state() == State::Ready, "early_param.ready");
    require!(earlycon::is_online(), "earlycon.online");
    require!(ctx.memblock.state() == State::Online, "memblock.online");
    require!(
        ctx.memblock.boot_init_setup_facts_ready(),
        "memblock.boot_init_setup_facts_ready"
    );
    require!(ctx.device_tree.state() == State::Ready, "device_tree.ready");
    require!(ctx.zones.state() == State::Ready, "zones.ready");
    require!(
        ctx.page_metadata_map.state() == State::Ready,
        "page_metadata_map.ready"
    );
    require!(
        ctx.resource_lock.state() == State::Ready,
        "resource_lock.ready_state"
    );
    require!(ctx.resource_lock.ready(), "resource_lock.ready_fact");
    require!(
        ctx.resource_lock.boot_init_task_write_guard_completed(),
        "resource_lock.boot_init_task_write_guard_completed"
    );
    require!(
        ctx.resource_tree.state() == State::Ready,
        "resource_tree.ready"
    );
    require!(
        ctx.resource_tree.write_lock_guard_used(),
        "resource_tree.write_lock_guard_used"
    );
    require!(
        ctx.resource_tree
            .resource_lock_write_guard_used_by(&ctx.resource_lock),
        "resource_tree.resource_lock_write_guard_used"
    );
    require!(ctx.cpu_group.state() == State::Ready, "cpu_group.ready");
    require!(
        !ctx.cpu_group.smp_concurrency_open(),
        "cpu_group.smp_concurrency_closed"
    );
    require!(
        ctx.cache_block_info.state() == State::Ready,
        "cache_block_info.ready"
    );
    require!(
        ctx.cpu_capabilities.state() == State::Ready,
        "cpu_capabilities.ready"
    );
    require!(
        ctx.dma_cache_policy.state() == State::Ready,
        "dma_cache_policy.ready"
    );
    require!(
        start_kernel_facts_ready(),
        "start_kernel.trimmed_facts_ready"
    );
    None
}

fn direct_setup_failure(ctx: &Context) -> EventResult {
    failed_condition(
        LifecycleEvent::Setup,
        ctx.boot_task.task().flow_state(),
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

fn require_setup_leaf(child_online: bool, child: &str) {
    let result = if child_online
        && super::require_guarded_state(LifecycleEvent::Setup, State::Prepared, State::Ready)
            .is_ok()
    {
        Ok(())
    } else {
        super::phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    let message = match child {
        "core prepare" => "arceos_ex boot init after core prepare failed\n",
        "mm core init" => "arceos_ex boot init after mm core init failed\n",
        "sched init" => "arceos_ex boot init after sched init failed\n",
        "irq time init" => "arceos_ex boot init after irq time init failed\n",
        "local irq enable" => "arceos_ex boot init after local irq enable failed\n",
        "irq open prepare" => "arceos_ex boot init after irq open prepare failed\n",
        _ => "arceos_ex boot init after process prepare failed\n",
    };
    crate::phases::shutdown_on_error(result, message)
}

fn start_kernel_entry_guard_satisfied() -> bool {
    let ctx = crate::context::context_ref();
    let flow = ctx.boot_task.task().embedded_flow();
    let cpu_ref = ctx.boot_task.task().flow_cpu_ref();
    crate::systems::kernel::enable_in_progress()
        && ctx.boot_task.task().flow_state() == State::Prepared
        && super::boot_task_on_cpu_and_canonical()
        && ctx.boot_task.task().flow().same_identity(flow.flow_ref())
        && task_flow_execution_guard_satisfied(flow, ctx.boot_task.task())
        && cpu_ref.is_some()
        && cpu_ref == ctx.cpu_group.boot_cpu_ref()
        && ctx
            .current_cpu()
            .is_ok_and(|current| Some(current.cpu_ref()) == cpu_ref)
}
