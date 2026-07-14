# Kernel Project Charter

`KernelProject` names the kernel engineering product, not a running kernel.
It covers the inputs, constraints, build products, and evaluation boundary that
produce a bootable kernel image.

## Intent

- Keep project-level work separate from runtime system behavior.
- Treat model, coding guidance, source layout, build inputs, linker layout, and
  test expectations as parts of one engineering artifact.
- Produce a kernel image that can be launched as a `Kernel` system instance.

## Inputs

- Architecture, firmware, platform, linker, and configuration facts required to
  construct the image.
- The `Kernel` system specification that defines the runtime lifecycle the image
  must realize after launch.
- Coding constraints that map project-owned preparation and build artifacts into
  implementation files.

## Outputs

- A constructed kernel image with its required static layout and configuration
  facts established.
- A handoff point that starts a `Kernel` instance for boot and evaluation.
- Project-level evidence that the image was built and exercised under the
  declared specification chain.

## Boundary

`KernelProject` may require the `Kernel` system specification and may start a
kernel instance, but it does not own the runtime phase choreography. Runtime
boot, interrupt opening, multitask setup, SMP/runtime setup, and payload handoff
belong to `Kernel`.

## Mapping

- Model: `spec/model/projects/kernel.spec`
- Coding: `spec/coding/projects/kernel.md`
- Implementation: `impl/arceos_ex/src/projects/kernel.rs`
