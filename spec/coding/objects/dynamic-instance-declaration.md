# Dynamic instance declaration coding

This file maps charter/model `declare name of Type;` semantics into the shared
parser/model/check/derive/view/render toolchain. It applies to every Type; Task
and UserAppFlow are the first production users.

It also maps indexed-owned declarations such as `declare self.cpus[0] of CPU;`.

## Ordered representation

The shared AST must represent each drives statement explicitly and in source
order. A declaration statement records `kind=declare`, alias, declared Type,
owner process, owner-local source ordinal and diagnostic span. It must not be
encoded as a static object, fact, `let` result or unparsed call. Model JSON keeps
declaration sites on their owner process while `ObjectModel.objects` remains the
static named-object inventory.

An indexed declaration additionally records owner receiver, owned field, key
expression and declared element Type. The checker resolves the `owned { indexed
field[key: KeyType]: ElementType; }` declaration, type-checks the key, requires
the current parent handler to own the collection, and rejects direct insertion
by any other receiver. Duplicate keys and references to missing elements are
runtime errors, never implicit aliasing.

Parser rejection is required for declarations outside transition/action or
nested `within` drives, missing semicolons and malformed declaration syntax.
The model checker maintains one lexical SSA namespace for process parameters,
`let` bindings and declarations. It rejects unknown Types, use before declare,
duplicate/shadowed aliases, visible static-object collisions, missing dynamic
receiver processes and argument/type mismatches. A child scope inherits earlier
bindings but its declarations do not escape to its parent or siblings.

## Runtime lowering

Derive creates a fresh runtime record only when execution reaches a declaration.
Each record starts in the `initial_state` of the nearest Type that declares its
lifecycle and owns independent lifecycle state, facts, transition commits and
trace nodes. Static objects with no local lifecycle and runtime records of the
same Type must resolve the same complete state graph. On every inherited state
entry, derive substitutes the concrete static or runtime identity for `self`
before checking the Type invariant. Declaration does not run Preset, establish
relationships or register rollback/cleanup. Scope exit only drops the lexical
alias; it does not destroy the instance.

Indexed instances use `Owner.field[key]` as their runtime identity, Signal
target and snapshot state key. Publication is transactional at the parent
handler boundary: child declaration, child calls, parent state/facts and all
descendant effects become visible together only when the handler succeeds.
Failure leaves no element, key reservation, state entry or trace commit.

An object-local state graph is rejected when its Type already supplies one,
unless the object explicitly requests a complete lifecycle override. Override
replaces the whole Type graph; partial state, transition, ensure or invariant
merging is forbidden. Runtime declarations have no object declaration and
therefore cannot override their Type lifecycle.

Runtime identity is composed from root call path, owner process, owner-local
declaration ordinal, alias and occurrence under that call path. Physical line
numbers are diagnostic-only. Replaying the same roots must be deterministic,
while separate executions of the same site must have different occurrence and
identity.

Static view/render output shows declaration sites. Derive/trace output shows
every concrete runtime identity separately and includes declaration-site data,
alias, declared Type and `static_object: null`. JSON schema versions must change
when these shapes change; pyveri must reject stale producer/consumer versions.

## Initial capability boundary

The first lowering is sequential. It does not add loops, concurrent declaration
scheduling, implicit garbage collection or rollback. Runtime instances survive
through explicit facts, collections, typed Ref results and explicit process
parameters; tools must never synthesize a global object name from an alias.
