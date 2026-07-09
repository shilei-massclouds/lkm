# Kernel System Coding

本文件承载 `systems/kernel.spec` 的说明性正文。Formal 文件只保留 Kernel system 层级的 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/systems/kernel.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/systems/kernel.spec` 的长注释迁移而来。`*.spec` 文件只保留 rule ID、type 分组、MUST/SHOULD/MAY/NOTE 层级和最短标签；解释、背景、参考路径、阶段性取舍与例子在这里维护。

### KernelSystemCoding

#### Mapping path

Kernel system implementation belongs under impl/arceos_ex/src/systems/.
The Kernel object maps to impl/arceos_ex/src/systems/kernel.rs.

#### Kernel lifecycle boundary

systems/kernel.rs must provide the system lifecycle entry points that
preserve the model Kernel lifecycle states. Kernel.Preset completes
only after BootPhase.Ready and moves Kernel from Base to Prepared;
Kernel.Setup completes only after InterruptPhase.Ready and moves
Kernel from Prepared to Ready; Kernel.Enable drives
UpMultitaskPhase, SmpRuntimePhase and PayloadPhase, and marks Kernel
Online only after PayloadPhase.Online.

#### Runtime choreography

Kernel owns the ordering relationship among BootPhase,
InterruptPhase, UpMultitaskPhase, SmpRuntimePhase and PayloadPhase.
Individual phase setup/handoff code remains in the mapped phase
files.

#### Kernel state

The system module owns the Kernel lifecycle state and emits Kernel
system checkpoints. Crate-root entry code must not retain separate
startup timeline state.

#### Behavior preservation

Kernel system migration must not reorder existing phase calls or
checkpoint emission order.

#### Phase ownership

System-level mapping records the lifecycle owner but must not absorb
phase-local checks, diagnostics or checkpoints out of their phase
modules.

#### Payload handoff

The selected payload remains a sibling phase handoff after
SMP/runtime readiness, not a project-level build action and not a
nested runtime-core side effect.

<!-- formal-predicate-notes:spec/coding/systems/kernel.spec END -->
