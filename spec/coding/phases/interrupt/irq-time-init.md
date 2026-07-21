# IrqTimeInitPhase coding

model 来源为 `spec/model/phases/interrupt/irq-time-init/phase.spec`，实现落点为
`impl/arceos_ex/src/phases/interrupt/irq_time_init.rs`。本文件同时承载原 legacy formal rule 的
有效实现约束；coding 层不再保留同主题 `.spec`。

## 生命周期映射

| Transition | depends / drives / ensures | checkpoint 与 continuation |
| --- | --- | --- |
| Preset: Base -> Prepared | 在 `SingleTaskContext` 下精确检查 Base、SchedInit Online、SIE 关闭及对象依赖；按 model 顺序执行现有 IRQ/time 对象动作；确认中断、任务和 SMP 并发仍关闭 | 接受后发出 Started；提交后发出 Prepared，调用 Setup |
| Setup: Prepared -> Ready | 精确检查 Prepared，重新检查 `irq_time_init_phase_ready()` 和中断关闭事实 | 提交后发出 Ready，调用 Enable |
| Enable: Ready -> Online | 精确检查 Ready，重新检查同一 invariant | 提交后发出 Online，返回 `interrupt::preset_after_irq_time_init()` |

`is_online()` 只接受精确 Online。IrqTimeInit 不得启动 LocalIrqEnable；父 Interrupt continuation
持有后续顺序。

## 已迁移实现约束

#### Model path

IrqTimeInitPhase is InterruptPhase subphase 1. Its formal model
path is spec/model/phases/interrupt/irq-time-init/, not
spec/model/phases/boot/irq-time-init/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/irq_time_init.rs,
rather than the boot phase subtree.

#### Interrupt-open boundary

IrqTimeInitPhase must finish with IRQ/time infrastructure ready but
boot CPU local interrupts still disabled. The local_irq_enable()
boundary belongs to the following LocalIrqEnablePhase so this setup
body can remain under the global exclusive boot context.

#### RISC-V IRQ stack/SCS setup

init_IRQ() runs init_irq_scs() and init_irq_stacks() before
irqchip_init(). With the current config CONFIG_IRQ_STACKS=y and
CONFIG_VMAP_STACK=y, the implementation must expose an
RiscvIrqStackSet object/facts for per-CPU IRQ stack pointer setup.
CONFIG_SHADOW_CALL_STACK is disabled, so IRQ SCS allocation must be
recorded as a trimmed/no-op path. Runtime call_on_irq_stack() entry
switching remains deferred and must not be claimed Online here.

#### Timekeeper lock protocol

timekeeping_init() writes tk_core under raw_spin_lock_irqsave()
on timekeeper_lock and write_seqcount_begin/end on tk_core.seq.
The implementation must keep both the irqsave/raw-spinlock fact
and the seqcount-writer fact observable instead of relying only on
the enclosing boot-exclusive context.

#### Config-trimmed calls

rcu_init_nohz() and kfence_init() are real call points inside the
IrqTimeInitPhase Linux range. For the current config,
CONFIG_RCU_NOCB_CPU=n makes rcu_init_nohz() an inline no-op and
CONFIG_KFENCE=n makes kfence_init() an inline no-op. The
implementation must record both positions as structured trimmed
facts, not as comments or implicit absence.

#### PLIC driver split

PLIC must be represented as an independent irqchip driver entry and
provider object. The driver registration boundary is separate from
the PLIC provider setup result; it must not collapse back into an
IrqController boolean or an ordinary PlatformBus probe.

#### IRQCHIP_DECLARE lowering

PLIC irqchip init registration must lower to a retained static
entry in an irqchip init LDS section. The implementation must not
build the primary irqchip table by runtime push into a growable
registry.

#### init_IRQ call chain

The IrqTimeInitPhase implementation of init_IRQ() must call
irqchip_init(), which must call of_irq_init() over the LDS-collected
irqchip init section. PLIC setup must be reached because this chain
finds the PLIC entry in that section, not because the phase directly
invokes a PLIC-specific setup function.

#### Compatible match and callback

of_irq_init() must match the DeviceTree interrupt-controller node's
compatible against the section entry and invoke the matched PLIC
init callback. Tests must cover both observations: the section entry
is present, and the callback was invoked through traversal.

#### Parent interrupt link

PLIC's interrupt output must be modeled and implemented as feeding
the external interrupt input of the parent RISC-V CPU local INTC.
The UART external interrupt physical propagation chain is
UART -> PLIC -> RISC-V root INTC -> CPU. This link records the
physical parent chain only; it is separate from the software
dispatch/claim chain. The chain has two independent gates
before a UART interrupt can reach the CPU: the PLIC UART source
enable gate and the root INTC supervisor external input enable
gate. Their ownership and identity must be explicit.

#### Named interrupt causes

Interrupt causes must be modeled and implemented through named
architecture constants or refs. The RISC-V supervisor external
interrupt cause is the root INTC EXT_IRQ/SEI input; specs and code
must not describe the route by embedding the raw cause number in
control-flow logic or prose.

#### External IRQ gates

The two UART external propagation gates must be modeled as named
gates with observable Closed state before any runtime source-enable
work. The root supervisor external input gate is defined when the
RiscvIntc/InterruptStream external route is installed. The PLIC
UART source gate is defined by the PlicIrqMapping that binds
HwirqRef::PlicUart0 to the UART logical IRQ. Mapping,
request_irq(), and chained-handler setup may define these facts,
but must defer the explicit Enable action and must not silently
open either gate.

#### PLIC MMIO ownership

PLIC MMIO mapping must use the runtime ioremap/vmalloc mapping
execution path, but its owner is the system irqchip itself. The
implementation must not fabricate a PlatformDevice just to reuse
device MMIO ownership, because UART platform devices later consume
PLIC as their interrupt parent rather than owning the controller.

#### PLIC DT setup

The matched PLIC init callback must parse the DeviceTree interrupt
controller node's reg range, riscv,ndev source count, and
interrupts-extended parent input before marking Plic.Ready. The
parent input must represent a RISC-V external interrupt line; strict
phandle-to-boot-hart binding belongs to the later IRQ domain/source
mapping step once DeviceTree phandle lookup is modeled.

#### IRQ domain split

IrqDomain is the generic IRQ core mapping contract, while
PlicIrqDomain is the concrete instance owned by the PLIC provider.
The implementation must not collapse logical IRQ allocation into
Plic itself or into ns16550a driver-private state.

#### PLIC source mapping

PlicIrqDomain may translate a one-cell PLIC interrupt specifier and
create an idempotent source -> logical IRQ mapping. It must reject
source 0 and sources outside the PLIC source count, and duplicate
mapping of the same source must return the existing logical IRQ.

#### UART IRQ resource

ns16550a platform probe must parse its IRQ resource from the
platform device's DeviceTree node, including the interrupt specifier
and PLIC interrupt parent. It must then bind the UART port to the
PLIC logical IRQ returned by PlicIrqDomain.

#### Deferred interrupt output

The UART IRQ resource mapping step records the UART source/logical
IRQ binding only. It must not enable the PLIC source or declare
serial8250 interrupt-driven console output ready. The root INTC
supervisor external input gate and PLIC UART source gate must both
remain Closed; registering a handler or mapping a source must not
silently open either gate.

#### Explicit external IRQ enable

After UART IRQ resource mapping and request_irq() action recording,
the implementation may open the two external propagation gates only
through a named UartExternalIrqEnable boundary. That boundary must
perform the PLIC UART source enable and the root INTC supervisor
external input unmask as separate observable facts. It must not
fabricate a UART interrupt, call the handler, run PLIC claim or
complete, or mark serial8250 console output interrupt-driven.

#### Production UART interrupt-chain probe

The first real UART interrupt may be triggered only by a named
production boundary after UartExternalIrqEnable has opened both
gates. That boundary must follow the Linux-like 8250 THRI shape:
raise the UART interrupt-output precondition such as MCR.OUT2,
enable UART_IER_THRI, and create a real TX-empty transition instead
of assuming an IER write alone will always assert an interrupt. It
then waits for the real root INTC -> PLIC claim -> IRQ dispatch ->
UART handler -> PLIC complete path. KUnit/smoke code must only
observe the resulting facts and counters; it must not call trigger,
claim, complete, dispatch, or handler APIs directly.

#### UART interrupt cycle observation

UartInterruptChainProbe must not stop at the first handler call.
It must observe one complete real IRQ cycle from a pre-trigger
snapshot: THRE request, PLIC non-zero claim, IRQ dispatch, UART
handler, PLIC complete, zero-claim loop exit, with non-zero claims
paired with completes on the current UART source. Global
claim/complete counters are auxiliary observations only; a mismatch
caused by another source or a concurrent sampling window must not
fail the current UART source cycle when source-scoped deltas match.
Zero-claim loop exit closure means that both the zero claim and the
following loop exit are observed in the same real claim-loop
boundary; when a provider records them as separate counters,
diagnostics must not fail only because a concurrent sample sees the
two counters temporarily differ.

#### IRQ handler registry

request_irq-style handler registration belongs to an IRQ core-side
IrqHandlerRegistry/IrqAction object. The implementation must not
store handler ownership in PlicIrqDomain, PlicIrqMapping, or
ns16550a driver-private ad hoc tables.

#### request_irq input contract

The minimal request_irq path must require a logical IRQ that was
already produced by PlicIrqDomain for a valid PLIC source. Attempts
to register an unmapped logical IRQ must fail, and duplicate
registration for the same logical IRQ/device must be rejected or
represented as the explicit duplicate policy.

#### Handler registration boundary

ns16550a probe may request a UART handler record once its logical
IRQ is known, but this must not enable the PLIC source, install a
private claim/complete route, or mark serial8250 console output
interrupt-driven. The route belongs to the root INTC/PLIC/IRQ core
dispatch chain.

#### Context guard

IrqAction records must carry a hardirq-context requirement before
they are dispatchable. IRQ dispatch must not open sleep/process-only
paths from interrupt context.

#### External interrupt dispatch contract

The RISC-V root INTC external interrupt entry must be a parent
EXT_IRQ/SEI entry that forwards to the PLIC chained handler. It
must not know about UART or dispatch leaf device handlers directly.

#### PLIC claim/complete order

The PLIC runtime handler must follow the Linux-like order: claim by
reading the claim register, translate the claimed source through
PlicIrqDomain/generic IRQ dispatch, run the registered action, then
complete by writing the claimed source back. It must loop until
claim returns zero, and a zero claim must stop dispatch without
calling the UART handler. Missing mapping/action may be reported,
but each non-zero claimed source must still reach complete.

#### IRQ core action dispatch

IRQ core dispatch must use the logical IRQ returned by
PlicIrqDomain and run only an action registered in
IrqHandlerRegistry. Missing mapping or missing action must not be
treated as a successful UART interrupt.

#### KUnit capability boundary

New checkpoint KUnit handlers must receive Context as read-only
input by default. Writable access is limited to an explicit sink
capability such as KTAP output, tracer or auditor objects.

MUST: HandlerRun has exactly one ordinary checkpoint-handler
capability shape, equivalent to:

  Observe(fn(Checkpoint, &Context, &mut dyn Sink) -> CheckpointOutcome)

The enum must not regain Read/Write variants or any variant that
accepts &mut Context. Changing this prototype requires a prior
coding-spec update that names a separate action-level probe
capability; it must not be done as a local handler convenience.

#### Checkpoint consumers

Checkpoints are observation points. Default builds must not enable a
heavy consumer. PROBE=announce enables the
checkpoint_handler_announce consumer, where each checkpoint emits a
minimal self-announcement. LOG=trace is only a compatibility alias
for PROBE=announce and must not be extended as the future
Linux-like trace interface. Other PROBE=... values enable
checkpoint_handler_* observers such as uart-irq-chain. These
consumers may read and emit facts, but they must remain distinct
from ordinary execution and from each other.

#### Checkpoint inventory export

The first Linux-differential checkpoint stage is inventory only.
Its sole source of truth is impl/arceos_ex/src/checkpoint/mod.rs:
Checkpoint enum order defines the stable index/order baseline,
Checkpoint::name() defines the public stable name, and explicit
early_byte() match arms define optional early announce metadata.
The export surface is tools/out/checkpoints/ with machine-readable
JSON and human-readable Markdown. Each JSON row must contain only
index, variant, name, early_byte and source_file.

This stage must not change checkpoint behavior, handlers, KUnit
output, runtime observations, memory collection, or any Linux
source tree. Linux insertion mapping and memory collection belong
to later stages consuming the exported inventory.

The inventory tool must also support a read-only regeneration check:
it regenerates JSON and Markdown in memory, compares them with the
tracked tools/out/checkpoints/ artifacts, and reports file drift as
failure without rewriting repository outputs.

#### Linux checkpoint alignment mapping

The second Linux-differential checkpoint stage is mapping only. It
consumes the exported arceos_ex checkpoint inventory and reads a
Linux reference source tree, defaulting to ../linux-6.12, to build
a reviewable candidate alignment list. It must preserve the
checkpoint inventory order and produce one row per checkpoint.

Each row must classify the mapping as exact, range or unmapped.
exact means a concrete Linux function or call anchor was found;
range means only an ordered Linux boot interval can be named;
unmapped means the current stage cannot justify a reliable mapping
and must record the reason instead of guessing.

The read-only source parser may resolve C function definitions,
SYSCALL_DEFINE* syscall wrapper macro definitions and assembly
symbols/labels. If the reference Linux tree already contains
whole-line marker comments or runtime recorder calls generated by
this project, the mapping parser must ignore those
LKM_CHECKPOINT/lkm_checkpoint_record/LKM_RUNTIME_CHECKPOINT lines
in its in-memory view before resolving symbols, anchors and line
numbers. This keeps the tracked mapping artifacts stable across
clean, marker-annotated and runtime-instrumented Linux trees, and
it must not replace the explicit marker stale/mismatch check path.
User-mode syscall checkpoints may map to the
corresponding SYSCALL_DEFINE* wrapper around the core helper call.
If a syscall ABI wrapper is arch/config conditional and therefore
not a stable RISC-V64 semantic boundary, the mapping must use the
shared implementation helper or remain unmapped, with medium-or-lower
confidence and notes that explain the conditional ABI layer. User
exec, return-to-user and wait boundaries may use RISC-V64
architecture-scoped anchors, but must keep range/medium confidence
where the correspondence is phase-level rather than a single exact
object boundary.
Linux runtime instrumentation must keep boot-time kernel_execve()
ownership separate from user syscall execve()/execveat()
ownership even when both paths share load_elf_binary(),
begin_new_exec(), exec_mmap(), start_thread() or
ret_from_exception anchors. Records emitted while kernel_init()
reaches run_init_process()/kernel_execve() belong to UserBoot.*,
UserAddressSpace.Ready and the first dynamic PID 1 UserAppFlow entry
boundary; they must not also emit UserExec.*. UserExec.* records
belong only to a runtime user exec that entered through
do_execveat_common() after SyscallTable.ExecveArgsReady.
Return-to-user markers used for UserExec or dynamic UserAppFlow must
also avoid flooding every ordinary syscall return; if the Linux
instrumentation cannot apply that ownership guard, those events
must remain outside active paired hard scope.
RISC-V64 ret_from_exception return-to-user Linux checkpoints must
be guarded by the saved SPP bit so they only record the user return
path. They must be emitted before restoring general registers, or
through an equivalent register-preserving recorder; placing the
current LKM_RUNTIME_CHECKPOINT macro after t4/t5/t6 restoration
corrupts the return frame, and placing it after the kernel/user
merge label misreports supervisor returns as user events.

The mapping stage may include explicitly architecture-scoped RISC-V64
entry anchors. For that scope it must be able to resolve
arch/riscv/kernel/head.S symbols declared through SYM_CODE_START /
SYM_CODE_END and ordinary assembly labels such as
relocate_enable_mmu, and it may pair them with
arch/riscv/mm/init.c::setup_vm() anchors. EntryPreludePhase.Started
maps to head.S::_start when present; EntryPreludePhase.Ready maps
to the _start_kernel tail start_kernel handoff when present. Early
VM, FDT/fixmap, kernel-image and trap-stream checkpoints must use
exact anchors only when a single Linux boundary is found; otherwise
they must use ordered ranges or remain unmapped.

RISC-V64 entry mappings are not portable Linux init/main.c anchors.
The emitted notes must keep this architecture scope visible and must
not claim cross-architecture equivalence for head.S/setup_vm()
boundaries.

The output surface remains tools/out/checkpoints/ with
machine-readable JSON and human-readable Markdown. Each JSON row
must contain checkpoint_index, checkpoint_name, checkpoint_variant,
linux_file, linux_symbol, linux_anchor, mapping_kind, confidence
and notes.

This stage must not modify any Linux source tree, add
instrumentation, change arceos_ex runtime behavior, add checkpoint
handlers, collect memory/runtime payloads, or treat the candidate
mapping as proof that a Linux insertion point has been implemented.

The mapping tool must also support a read-only regeneration check:
it regenerates JSON and Markdown in memory from the tracked
inventory and reference Linux tree, compares them with the tracked
mapping artifacts, and reports drift as failure without rewriting
repository outputs.

#### Linux checkpoint mapping coverage review

A third Linux-differential checkpoint artifact may summarize the
tracked Linux checkpoint mapping as a compact coverage review. This
stage is mapping-only: it consumes
tools/out/checkpoints/linux_checkpoint_mapping.json and must not
read or mutate a Linux tree, alter mapping_kind/classification
semantics, add instrumentation, collect runtime data, or change
checkpoint handlers.

The output surface remains tools/out/checkpoints/ with
machine-readable JSON and human-readable Markdown. The JSON must
contain only aggregate review data: total checkpoint count, mapping
kind counts, confidence counts, mapped Linux file counts, unmapped
checkpoint family counts and the singleton unmapped family count.
It must not copy the full per-checkpoint mapping rows or include
timestamps. The Markdown should keep the same compact view and
list only unmapped families with count >= 2, while summarizing the
number of singleton unmapped families.

The coverage tool must also support a read-only regeneration check:
it regenerates JSON and Markdown in memory from the tracked mapping
artifact, compares them with the tracked coverage artifacts, and
reports drift as failure without rewriting repository outputs.

Paired checkpoint difftest coverage:

paired_checkpoint_diff checkpoint_scope is a case-local hard
comparison scope. It is not a declaration that every latest exact
Linux mapping is actively compared by that case. A case that enables
[paired.checkpoint_coverage] must consume the tracked
tools/out/checkpoints/linux_checkpoint_mapping.json, filter to
required_mapping_kinds, and require every required checkpoint to be
either present in checkpoint_scope or explicitly listed in
accounted_outside_scope with a stable reason string. Missing
accounting is a configuration failure before dry-run or real QEMU
execution. Manifest, summary and paired diff/report outputs must
carry required_total, in_scope, accounted_outside_scope and
unaccounted counts so the default rc.local difftest cannot be
misreported as full exact-mapping agreement.

#### Linux checkpoint marker patch generation

Marker patch generation is an explicit synchronization action. It
must consume the tracked
tools/out/checkpoints/linux_checkpoint_instrumentation_plan.json
artifact and a reference Linux tree, then write only a caller-named
unified diff. It must not directly mutate the reference Linux tree
or maintain an independent checkpoint list outside the plan.

The patch may insert only markers present in the plan. Markers are
inserted immediately before their anchor line and inherit that line's
indentation. When multiple planned markers share an anchor, their
generated insertion order must be checkpoint_index order. An
already-present identical marker must not be duplicated.

Patch generation must fail before writing the patch if the Linux
tree contains a marker with the same checkpoint_name +
checkpoint_variant but a different fingerprint, or if it contains
a stale marker whose identity is absent from the current plan.
The explicit marker check mode must report missing marker, stale
marker and fingerprint mismatch counts as a summary. It must remain
outside the default test-checkpoints gate while the reference Linux
tree is not a controlled repository artifact.

#### Observation levels and domains

The implementation must distinguish default, light, failure-only,
probe-heavy and stress/nightly observation levels. Observation
domains must be stable subsystem or object scopes such as PLIC,
IRQ-domain, UART8250/TTY, virtio-blk/block, VFS/ext2,
scheduler/task, payload and phase boundaries. Long-term facts
belong to objects/providers; handlers only consume them.

#### Failure diagnostic lifecycle

failure_diagnostic is collected on a failing predicate/check path,
attached to EventError, propagated, and emitted by the final error
reporter. It is not a checkpoint handler and must not change the
successful checkpoint sequence.

#### Sink-only writes

A KUnit sink may record, audit or emit diagnostics, but it must not
expose access to Context, lifecycle state, IRQ state, device state
or scheduler state. Adding a new writable sink requires an explicit
coding/spec contract.

#### Smoke separation

MUST: app smoke cases remain under the smoke app/harness, not under
checkpoint KUnit handlers. Mutating object API tests that are useful
should be modeled as app smoke or explicit action-level probes, not
by re-registering smoke as a checkpoint handler.

#### UART IRQ chain KUnit boundary

The checkpoint KUnit for the first UART external interrupt chain is
an observer. It may read trace points, counters and object facts
from read-only Context, and may write only to its KUnit sink. It
must not call the UART handler, PLIC claim/complete, root intc
entry, request_irq, source-enable APIs, or mutate pending/claimed
state to manufacture progress.

#### Flow ownership

The interrupt flow must be advanced by real implementation paths:
UART interrupt emission, hart external interrupt entry, root intc
dispatch, PLIC claim, IRQ core dispatch, UART handler and PLIC
complete. KUnit can only assert before/after snapshots of those
facts.

#### Concurrency scope

IrqTimeInitPhase opens only the boot CPU local interrupt gate.
Task concurrency and SMP concurrency remain closed when this phase
reaches Ready.

#### Runtime services

Opening the boot CPU interrupt gate must not implicitly advance
periodic tick service, full softirq execution, IPI enable, workqueue
workers, RCU GP kthreads or secondary CPU execution to Online.

#### Smoke actions

RiscvTimerProvider.setup() must expose enough action surface for two
smoke checks after InterruptStream.enable(): a monotonic time read
check and a one-shot clockevent callback check through the timer IRQ
route. These checks do not imply full periodic tick service.
