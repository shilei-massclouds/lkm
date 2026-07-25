/*
 * Model Specification Entry
 *
 * This is the formal entry for the object/state/transition model under spec/model.
 * The entry composes one top-level system tree rooted at Computer. File-level coding
 * ownership for every objects/*.spec input is indexed by
 * spec/coding/objects/README.md; the index does not change model semantics.
 */

include "objects/main.spec";
include "systems/riscv64-platform.spec";
include "systems/opensbi.spec";
include "systems/kernel.spec";
include "systems/computer.spec";
