/*
 * BootInitFlow Enable/Online support declarations.
 *
 * The three existing child PhaseObjects stay registered through the legacy
 * boot-init phase namespace entry; they remain distinct PhaseObjects rather
 * than physical fragments of the BootInitFlow root transition.
 */

include "../../phases/boot-init/main.spec";

predicate boot_init_flow_switch_precommit_ready<B, S, T, K>(
    boot_init: B,
    scheduler: S,
    boot_task: T,
    kernel_init_task: K
) -> bool;

predicate boot_init_flow_schedule_returned<B, S>(boot_init: B, scheduler: S) -> bool;
