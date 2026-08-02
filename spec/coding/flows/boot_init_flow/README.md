# BootInitFlow coding constraints

BootInitFlow is the BootTask's immutable lifetime Flow. It is statically bound before `_start`, becomes
Online once boot preparation and schedule preflight complete, and stays Online while BootTask alternates
between OnCpu and Online.

Its Online actions own idle setup, the `schedule_current()` yield boundary, post-schedule idle entry and
the idle loop. The call must return through the same Rust continuation when identity is selected and
through the restored TaskThreadContext plus contextual Continue after a non-identity A->B->A sequence.

No separate idle Flow storage, FlowRef, checkpoint family or dispatch branch may exist. The Rust module
path remains `crate::flows::boot_init_flow`; the physical transition files do not create additional
objects, lifecycles or public API namespaces.

## Transition mappings

- [`Preset`](preset.md) lowers the kernel-entry assembly, adoption and Prepared commit.
- [`Setup`](setup.md) lowers `start_kernel()`, direct object/leaf orchestration and the Ready commit.
- [`Enable / Online`](enable.md) lowers the child PhaseObjects, first scheduling boundary, restored
  continuation and idle entry.

`BootInitFlow.Setup` 的 `start_kernel()` 到 `setup_arch()` 返回 helper 属于 Flow 私有实现，不拥有
lifecycle、state、checkpoint 或公开 API。
