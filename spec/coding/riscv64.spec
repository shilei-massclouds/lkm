/*
 * RISC-V64 Coding Formal Specification
 *
 * This file contains architecture-specific mandatory rules for object-level
 * RISC-V64 kernel code generation. Human-facing explanation stays in
 * riscv64.md.
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
        /*
         * No fixed physical kernel load address:
         *
         * RISC-V64 linker script generation must not define, require or
         * consume a fixed kernel physical load address such as
         * KERNEL_PHYS_ADDR. The physical load address is not a Config fact.
         */
        riscv64_must_linker_script_ignore_fixed_kernel_phys_addr();

        /*
         * Link address source:
         *
         * The kernel link virtual address used by the linker script must come
         * from Config.kernel_link_addr. The linker script may also depend on
         * model Lds facts and other Config facts such as page size, PMD size,
         * boot-stack size, section alignment and head-text layout.
         */
        riscv64_must_linker_script_use_config_kernel_link_addr();

        /*
         * Runtime physical start:
         *
         * The kernel physical image start is a runtime fact established by
         * KernelImage.Preset from the actual entry/image position observed in
         * the pre-MMU stage. It must be represented as KernelImage.phys_start
         * or an equivalent model-backed resource fact.
         */
        riscv64_must_kernel_phys_start_from_kernel_image();

        /*
         * Address translation source:
         *
         * Runtime physical/link/virtual address conversion must derive its
         * offset from KernelImage.phys_start and Config.kernel_link_addr. It
         * must not use a fixed Config.kernel_phys_addr-style constant.
         */
        riscv64_must_address_translation_use_kernel_image_offset();
    }
}

type Riscv64AddressTranslationMust {
    invariant {
        /*
         * Trampoline page table handoff:
         *
         * Code generated for TrampolineVm.Enable must flush or otherwise
         * invalidate the local address-translation cache after building the
         * trampoline page table and before writing the trampoline SATP value.
         * This maps Linux/RISC-V relocate_enable_mmu()'s sfence.vma before
         * loading trampoline_pg_dir into SATP.
         *
         * The minimum acceptable implementation order is:
         * 1. compute or load the trampoline SATP value,
         * 2. execute sfence.vma or an equivalent local TLB flush,
         * 3. only then write SATP to the trampoline value,
         * 4. only then commit the implementation fact corresponding to
         *    trampoline_vm_translation_sync_ready_before_satp(TrampolineVm).
         */
        riscv64_must_trampoline_vm_enable_flush_tlb_before_satp();
        riscv64_must_trampoline_vm_enable_commit_sync_fact_after_tlb_flush();

        /*
         * Early page table handoff:
         *
         * Code generated for EarlyVm.Enable must flush or otherwise
         * invalidate the local address-translation cache after writing the
         * early SATP value. This maps Linux/RISC-V relocate_enable_mmu()'s
         * sfence.vma after switching from trampoline_pg_dir to early_pg_dir.
         *
         * The minimum acceptable implementation order is:
         * 1. compute or load the early SATP value,
         * 2. write SATP,
         * 3. execute sfence.vma or an equivalent local TLB flush,
         * 4. only then commit the implementation fact corresponding to
         *    early_vm_translation_sync_complete(EarlyVm).
         */
        riscv64_must_early_vm_enable_flush_tlb_after_satp();
        riscv64_must_early_vm_enable_commit_sync_fact_after_tlb_flush();

        /*
         * Swapper page table handoff:
         *
         * Code generated for SwapperVm.Enable must flush or otherwise
         * invalidate the local address-translation cache after writing the
         * swapper SATP value, and must expose that completion as the model
         * fact swapper_vm_translation_sync_complete(SwapperVm). This maps
         * Linux/RISC-V setup_vm_final()'s local_flush_tlb_all() boundary.
         *
         * The minimum acceptable implementation order is:
         * 1. compute or load the swapper SATP value,
         * 2. write SATP,
         * 3. execute sfence.vma or an equivalent local TLB flush,
         * 4. only then commit the implementation fact corresponding to
         *    swapper_vm_translation_sync_complete(SwapperVm).
         */
        riscv64_must_swapper_vm_enable_flush_tlb_after_satp();
        riscv64_must_swapper_vm_enable_commit_sync_fact_after_tlb_flush();
    }
}

type Riscv64SchedulerCodingShould {
    invariant {
        /*
         * Current task reference:
         *
         * RISC-V64 code should realize the model's CPU-local CurrentTaskRef by
         * following the Linux-style use of the tp register as the current-task
         * view. The object-level CurrentTaskSlot is the implementation
         * boundary; on RISC-V64 its backend should use tp as the carrier or
         * fast entry for the current task pointer. This follows Linux
         * 6.12.37's arch/riscv/kernel/entry.S::__switch_to, which moves next
         * task_struct from a1 into tp. This is an implementation reference for
         * this target; the model semantics remain CPU-view based and do not
         * require per-cpu storage as the CurrentTaskRef abstraction.
         */
        riscv64_should_current_task_ref_follow_linux_tp();
    }
}
