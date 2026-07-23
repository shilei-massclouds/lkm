# Kernel Project Coding

本文件是 KernelProject 层级的权威 coding 规格，保留稳定 rule ID、原 `KernelProjectCoding`
type 分组和 MUST 层级；这些 ID 用于评审和追踪，不是 pyveri predicate。

## Rule catalog

### KernelProjectCoding

#### Mapping path

Rule ID: `kernel_project_coding_maps_to_arceos_ex_project_module` (MUST).

KernelProject implementation metadata belongs under
impl/arceos_ex/src/projects/. The current skeleton maps the project
object to impl/arceos_ex/src/projects/kernel.rs.

#### Spec chain

Rule ID: `kernel_project_coding_records_spec_chain` (MUST).

The implementation entry must record the charter, model and coding
paths that define the project object before later behavior is added.

#### Build inputs

Rule ID: `kernel_project_coding_uses_model_lds_and_config_inputs` (MUST).

Project-level construction must build model Config completely before Lds, then
establish the kernel-image fact. Architecture/firmware/platform specifications
and build configuration facts are inputs; Config and Lds are KernelProject-owned
products.

#### Runtime separation

Rule ID: `kernel_project_coding_does_not_drive_runtime_phases` (MUST).

KernelProject implementation must not drive BootInitFlow, KernelInitFlow or
any of their direct leaf phases. Those transitions belong to the Kernel
system, TaskFlow and phase mappings.

#### Checkpoint ownership

Rule ID: `kernel_project_coding_does_not_emit_runtime_checkpoints` (MUST).

Project mapping code must not emit runtime checkpoints or synthetic
phase observations merely to represent system progress.

#### Ownership

Rule ID: `kernel_project_coding_keeps_build_inputs_project_owned` (MUST).

Build inputs, image construction facts and project/product metadata remain
project-owned. KernelProject stops at Ready and must not start Kernel or OpenSBI.
