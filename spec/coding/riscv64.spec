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

type Riscv64SchedulerCodingShould {
    invariant {
        /*
         * Current task reference:
         *
         * RISC-V64 code should realize the model's CPU-local CurrentTaskRef by
         * following the Linux-style use of the tp register as the current-task
         * view. This is an implementation reference for this target; the model
         * semantics remain CPU-view based and do not require per-cpu storage as
         * the CurrentTaskRef abstraction.
         */
        riscv64_should_current_task_ref_follow_linux_tp();
    }
}
