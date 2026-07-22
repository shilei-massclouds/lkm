/*
 * KernelInitFlow leaf namespace
 *
 * SmpRuntimePhase no longer exists.  KernelInitFlow directly drives the BP
 * leaves composed here.  AP-owned phase families remain under SmpBringup.
 */

include "pre-smp-init/main.spec";
include "smp-bringup/main.spec";
include "runtime-core/main.spec";
include "initcall/main.spec";
include "rootfs/main.spec";
include "finalize/main.spec";
