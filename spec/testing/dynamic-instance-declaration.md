# Dynamic instance declaration testing contract

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

## Task and Flow scenarios

- Two fork executions yield different Task, TaskRef, UserTaskFlow, and
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
