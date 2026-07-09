/*
 * arceos_ex coding formal entry
 *
 * This compatibility entry includes the split arceos_ex coding rule files.
 * Keep predicate/type names and invariant call sets in the included files.
 */

include "objects/device-tree-must.spec";
include "phases/boot/core-prepare.spec";
include "objects/effective-context.spec";
include "objects/device-tree-should.spec";
include "phases/boot/mm-core-init.spec";
include "phases/interrupt/local-irq-enable.spec";
include "phases/boot/entry-prelude.spec";
include "phases/boot/entry-successor.spec";
include "systems/kernel.spec";
include "phases/interrupt/irq-time-init.spec";
include "phases/interrupt/irq-open-prepare.spec";
include "phases/interrupt/process-prepare.spec";
include "objects/completion.spec";
include "objects/block-io.spec";
include "phases/up-multitask/rest-init.spec";
include "phases/smp-runtime/pre-smp-init.spec";
include "phases/smp-runtime/smp-bringup.spec";
include "phases/smp-runtime/runtime-core.spec";
include "phases/smp-runtime/initcall.spec";
include "phases/smp-runtime/rootfs.spec";
include "phases/smp-runtime/finalize.spec";
