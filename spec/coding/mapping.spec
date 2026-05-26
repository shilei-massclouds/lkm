/*
 * Object Coding Mapping MUST Specification
 *
 * These predicates name mandatory constraints for generating object-level code
 * from spec/model. They are intentionally separate from mapping.md so tools and
 * AI agents can consume the hard rules through the formal *.spec entry.
 */

predicate coding_must_model_priority() -> bool;
predicate coding_must_map_before_coding() -> bool;
predicate coding_must_preserve_object_ownership() -> bool;
predicate coding_must_phase_path_matches_tree() -> bool;
predicate coding_must_phase_dfs_generation() -> bool;
predicate coding_must_phase_not_resource_lifecycle() -> bool;
predicate coding_must_keep_event_boundaries() -> bool;
predicate coding_must_check_before_checkpoint() -> bool;
predicate coding_must_record_exceptions() -> bool;
predicate coding_must_run_verification_gates() -> bool;

type CodingMappingMust {
    invariant {
        /*
         * Model priority:
         *
         * Implementation must strictly follow objects, states, events,
         * dependencies, drive order, phase boundaries and proof obligations
         * derived from spec/model. Implementation convenience, directory habit,
         * reused code, early Rust entry, reduced assembly or build-tool limits
         * are not valid reasons to diverge from model semantics.
         */
        coding_must_model_priority();

        /*
         * Map before coding:
         *
         * Code generation or modification must first identify the model object
         * to source-file mapping. Code may only be written to the mapped file or
         * its explicit submodules unless a recorded exception exists.
         */
        coding_must_map_before_coding();

        /*
         * Object ownership:
         *
         * The main implementation of a model object must live in that object's
         * mapped file. Phase setup/handoff/checkpoint code belongs to the Phase
         * file; resource-object state and event methods belong to the resource
         * object file.
         */
        coding_must_preserve_object_ownership();

        /*
         * Phase tree path:
         *
         * Phase file paths must reflect the model parent/child phase tree.
         */
        coding_must_phase_path_matches_tree();

        /*
         * DFS generation:
         *
         * Phase code must be generated from the phase tree using depth-first
         * traversal. Runtime control between sibling phases is connected by
         * handoff() -> next.setup(), not by flattening all child phase bodies
         * into the parent phase.
         */
        coding_must_phase_dfs_generation();

        /*
         * Phase shape:
         *
         * Phase objects must not be implemented as resource-object style
         * struct + impl lifecycle state machines. They are process modules with
         * setup(), handoff(), boundary checks and checkpoints.
         */
        coding_must_phase_not_resource_lifecycle();

        /*
         * Event boundaries:
         *
         * Each model event must keep a locatable code boundary. If low-level
         * code must complete adjacent events without a hardware-visible gap, the
         * mapped source must still preserve event functions, state adoption,
         * checks and checkpoint boundaries.
         */
        coding_must_keep_event_boundaries();

        /*
         * Checks before checkpoints:
         *
         * depends_on, ensures and invariant facts must be checked before the
         * target state is committed, a checkpoint is emitted or later code uses
         * the fact as an established dependency.
         */
        coding_must_check_before_checkpoint();

        /*
         * Explicit exceptions:
         *
         * Architecture, linker, Rust-language or boot-ABI constraints that force
         * code outside the default mapped file must be recorded with reason,
         * scope and the preserved model boundary.
         */
        coding_must_record_exceptions();

        /*
         * Verification gates:
         *
         * Changes touching model semantics, phase call chains, object states or
         * entry paths must pass the configured build/verify/run gates before
         * they are treated as complete.
         */
        coding_must_run_verification_gates();
    }
}
