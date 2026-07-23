# Kernel Project Charter

`KernelProject` names the kernel engineering product, not a running kernel.
It covers the inputs, constraints, and build products that produce a bootable
kernel image. It is a direct child of `ComputerProject` and stops at Ready.

## Intent

- Keep project-level work separate from runtime system behavior.
- Treat model, coding guidance, source layout, build inputs, linker layout, and
  test expectations as parts of one engineering artifact.
- Produce a kernel image that can be launched as a `Kernel` system instance.

## Inputs

- Architecture, firmware, and platform specifications required to construct the
  image.
- The `Kernel` system specification that defines the runtime lifecycle the image
  must realize after launch.
- Coding constraints that map project-owned preparation and build artifacts into
  implementation files.

## Outputs

- A constructed kernel image with its required static layout and configuration
  facts established.
- Project-owned `Config` and `Lds` objects in Online state, constructed in that
  order during `KernelProject.Setup`.
- Project-level evidence that the image was built and exercised under the
  declared specification chain.

## Boundary

`KernelProject.Preset` establishes the `Kernel` system specification and stops
at Prepared. `KernelProject.Setup` fully constructs `Config`, then `Lds`, then
the kernel image and stops at Ready. It does not execute Enable or start a
kernel instance. Runtime
boot, interrupt opening, multitask setup, SMP/runtime setup, and payload handoff
belong to `Kernel`.

## Mapping

- Model: `spec/model/projects/kernel.spec`
- Coding: `spec/coding/projects/kernel.md`
- Implementation: `impl/arceos_ex/src/projects/kernel.rs`
