/*
 * RISC-V64 coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in riscv64.md.
 */

predicate riscv64_must_linker_script_ignore_fixed_kernel_phys_addr() -> bool;
predicate riscv64_must_linker_script_use_config_kernel_link_addr() -> bool;
predicate riscv64_must_kernel_phys_start_from_kernel_image() -> bool;
predicate riscv64_must_address_translation_use_kernel_image_offset() -> bool;
predicate riscv64_must_swapper_vm_enable_flush_tlb_after_satp() -> bool;
predicate riscv64_must_swapper_vm_enable_commit_sync_fact_after_tlb_flush() -> bool;
predicate riscv64_must_trampoline_vm_enable_flush_tlb_before_satp() -> bool;
predicate riscv64_must_trampoline_vm_enable_commit_sync_fact_after_tlb_flush() -> bool;
predicate riscv64_must_early_vm_enable_flush_tlb_after_satp() -> bool;
predicate riscv64_must_early_vm_enable_commit_sync_fact_after_tlb_flush() -> bool;
predicate riscv64_should_current_task_ref_follow_linux_tp() -> bool;

type Riscv64LinkerScriptMust {
    invariant {
        /* No fixed physical kernel load address. */
        riscv64_must_linker_script_ignore_fixed_kernel_phys_addr();

        /* Link address source. */
        riscv64_must_linker_script_use_config_kernel_link_addr();

        /* Runtime physical start. */
        riscv64_must_kernel_phys_start_from_kernel_image();

        /* Address translation source. */
        riscv64_must_address_translation_use_kernel_image_offset();
    }
}

type Riscv64AddressTranslationMust {
    invariant {
        /* Trampoline page table handoff. */
        riscv64_must_trampoline_vm_enable_flush_tlb_before_satp();
        riscv64_must_trampoline_vm_enable_commit_sync_fact_after_tlb_flush();

        /* Early page table handoff. */
        riscv64_must_early_vm_enable_flush_tlb_after_satp();
        riscv64_must_early_vm_enable_commit_sync_fact_after_tlb_flush();

        /* Swapper page table handoff. */
        riscv64_must_swapper_vm_enable_flush_tlb_after_satp();
        riscv64_must_swapper_vm_enable_commit_sync_fact_after_tlb_flush();
    }
}

type Riscv64SchedulerCodingShould {
    invariant {
        /* Current task reference. */
        riscv64_should_current_task_ref_follow_linux_tp();
    }
}
