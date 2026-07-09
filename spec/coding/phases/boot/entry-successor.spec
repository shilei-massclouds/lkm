/*
 * EntrySuccessorPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in entry-successor.md.
 */

predicate arceos_ex_must_entry_successor_keep_start_kernel_deferred_facts() -> bool;
predicate arceos_ex_must_entry_successor_memblock_record_riscv_setup_bootmem_facts() -> bool;
predicate arceos_ex_must_entry_successor_keep_swapper_rwx_boundary_deferred() -> bool;

type ArceosExEntrySuccessorCodingMust {
    invariant {
        /* start_kernel() deferred calls. */
        arceos_ex_must_entry_successor_keep_start_kernel_deferred_facts();

        /* RISC-V setup_bootmem() facts. */
        arceos_ex_must_entry_successor_memblock_record_riscv_setup_bootmem_facts();

        /* SwapperVm permission boundary. */
        arceos_ex_must_entry_successor_keep_swapper_rwx_boundary_deferred();
    }
}
