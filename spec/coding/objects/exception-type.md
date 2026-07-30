# ExceptionType

Each `TrapType` embeds one `ExceptionType`, which owns the RISC-V `scause` exception classifier, fatal fallback and the
four concrete exception resources. `Preset` covers every cause with fallback before any specific binding; `Setup`
installs page-fault, breakpoint and unexpected bindings; `Enable` requires all concrete resources online.

`ExceptionType.Preset` is driven only from its parent `TrapType.Setup` while the parent remains Prepared. It drives
the page-fault, syscall, breakpoint and unexpected Presets and verifies all four fallback bindings before the parent
writes the formal response entry to `stvec`. It must not install the later Setup handlers or open interrupt entry.

The dispatcher creates and synchronously completes an `ExceptionFlowType`, then one concrete exception Flow. Unknown
causes never bypass the unexpected resource. Failure to resolve any CPU/task/Flow generation emits the diagnostic
identity tuple and shuts the system down.

Mapping: charter [`exception-type.md`](../../charter/objects/exception-type.md), model
[`exception_type.spec`](../../model/objects/exception_type.spec), implementation
[`exception_type.rs`](../../../impl/arceos_ex/src/objects/exception_type.rs).
