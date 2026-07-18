# arceos_ex Object Implementation

This directory is a local object-level kernel implementation experiment driven
by `spec/model` and `spec/coding`. It intentionally avoids the ArceOS workspace,
xtask, Cargo feature forwarding, `axlog`, `ax-alloc`, and external crates.

Current goals:

- build a standalone RISC-V64 no-alloc kernel image;
- keep phases as process orchestration;
- give non-phase model objects explicit Rust carriers;
- expose kernel-local Makefile targets used by the repository root Makefile.

Stable commands are invoked from the repository root:

```bash
make build
make build TEST=kernel-smoke-native
make fmt
make fmt-check
make clippy-check
make run
make run TEST=user-smoke-native
make run TEST=distro-sh-native
make disk
make verify
make verify REPORT=graph
make clean
```

Basic tests are single TOML configurations below `tests/basic/cases`. Their
runner owns config validation, kernel build, disk policy, scripts, QEMU,
expectations, structured results and cleanup. `make disk` builds the reusable
canonical Alpine image with repository fixtures under `/opt/lkm/tests` and the
sibling LTP staging under `/opt/ltp`. The automated dual-provider LTP entries are:

```sh
make run TEST=ltp
make run TEST=ltp-lo
```

Each entry first lists exactly `uname01`, `uname02` and `uname04`, then executes
the same set and requires all three to pass.

Use another unpacked LTP staging tree with:

```bash
make disk ROOTFS_LTP_DIR=/path/to/rootfs FORCE=1
```

The canonical input fingerprint automatically rebuilds after LTP, fixture,
tarball or builder changes; `FORCE=1` is an explicit rebuild. A missing staging
directory or missing/non-executable `opt/ltp/run-syscalls.sh` is an error.

From the repository root, `make test` first runs the read-only Rust format gate,
then the full `-D warnings -D clippy::all` gate, followed by the full validation
path: strict formal derive, checkpoint/KUnit handlers listed in
`tests/kunit.handlers`, and the final `APP=smoke` payload smoke run. The Clippy
gate checks smoke, ordinary hello, handler-enabled hello and user-boot with both
native and Linux-object PLIC providers; clean builds must likewise emit no Rust
warnings. Use root `make fmt-check`,
`make clippy-check`, `make test-kunit` or `make test-smoke` when only one
validation path is needed. `make fmt` uses the same pinned stable toolchain and
edition as the format gate to normalize every Rust source below `src/`.

Stress tests are run from the repository root with the dedicated runner:

```bash
impl/arceos_ex/tests/stress/runner.py --runs 30
```

When no case is specified, the runner executes the standard stress suite. Use
`--runs 0` for a dry configuration check without starting QEMU, or pass a case
file such as `impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml` to run
only that case. Detailed output layout and case policy are documented in
`impl/arceos_ex/tests/stress/README.md`.

Prerequisites:

```bash
rustup toolchain install
```

`qemu-system-riscv64` must also be available on `PATH`. The Makefile resolves
`rust-objcopy` from the active compiler sysroot installed by
`llvm-tools-preview`; it does not require a separate `rust-objcopy` proxy on
`PATH`.
The repository-root `rust-toolchain.toml` is the single source of truth for the
Rust 1.97.0 toolchain, RISC-V64 target, rustfmt, Clippy and llvm-tools-preview.
The Makefile uses that active rustup toolchain without repeating its version;
override an individual tool command only when an alternate tool is deliberately
prepared. `APP ?= smoke` selects the built-in smoke payload.  The Makefile maps app names to
`--cfg app_<name>`; additional payloads should live under `src/apps/` and expose
`run() -> !`.

The current code covers the minimal `EntryPreludePhase`, `EntrySuccessorPhase`,
`CorePreparePhase`, and final `PayloadPhase` handoff.  The default `smoke`
payload runs smoke cases under `src/apps/smoke/cases/` and then shuts down.
`APP=hello` remains available as the minimal standalone payload.
