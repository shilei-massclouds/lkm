# Completion coding

本文件承载 `spec/coding/objects/completion.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/objects/completion.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`completion.spec`](completion.spec)。

### ArceosExCompletionCodingMust

#### Reusable object

The formal Completion Type must map to a reusable Rust resource
object, not to ad-hoc boolean fields on each user. Its implementation
target is impl/arceos_ex/src/objects/completion.rs.

#### Owned wait queue

Completion must own a SimpleWaitQueue field. The field is an owned
child resource corresponding to Linux swait_queue_head, not an
external wait-queue reference supplied by the caller.

#### Event/action boundary

Completion.setup()/enable() advance the ordinary lifecycle state.
complete(), complete_all(), wait(), try_wait(), reinit() operate on
CompletionExtState and token count while preserving the main
lifecycle state, and done() is a read-only action.

#### Instance inheritance

A model object declared as an instance of Completion, such as
KthreaddReadyGate, must drive the reusable Completion implementation
instead of copying completion-specific pending/completed bookkeeping
into a private object-local state machine.

#### Smoke coverage

Smoke tests must cover Completion.setup(), Completion.enable(),
complete(), token observation and token consumption, and must also
verify the live kthreadd_done/KthreaddReadyGate instance.

<!-- formal-predicate-notes:spec/coding/objects/completion.spec END -->
