TARGET := riscv64gc-unknown-none-elf
LOG ?= info
PROFILE ?= release
BUILD_DIR := build/$(TARGET)/$(PROFILE)
KERNEL_ELF := $(BUILD_DIR)/arceos_ex.elf
KERNEL_BIN := $(BUILD_DIR)/arceos_ex.bin
LINKER := linker/riscv64.lds
SOURCES := $(shell find src -name '*.rs')
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

.PHONY: build run clean

build: $(KERNEL_BIN)

$(KERNEL_ELF): $(SOURCES) $(LINKER)
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

run:
ifeq ($(LOG),trace)
	$(MAKE) PROFILE=trace EXTRA_RUSTFLAGS="--cfg checkpoint_sbi_char" build
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp 1 -nographic -serial mon:stdio -kernel build/$(TARGET)/trace/arceos_ex.elf
else
	$(MAKE) build
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp 1 -nographic -serial mon:stdio -kernel $(KERNEL_ELF)
endif

clean:
	rm -rf build
