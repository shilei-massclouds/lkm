# Build and Script Coding Guidance

This document explains the formal rules in [`build.spec`](build.spec). It covers Makefile targets and helper scripts used by object-level implementation, generated artifacts, disk images, QEMU runs and validation.

## Scope

Build files and scripts are part of the object-coding contract when they decide which model is verified, which generated artifacts are consumed, which payload is selected, which runtime inputs QEMU receives, or which validation gates count as acceptance.

They do not define model semantics. If a build target needs a new kernel object, payload, disk fixture or observable fact, the object/model/coding specification must be updated first.

## Entry Points

The repository top-level `Makefile` is the stable entry for normal work:

```bash
make build
make run
make verify
make test
make test-verify
make test-kunit
make test-smoke
make clean
```

Kernel-specific command bodies belong under the selected kernel directory, currently `impl/arceos_ex/Makefile`. The top-level file should pass explicit parameters such as `KERNEL`, `APP`, `LOG`, `SPEC`, `PROBE`, `PROBE_FILE`, `KUNIT_HANDLERS`, `KUNIT_APP` and `SMOKE_APP`; it should not duplicate the kernel-specific Rust, linker, disk or QEMU command lines.

## Target Boundaries

Targets must remain composable:

- `generate` creates generated source inputs such as linker scripts from model/codegen inputs.
- `build` compiles the kernel image and must not mutate runtime disk images.
- `disk` builds block-device images from documented rootfs inputs.
- `run` prepares required runtime inputs, builds the selected kernel/payload and starts QEMU.
- `verify` runs formal derivation or trace generation.
- `test-verify`, `test-kunit` and `test-smoke` are independently runnable validation stages.
- `test` may aggregate validation stages, but must preserve their order and individual entry points.

A helper script may improve reporting, for example by aggregating test summaries, but it must not make a hidden validation stage impossible to rerun directly.

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
- `RUST_OBJCOPY`
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
