TARGET := riscv64gc-unknown-none-elf
PROFILE ?= release
BUILD_DIR := build/$(TARGET)/$(PROFILE)
KERNEL_ELF := $(BUILD_DIR)/arceos_ex.elf
KERNEL_BIN := $(BUILD_DIR)/arceos_ex.bin
LINKER := linker/riscv64.lds
ROOT := ../..
RUSTC ?= rustc +nightly-2025-05-20
RUST_OBJCOPY ?= rustup run nightly-2025-05-20 rust-objcopy
QEMU ?= qemu-system-riscv64
PYVERI ?= $(ROOT)/tools/pyveri/bin/pyveri
EXTRA_RUSTFLAGS ?=

RUSTFLAGS := \
	-C opt-level=2 \
	-C panic=abort \
	-C relocation-model=static \
	-C code-model=medium \
	-C link-arg=-T$(LINKER) \
	-C link-arg=-static \
	$(EXTRA_RUSTFLAGS)

.PHONY: build run trace check-spec clean

build: $(KERNEL_BIN)

$(KERNEL_ELF): src/main.rs $(LINKER)
	mkdir -p $(BUILD_DIR)
	$(RUSTC) src/main.rs \
		--crate-name arceos_ex \
		--crate-type bin \
		--edition 2024 \
		--target $(TARGET) \
		$(RUSTFLAGS) \
		-o $@

$(KERNEL_BIN): $(KERNEL_ELF)
	$(RUST_OBJCOPY) --strip-all -O binary $< $@

run: build
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp 1 -nographic -serial mon:stdio -kernel $(KERNEL_ELF)

trace:
	$(MAKE) PROFILE=trace EXTRA_RUSTFLAGS="--cfg checkpoint_sbi_char" build
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp 1 -nographic -serial mon:stdio -kernel build/$(TARGET)/trace/arceos_ex.elf

check-spec:
	$(PYVERI) $(ROOT)/spec/model/startup-timeline.spec --derive --strict

clean:
	rm -rf build
