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
QEMU_APPEND ?= earlycon=sbi
PYVERI ?= $(ROOT)/tools/pyveri/bin/pyveri
CODEGEN_PYTHONPATH := $(ROOT)/tools/common/src:$(ROOT)/tools/codegen/src
MODEL_JSON := $(ROOT)/tools/out/codegen/main.model.json
MODEL_SPECS := $(shell find $(ROOT)/spec/model -name '*.spec')
CODEGEN_SOURCES := $(shell find $(ROOT)/tools/codegen/src $(ROOT)/tools/common/src -name '*.py')
EXTRA_RUSTFLAGS ?=

RUSTFLAGS := \
	-C opt-level=2 \
	-C panic=abort \
	-C relocation-model=static \
	-C code-model=medium \
	-C link-arg=-T$(LINKER) \
	-C link-arg=-static \
	$(EXTRA_RUSTFLAGS)

.PHONY: build run generate clean

build: generate $(KERNEL_BIN)

generate: $(LINKER)

$(LINKER): $(MODEL_SPECS) $(CODEGEN_SOURCES) Makefile
	mkdir -p $(ROOT)/tools/out/codegen
	$(PYVERI) $(ROOT)/spec/model/main.spec --derive --work-dir $(ROOT)/tools/out/codegen -o /dev/null
	PYTHONPATH=$(CODEGEN_PYTHONPATH) python -m codegen_tool linker-script $(MODEL_JSON) -o $(LINKER)

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
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp 1 -nographic -serial mon:stdio -append "$(QEMU_APPEND)" -kernel build/$(TARGET)/trace/arceos_ex.bin
else
	$(MAKE) build
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp 1 -nographic -serial mon:stdio -append "$(QEMU_APPEND)" -kernel $(KERNEL_BIN)
endif

clean:
	rm -rf build
