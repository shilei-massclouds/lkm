# Kernel System Charter

`Kernel` names a running kernel system instance created from a prepared
`KernelProject` image. It owns the runtime lifecycle from boot readiness through
payload handoff.

## Intent

- Keep runtime state and phase choreography separate from project construction.
- Make the kernel instance lifecycle visible as a system object, with phase
  boundaries preserving model ownership.
- Preserve compatibility with the current startup timeline while providing a
  stable mapping target for later top-down refinement.

## Lifecycle

The system instance starts from project-provided boot inputs, reaches a boot
ready boundary, completes interrupt preparation, enters the multitask/runtime
sequence, and finally hands control to the selected payload.

## Responsibilities

- Own the ordering relationship among boot, interrupt, up-multitask,
  SMP/runtime, and payload phases.
- Keep phase setup and handoff boundaries mapped to their phase implementation
  files.
- Require project-level inputs such as architecture, firmware, linker layout,
  and configuration facts without redefining them.

## Compatibility Boundary

The current `startup_timeline_ready` and `startup_timeline_event` entry points
remain the implementation compatibility boundary. This charter does not require
moving those functions or changing checkpoint order.

## Mapping

- Model: `spec/model/systems/kernel.spec`
- Coding: `spec/coding/systems/kernel.spec`
- Implementation: `impl/arceos_ex/src/systems/kernel.rs`
