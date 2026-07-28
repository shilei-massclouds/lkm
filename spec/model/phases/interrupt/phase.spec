/*
 * Interrupt leaf namespace
 *
 * InterruptPhase no longer exists.  The four leaves are direct BootInitFlow
 * children; this file only composes them and defines the post-SIE context.
 */

include "irq-time-init/main.spec";
include "local-irq-enable/main.spec";
include "irq-open-prepare/main.spec";
include "process-prepare/main.spec";

context SingleTaskInterruptContext: Context {
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: enabled;
            preemption: disabled;
        }
    }
}
