# Kernel Project Coding

本文件承载 `projects/kernel.spec` 的说明性正文。Formal 文件只保留 KernelProject 层级的 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/projects/kernel.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/projects/kernel.spec` 的长注释迁移而来。`*.spec` 文件只保留 rule ID、type 分组、MUST/SHOULD/MAY/NOTE 层级和最短标签；解释、背景、参考路径、阶段性取舍与例子在这里维护。

### KernelProjectCoding

#### Mapping path

KernelProject implementation metadata belongs under
impl/arceos_ex/src/projects/. The current skeleton maps the project
object to impl/arceos_ex/src/projects/kernel.rs.

#### Spec chain

The implementation entry must record the charter, model and coding
paths that define the project object before later behavior is added.

#### Build inputs

Project-level construction may depend on model Lds and Config facts,
architecture/firmware/platform facts, and build configuration facts.

#### Runtime separation

KernelProject implementation must not drive BootPhase,
InterruptPhase, UpMultitaskPhase, SmpRuntimePhase or PayloadPhase.
Those transitions belong to the Kernel system and phase mappings.

#### Checkpoint ownership

Project mapping code must not emit runtime checkpoints or synthetic
phase observations merely to represent system progress.

#### Ownership

Build inputs, image construction facts and project/product metadata
remain project-owned unless a later model/coding update records a
narrower object owner.

<!-- formal-predicate-notes:spec/coding/projects/kernel.spec END -->
