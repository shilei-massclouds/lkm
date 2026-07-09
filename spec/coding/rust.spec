/*
 * Rust coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in rust.md.
 */

predicate rust_should_use_result_propagation_for_transition_chains() -> bool;
predicate rust_should_centralize_transition_boundary_error_handling() -> bool;
predicate rust_should_parameterize_global_asm_operands() -> bool;
predicate rust_should_use_explicit_abi_for_low_level_boundaries() -> bool;
predicate rust_should_not_use_extern_c_for_pure_rust_internal_functions() -> bool;
predicate rust_may_use_naked_functions_for_tiny_asm_boundaries() -> bool;

type RustCodingShould {
    invariant {
        /* Transition-chain error propagation. */
        rust_should_use_result_propagation_for_transition_chains();

        /* Boundary error handling. */
        rust_should_centralize_transition_boundary_error_handling();

        /* Parameterized global assembly. */
        rust_should_parameterize_global_asm_operands();

        /* Explicit ABI at low-level boundaries. */
        rust_should_use_explicit_abi_for_low_level_boundaries();

        /* Avoid unnecessary extern "C". */
        rust_should_not_use_extern_c_for_pure_rust_internal_functions();
    }
}

type RustCodingMay {
    invariant {
        /* Tiny assembly boundaries. */
        rust_may_use_naked_functions_for_tiny_asm_boundaries();
    }
}
