# Dynamic instance declaration and snapshot materialization testing contract

This file defines the required regression coverage for runtime `declare`
statements. It supplements the Task/TaskFlow scenarios without redefining the
model semantics.

## Parser and checker coverage

- Parser cases preserve every `drives` statement in source order and serialize
  declaration alias, declared Type, owner process, source ordinal and span.
- Negative parser cases cover declarations outside `drives`, missing semicolons
  and malformed declaration forms.
- Checker cases reject use-before-declare, duplicate or shadowed aliases,
  collisions with static objects, unknown Types and missing Type processes.
- A declaration remains visible to later statements and later nested `within`
  scopes. A nested declaration does not leak to its parent scope.
- A callee receives a runtime instance only through a typed explicit parameter;
  an unbound callee-local receiver is an error.

## Derive, JSON and view coverage

- Repeated execution of one declaration site creates distinct occurrence IDs,
  each starting from `Base` with an independent lifecycle state map.
- Runtime identity assertions use root call path, owner process, statement
  ordinal, alias and occurrence. Adding unrelated lines must not change it.
- Derive JSON distinguishes static objects, declaration sites and runtime
  instances, including Type, current state, owner/ref targets and occurrence.
- Object views show declaration sites as templates. Trace views show each
  concrete occurrence separately and must not merge nodes merely because their
  lexical aliases match.
- Failure after an ordinary non-indexed declaration preserves the created
  instance in diagnostics; tests must not expect implicit rollback or Cleanup.
- An `indexed` owned declaration is instead a parent-handler publication
  transaction. Tests must prove that the child identity/index and parent update
  become stable together, and that child or parent failure rolls back the whole
  unpublished transaction without leaving an element or occupied key.
- Indexed coverage must include a valid dynamic create, duplicate key, wrong key
  type, out-of-bounds key, insertion by a non-owner, missing-element target,
  indexed Signal target and snapshot round-trip with canonical identity
  `Parent.collection[key]`.

## Snapshot materialization coverage

- Parser/model JSON preserves a named `snapshot` block and ordered `materialize`
  statements separately from ordinary `declare`, including owner-local ordinal,
  declared Type, requested state and span.
- Stateful materialization starts directly in the requested state and produces no
  `Preset`, `Setup`, `Enable` or other lifecycle transition Signal. Stateless
  materialization omits state and has no entry in the snapshot state map.
- All fresh identities, references, facts and state invariants publish in one
  `snapshot_committed` event after validation of the complete candidate.
- Negative cases cover block-external materialize, nested snapshot, duplicate alias,
  unknown Type/state, missing state for a stateful Type, state supplied for a
  stateless Type, lifecycle Transition calls in a snapshot, missing binding and a
  failing mid-block Action. Every failure must produce `snapshot_rolled_back` and
  preserve the exact pre-block committed instances, states, references, facts and
  occurrence counters.
- Snapshot protocol round-trip, trace view, text render and animation must retain
  `construction = materialize`, block site and committed state; no consumer may
  infer materialization from an ordinary state delta.

## Task and Flow scenarios

- Two fork executions yield different Task, TaskRef, TaskFlow, and
  UserAppRuntime identities with independent occurrence/lifecycle state and Ref
  targets.
- Two exec executions on one Task preserve Task, TaskFlow, and UserAppRuntime
  identity while creating distinct internal ApplicationInstance generations.
- A Flow cannot be owned by two Tasks, and Task terminal cleanup cannot complete
  until its one Flow and any owned Runtime have completed terminal cleanup.

## Stress and differential gates

- The tool stress case executes one declaration site at least 128 times and
  asserts unique identities, consecutive occurrences and independent states.
- Repository runtime stress uses the canonical root `make stress-test` target.
- Linux differential coverage uses the canonical root `make difftest` target and
  the current checkpoint mapping. A checkpoint rename requires regenerated
  inventory/mapping/coverage/instrumentation artifacts and marker drift checks.
- After every code change, the final regression gate remains the direct
  repository-root `make test` command.
