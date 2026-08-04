# UserFrame coding contract

`UserFrameRef` is the checked ownership representation for one order-0 frame used as resident private user
backing. Page-table pages and kernel-only pages remain unique `PageRef` owners and must never enter this
reference protocol. A stack or non-stack sparse backing slot owns exactly one `UserFrameRef`; temporary fork
or COW transaction state may own an additional reference until commit or rollback.

The page's `PageMetadataMap` entry stores an explicit user-frame kind and a nonzero `usize` reference count.
Constructing a `UserFrameRef` from a fresh allocator page sets kind=user-frame and count=1. `acquire` validates
the page identity, kind, live nonzero count and checked-add capacity before incrementing. `release` validates
the same identity/kind/count and a live owning slot before decrementing; exactly the transition 1→0 clears the
kind and returns the page once to the order-0 allocator. Stale, overflow, underflow and repeated release are
deterministic errors. Generic `PageRef` free is forbidden while the user-frame count is live.

The count invariant is the number of live user leaf PTE/backing owners plus unpublished transaction owners for
that physical frame. Refcount 1 is the only unique state; values greater than 1 are shared. Fork, COW replacement,
exec, munmap, exit and rollback update the owning slot and count as one ordered transaction. A failure before the
new owner is committed releases only staged references and leaves every published owner, PTE and byte unchanged.
