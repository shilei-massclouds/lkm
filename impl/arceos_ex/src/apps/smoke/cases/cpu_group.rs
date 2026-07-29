use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::csr,
    context::context,
    objects::{
        cpu::{
            BOOT_CPU_LOGICAL_ID, CpuRole, TranslationOwner, TranslationState,
            TranslationTakeoverTrace,
        },
        printk,
        state::State,
        trampoline_vm::trampoline_window_contains_range,
        vm::{ap_translation_chain, boot_translation_chain},
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let cpu_group = &ctx.cpu_group;

    if cpu_group.state() != State::Ready {
        printk::write_str("CpuGroup is not ready\n");
        return SmokeResult::Failed;
    }
    if !cpu_group.possible_cpu_boundary_ready() {
        printk::write_str("CpuGroup possible CPU boundary is invalid\n");
        return SmokeResult::Failed;
    }

    let count = cpu_group.possible_cpu_count();
    if count == 0 {
        printk::write_str("CpuGroup has no possible CPUs\n");
        return SmokeResult::Failed;
    }

    let Some(boot_cpu) = cpu_group.boot_cpu() else {
        printk::write_str("CpuGroup boot CPU is missing\n");
        return SmokeResult::Failed;
    };
    let Some(boot_cpu_ref) = cpu_group.boot_cpu_ref() else {
        printk::write_str("CpuGroup boot CPU ref is missing\n");
        return SmokeResult::Failed;
    };
    if boot_cpu_ref != boot_cpu.cpu_ref()
        || boot_cpu.logical_id() != BOOT_CPU_LOGICAL_ID
        || !boot_cpu_ref.is_boot_cpu()
        || boot_cpu.role() != CpuRole::Boot
        || boot_cpu.state() != State::Online
        || !boot_cpu.is_possible()
        || !boot_cpu.is_present()
        || !boot_cpu.is_online()
        || !cpu_group.possible_contains(boot_cpu_ref)
        || !cpu_group.present_contains(boot_cpu_ref)
        || !cpu_group.online_contains(boot_cpu_ref)
    {
        printk::write_str("CpuGroup boot CPU facts are invalid\n");
        return SmokeResult::Failed;
    }

    let mut logical_id = 0usize;
    while logical_id < count {
        let Some(cpu) = cpu_group.cpu(logical_id) else {
            printk::write_str("CpuGroup logical CPU is missing\n");
            return SmokeResult::Failed;
        };
        let Some(cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
            printk::write_str("CpuGroup possible CPU ref is missing\n");
            return SmokeResult::Failed;
        };

        if cpu.cpu_ref() != cpu_ref
            || cpu_ref.logical_id() != logical_id
            || cpu.logical_id() != logical_id
            || !cpu.is_possible()
            || cpu_group.present_contains(cpu_ref) != cpu.is_present()
            || cpu_group.online_contains(cpu_ref) != cpu.is_online()
        {
            printk::write_str("CpuGroup logical CPU view is invalid\n");
            return SmokeResult::Failed;
        }
        if logical_id == BOOT_CPU_LOGICAL_ID {
            if cpu.role() != CpuRole::Boot
                || !cpu.is_online()
                || !ctx.vm.entry_successor_ready_for(cpu)
            {
                printk::write_str("CpuGroup boot CPU slot is invalid\n");
                return SmokeResult::Failed;
            }
        } else if cpu.role() != CpuRole::Secondary || !ctx.vm.ap_translation_ready(cpu) {
            printk::write_str("CpuGroup secondary CPU translation is invalid\n");
            return SmokeResult::Failed;
        }

        let expected_chain = if logical_id == BOOT_CPU_LOGICAL_ID {
            let expected = boot_translation_chain(
                ctx.vm.trampoline_vm().satp(),
                ctx.vm.early_vm_satp(),
                ctx.vm.swapper_vm().satp(),
            );
            if csr::read_satp() != ctx.vm.swapper_vm().satp()
                || !cpu.translation_chain_matches(&expected)
            {
                printk::write_str("CpuGroup boot translation journal is invalid\n");
                return SmokeResult::Failed;
            }
            expected.len()
        } else {
            let expected =
                ap_translation_chain(ctx.vm.trampoline_vm().satp(), ctx.vm.swapper_vm().satp());
            if !cpu.translation_chain_matches(&expected) {
                printk::write_str("CpuGroup AP translation journal is invalid\n");
                return SmokeResult::Failed;
            }
            expected.len()
        };
        let journal = cpu.translation_takeover_journal();
        if cpu.active_translation_owner() != TranslationOwner::SwapperVm
            || journal.committed_count != expected_chain
            || journal.receipts[expected_chain - 1].new_owner != TranslationOwner::SwapperVm
            || journal.receipts[expected_chain - 1].satp != ctx.vm.swapper_vm().satp()
            || journal.receipts[expected_chain - 1].commit_sequence != expected_chain
            || !journal.receipts[expected_chain - 1].synchronization_complete
        {
            printk::write_str("CpuGroup per-CPU translation owner journal is invalid\n");
            return SmokeResult::Failed;
        }

        let mut previous = 0usize;
        while previous < logical_id {
            let Some(previous_cpu) = cpu_group.cpu(previous) else {
                printk::write_str("CpuGroup previous CPU view is missing\n");
                return SmokeResult::Failed;
            };
            if previous_cpu.hartid() == cpu.hartid() {
                printk::write_str("CpuGroup hartid is duplicated\n");
                return SmokeResult::Failed;
            }
            previous += 1;
        }

        if cpu_group
            .logical_id_for_hartid(cpu.hartid())
            .map(|id| id.get())
            != Some(logical_id)
        {
            printk::write_str("CpuGroup hartid reverse mapping is invalid\n");
            return SmokeResult::Failed;
        }

        logical_id += 1;
    }

    if cpu_group.cpu(count).is_some()
        || cpu_group.cpu_ref_at(count).is_some()
        || cpu_group.possible_cpu_ref_at(count).is_some()
    {
        printk::write_str("CpuGroup exposes CPU outside possible boundary\n");
        return SmokeResult::Failed;
    }
    if !translation_state_negative_cases() {
        printk::write_str("CpuGroup translation-state negative coverage failed\n");
        return SmokeResult::Failed;
    }
    printk::write_fmt(format_args!(
        "CpuGroup topology:\n  Possible CPUs : {}\n  Secondary CPUs: {}\n  Boot hartid   : {}\n",
        count,
        cpu_group.secondary_count(),
        boot_cpu.hartid()
    ));

    logical_id = 0;
    while logical_id < count {
        let Some(cpu) = cpu_group.cpu(logical_id) else {
            return SmokeResult::Failed;
        };
        printk::write_fmt(format_args!(
            "  CPU[{}] hart={} role={} online={}\n",
            logical_id,
            cpu.hartid(),
            role_name(cpu.role()),
            if cpu.is_online() { "yes" } else { "no" }
        ));
        logical_id += 1;
    }

    SmokeResult::Passed
}

fn translation_state_negative_cases() -> bool {
    let state = TranslationState::new();
    let unchanged = |state: &TranslationState, owner, count| {
        state.owner() == owner && state.committed_count() == count
    };

    if state.commit_take_over(
        TranslationOwner::PhysicalDirect,
        TranslationOwner::TrampolineVm,
        0x100,
        0x100,
    ) || !unchanged(&state, TranslationOwner::None, 0)
        || state.commit_take_over(
            TranslationOwner::None,
            TranslationOwner::PhysicalDirect,
            0,
            1,
        )
        || !unchanged(&state, TranslationOwner::None, 0)
        || state.commit_take_over(
            TranslationOwner::None,
            TranslationOwner::TrampolineVm,
            0x100,
            0x100,
        )
        || !unchanged(&state, TranslationOwner::None, 0)
        || !state.commit_take_over(
            TranslationOwner::None,
            TranslationOwner::PhysicalDirect,
            0,
            0,
        )
        || state.commit_take_over(
            TranslationOwner::None,
            TranslationOwner::PhysicalDirect,
            0,
            0,
        )
        || !unchanged(&state, TranslationOwner::PhysicalDirect, 1)
        || state.commit_take_over(
            TranslationOwner::PhysicalDirect,
            TranslationOwner::EarlyVm,
            0x200,
            0x200,
        )
        || !unchanged(&state, TranslationOwner::PhysicalDirect, 1)
        || state.commit_take_over(
            TranslationOwner::PhysicalDirect,
            TranslationOwner::TrampolineVm,
            0x200,
            0x201,
        )
        || !unchanged(&state, TranslationOwner::PhysicalDirect, 1)
    {
        return false;
    }

    let half_state = TranslationState::new();
    if !half_state.commit_take_over(
        TranslationOwner::None,
        TranslationOwner::PhysicalDirect,
        0,
        0,
    ) || !half_state.inject_unpublished_receipt_for_test(
        TranslationOwner::PhysicalDirect,
        TranslationOwner::TrampolineVm,
        0x200,
    ) || half_state.receipt(1).is_some()
        || half_state.committed_count() != 1
        || half_state.owner() != TranslationOwner::PhysicalDirect
        || half_state.matches_chain(&[
            TranslationTakeoverTrace::completed(
                TranslationOwner::None,
                TranslationOwner::PhysicalDirect,
                0,
                1,
            ),
            TranslationTakeoverTrace::completed(
                TranslationOwner::PhysicalDirect,
                TranslationOwner::TrampolineVm,
                0x200,
                2,
            ),
        ])
    {
        return false;
    }

    trampoline_window_contains_range(0x1000, 0x2000, 0x1f80, 0x80)
        && !trampoline_window_contains_range(0x1000, 0x2000, 0x1f80, 0x81)
        && !trampoline_window_contains_range(0x1000, 0x2000, usize::MAX - 3, 8)
        && !trampoline_window_contains_range(0x1000, 0x2000, 0x1800, 0)
}

fn role_name(role: CpuRole) -> &'static str {
    match role {
        CpuRole::Boot => "boot",
        CpuRole::Secondary => "secondary",
    }
}
