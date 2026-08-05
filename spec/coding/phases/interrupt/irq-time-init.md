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

IrqTimeInitPhase is a direct BootInitFlow.Setup phase. Its formal model
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
RiscvIntc/InterruptType external route is installed. The PLIC
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

Checkpoint inventory, Linux static alignment, coverage review,
marker patch generation and paired difftest rules are supporting
cross-reference/testing responsibilities defined by
[`checkpoint-cross-reference.md`](../../../testing/checkpoint-cross-reference.md),
not Model-to-code constraints of this phase.

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
smoke checks after InterruptType.enable(): a monotonic time read
check and a one-shot clockevent callback check through the timer IRQ
route. The later SMP runtime phase extends this same provider into a per-CPU
deadline mux and adds a 10 ms scheduler deadline without changing the CPU0
one-shot callback contract; no second hardware timer owner may be introduced here.
