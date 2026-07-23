/*
 * Model Specification Entry
 *
 * This is the formal entry for the object/state/transition model under spec/model.
 * The entry composes two independent static trees. ComputerProject is the
 * engineering root; Computer is the runtime-system root. File-level coding
 * ownership for every objects/*.spec input is indexed by
 * spec/coding/objects/README.md; the index does not change model semantics.
 */

include "objects/main.spec";
include "projects/hardware.spec";
include "projects/firmware.spec";
include "projects/kernel.spec";
include "systems/riscv64-platform.spec";
include "systems/opensbi.spec";
include "systems/kernel.spec";
include "systems/computer.spec";
include "projects/computer.spec";
