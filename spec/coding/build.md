# Build and Script Coding Guidance

This document explains the formal rules in [`build.spec`](build.spec). It covers Makefile targets and helper scripts used by object-level implementation, generated artifacts, disk images, QEMU runs and validation.

## Scope

Build files and scripts are part of the object-coding contract when they decide which model is verified, which generated artifacts are consumed, which payload is selected, which runtime inputs QEMU receives, or which validation gates count as acceptance.

They do not define model semantics. If a build target needs a new kernel object, payload, disk fixture or observable fact, the object/model/coding specification must be updated first.

## Entry Points

The repository top-level `Makefile` is the stable entry for normal work:

```bash
make build
make checkpoints
make fmt
make fmt-check
make run
make verify
make test
make test-verify
make test-checkpoints
make checkpoints-linux-check
make test-kunit
make test-smoke
make test-stress
make difftest
make clean
```

Kernel-specific command bodies belong under the selected kernel directory, currently `impl/arceos_ex/Makefile`. The top-level file should pass explicit parameters such as `KERNEL`, `APP`, `LOG`, `SPEC`, `PROBE`, `PROBE_FILE`, `KUNIT_HANDLERS`, `KUNIT_APP` and `SMOKE_APP`; it should not duplicate the kernel-specific Rust, linker, disk or QEMU command lines.

`make clean` is the repository-level cleanup entry point. It must delegate kernel-specific cleanup to the selected kernel directory and may also remove routine repository-level build, code-generation and test-cache artifacts such as `tools/build/`, non-checkpoint `tools/out/` contents, Python bytecode files, Python `__pycache__/` directories and common Python tool caches. Stress, difftest and focused diagnostic reports under the managed `impl/arceos_ex/tests/stress/out/` root are routine test artifacts; cleanup must preserve only that directory's tracked `.gitignore`, and developers must move any report that needs long-term retention elsewhere before cleanup. It must not remove tracked checkpoint review artifacts under `tools/out/checkpoints/`, user-local environments such as `.venv/` or `venv/`, ordinary diagnostic logs outside that managed report root, editor state or other unlisted local files.

## Target Boundaries

Targets must remain composable:

- `generate` creates generated source inputs such as linker scripts from model/codegen inputs.
- `build` compiles the kernel image and must not mutate runtime disk images.
- `disk` builds block-device images from documented rootfs inputs.
- `run` prepares required runtime inputs, builds the selected kernel/payload and starts QEMU.
- `verify` runs formal derivation or trace generation.
- `test-verify`, `test-kunit` and `test-smoke` are independently runnable validation stages.
- `checkpoints` regenerates tracked checkpoint review artifacts in dependency order: inventory, Linux mapping, Linux mapping coverage and the Linux instrumentation plan.
- `test-checkpoints` validates those tracked checkpoint review artifacts in read-only check mode and must not rewrite them.
- `checkpoints-linux-check` validates the tracked instrumentation plan and the sibling Linux marker names, variants and fingerprints in read-only mode. It must report missing, stale and mismatched markers and must not regenerate artifacts, emit a patch or modify the Linux tree.
- `difftest-preflight` must run `test-checkpoints` before `checkpoints-linux-check`; `difftest` must not start its paired runner unless both read-only checks pass. A preflight failure must retain the detailed drift diagnostics and direct the developer to the explicit manual synchronization workflow instead of invoking `checkpoints` automatically.
- `fmt` formats every Rust source file under the selected kernel's `src/` tree with the pinned kernel toolchain, edition and explicit non-recursive-per-file configuration.
- `fmt-check` applies the exact same source set and rustfmt configuration in read-only `--check` mode.
- `test` must run `fmt-check` before formal verification, checkpoint checks, builds or runtime stages, then preserve the remaining validation order and individual entry points.
- `clean` removes generated build and cache artifacts, including all reports below the managed stress output root except its tracked `.gitignore`, while preserving tracked checkpoint review artifacts and user-local state that is not part of routine build cleanup.

A helper script may improve reporting, for example by aggregating test summaries, but it must not make a hidden validation stage impossible to rerun directly.

Checkpoint synchronization is an explicit reviewed workflow: change the mapping or semantic specification, review and update the sibling Linux instrumentation, run `make checkpoints`, then run `make checkpoints-linux-check` before `make difftest`. The read-only `make difftest` entry point must never rewrite source, regenerate tracked checkpoint artifacts or emit/apply a Linux patch.

## Disk Images

`make disk` is the canonical way to build runtime block-device inputs. It must be reproducible from explicit variables such as:

- `VIRTIO_BLK_IMAGE`
- `VIRTIO_BLK_IMAGE_SIZE`
- `FS_TYPE`
- `EXT2_BLOCK_SIZE`
- `ROOTFS_URL`
- `ROOTFS_TARBALL`
- `ROOTFS_STAGING_DIR`
- tool variables such as `WGET`, `TAR` and `MKFS_EXT2`

By default, `make disk` should create the configured `VIRTIO_BLK_IMAGE` only when that image does not exist. Existing disk images are local runtime state and must not be reformatted by routine `run` or `test` entry points. Regenerating the image requires an explicit clean/delete/rebuild action.

For ext2 rootfs images, the default source is the Alpine minirootfs tarball identified by `ROOTFS_URL`. The tarball must be cached under the kernel build directory through `ROOTFS_TARBALL`, for example `build/rootfs-cache/alpine-minirootfs-3.24.1-riscv64.tar.gz`. If the cached tarball exists, `make disk` must reuse it instead of downloading it again. The extracted staging tree belongs under `ROOTFS_STAGING_DIR` and is generated runtime input, not source.

When a QEMU configuration includes `virtio-blk`, `make run` must depend on `disk`. `make build` should not create or reformat disk images.

## Payload Selection

Payload selection must stay explicit. `APP=smoke`, `APP=hello` or a future `APP=user-hello` must flow through Make variables into compile-time cfg or an equivalent explicit selection mechanism.

Build scripts must not infer the selected payload from a previous run, a local disk image, or an environment side effect.

## QEMU Devices

QEMU devices should be data-driven through variables such as `QEMU_DEVICES`, `QEMU_APPEND`, `QEMU_SMP` and `VIRTIO_BLK_IMAGE`.

The default may target QEMU virt and include the devices needed by the current acceptance path, such as `virtio-rng-device` and `virtio-blk-device`. Tests that need a smaller device set should override `QEMU_DEVICES=` or a documented variable rather than editing the Makefile.

## Model and Codegen

Build targets must keep model verification and generated-artifact boundaries visible. In particular:

- Generated linker scripts must be produced from model/codegen inputs, not manually substituted by stale local copies.
- Verification failures must fail the target that depends on them.
- Generated outputs should go under `tools/out/`, `build/` or another documented artifact directory.
- A target that consumes generated artifacts must express the relevant dependencies in Makefile form where practical.

## External Tools

External tools must be configurable by variables or documented script parameters. Current examples include:

- `RUSTC`
- `RUSTFMT`
- `RUST_OBJCOPY`
- `RUST_TOOLCHAIN`
- `QEMU`
- `PYVERI`
- `WGET`
- `TAR`
- `MKFS_EXT2`

Ordinary build targets must not depend on host-local absolute paths unless the path is a documented source input, such as a third-party object intentionally stored under the repository.

## Failure Behavior

Unknown configuration must fail fast. Examples:

- unsupported `FS_TYPE`
- unsupported provider name
- unsupported payload name
- missing required external tool

Silent fallback to a nearby default is not acceptable when it changes the object path, payload, disk contents, provider or validation scope.

<!-- formal-predicate-notes:spec/coding/build.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/build.spec` 的长注释迁移而来。`*.spec` 文件只保留 rule ID、type 分组、MUST/SHOULD/MAY/NOTE 层级和最短标签；解释、背景、参考路径、阶段性取舍与例子在这里维护。

### BuildAndScriptCodingMust

#### Stable entry

The repository top-level Makefile is the stable developer and CI
entry for object-level implementation. New routine commands should
be reachable from it or intentionally documented as lower-level
implementation details.

#### Delegation

Kernel-specific build, image, disk and QEMU rules belong to the
selected kernel implementation directory, for example
impl/arceos_ex/Makefile. The top-level Makefile should delegate with
explicit parameters instead of duplicating kernel-specific command
bodies.

#### Composable targets

Targets such as build, generate, disk, run, verify, test-verify,
test-kunit, test-smoke and test must remain separately callable.
An aggregate target may sequence them, but it must not hide a step
so that developers cannot rerun or diagnose it independently.

#### Disk image input

make disk is the canonical builder for runtime block-device images
used by QEMU. It must be reproducible from explicit Make variables
such as image path, size, file-system type, file-system block size
and the rootfs source URL/cache path. Kernel runtime code must not
depend on manually prepared local disk state.

#### Idempotent disk creation

The default make disk behavior must create the configured disk image
only when the image path is missing. Existing runtime disk images
are local runtime state and must not be reformatted by default; an
explicit clean/delete/rebuild step is required to regenerate them.

#### Downloaded rootfs cache

Downloaded rootfs inputs such as Alpine minirootfs tarballs must be
cached under a build artifact directory controlled by Make
variables. If the cached tarball exists, routine disk creation must
not download it again.

#### Runtime dependencies

make run must depend on runtime inputs it needs, including the disk
image when QEMU devices include virtio-blk. make build must not
create or mutate runtime disk images unless the target explicitly
requires it.

#### Visible model/codegen boundary

Makefiles and helper scripts must keep model derivation, generated
artifact creation and kernel compilation as visible target edges.
They must not silently bypass pyveri/codegen outputs, substitute
stale generated files, or turn verification failures into warnings.

#### Payload selection

Selected payload or test app must remain an explicit build parameter
such as APP. Scripts must not infer a different payload from local
files, previous runs or environment side effects.

#### External tool commands

External tools such as rustc, rust-objcopy, QEMU, wget, tar, mkfs
and pyveri must be configurable through Make variables or
documented script parameters. Hard-coded host-local absolute paths
are not allowed in ordinary build targets.

#### Pinned Rust source formatting

The selected kernel implementation owns the complete Rust source set under its `src/` tree. `fmt` and
`fmt-check` must use the same pinned nightly as the default kernel compiler, Rust edition 2024 and an
explicit `skip_children=true` configuration while passing every source file once. This avoids recursive
module traversal changing files outside the enumerated set and makes formatting independent of the host
default toolchain.

#### Early Rust format gate

The aggregate `make test` target must depend on the independently runnable, read-only `fmt-check` target
before starting formal verification, checkpoint artifact validation, compilation or QEMU. Format drift
must fail fast and must never be repaired implicitly by `make test`.

#### Decomposable tests

The aggregate make test target must preserve independently runnable
verify, KUnit/checkpoint and smoke stages. Adding a new validation
stage requires documenting its ordering, inputs and whether it is
part of the default acceptance gate.

#### Checkpoint artifact drift gate

The aggregate make test target must run checkpoint inventory,
Linux mapping, Linux mapping coverage and Linux instrumentation
plan artifact checks before QEMU/runtime stages. The gate must be
independently callable, must report drift as a test failure with
retained logs, and must not rewrite tracked checkpoint output files
while running in test mode. Linux marker scans are useful for trees
that already carry marker comments, but they must remain an explicit
opt-in check until the referenced Linux tree is instrumented.

#### Generated output hygiene

Generated files, runtime disk images, QEMU logs, temporary debugfs
scripts and trace reports must stay in build/tools/out/tmp-style
locations or documented artifact directories. They must not be
committed unless a specification explicitly classifies them as
stable source inputs.

#### Clean scope

The repository top-level make clean target must remove routine
build, code-generation and test-cache artifacts created under the
repository, including successful, failed, difftest and focused
diagnostic reports below impl/arceos_ex/tests/stress/out/. It must
preserve that managed output directory's tracked .gitignore,
tracked checkpoint review artifacts under tools/out/checkpoints/,
user-local environments, ordinary diagnostic logs outside the
managed stress output root, editor state and other unlisted local
files. It must keep the selected kernel implementation clean as
the owner of kernel-specific build artifacts.

### BuildAndScriptCodingShould

#### Explicit image and file-system knobs

Disk-image paths, sizes, FS_TYPE, ext2 block size and deterministic
fixture file knobs should have explicit variable names. Avoid
embedding these decisions in opaque shell fragments.

#### Data-driven QEMU devices

QEMU devices should be composed from variables such as QEMU_DEVICES
and disk-image paths. The default may target QEMU virt, but the rule
should allow tests to remove or replace devices explicitly.

#### Unknown configuration

Unknown provider names, file-system types, payload names or feature
values should fail fast at build time rather than falling back to a
nearby default.

#### Target documentation

New Makefile targets or helper scripts should be documented in this
coding directory before they become part of the normal workflow.

<!-- formal-predicate-notes:spec/coding/build.spec END -->
