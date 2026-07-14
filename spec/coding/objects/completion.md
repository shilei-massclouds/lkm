# Completion coding

本文件是 Completion 对象实现映射的权威 coding 规格，保留稳定 rule ID、原
`ArceosExCompletionCodingMust` type 分组和 MUST 层级；这些 ID 用于评审和追踪，不是 pyveri predicate。

## Rule catalog

### ArceosExCompletionCodingMust

#### Reusable object

Rule ID: `arceos_ex_must_completion_map_to_reusable_object` (MUST).

The formal Completion Type must map to a reusable Rust resource
object, not to ad-hoc boolean fields on each user. Its implementation
target is impl/arceos_ex/src/objects/completion.rs.

#### Owned wait queue

Rule ID: `arceos_ex_must_completion_own_simple_wait_queue` (MUST).

Completion must own a SimpleWaitQueue field. The field is an owned
child resource corresponding to Linux swait_queue_head, not an
external wait-queue reference supplied by the caller.

#### Event/action boundary

Rule ID: `arceos_ex_must_completion_processes_keep_state_effects` (MUST).

Completion.setup()/enable() advance the ordinary lifecycle state.
complete(), complete_all(), wait(), try_wait(), reinit() operate on
CompletionExtState and token count while preserving the main
lifecycle state, and done() is a read-only action.

#### Instance inheritance

Rule ID: `arceos_ex_must_completion_instances_drive_type_processes` (MUST).

A model object declared as an instance of Completion, such as
KthreaddReadyGate, must drive the reusable Completion implementation
instead of copying completion-specific pending/completed bookkeeping
into a private object-local state machine.

#### Smoke coverage

Rule ID: `arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow` (MUST).

Smoke tests must cover Completion.setup(), Completion.enable(),
complete(), token observation and token consumption, and must also
verify the live kthreadd_done/KthreaddReadyGate instance.
