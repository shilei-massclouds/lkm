use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        cpu::CpuRef,
        kernel_task::{self, InboundKind},
        state::State,
    },
    phases::smp_runtime::ap_online_idle,
};

const TARGET_CPU: usize = 1;
const WAIT_ATTEMPTS: usize = 50_000_000;
const STAGE_NONE: usize = 0;
const STAGE_ENTERED: usize = 1;
const STAGE_IDENTITY_YIELDED: usize = 2;
const STAGE_BLOCKING: usize = 3;
const STAGE_RESUMED: usize = 4;
const STAGE_EXITING: usize = 5;
const STAGE_FAILURE: usize = usize::MAX;

static WORKER_STAGE: AtomicUsize = AtomicUsize::new(STAGE_NONE);
static WORKER_CPU: AtomicUsize = AtomicUsize::new(usize::MAX);
static WORKER_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);
static WORKER_RESUME_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn run() -> SmokeResult {
    let (target_cpu, target_hartid, created_before) = {
        let ctx = context();
        let Some(cpu) = ctx.cpu_group.cpu(TARGET_CPU) else {
            return SmokeResult::Failed;
        };
        if !cpu.is_online() {
            return SmokeResult::Failed;
        }
        (
            cpu.cpu_ref(),
            cpu.hartid(),
            ctx.task_creation_core.kernel_task_created_count(),
        )
    };

    if !wait_until(|| {
        ap_online_idle::idle_runtime_observation(TARGET_CPU)
            .is_some_and(|(ready, entered, _)| ready && entered != 0)
    }) {
        return fail("CPU1 did not enter interruptible idle wfi");
    }

    let before = kernel_task::counters(TARGET_CPU).unwrap_or((0, 0, 0));
    // A reschedule IPI without a published mailbox item is a safe coalesced
    // identity request, never a phantom mailbox consumption.
    if crate::arch::riscv64::sbi::send_ipi(target_hartid).is_err()
        || !wait_until(|| {
            kernel_task::counters(TARGET_CPU)
                .is_some_and(|(_, received, consumed)| received > before.1 && consumed == before.2)
        })
    {
        return fail("IPI-before-mailbox was not safely coalesced");
    }

    let task_ref = match context().create_kernel_task(target_cpu, ap_worker_entry) {
        Ok(task_ref) => task_ref,
        Err(_) => return fail("dynamic kernel Task creation failed"),
    };
    if context().task_creation_core.kernel_task_created_count() != created_before + 1 {
        return fail("TaskCreationCore did not account for dynamic kernel Task");
    }
    if kernel_task::publish(
        task_ref,
        CpuRef::new(TARGET_CPU + 1),
        InboundKind::Activate,
        1,
    )
    .is_ok()
        || kernel_task::publish(
            task_ref.with_generation_for_test(task_ref.generation() + 1),
            target_cpu,
            InboundKind::Activate,
            1,
        )
        .is_ok()
    {
        return fail("mailbox accepted wrong target or stale generation");
    }

    if kernel_task::publish_and_ipi(
        task_ref,
        target_cpu,
        target_hartid,
        InboundKind::Activate,
        1,
    )
    .is_err()
    {
        return fail("remote activation publication failed");
    }
    if !wait_until(|| WORKER_STAGE.load(Ordering::Acquire) >= STAGE_BLOCKING) {
        return fail("CPU1 worker did not reach blocking boundary");
    }
    if WORKER_STAGE.load(Ordering::Acquire) == STAGE_FAILURE
        || WORKER_CPU.load(Ordering::Acquire) != TARGET_CPU
        || WORKER_ENTRY_COUNT.load(Ordering::Acquire) != 1
        || kernel_task::publish(task_ref, target_cpu, InboundKind::Wake, 1).is_ok()
    {
        return fail("worker identity/yield or duplicate ordinal contract failed");
    }

    if kernel_task::publish_and_ipi(task_ref, target_cpu, target_hartid, InboundKind::Wake, 2)
        .is_err()
    {
        return fail("remote wake publication failed");
    }
    if !wait_until(|| {
        WORKER_STAGE.load(Ordering::Acquire) == STAGE_EXITING
            && kernel_task::switch_counters(TARGET_CPU)
                .is_some_and(|(switches, _, idle_restores)| switches >= 4 && idle_restores >= 2)
    }) {
        return fail("wake/resume/exit/idle switch chain did not close");
    }

    let Some(task) = kernel_task::task_by_ref(task_ref) else {
        return fail("completed TaskRef no longer resolves");
    };
    let (sent, received, consumed) = kernel_task::counters(TARGET_CPU).unwrap_or((0, 0, 0));
    let (switches, identities, idle_restores) =
        kernel_task::switch_counters(TARGET_CPU).unwrap_or((0, 0, 0));
    let idle_woke = ap_online_idle::idle_runtime_observation(TARGET_CPU)
        .is_some_and(|(ready, entered, woken)| ready && entered != 0 && woken != 0);
    if WORKER_RESUME_COUNT.load(Ordering::Acquire) != 1
        || task.state() != State::Online
        || task.flow_state() != State::Offline
        || task.flow_cpu_ref() != Some(target_cpu)
        || task.running()
        || task.runqueue_published()
        || sent != 2
        || received < 3
        || consumed != 2
        || switches < 4
        || identities == 0
        || idle_restores < 2
        || !idle_woke
    {
        return fail("final Task/runqueue/IPI/context facts are inconsistent");
    }

    crate::objects::printk::write_fmt(format_args!(
        "smp_ap_scheduler cpu={} ipi={}/{} mailbox={} switches={} identity={} idle_restores={}\n",
        TARGET_CPU, sent, received, consumed, switches, identities, idle_restores,
    ));
    SmokeResult::Passed
}

extern "C" fn ap_worker_entry() -> ! {
    let identity = crate::arch::riscv64::csr::read_tp();
    let Some(candidate) = kernel_task::task_candidate_by_identity(identity) else {
        worker_fail();
    };
    let task_ref = candidate.task.task_ref();
    let logical_id = candidate.flow.cpu_id();
    WORKER_CPU.store(logical_id, Ordering::Release);
    WORKER_ENTRY_COUNT.fetch_add(1, Ordering::AcqRel);
    WORKER_STAGE.store(STAGE_ENTERED, Ordering::Release);

    // With no other fair task this cooperative yield is intentionally an
    // identity selection: no Save/Suspend/Dispatch/Enter is fabricated.
    if crate::context::schedule_secondary_current(logical_id, task_ref).is_err() {
        worker_fail();
    }
    WORKER_STAGE.store(STAGE_IDENTITY_YIELDED, Ordering::Release);

    if crate::context::declare_secondary_task_sleep(logical_id, task_ref).is_err() {
        worker_fail();
    }
    WORKER_STAGE.store(STAGE_BLOCKING, Ordering::Release);
    if crate::context::schedule_secondary_current(logical_id, task_ref).is_err() {
        worker_fail();
    }

    WORKER_RESUME_COUNT.fetch_add(1, Ordering::AcqRel);
    WORKER_STAGE.store(STAGE_RESUMED, Ordering::Release);
    if crate::context::disable_secondary_task_flow_for_exit(logical_id, task_ref).is_err()
        || crate::context::declare_secondary_task_sleep(logical_id, task_ref).is_err()
    {
        worker_fail();
    }
    WORKER_STAGE.store(STAGE_EXITING, Ordering::Release);
    if crate::context::schedule_secondary_current(logical_id, task_ref).is_err() {
        worker_fail();
    }
    worker_fail()
}

fn wait_until(mut predicate: impl FnMut() -> bool) -> bool {
    let mut remaining = WAIT_ATTEMPTS;
    while remaining != 0 {
        if predicate() {
            return true;
        }
        core::hint::spin_loop();
        remaining -= 1;
    }
    false
}

fn worker_fail() -> ! {
    WORKER_STAGE.store(STAGE_FAILURE, Ordering::Release);
    loop {
        core::hint::spin_loop();
    }
}

fn fail(message: &'static str) -> SmokeResult {
    crate::objects::printk::write_str("smp_ap_scheduler: ");
    crate::objects::printk::write_str(message);
    crate::objects::printk::write_str("\n");
    SmokeResult::Failed
}
