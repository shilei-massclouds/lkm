use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        cpu::MAX_CPUS,
        cpu_group::CpuGroup,
        smp_bringup::smp_bringup_runtime_ready,
        state::{EventResult, FailureDiagnostic, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const AP_WAIT_SPINS: usize = 50_000_000;

#[unsafe(link_section = ".data.phase")]
static SMP_BRINGUP_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));
static AP_PREREQUISITES_READY: AtomicBool = AtomicBool::new(false);

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(),
        "arceos_ex smp bringup preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::SmpBringupPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared(ctx)),
        "arceos_ex smp bringup preset failed\n",
    );
    setup(ctx)
}

fn preset_start() -> EventResult {
    let state = crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE);
    if state != State::Base
        || !crate::flows::boot_init_flow::is_online()
        || !crate::phases::smp_runtime::pre_smp_init::is_online()
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    preset_step(
        ctx.secondary_idle_tasks.preset(
            &ctx.pre_smp_boundary,
            &mut ctx.cpu_group,
            &ctx.per_cpu_storage,
        ),
        "secondary_idle_tasks.preset",
    )?;
    preset_step(
        ctx.smpboot_threads_lock.preset_static(),
        "smpboot_threads_lock.preset",
    )?;
    preset_step(
        ctx.smpboot_threads_lock.setup(),
        "smpboot_threads_lock.setup",
    )?;
    preset_step(
        ctx.cpu_hotplug_sync.preset(
            &mut ctx.cpu_group,
            &ctx.boot_idle_flow,
            &ctx.kthreadd_task,
            &mut ctx.cpu_hotplug_lock,
            &mut ctx.smpboot_threads_lock,
        ),
        "cpu_hotplug_sync.preset",
    )?;
    preset_step(
        ctx.cpu_add_remove_lock.preset_static(),
        "cpu_add_remove_lock.preset",
    )?;
    preset_step(ctx.cpu_add_remove_lock.setup(), "cpu_add_remove_lock.setup")?;
    preset_step(
        ctx.cpu_running_wait_lock
            .setup_with_checkpoint(Checkpoint::CpuRunningWaitLockReady),
        "cpu_running_wait_lock.setup",
    )?;
    preset_step(
        ctx.done_up_wait_lock
            .setup_with_checkpoint(Checkpoint::CpuDoneUpWaitLockReady),
        "done_up_wait_lock.setup",
    )?;
    reset_ap_phase_families(&ctx.cpu_group);
    publish_ap_prerequisites(ctx)?;
    ctx.cpu_start_provider.setup(
        &mut ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.cpu_hotplug_sync,
        &ctx.sbi,
        &ctx.sbi_ipi,
        &ctx.kernel_image,
        &ctx.static_objects,
        &ctx.vm,
        &ctx.lds,
        &mut ctx.cpu_add_remove_lock,
        &mut ctx.cpu_hotplug_lock,
    )?;
    if !wait_for_ap_phase_families(&ctx.cpu_group) {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    }
    ctx.secondary_cpu_startup_ack.setup(
        &ctx.cpu_start_provider,
        &mut ctx.cpu_group,
        &mut ctx.cpu_hotplug_sync,
        &mut ctx.cpu_running_wait_lock,
    )?;
    ctx.secondary_cpu_online_ack.setup(
        &ctx.secondary_cpu_startup_ack,
        &mut ctx.cpu_hotplug_sync,
        &mut ctx.cpu_group,
        &ctx.sbi_ipi,
        &mut ctx.done_up_wait_lock,
    )?;
    ctx.smp_bringup_boundary
        .setup(&ctx.secondary_cpu_online_ack, &ctx.cpu_group)
}

fn preset_step(result: EventResult, step: &'static str) -> EventResult {
    result.map_err(|error| {
        error.with_diagnostic_if_absent(FailureDiagnostic::new(
            "SmpBringupPhase",
            "preset_objects",
            "SmpBringupPhase",
            "preset object step",
            step,
        ))
    })
}

fn adopt_prepared(ctx: &Context) -> EventResult {
    transition_if_ready(
        ctx,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::SmpBringupPhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SmpBringupPhaseReady,
        ),
        "arceos_ex smp bringup setup failed\n",
    );
    enable(ctx)
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::SmpBringupPhaseOnline,
        ),
        "arceos_ex smp bringup enable failed\n",
    );
    crate::phases::smp_runtime::preset_after_smp_bringup()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let state = crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE);
    if state != expected || !smp_bringup_phase_ready(ctx) {
        return failed_condition(event, state, expected, target);
    }

    crate::phases::state::mark_checked(
        &SMP_BRINGUP_PHASE_STATE,
        event,
        expected,
        target,
        checkpoint,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE) == State::Online
}

fn smp_bringup_phase_ready(ctx: &Context) -> bool {
    smp_bringup_runtime_ready(
        &ctx.kernel_init_task,
        &ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.smpboot_threads_lock,
        &ctx.cpu_hotplug_sync,
        &ctx.cpu_add_remove_lock,
        &ctx.cpu_hotplug_lock,
        &ctx.cpu_running_wait_lock,
        &ctx.done_up_wait_lock,
        &ctx.cpu_start_provider,
        &ctx.secondary_cpu_startup_ack,
        &ctx.secondary_cpu_online_ack,
        &ctx.smp_bringup_boundary,
    )
}

pub(crate) fn ap_after_entry_prelude(logical_id: usize) -> ! {
    if super::ap_entry_prelude::state_for(logical_id) != State::Online {
        ap_phase_fail_stop(
            "ApEntryPreludePhase",
            logical_id,
            super::ap_entry_prelude::state_for(logical_id),
            "parent-continuation",
        );
    }
    super::ap_smp_callin::preset(logical_id)
}

pub(crate) fn ap_after_smp_callin(logical_id: usize) -> ! {
    if super::ap_smp_callin::state_for(logical_id) != State::Online {
        ap_phase_fail_stop(
            "ApSmpCallinPhase",
            logical_id,
            super::ap_smp_callin::state_for(logical_id),
            "parent-continuation",
        );
    }
    super::ap_online_idle::preset(logical_id)
}

pub(crate) fn ap_after_online_idle(logical_id: usize) -> ! {
    if super::ap_online_idle::state_for(logical_id) != State::Online {
        ap_phase_fail_stop(
            "ApOnlineIdlePhase",
            logical_id,
            super::ap_online_idle::state_for(logical_id),
            "parent-continuation",
        );
    }
    super::ap_online_idle::mark_park_loop_entered(logical_id);

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
        core::hint::spin_loop();
    }
}

pub(crate) fn ap_prerequisites_ready() -> bool {
    AP_PREREQUISITES_READY.load(Ordering::Acquire)
}

pub(super) fn reset_ap_state(states: &[AtomicU8; MAX_CPUS], logical_id: usize) {
    if logical_id < MAX_CPUS {
        states[logical_id].store(crate::phases::state::encode(State::Base), Ordering::Release);
    }
}

pub(super) fn ap_state_for(states: &[AtomicU8; MAX_CPUS], logical_id: usize) -> State {
    if logical_id >= MAX_CPUS {
        return State::Destroyed;
    }
    crate::phases::state::decode(states[logical_id].load(Ordering::Acquire))
}

pub(super) fn ap_family_all_online(states: &[AtomicU8; MAX_CPUS], cpu_group: &CpuGroup) -> bool {
    let secondary_count = cpu_group.secondary_count();
    if secondary_count == 0 || secondary_count >= MAX_CPUS {
        return false;
    }

    let mut logical_id = 1usize;
    while logical_id <= secondary_count {
        if ap_state_for(states, logical_id) != State::Online {
            return false;
        }
        logical_id += 1;
    }
    true
}

pub(super) fn transition_ap_state(
    states: &[AtomicU8; MAX_CPUS],
    logical_id: usize,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
    phase: &'static str,
) {
    if logical_id >= MAX_CPUS
        || states[logical_id]
            .compare_exchange(
                crate::phases::state::encode(expected),
                crate::phases::state::encode(target),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        || ap_state_for(states, logical_id) != target
    {
        ap_phase_fail_stop(
            phase,
            logical_id,
            ap_state_for(states, logical_id),
            "transition",
        );
    }
    crate::checkpoint::ap_checkpoint(checkpoint, logical_id);
}

pub(crate) fn ap_phase_fail_stop(
    phase: &'static str,
    logical_id: usize,
    state: State,
    check: &'static str,
) -> ! {
    crate::arch::riscv64::sbi::putstr("ap phase failure phase=");
    crate::arch::riscv64::sbi::putstr(phase);
    crate::arch::riscv64::sbi::putstr(" logical_id=");
    put_usize_decimal(logical_id);
    crate::arch::riscv64::sbi::putstr(" state=");
    crate::arch::riscv64::sbi::putchar(state.code());
    crate::arch::riscv64::sbi::putstr(" check=");
    crate::arch::riscv64::sbi::putstr(check);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn reset_ap_phase_families(cpu_group: &CpuGroup) {
    AP_PREREQUISITES_READY.store(false, Ordering::Release);
    let mut logical_id = 1usize;
    while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
        super::ap_entry_prelude::reset_secondary(logical_id);
        super::ap_smp_callin::reset_secondary(logical_id);
        super::ap_online_idle::reset_secondary(logical_id);
        logical_id += 1;
    }
}

fn publish_ap_prerequisites(ctx: &Context) -> EventResult {
    let ready = ctx.cpu_group.state() == State::Ready
        && ctx.cpu_group.secondary_cpus_present_not_online()
        && ctx.secondary_idle_tasks.state() == State::Prepared
        && ctx.secondary_idle_tasks.per_secondary_idle_task()
        && ctx.secondary_idle_tasks.dedicated_stack()
        && ctx.cpu_hotplug_sync.state() == State::Prepared
        && ctx.cpu_hotplug_sync.cpu_running_ready()
        && ctx.cpu_hotplug_sync.done_up_ready()
        && ctx.sbi.state() == State::Ready
        && ctx.sbi.hsm_available()
        && ctx.sbi_ipi.state() == State::Ready
        && ctx.init_mm.state() == State::Ready
        && ctx.vm.state() == State::Online
        && ctx.kernel_addr_space.state() == State::Online
        && ctx.vm.swapper_vm().state() == State::Ready
        && ctx.boot_cpu_trap().state() == State::Ready
        && ctx.boot_cpu_exception().state() == State::Ready;
    if !ready {
        return failed_condition(
            LifecycleEvent::Preset,
            crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE),
            State::Base,
            State::Prepared,
        );
    }
    AP_PREREQUISITES_READY.store(true, Ordering::Release);
    Ok(())
}

fn wait_for_ap_phase_families(cpu_group: &CpuGroup) -> bool {
    let mut spin = 0usize;
    while spin < AP_WAIT_SPINS {
        if super::ap_entry_prelude::all_online(cpu_group)
            && super::ap_smp_callin::all_online(cpu_group)
            && super::ap_online_idle::all_online(cpu_group)
            && super::ap_online_idle::all_park_loops_entered(cpu_group)
        {
            return true;
        }
        core::hint::spin_loop();
        spin += 1;
    }

    diagnose_first_incomplete_ap_phase(cpu_group);
    false
}

fn diagnose_first_incomplete_ap_phase(cpu_group: &CpuGroup) {
    let mut logical_id = 1usize;
    while logical_id <= cpu_group.secondary_count() && logical_id < MAX_CPUS {
        let entry_state = super::ap_entry_prelude::state_for(logical_id);
        if entry_state != State::Online {
            print_ap_wait_timeout("ApEntryPreludePhase", logical_id, entry_state);
            return;
        }
        let callin_state = super::ap_smp_callin::state_for(logical_id);
        if callin_state != State::Online {
            print_ap_wait_timeout("ApSmpCallinPhase", logical_id, callin_state);
            return;
        }
        let online_idle_state = super::ap_online_idle::state_for(logical_id);
        if online_idle_state != State::Online
            || !super::ap_online_idle::park_loop_entered_for(logical_id)
        {
            print_ap_wait_timeout("ApOnlineIdlePhase", logical_id, online_idle_state);
            return;
        }
        logical_id += 1;
    }
    print_ap_wait_timeout("ApEntryPreludePhase", logical_id, State::Destroyed);
}

fn print_ap_wait_timeout(phase: &'static str, logical_id: usize, state: State) {
    crate::arch::riscv64::sbi::putstr("ap phase timeout phase=");
    crate::arch::riscv64::sbi::putstr(phase);
    crate::arch::riscv64::sbi::putstr(" logical_id=");
    put_usize_decimal(logical_id);
    crate::arch::riscv64::sbi::putstr(" state=");
    crate::arch::riscv64::sbi::putchar(state.code());
    crate::arch::riscv64::sbi::putchar(b'\n');
}

fn put_usize_decimal(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut index = digits.len();
    if value == 0 {
        crate::arch::riscv64::sbi::putchar(b'0');
        return;
    }
    while value != 0 && index != 0 {
        index -= 1;
        digits[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    crate::arch::riscv64::sbi::putstr(core::str::from_utf8(&digits[index..]).unwrap_or("?"));
}
