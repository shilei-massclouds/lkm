use core::{
    arch::global_asm,
    sync::atomic::{AtomicUsize, Ordering},
};

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
const STAGE_FAULT_ARMED: usize = 2;
const STAGE_FAULT_RETURNED: usize = 3;
const STAGE_YIELDED: usize = 4;
const STAGE_EXITING: usize = 5;
const STAGE_FAILURE: usize = usize::MAX;
const FIXUP_RESULT: usize = 0x5a17_f17e;

static WORKER_A_STAGE: AtomicUsize = AtomicUsize::new(STAGE_NONE);
static WORKER_B_STAGE: AtomicUsize = AtomicUsize::new(STAGE_NONE);
static WORKER_A_CPU: AtomicUsize = AtomicUsize::new(usize::MAX);
static WORKER_B_CPU: AtomicUsize = AtomicUsize::new(usize::MAX);
static WORKER_A_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);
static WORKER_B_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);
static WORKER_A_RESUME_COUNT: AtomicUsize = AtomicUsize::new(0);
static WORKER_B_RESUME_COUNT: AtomicUsize = AtomicUsize::new(0);

global_asm!(
    r#"
    .section .text.smp_ap_controlled_fault, "ax"
    .align 2
    .globl smp_ap_controlled_fault
smp_ap_controlled_fault:
1:
    ld      a0, 0(zero)
2:
    li      a0, {fixup_result}
    ret

    .pushsection __ex_table, "a"
    .balign 4
    .long   1b - .
    .long   2b - .
    .short  0
    .short  0
    .popsection
"#,
    fixup_result = const FIXUP_RESULT,
);

unsafe extern "C" {
    fn smp_ap_controlled_fault() -> usize;
}

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

    let mailbox_before = kernel_task::counters(TARGET_CPU).unwrap_or((0, 0, 0));
    let switches_initial = kernel_task::switch_counters(TARGET_CPU).unwrap_or((0, 0, 0));
    let Some(traps_before) = crate::objects::trap_type::observation(TARGET_CPU) else {
        return fail("CPU1 trap observation is unavailable");
    };

    // A mailbox-free SSIP must still complete a formal InterruptFlow overlay
    // while leaving mailbox ownership to the idle loop.
    if crate::arch::riscv64::sbi::send_ipi(target_hartid).is_err()
        || !wait_until(|| {
            kernel_task::counters(TARGET_CPU).is_some_and(|(_, received, consumed)| {
                received > mailbox_before.1 && consumed == mailbox_before.2
            }) && kernel_task::switch_counters(TARGET_CPU)
                .is_some_and(|(_, identities, _)| identities > switches_initial.1)
        })
    {
        return fail("mailbox-free SSIP overlay did not complete");
    }
    let switches_before = kernel_task::switch_counters(TARGET_CPU).unwrap_or((0, 0, 0));

    let worker_a = match context().create_kernel_task(target_cpu, ap_worker_a_entry) {
        Ok(task_ref) => task_ref,
        Err(_) => return fail("worker A creation failed"),
    };
    let worker_b = match context().create_kernel_task(target_cpu, ap_worker_b_entry) {
        Ok(task_ref) => task_ref,
        Err(_) => return fail("worker B creation failed"),
    };
    if worker_a.same_identity(worker_b)
        || context().task_creation_core.kernel_task_created_count() != created_before + 2
    {
        return fail("two dynamic kernel Task identities were not created");
    }

    if kernel_task::publish(
        worker_a,
        CpuRef::new(TARGET_CPU + 1),
        InboundKind::Activate,
        1,
    )
    .is_ok()
        || kernel_task::publish(
            worker_a.with_generation_for_test(worker_a.generation() + 1),
            target_cpu,
            InboundKind::Activate,
            1,
        )
        .is_ok()
    {
        return fail("mailbox accepted wrong CPU or stale generation");
    }

    if kernel_task::publish_and_ipi(
        worker_a,
        target_cpu,
        target_hartid,
        InboundKind::Activate,
        1,
    )
    .is_err()
        || !wait_until(|| WORKER_A_STAGE.load(Ordering::Acquire) == STAGE_ENTERED)
    {
        return fail("worker A activation did not enter CPU1");
    }

    // The second publication wakes A through SSIP. The SSIP handler only
    // coalesces need_resched; A deliberately enters a recoverable load fault,
    // whose leaf consumes B and performs A -> B.
    if kernel_task::publish_and_ipi(
        worker_b,
        target_cpu,
        target_hartid,
        InboundKind::Activate,
        2,
    )
    .is_err()
        || !wait_until(|| {
            WORKER_A_STAGE.load(Ordering::Acquire) >= STAGE_FAULT_RETURNED
                && WORKER_B_STAGE.load(Ordering::Acquire) >= STAGE_YIELDED
        })
    {
        return fail("page-fault A -> B -> A round trip did not return");
    }

    if !wait_until(|| {
        WORKER_A_STAGE.load(Ordering::Acquire) == STAGE_EXITING
            && WORKER_B_STAGE.load(Ordering::Acquire) == STAGE_EXITING
            && kernel_task::switch_counters(TARGET_CPU).is_some_and(
                |(switches, _, idle_restores)| {
                    switches >= switches_before.0 + 5 && idle_restores > switches_before.2
                },
            )
    }) {
        return fail("worker exit chain did not restore CPU1 idle");
    }

    let Some(task_a) = kernel_task::task_by_ref(worker_a) else {
        return fail("completed worker A no longer resolves");
    };
    let Some(task_b) = kernel_task::task_by_ref(worker_b) else {
        return fail("completed worker B no longer resolves");
    };
    let mailbox_after = kernel_task::counters(TARGET_CPU).unwrap_or((0, 0, 0));
    let switches_after = kernel_task::switch_counters(TARGET_CPU).unwrap_or((0, 0, 0));
    let Some(traps_after) = crate::objects::trap_type::observation(TARGET_CPU) else {
        return fail("final CPU1 trap observation is unavailable");
    };
    let idle_woke = ap_online_idle::idle_runtime_observation(TARGET_CPU)
        .is_some_and(|(ready, entered, woken)| ready && entered != 0 && woken != 0);

    if WORKER_A_STAGE.load(Ordering::Acquire) == STAGE_FAILURE
        || WORKER_B_STAGE.load(Ordering::Acquire) == STAGE_FAILURE
        || WORKER_A_CPU.load(Ordering::Acquire) != TARGET_CPU
        || WORKER_B_CPU.load(Ordering::Acquire) != TARGET_CPU
        || WORKER_A_ENTRY_COUNT.load(Ordering::Acquire) != 1
        || WORKER_B_ENTRY_COUNT.load(Ordering::Acquire) != 1
        || WORKER_A_RESUME_COUNT.load(Ordering::Acquire) != 1
        || WORKER_B_RESUME_COUNT.load(Ordering::Acquire) != 1
        || !completed_task_matches(task_a, target_cpu)
        || !completed_task_matches(task_b, target_cpu)
        || mailbox_after.0 != mailbox_before.0 + 2
        || mailbox_after.1 != mailbox_before.1 + 3
        || mailbox_after.2 != mailbox_before.2 + 2
        || switches_after.0 != switches_before.0 + 5
        || switches_after.1 != switches_before.1
        || switches_after.2 != switches_before.2 + 1
        || traps_after.ssip_completed != traps_before.ssip_completed + 3
        || traps_after.interrupts_completed != traps_before.interrupts_completed + 3
        || traps_after.exceptions_completed != traps_before.exceptions_completed + 1
        || traps_after.roots_completed != traps_before.roots_completed + 4
        || traps_after.return_tokens_consumed != traps_before.return_tokens_consumed + 4
        || traps_after.leaf_switch_resumes != traps_before.leaf_switch_resumes + 1
        || traps_after.last_generation == traps_before.last_generation
        || !idle_woke
    {
        crate::objects::printk::write_fmt(format_args!(
            "smp_ap_scheduler observed stages={}/{} entries={}/{} resumes={}/{} task_a={}/{}/{}/{} task_b={}/{}/{}/{} mailbox={}/{}/{} switches={}/{}/{} traps={}/{}/{}/{}/{}/{} generation={}->{} idle={}\n",
            WORKER_A_STAGE.load(Ordering::Acquire),
            WORKER_B_STAGE.load(Ordering::Acquire),
            WORKER_A_ENTRY_COUNT.load(Ordering::Acquire),
            WORKER_B_ENTRY_COUNT.load(Ordering::Acquire),
            WORKER_A_RESUME_COUNT.load(Ordering::Acquire),
            WORKER_B_RESUME_COUNT.load(Ordering::Acquire),
            task_a.state() as usize,
            task_a.flow_state() as usize,
            task_a.running(),
            task_a.runqueue_published(),
            task_b.state() as usize,
            task_b.flow_state() as usize,
            task_b.running(),
            task_b.runqueue_published(),
            mailbox_after.0.saturating_sub(mailbox_before.0),
            mailbox_after.1.saturating_sub(mailbox_before.1),
            mailbox_after.2.saturating_sub(mailbox_before.2),
            switches_after.0.saturating_sub(switches_before.0),
            switches_after.1.saturating_sub(switches_before.1),
            switches_after.2.saturating_sub(switches_before.2),
            traps_after
                .roots_completed
                .saturating_sub(traps_before.roots_completed),
            traps_after
                .interrupts_completed
                .saturating_sub(traps_before.interrupts_completed),
            traps_after
                .exceptions_completed
                .saturating_sub(traps_before.exceptions_completed),
            traps_after
                .ssip_completed
                .saturating_sub(traps_before.ssip_completed),
            traps_after
                .return_tokens_consumed
                .saturating_sub(traps_before.return_tokens_consumed),
            traps_after
                .leaf_switch_resumes
                .saturating_sub(traps_before.leaf_switch_resumes),
            traps_before.last_generation,
            traps_after.last_generation,
            idle_woke,
        ));
        return fail("final Task/mailbox/switch/trap facts are inconsistent");
    }

    crate::objects::printk::write_fmt(format_args!(
        "smp_ap_scheduler cpu={} ssip={} roots={} tokens={} leaf_resume={} switches={} mailbox={} fixup={:#x}\n",
        TARGET_CPU,
        traps_after.ssip_completed - traps_before.ssip_completed,
        traps_after.roots_completed - traps_before.roots_completed,
        traps_after.return_tokens_consumed - traps_before.return_tokens_consumed,
        traps_after.leaf_switch_resumes - traps_before.leaf_switch_resumes,
        switches_after.0 - switches_before.0,
        mailbox_after.2 - mailbox_before.2,
        FIXUP_RESULT,
    ));
    SmokeResult::Passed
}

extern "C" fn ap_worker_a_entry() -> ! {
    let (task_ref, logical_id) = current_kernel_task_or_fail(&WORKER_A_STAGE);
    WORKER_A_CPU.store(logical_id, Ordering::Release);
    WORKER_A_ENTRY_COUNT.fetch_add(1, Ordering::AcqRel);
    WORKER_A_STAGE.store(STAGE_ENTERED, Ordering::Release);

    while !(kernel_task::has_inbound(logical_id) && kernel_task::need_resched_pending(logical_id)) {
        core::hint::spin_loop();
    }
    WORKER_A_STAGE.store(STAGE_FAULT_ARMED, Ordering::Release);
    let fixup_result = unsafe { smp_ap_controlled_fault() };
    if fixup_result != FIXUP_RESULT {
        worker_fail(&WORKER_A_STAGE);
    }
    WORKER_A_RESUME_COUNT.fetch_add(1, Ordering::AcqRel);
    WORKER_A_STAGE.store(STAGE_FAULT_RETURNED, Ordering::Release);

    exit_current_worker(logical_id, task_ref, &WORKER_A_STAGE)
}

extern "C" fn ap_worker_b_entry() -> ! {
    let (task_ref, logical_id) = current_kernel_task_or_fail(&WORKER_B_STAGE);
    WORKER_B_CPU.store(logical_id, Ordering::Release);
    WORKER_B_ENTRY_COUNT.fetch_add(1, Ordering::AcqRel);
    WORKER_B_STAGE.store(STAGE_ENTERED, Ordering::Release);

    // A remains runnable on CPU1, so this ordinary cooperative yield performs
    // B -> A and resumes the saved page-fault leaf rather than re-entering it.
    if crate::context::schedule_secondary_current(logical_id, task_ref).is_err() {
        worker_fail(&WORKER_B_STAGE);
    }
    WORKER_B_RESUME_COUNT.fetch_add(1, Ordering::AcqRel);
    WORKER_B_STAGE.store(STAGE_YIELDED, Ordering::Release);

    exit_current_worker(logical_id, task_ref, &WORKER_B_STAGE)
}

fn current_kernel_task_or_fail(stage: &AtomicUsize) -> (crate::objects::task::TaskRef, usize) {
    let identity = crate::arch::riscv64::csr::read_tp();
    let Some(candidate) = kernel_task::task_candidate_by_identity(identity) else {
        worker_fail(stage);
    };
    (candidate.task.task_ref(), candidate.flow.cpu_id())
}

fn exit_current_worker(
    logical_id: usize,
    task_ref: crate::objects::task::TaskRef,
    stage: &AtomicUsize,
) -> ! {
    if crate::context::disable_secondary_task_flow_for_exit(logical_id, task_ref).is_err()
        || crate::context::declare_secondary_task_sleep(logical_id, task_ref).is_err()
    {
        worker_fail(stage);
    }
    stage.store(STAGE_EXITING, Ordering::Release);
    if crate::context::schedule_secondary_current(logical_id, task_ref).is_err() {
        worker_fail(stage);
    }
    worker_fail(stage)
}

fn completed_task_matches(task: &crate::objects::task::Task, cpu_ref: CpuRef) -> bool {
    task.state() == State::Online
        && task.flow_state() == State::Offline
        && task.flow_cpu_ref() == Some(cpu_ref)
        && !task.running()
        && !task.runqueue_published()
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

fn worker_fail(stage: &AtomicUsize) -> ! {
    stage.store(STAGE_FAILURE, Ordering::Release);
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
