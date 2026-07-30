use core::arch::global_asm;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use crate::{
    arch::riscv64::csr,
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        boot_args::BootArgs,
        cpu::TranslationController,
        soc::Soc,
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::TaskRef,
        task_flow::TaskFlowRef,
    },
};

#[unsafe(link_section = ".data.phase")]
static BOOT_TASK_ENTRY_PREEMPTION_INITIALIZED: AtomicBool = AtomicBool::new(false);
#[unsafe(link_section = ".data.phase")]
static BOOT_TASK_ENTRY_BIND_COUNT: AtomicUsize = AtomicUsize::new(0);
#[unsafe(link_section = ".data.phase")]
static BOOT_TASK_ENTRY_BIND_DIAGNOSTIC: AtomicU8 = AtomicU8::new(0);

const HEAD_TEXT_ALIGN: usize = 2;
const SSTATUS_FPU_VECTOR_MASK: usize = (0b11 << 9) | (0b11 << 13);
#[cfg(checkpoint_handler_announce)]
const SBI_LEGACY_CONSOLE_PUTCHAR: usize = 1;

const TRACE_KERNEL_STARTED: usize = Checkpoint::KernelStarted.early_byte() as usize;
const TRACE_BOOT_TASK_ON_CPU: usize = Checkpoint::BootTaskOnCpu.early_byte() as usize;
const TRACE_INTERRUPT_PRESET: usize = b'I' as usize;
const TRACE_KERNEL_IMAGE_PRESET: usize = b'K' as usize;
const TRACE_BSS_ZEROED: usize = b'Z' as usize;
const TRACE_BOOT_CPU_PRESET: usize = b'H' as usize;
const TRACE_INIT_STACK_PRESET: usize = b'S' as usize;

#[repr(C, align(8))]
struct HeadHandoffWord(usize);

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".head.handoff")]
static mut head_boot_hartid: HeadHandoffWord = HeadHandoffWord(0);

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".head.handoff")]
static mut head_init_stack_sp: HeadHandoffWord = HeadHandoffWord(0);

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".head.handoff")]
static mut HEAD_BSS_CLEAR_COMPLETED: HeadHandoffWord = HeadHandoffWord(0);

global_asm!(
    r#"
    .section .head.text.entry, "ax"
    .align {head_text_align}
    .globl _start
_start:
    # Kernel.Enable entry guard: Linux/RV64 requires translation disabled.
    # Reject before KernelStarted or any BootInitFlow child action is observed.
    csrr t0, satp
    bnez t0, 9f

    # Preserve OpenSBI boot arguments while the head segment rewrites a0.
    mv s0, a0
    mv s1, a1

    # Kernel.Preset starts before its first driven phase.
    li a0, {trace_kernel_started}
    call {head_checkpoint}

    # The linker-visible PID 0 carrier already has its only state: Online.
    li a0, {trace_boot_task_on_cpu}
    call {head_checkpoint}

    /*
     * BootInitFlow.Preset head segment.
     *
     * This code performs only the model
     * events that must happen before Rust can run: close the interrupt class
     * gates, complete one pending-clear write, establish gp, disable kernel
     * FPU/vector use, zero BSS, record the boot CPU group input, install the
     * init task pointer, and create the initial stack. The Rust segment below
     * continues the same Preset event.
     */
    # InterruptType.Preset: close class gates, then complete one pending-clear write.
    # Hardware-driven pending bits may be asserted again after this boundary.
    csrw sie, zero
    csrw sip, zero
    li a0, {trace_interrupt_preset}
    call {head_checkpoint}

    # KernelImage.Preset: establish gp before accessing small data.
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    li a0, {trace_kernel_image_preset}
    call {head_checkpoint}

    # BootInitFlow.Preset private action: disable kernel FPU/vector use in sstatus.
    li t0, {sstatus_fpu_vector_mask}
    csrrc zero, sstatus, t0

    # KernelImage.Setup: clear BSS before Rust observes static storage.
    la t0, _sbss
    la t1, _ebss
1:
    bgeu t0, t1, 2f
    sd zero, 0(t0)
    addi t0, t0, 8
    j 1b
2:
    la t0, {head_bss_clear_completed}
    li t1, 1
    sd t1, 0(t0)
    li a0, {trace_bss_zeroed}
    call {head_checkpoint}

    # CpuGroup.cpus[0].Preset: publish the boot hart id for Rust-side adoption.
    la t0, {head_boot_hartid}
    sd s0, 0(t0)
    li a0, {trace_boot_cpu_preset}
    call {head_checkpoint}

    # BootInitFlow.BindBootTaskEntry: bind the physical BootTask carrier in tp.
    # The repeatable action does not drive a separate binding lifecycle.
    la tp, {init_task_storage}

    # InitStack.Preset: reserve temporary page-table space on the boot stack.
    la sp, init_stack_end
    addi sp, sp, -{pt_size_on_stack}
    la t0, {head_init_stack_sp}
    sd sp, 0(t0)
    la t0, {head_trap_entry}
    csrw stvec, t0
    li a0, {trace_init_stack_preset}
    call {head_checkpoint}

    # Continue BootInitFlow.Preset in Rust with the original boot args.
    mv a0, s0
    mv a1, s1
    tail {rust_entry}

9:
    # No stack, gp, lifecycle checkpoint, or child action is valid yet.
    # Stay fail-stopped at the exact rejected handoff boundary.
    j 9b

"#,
    head_boot_hartid = sym head_boot_hartid,
    head_bss_clear_completed = sym HEAD_BSS_CLEAR_COMPLETED,
    head_checkpoint = sym arceos_ex_head_checkpoint,
    head_init_stack_sp = sym head_init_stack_sp,
    head_trap_entry = sym arceos_ex_head_trap_entry,
    head_text_align = const HEAD_TEXT_ALIGN,
    init_task_storage = sym crate::objects::boot_task::init_task_storage,
    pt_size_on_stack = const crate::objects::init_stack::PT_SIZE_ON_STACK,
    rust_entry = sym boot_init_flow_preset_rust_entry,
    sstatus_fpu_vector_mask = const SSTATUS_FPU_VECTOR_MASK,
    trace_boot_cpu_preset = const TRACE_BOOT_CPU_PRESET,
    trace_boot_task_on_cpu = const TRACE_BOOT_TASK_ON_CPU,
    trace_bss_zeroed = const TRACE_BSS_ZEROED,
    trace_init_stack_preset = const TRACE_INIT_STACK_PRESET,
    trace_interrupt_preset = const TRACE_INTERRUPT_PRESET,
    trace_kernel_started = const TRACE_KERNEL_STARTED,
    trace_kernel_image_preset = const TRACE_KERNEL_IMAGE_PRESET,
);

global_asm!(
    r#"
    .section .head.text.trap, "ax"
    .align 2
    .globl arceos_ex_head_trap_entry
arceos_ex_head_trap_entry:
    mv      a3, ra
    mv      a4, sp
    csrr    a5, satp
    csrr    a0, scause
    csrr    a1, sepc
    csrr    a2, stval
    tail    {head_trap_rust}
"#,
    head_trap_rust = sym arceos_ex_head_trap_rust,
);

unsafe extern "C" {
    fn arceos_ex_head_trap_entry();
}

#[cfg(checkpoint_handler_announce)]
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn arceos_ex_head_checkpoint() {
    core::arch::naked_asm!(
        "li a7, {sbi_legacy_console_putchar}",
        "ecall",
        "ret",
        sbi_legacy_console_putchar = const SBI_LEGACY_CONSOLE_PUTCHAR,
    )
}

#[cfg(not(checkpoint_handler_announce))]
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn arceos_ex_head_checkpoint() {
    core::arch::naked_asm!("ret")
}

#[unsafe(no_mangle)]
extern "C" fn arceos_ex_head_trap_rust(
    scause: usize,
    sepc: usize,
    stval: usize,
    ra: usize,
    sp: usize,
    satp: usize,
) -> ! {
    crate::arch::riscv64::sbi::putstr("arceos_ex head trap\n");
    crate::arch::riscv64::sbi::putstr("scause=0x");
    print_hex(scause);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(stval);
    crate::arch::riscv64::sbi::putstr(" ra=0x");
    print_hex(ra);
    crate::arch::riscv64::sbi::putstr(" sp=0x");
    print_hex(sp);
    crate::arch::riscv64::sbi::putstr(" satp=0x");
    print_hex(satp);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn print_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        crate::arch::riscv64::sbi::putchar(HEX[(value >> shift) & 0xf]);
    }
}

#[unsafe(no_mangle)]
extern "C" fn boot_init_flow_preset_rust_entry(hartid: usize, dtb_pa: usize) -> ! {
    let boot_args = BootArgs::new(hartid, dtb_pa);
    crate::phases::shutdown_on_error(
        crate::phases::prepare::adopt_head_prefix(&boot_args),
        "arceos_ex prepare event failed\n",
    );
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        ctx.cpu_group.preset(&boot_args),
        "arceos_ex cpu group preset failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::CpuGroupPrepared);
    let Some(boot_cpu_ref) = ctx.cpu_group.boot_cpu_ref() else {
        crate::arch::riscv64::sbi::system_shutdown()
    };
    if !ctx.boot_init_flow.bind_cpu_ref(boot_cpu_ref) {
        crate::arch::riscv64::sbi::system_shutdown()
    }
    crate::phases::shutdown_on_error(
        crate::systems::kernel::accept_enable_at_entry(&boot_args),
        "arceos_ex kernel enable failed\n",
    );
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        crate::arch::riscv64::sbi::system_shutdown()
    };
    crate::phases::shutdown_on_error(
        ctx.vm.activate_physical_on_cpu(boot_cpu),
        "arceos_ex physical translation activation failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::BootInitFlowStarted);
    crate::phases::shutdown_on_error(
        super::adopt_head_preset_start(),
        "arceos_ex boot init preset start failed\n",
    );
    crate::phases::shutdown_on_error(
        adopt_preset_dependencies(&boot_args),
        "arceos_ex boot init preset dependency failed\n",
    );
    preset_flow(&boot_args)
}

fn adopt_preset_dependencies(boot_args: &BootArgs) -> EventResult {
    let ctx = crate::context::context_ref();
    let state = ctx.boot_init_flow.state();
    if state != State::Base
        || boot_args.state() != State::Online
        || !crate::phases::prepare::is_online()
        || ctx.config.state() != State::Online
        || !ctx.config.entry_prelude_ready()
        || ctx.lds.state() != State::Online
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    Ok(())
}

/// Continues `BootInitFlow.Preset` after the `_start` head segment.
///
/// The head segment above has already performed and checkpointed the pre-Rust
/// lifecycle events:
///
/// - `InterruptType.Preset`
/// - `KernelImage.Preset`
/// - BootInitFlow.Preset private FPU/vector disable action
/// - `KernelImage.Setup`
/// - `CpuGroup.cpus[0].Preset`
/// - physical `tp` installation for the later BindBootTaskEntry action
/// - `InitStack.Preset`
///
/// This Rust segment adopts those completed events into the resource objects,
/// then continues the remaining direct entry-object drives in model order. It
/// also supplies the Flow-owned continuation that resumes this same Preset
/// after `Vm.Setup` switches to the early virtual address space.
fn preset_flow(boot_args: &BootArgs) -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        preset_until_vm_switch(ctx, boot_args),
        "arceos_ex boot init preset failed\n",
    );
    ctx.vm.setup(
        &ctx.config,
        &ctx.static_objects,
        &ctx.lds,
        &mut ctx.kernel_image,
        ctx.cpu_group
            .boot_cpu()
            .unwrap_or_else(|| crate::arch::riscv64::sbi::system_shutdown()),
        after_vm_setup_continuation,
    )
}

fn preset_until_vm_switch(ctx: &mut Context, boot_args: &BootArgs) -> EventResult {
    adopt_head_prefix(ctx, boot_args)?;
    ctx.kernel_addr_space
        .preset(&ctx.config, &ctx.lds, &ctx.kernel_image)?;
    {
        let Context {
            cpu_group,
            kernel_image,
            ..
        } = ctx;
        let Some(trap) = cpu_group.boot_cpu_trap_mut() else {
            return failed_condition(
                LifecycleEvent::Preset,
                State::Base,
                State::Base,
                State::Prepared,
            );
        };
        trap.preset(kernel_image)?;
    }
    ctx.vm.preset(
        &ctx.config,
        &mut ctx.static_objects,
        &ctx.lds,
        &ctx.kernel_image,
        boot_args,
        &mut ctx.raw_dtb,
        &mut ctx.fix_map,
        &mut ctx.kernel_addr_space,
    )
}

fn adopt_head_prefix(ctx: &mut Context, boot_args: &BootArgs) -> EventResult {
    let Some(interrupt) = ctx.cpu_group.boot_cpu_interrupt_mut() else {
        return failed_condition(
            LifecycleEvent::Preset,
            State::Base,
            State::Base,
            State::Prepared,
        );
    };
    interrupt.adopt_head_preset()?;
    ctx.kernel_image.adopt_head_preset(&ctx.config, &ctx.lds)?;
    if !csr::kernel_fpu_vector_disabled() {
        return failed_condition(
            LifecycleEvent::Preset,
            ctx.boot_init_flow.state(),
            State::Base,
            State::Prepared,
        );
    }
    ctx.kernel_image
        .adopt_head_setup(&ctx.lds, head_bss_clear_completed())?;
    ctx.cpu_group
        .setup_boot_cpu(ctx.cpu_group.boot_hartid() == Some(boot_args.boot_hartid()))?;
    let Some(local_interrupt) = ctx.cpu_group.boot_cpu_local_interrupt_mut() else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Base,
            State::Ready,
        );
    };
    local_interrupt.setup_local_control()?;
    local_interrupt.setup()?;
    bind_boot_task_entry(ctx, TaskRef::BOOT)?;
    let Ok(current_task) = ctx.current_task() else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    };
    if !current_task.task_ref().same_identity(TaskRef::BOOT) {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    }
    let Ok(current_cpu) = ctx.current_cpu() else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    };
    if current_cpu.logical_id() != 0 {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    }
    ctx.init_stack
        .adopt_head_preset(&ctx.kernel_image, &ctx.lds)
}

/// Continues the same `BootInitFlow.Preset` after `Vm.Setup` has switched to
/// the early virtual address space. Control returns here directly through the
/// Flow-owned continuation.
extern "C" fn after_vm_setup_continuation() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        after_vm_setup(ctx),
        "arceos_ex boot init preset tail failed\n",
    );
    super::preset_after_entry_objects()
}

/// Finishes `BootInitFlow.Preset` after `Vm.Setup` has switched address
/// spaces and returned through the virtual continuation path.
fn after_vm_setup(ctx: &mut Context) -> EventResult {
    let Some(boot_cpu_ref) = ctx.cpu_group.boot_cpu_ref() else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    };
    let boot_task_identity = ctx.boot_task.carrier_address();
    let boot_stack_base = ctx.lds.init_stack_start();
    let boot_stack_top = ctx.lds.init_stack_end();
    let Context {
        cpu_group,
        vm,
        static_objects,
        ..
    } = ctx;
    let Some(trap) = cpu_group.boot_cpu_trap_mut() else {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    };
    trap.setup(
        vm,
        static_objects,
        boot_cpu_ref,
        boot_task_identity,
        boot_stack_base,
        boot_stack_top,
    )?;
    bind_boot_task_entry(ctx, TaskRef::BOOT)?;
    if !ctx
        .boot_task
        .task_mut()
        .set_kernel_stack_bounds(ctx.lds.init_stack_start(), ctx.lds.init_stack_end())
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Prepared,
            State::Ready,
        );
    }
    verify_boot_task_online_virtual(ctx)?;
    ctx.init_stack.setup(&ctx.vm)?;
    Soc::preset()
}

/// Repeatable lowering of `BootInitFlow.Action::BindBootTaskEntry`.
/// It has no lifecycle/snapshot state; the sole persistent fact is whether
/// the entry preemption count has been initialized once.
fn bind_boot_task_entry(ctx: &Context, current_task_ref: TaskRef) -> EventResult {
    let carrier_address = ctx.boot_task.carrier_address();
    let Some(init_task_phys) = ctx.kernel_image.runtime_to_phys(carrier_address) else {
        return bind_boot_task_entry_failed(1);
    };
    let Some(init_task_virt) = ctx.kernel_image.runtime_to_link(carrier_address) else {
        return bind_boot_task_entry_failed(1);
    };
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return bind_boot_task_entry_failed(2);
    };
    if !current_task_ref.same_identity(TaskRef::BOOT)
        || ctx.boot_task.state() != State::OnCpu
        || ctx.boot_task.task().task_ref() != TaskRef::BOOT
        || ctx.boot_task.task().pid() != 0
        || !ctx
            .boot_task
            .task()
            .active_flow()
            .same_identity(TaskFlowRef::BOOT_INIT)
        || !ctx.boot_init_flow.core().active()
        || ctx.boot_task.task_ref() != TaskRef::BOOT
    {
        return bind_boot_task_entry_failed(3);
    }
    let observed_tp = csr::read_tp();
    if observed_tp != init_task_phys && observed_tp != init_task_virt {
        return bind_boot_task_entry_failed(4);
    }

    match boot_cpu.active_translation_controller() {
        Ok(Some(TranslationController::PhysicalDirect)) => {
            if csr::read_satp() != 0 || ctx.kernel_image.state() != State::Ready {
                return bind_boot_task_entry_failed(5);
            }
            csr::write_tp(init_task_phys);
            let _ = BOOT_TASK_ENTRY_PREEMPTION_INITIALIZED.compare_exchange(
                false,
                true,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        Ok(Some(
            controller @ (TranslationController::EarlyVm | TranslationController::SwapperVm),
        )) => {
            let expected_satp = if controller == TranslationController::EarlyVm {
                ctx.vm.early_vm_satp()
            } else {
                ctx.vm.swapper_vm().satp()
            };
            if expected_satp == 0
                || csr::read_satp() != expected_satp
                || !boot_task_entry_preemption_initialized()
                || init_task_virt != carrier_address
            {
                return bind_boot_task_entry_failed(5);
            }
            csr::write_tp(init_task_virt);
        }
        Ok(None | Some(TranslationController::TrampolineVm)) | Err(_) => {
            return bind_boot_task_entry_failed(6);
        }
    }
    BOOT_TASK_ENTRY_BIND_COUNT.fetch_add(1, Ordering::Relaxed);
    BOOT_TASK_ENTRY_BIND_DIAGNOSTIC.store(0, Ordering::Release);
    Ok(())
}

fn bind_boot_task_entry_failed(code: u8) -> EventResult {
    BOOT_TASK_ENTRY_BIND_DIAGNOSTIC.store(code, Ordering::Release);
    crate::arch::riscv64::sbi::putstr("BindBootTaskEntry failed code=");
    crate::arch::riscv64::sbi::putchar(b'0' + code.min(9));
    crate::arch::riscv64::sbi::putchar(b'\n');
    failed_condition(
        LifecycleEvent::Continue,
        State::OnCpu,
        State::OnCpu,
        State::OnCpu,
    )
}

/// Verifies that the virtual binding still resolves to the pre-existing
/// Online BootTask. This phase never drives the Task lifecycle.
fn verify_boot_task_online_virtual(ctx: &Context) -> EventResult {
    let task_state = ctx.boot_task.state();
    let carrier_address = ctx.boot_task.carrier_address();
    let Some(init_task_virt) = ctx.kernel_image.runtime_to_link(carrier_address) else {
        return failed_condition(
            LifecycleEvent::Setup,
            task_state,
            State::OnCpu,
            State::OnCpu,
        );
    };

    if !boot_task_entry_preemption_initialized()
        || BOOT_TASK_ENTRY_BIND_COUNT.load(Ordering::Acquire) < 2
        || BOOT_TASK_ENTRY_BIND_DIAGNOSTIC.load(Ordering::Acquire) != 0
        || task_state != State::OnCpu
        || ctx.boot_task.task().task_ref() != TaskRef::BOOT
        || ctx.boot_task.task().pid() != 0
        || !ctx
            .boot_task
            .task()
            .active_flow()
            .same_identity(TaskFlowRef::BOOT_INIT)
        || !ctx.boot_init_flow.core().active()
        || ctx.vm.state() != State::Ready
        || !ctx
            .cpu_group
            .boot_cpu()
            .is_some_and(|cpu| ctx.vm.entry_prelude_ready_for(cpu))
        || ctx.kernel_image.state() != State::Online
        || init_task_virt != carrier_address
        || csr::read_tp() != init_task_virt
    {
        return failed_condition(
            LifecycleEvent::Setup,
            task_state,
            State::OnCpu,
            State::OnCpu,
        );
    }
    Ok(())
}

pub(crate) fn boot_task_entry_preemption_initialized() -> bool {
    BOOT_TASK_ENTRY_PREEMPTION_INITIALIZED.load(Ordering::Relaxed)
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn boot_task_entry_bind_count() -> usize {
    BOOT_TASK_ENTRY_BIND_COUNT.load(Ordering::Acquire)
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn boot_task_entry_bind_diagnostic() -> u8 {
    BOOT_TASK_ENTRY_BIND_DIAGNOSTIC.load(Ordering::Acquire)
}

fn head_bss_clear_completed() -> bool {
    unsafe {
        core::ptr::addr_of!(HEAD_BSS_CLEAR_COMPLETED)
            .read_volatile()
            .0
            == 1
    }
}

pub(super) fn entry_objects_ready(ctx: &Context) -> bool {
    csr::kernel_fpu_vector_disabled()
        && ctx.boot_cpu_interrupt().state() == State::Ready
        && ctx.boot_cpu_trap().state() == State::Ready
        && ctx.boot_cpu_exception().state() == State::Prepared
        && ctx.boot_cpu_exception().page_fault_state() == State::Prepared
        && ctx.boot_cpu_exception().syscall_state() == State::Prepared
        && ctx.boot_cpu_exception().breakpoint_state() == State::Prepared
        && ctx.boot_cpu_exception().unexpected_state() == State::Prepared
        && ctx.kernel_image.state() == State::Online
        && ctx.kernel_addr_space.state() == State::Ready
        && ctx.raw_dtb.state() == State::Ready
        && boot_task_entry_preemption_initialized()
        && BOOT_TASK_ENTRY_BIND_COUNT.load(Ordering::Acquire) >= 2
        && BOOT_TASK_ENTRY_BIND_DIAGNOSTIC.load(Ordering::Acquire) == 0
        && ctx.boot_task.state() == State::OnCpu
        && ctx.boot_task.task().task_ref() == TaskRef::BOOT
        && ctx.boot_task.task().pid() == 0
        && ctx
            .boot_task
            .task()
            .active_flow()
            .same_identity(TaskFlowRef::BOOT_INIT)
        && ctx.boot_init_flow.core().active()
        && csr::read_tp() == ctx.boot_task.carrier_address()
        && ctx.init_stack.state() == State::Ready
        && ctx.vm.state() == State::Ready
        && ctx
            .cpu_group
            .boot_cpu()
            .is_some_and(|cpu| ctx.vm.entry_prelude_ready_for(cpu))
        && ctx.boot_init_flow.cpu_ref() == ctx.cpu_group.boot_cpu_ref()
        && ctx
            .cpu_group
            .boot_cpu_local_interrupt()
            .map(|control| control.state() == State::Ready && control.disabled())
            .unwrap_or(false)
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(TaskRef::BOOT))
        && ctx.cpu_group.state() == State::Prepared
        && ctx.cpu_group.boot_cpu_state() == State::Ready
}
