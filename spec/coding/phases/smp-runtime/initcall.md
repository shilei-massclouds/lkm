# InitcallPhase coding

InitcallPhase 是 SmpRuntimePhase 的第 4 个直接子阶段，由 KernelInitTask 执行。model 路径为
`spec/model/phases/smp-runtime/initcall/`，实现落点为
`impl/arceos_ex/src/phases/smp_runtime/initcall.rs`。

## 生命周期映射

`preset()` 依赖 RuntimeCorePhase 精确 Online，接受 Preset 后发出 Started，按 model 顺序驱动
全部对象并提交 Prepared。现有 structured diagnostic 必须继续标识首个失败对象/谓词；生命周期
source/target 调整为 Base/Prepared。Setup/Enable 不再驱动对象，只复用 diagnostic ready check，
分别提交 Ready/Online；Online 只返回 `smp_runtime::enable_after_initcall()`。

四个 checkpoint 依次是 Started、Prepared、Ready、Online；既有 Started/Ready ID 和 stress scope
名称保持不变。

#### Entry gate

InitcallPhase must run after RuntimeCorePhase.Online and preserve
the do_basic_setup() entry boundary.

#### Pre-do_initcalls classification

The cpuset_init_smp(), driver_init(), init_irq_proc() and
do_ctors() slice must be classified against the current
../linux-6.12/.config before auditing do_initcalls(). Disabled
CONFIG_CPUSETS/CONFIG_CGROUPS and CONFIG_CONSTRUCTORS paths are
trimmed; enabled driver-core/procfs paths must be deferred or
formal explicitly, not treated as no-op.

#### Deferred synchronization

driver_init() deferred paths must still record Linux-visible
synchronization responsibilities: devtmpfs req_lock/completion/
kthread, of_core_init() of_mutex, and bus_register()'s subsys
mutex/klist initialization. The boot-only context does not erase
those protocols.

#### Deferred heavy subsystems

DriverCore and IrqProcView must remain explicit deferred
boundaries in this step.

#### Constructors

CtorTable must preserve the do_ctors() table position and record
the current trimmed/empty constructor table status.

#### Entry ABI

InitcallEntryPrototype must lower to a retained static function
pointer with the ABI fn(ContextRef) -> InitcallReturn. ContextRef is
the object graph entry; in the current Rust target it maps to
&mut crate::context::Context. InitcallReturn records the per-entry
Linux-like outcome and must be captured by InitcallTable.setup().
The entry must not lower to a captured closure, heap object or
runtime-dispatched callback that carries hidden payload arguments.

#### Static registration

InitcallTable.Register(level, entry) is abstract in the model, but
this target must realize it as a Linux-like initcall declaration
macro, e.g. arch_initcall_sync!() or device_initcall!(). The macro
emits a retained InitcallEntry element into the section selected by
level. It must not lower to a runtime function call that pushes the
entry into a second registry.

#### Linker collection

The static sections must be retained by the linker and represented
in memory as contiguous arrays of InitcallEntry elements. LDS/KEEP
start/end symbols, or a build-generated equivalent with the same
observable table boundaries, define each level's array.

#### Preset collection

InitcallTable.preset() must validate the pre-linked static ranges,
level mapping and entry operation bindings. For the Linux-like
backend the range itself is the table view; preset must not copy
entries into a secondary registration table and must not invoke
entries.

#### Setup execution

InitcallTable.setup() must represent do_initcalls() by directly
iterating the InitcallEntry arrays in Linux level order. It must
record level count, all-level execution, command-line scratch reuse,
parameter parsing, filtering and run-context checks without
promoting every entry to a top-level object.

#### Dispatcher shape

InitcallTable.setup() must stay Linux-like: the dispatcher iterates
static ranges in level order and invokes function pointers from the
descriptors. It must not branch on entry names, owners or concrete
operation identities; target effects belong to the entry wrappers
and owner objects.

#### do_one_initcall context repair

Per-entry records must preserve Linux do_one_initcall() observable
responsibilities: blacklist/filter check, trace start/finish
boundary, return recording, preempt-count snapshot with imbalance
repair-or-absent, disabled-IRQ repair-or-absent, and latent entropy
accounting. A boot-only context may simplify the implementation but
must not erase these facts.

#### Same-level order

The formal model fixes inter-level order only. Same-level order is
not a semantic guarantee unless an entry-effect commutativity proof
exists; until then this target records the proof gap and expects a
nightly permutation test to compare canonical final facts.

#### Mechanism/effect split

The initcall mechanism specification must stay separate from the
concrete side effects of individual entries such as
of_platform_default_populate_init(). Concrete targets are bound by
entry operation facts and may remain deferred until their owning
object model exists.

#### OF platform default populate source

of_platform_default_populate_init() belongs to PlatformBus and must
use the already Ready DeviceTree as its source object. The action is
a source-to-target populate boundary from DeviceTree to PlatformBus,
not an InitcallTable mechanism detail.

#### Linux-like traversal

The initial implementation must follow Linux 6.12
drivers/of/platform.c: of_platform_default_populate(NULL, ...)
resolves root to "/", then of_platform_populate() iterates the
root's direct children and calls of_platform_bus_create() with
strict=true.

#### Strict compatible

Candidate identification must require a compatible property for each
node considered by of_platform_bus_create(strict=true). Nodes without
compatible are skipped.

#### Default bus recursion

After identifying a compatible candidate, recursion into children
must occur only for nodes matching the Linux default bus match table:
simple-bus, simple-mfd, isa, and config-gated arm,amba-bus if the
target later enables that path.

#### Availability filter

Candidate identification must mirror of_device_is_available(): a node
is available when status is absent or is exactly "okay" or "ok".

#### Candidate printout

For this modeling step the action must print every identified
candidate's node name and compatible value so the traversal result is
inspectable from smoke/KUnit output.

#### Action checkpoint naming

Action checkpoints must use Entry for action entry, Exit for action
return boundary, and semantic names for necessary middle points.
Ambiguous names such as Called must not be used for postcondition
or middle checkpoints.

#### Device model naming

The formal DeviceType corresponds to Linux struct device, not Linux
struct device_type. Linux struct platform_device must be represented
as PlatformDeviceType embedding a core DeviceType member, not as a
subtype of the core device object itself.

#### Platform device core-member lookup

PlatformDeviceType coding must preserve a container_of-like
conversion from the embedded DeviceRef back to the owning platform
device, exposed through a Rust macro or equivalent typed helper such
as to_platform_device!().

#### DeviceObject category shell

DeviceObject is only an early object category label. Reusable
driver-core types such as DeviceType, BusType, and BusSubsysPrivate
must carry their own semantics directly instead of inheriting from
DeviceObject.

#### device_set_node boundary

DeviceType.SetNode must model Linux device_set_node()/dev.of_node by
binding a core device to a DeviceNodeRef. It must not copy the OF
compatible property into DeviceType or PlatformDeviceType; later
probe/match must reach compatible through the bound DeviceNodeRef.
The concrete Rust object must store a stable DeviceNodeId/node-index
or equivalent handle, not a long-lived borrowed DeviceNodeRef<'dt>.
That id must be resolved through the persistent DeviceTree whenever
name, compatible, status, or other OF properties are needed.

#### Platform device ownership

PlatformBus must own the PlatformDevice objects it creates during OF
population. The current arceos_ex backing must provide stable
platform-device storage, so DeviceRef entries in klist_devices never
outlive their containing PlatformDevice storage and are not
invalidated by container growth. A plain Vec<PlatformDevice> is not
sufficient if DeviceRef is a borrowed/raw reference to an embedded
Device member; use stable handles, arena-style ids, or non-moving
owned storage such as pinned/boxed platform devices. The platform
device must be inserted into owned storage before its DeviceRef is
published to klist_devices.

#### Bus device set storage

The model-level BusSubsysPrivate.klist_devices is a DeviceRefSet.
The first arceos_ex backing may use Vec<DeviceRef> as an append and
iterate view, not a small action-smoke slot array. A future
Linux-like intrusive-list backing must not expose the raw intrusive
list as the lifetime owner: raw nodes only express membership. It
must be wrapped by a SafeIntrusiveList-like abstraction that couples
stable object storage with the raw list, hides raw nodes from public
APIs, unlinks before drop, and returns stable DeviceRef views.

#### OF platform scan completion

The currently tested OF platform checkpoint must be named
OfPlatformDefaultPopulate.ScanComplete and must be emitted after
candidates have been identified and their name/compatible pairs have
been printed. KUnit coverage for candidate facts must attach to this
checkpoint, not to an Entry checkpoint.

#### OF platform device creation

After candidate scanning, of_platform_default_populate_init must
create PlatformDeviceType instances from candidate nodes, bind each
embedded DeviceType to its DeviceNodeRef, register the core device,
and add the resulting DeviceRef to PlatformBusSubsysPrivate's
DeviceRefSet through BusType.AddDevice. Driver match/probe/bind is
outside this OF-populate action and belongs to later driver
registration/probe initcalls.

#### Platform driver registration and probe

The first concrete platform-driver closure must be the real
ns16550a-compatible OF serial platform driver, modeled in
spec/model/objects/ns16550a_driver.spec and implemented in an
independent Rust module. Its Linux-like device_initcall!() static
entry represents the initcall action. Concrete platform drivers
must declare that initcall in the driver's own implementation
module, beside the driver descriptor/probe code; the generic
initcall core only provides the declaration macros, linker-section
table collection, and level-order execution. That action must call
platform_driver_register() after OF population has created platform
devices; platform_driver_register() then reaches BusType.AddDriver,
records a DeviceDriverRef in PlatformBus.klist_drivers, and drives
ProbeDriver when autoprobe is active. ProbeDevice remains the
symmetric path for devices added after drivers.

PlatformBus.klist_drivers stores DeviceDriverRef membership entries.
It may be backed by Vec<DeviceDriverRef> in the current target,
mirroring the earlier klist_devices compromise. The Vec owns only
copyable/stable refs, not driver objects.

DeviceDriverType.of_match_table is a static descriptor field, not a
runtime setter. The concrete target must represent each registered
driver with a stable descriptor carrying name, bus binding,
of_match_table and probe function. The OF match table should lower
to a static compatible array analogous to Linux
struct of_device_id[], and DeviceDriverRef/klist_drivers must
reference that descriptor rather than acting as a closed enum of
bus-specific special cases. The referenced driver must be owned by
static storage, pinned heap storage, or an arena/registry with stable
handles; a plain Vec<PlatformDriver> is not valid if refs can point
into elements that may move during growth.

PlatformBus.match() must be the common matching boundary: it reads
driver.of_match_table and resolves device.dev.of_node through the
persistent DeviceTree, then compares compatible strings. The
ns16550a driver may provide a static descriptor and probe function,
but the platform bus implementation must not hard-code an ns16550a
compatible branch as the only matching path.

Platform driver probe must receive a PlatformProbeContext-style
temporary window derived from Context for the single probe action.
The probe context is non-owning and must not be stored by drivers.
It must not be a public bag of Context fields: fields stay private
and driver code reaches subsystems only through narrow production
capability methods such as OF node lookup, platform-device MMIO
mapping, IRQ binding, and virtio device registration. Platform
probe functions must not receive raw &mut Context, and PlatformBus
public APIs must not expose virtio-specific or IRQ/MM allocator
argument lists.

#### Console/earlycon handoff

The next platform-driver increment must model Linux-like handoff
without jumping directly to a full UART backend. DeviceTree must
parse /chosen/stdout-path or linux,stdout-path, split optional
colon options, and resolve the selected node to a stable
DeviceNodeId. The ns16550a probe must then parse resources from the
matching PlatformDevice -> Device -> DeviceNodeId path, create a
minimal Uart8250Port object, and register a Serial8250Console only
when the probed device is the stdout-path device.

The console registry must expose register_console()-style policy:
real serial console registration switches the printk route and
unregisters BootConsole unless keep_bootcon is set. The first
implementation may record facts for the route switch before
replacing the actual sink with UART MMIO polling writes.

EarlyCon is the early SBI backend. BootConsole is the CON_BOOT
registry entry wrapping that backend. Serial8250Console is the real
console entry from the probed Uart8250Port. ConsoleRegistry owns the
route, handoff cursor transfer and keep_bootcon policy; drivers only
request registration and must not carry those global facts as
private state.

#### devfs first slice

DevFs must be an InitcallPhase object, not part of the initial
ProcessPreparePhase rootfs mount. It must mount /dev only after the
initial VFS rootfs exists and the hwrng/block registries plus their
live virtio drivers have published current/default device surfaces.
The first smoke validation must observe hwrng and block device nodes
and registry bindings only; it must not add test-only production APIs
and must not require reads through a VFS file path.
