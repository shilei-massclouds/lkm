# arceos_ex Object Implementation

This directory is a local object-level kernel implementation experiment driven
by `spec/model` and `spec/coding`. It intentionally avoids the ArceOS workspace,
xtask, Cargo feature forwarding, `axlog`, `ax-alloc`, and external crates.

Current goals:

- build a standalone RISC-V64 no-alloc kernel image;
- keep phases as process orchestration;
- give non-phase model objects explicit Rust carriers;
- expose Makefile targets for build, run, trace, and spec checks.

Useful commands:

```bash
make -C impl/arceos_ex build
make -C impl/arceos_ex run
make -C impl/arceos_ex trace
make -C impl/arceos_ex check-spec
make -C impl/arceos_ex clean
```

Prerequisites:

```bash
rustup target add riscv64gc-unknown-none-elf
```

`qemu-system-riscv64` and `rust-objcopy` must also be available on `PATH`.
The Makefile defaults to `rustc +nightly-2025-05-20` because that toolchain is
known to have the local RISC-V64 target installed in the current environment.
Override `RUSTC=...` if a different toolchain is prepared.

The current code is only the first boot skeleton. It proves the local build and
SBI output path before the full EntryPrelude/EntrySuccessor object model is
implemented.
