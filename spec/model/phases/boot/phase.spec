/*
 * Boot leaf namespace
 *
 * BootPhase no longer exists.  This file keeps the boot leaf includes and
 * SingleTaskContext used by BootInitFlow's direct drives.
 */

include "core-prepare/main.spec";
include "mm-core-init/main.spec";
include "sched-init/main.spec";

context SingleTaskContext: Context {
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }
}
