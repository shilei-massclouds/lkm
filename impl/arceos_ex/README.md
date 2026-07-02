# arceos_ex Object Implementation

This directory is a local object-level kernel implementation experiment driven
by `spec/model` and `spec/coding`. It intentionally avoids the ArceOS workspace,
xtask, Cargo feature forwarding, `axlog`, `ax-alloc`, and external crates.

Current goals:

- build a standalone RISC-V64 no-alloc kernel image;
- keep phases as process orchestration;
- give non-phase model objects explicit Rust carriers;
- expose kernel-local Makefile targets used by the repository root Makefile.

Useful commands:

```bash
make build
make build APP=smoke
make build APP=hello
make run
make run APP=smoke
make run APP=hello
make run PROBE=announce
make verify
make verify REPORT=graph
make clean
```

`LOG=trace` remains as a compatibility alias for `PROBE=announce`. Prefer
`PROBE=announce` for new checkpoint announcement runs.

From the repository root, `make test` runs the full validation path: strict
formal derive, checkpoint/KUnit handlers listed in `tests/kunit.handlers`, and
the final `APP=smoke` payload smoke run. Use root `make test-kunit` or
`make test-smoke` when only one runtime path is needed.

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
rustup target add riscv64gc-unknown-none-elf
```

`qemu-system-riscv64` and `rust-objcopy` must also be available on `PATH`.
The Makefile defaults to `rustc +nightly-2025-05-20` because that toolchain is
known to have the local RISC-V64 target installed in the current environment.
Override `RUSTC=...` if a different toolchain is prepared.  `APP ?= smoke`
selects the built-in smoke payload.  The Makefile maps app names to
`--cfg app_<name>`; additional payloads should live under `src/apps/` and expose
`run() -> !`.

The current code covers the minimal `EntryPreludePhase`, `EntrySuccessorPhase`,
`CorePreparePhase`, and final `PayloadPhase` handoff.  The default `smoke`
payload runs smoke cases under `src/apps/smoke/cases/` and then shuts down.
`APP=hello` remains available as the minimal standalone payload.
