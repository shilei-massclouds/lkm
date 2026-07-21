# Dynamic instance declaration coding

This file maps charter/model `declare name of Type;` semantics into the shared
parser/model/check/derive/view/render toolchain. It applies to every Type; Task
and UserAppFlow are the first production users.

## Ordered representation

The shared AST must represent each drives statement explicitly and in source
order. A declaration statement records `kind=declare`, alias, declared Type,
owner process, owner-local source ordinal and diagnostic span. It must not be
encoded as a static object, fact, `let` result or unparsed call. Model JSON keeps
declaration sites on their owner process while `ObjectModel.objects` remains the
static named-object inventory.

Parser rejection is required for declarations outside transition/action or
nested `within` drives, missing semicolons and malformed declaration syntax.
The model checker maintains one lexical SSA namespace for process parameters,
`let` bindings and declarations. It rejects unknown Types, use before declare,
duplicate/shadowed aliases, visible static-object collisions, missing dynamic
receiver processes and argument/type mismatches. A child scope inherits earlier
bindings but its declarations do not escape to its parent or siblings.

## Runtime lowering

Derive creates a fresh runtime record only when execution reaches a declaration.
Each record starts in `Base` and owns independent lifecycle state, facts,
transition commits and trace nodes. Declaration does not run Preset, establish
relationships or register rollback/cleanup. Scope exit only drops the lexical
alias; it does not destroy the instance.

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
