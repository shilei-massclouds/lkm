/*
 * Rust Coding Formal Specification
 *
 * This file contains Rust-language implementation guidance for object-level
 * kernel code. Human-facing explanation stays in rust.md; this formal file
 * gives AI agents and tools stable predicates to consume.
 */

predicate rust_should_use_result_propagation_for_event_chains() -> bool;
predicate rust_should_centralize_event_boundary_error_handling() -> bool;
predicate rust_should_parameterize_global_asm_operands() -> bool;
predicate rust_should_use_explicit_abi_for_low_level_boundaries() -> bool;
predicate rust_should_not_use_extern_c_for_pure_rust_internal_functions() -> bool;
predicate rust_may_use_naked_functions_for_tiny_asm_boundaries() -> bool;

type RustCodingShould {
    invariant {
        /*
         * Event-chain error propagation:
         *
         * Rust code that drives a sequence of model events should convert
         * EventResult-like values into Result-like outcomes and use the `?`
         * operator inside ordinary functions. The non-returning phase boundary
         * should handle the final error report and shutdown.
         */
        rust_should_use_result_propagation_for_event_chains();

        /*
         * Boundary error handling:
         *
         * Event-chain error reporting should be centralized at explicit phase
         * or startup boundaries. Interior event-driving functions should
         * propagate errors instead of open-coding repeated require-style
         * checks after each event.
         */
        rust_should_centralize_event_boundary_error_handling();

        /*
         * Parameterized global assembly:
         *
         * global_asm! blocks should use named const/sym operands for Rust-side
         * constants, Rust-defined statics and Rust-defined function symbols
         * when supported by the compiler and linker. This keeps assembly
         * dependencies visible to Rust and reduces string-only coupling.
         *
         * Linker-script provided symbols may remain as direct assembly symbol
         * references when sym operands would produce toolchain-renamed
         * undefined symbols.
         */
        rust_should_parameterize_global_asm_operands();

        /*
         * Explicit ABI at low-level boundaries:
         *
         * Functions that cross an assembly boundary, firmware ABI boundary,
         * naked-function boundary, manually stored function-pointer boundary,
         * function-pointer-to-integer round trip, or address-space switching
         * continuation should use an explicit stable ABI such as extern "C".
         * This includes functions reached indirectly after their address has
         * been converted, relocated, stored and transmuted back.
         */
        rust_should_use_explicit_abi_for_low_level_boundaries();

        /*
         * Avoid unnecessary extern "C":
         *
         * Pure Rust internal functions should not use extern "C" only for
         * stylistic uniformity. The explicit ABI marker should signal a real
         * low-level boundary or recorded exception.
         */
        rust_should_not_use_extern_c_for_pure_rust_internal_functions();
    }
}

type RustCodingMay {
    invariant {
        /*
         * Tiny assembly boundaries:
         *
         * Rust naked functions may be used for small, isolated assembly
         * boundaries such as a one-instruction trace hook or ABI trampoline
         * when the current toolchain supports them and the function body has
         * no Rust prologue/epilogue requirements.
         */
        rust_may_use_naked_functions_for_tiny_asm_boundaries();
    }
}
