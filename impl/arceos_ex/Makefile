TARGET := riscv64imac-unknown-none-elf
LOG ?= info
PROFILE ?= release
APP ?= smoke
comma := ,
empty :=
space := $(empty) $(empty)
PROBE ?=
PROBE_FILE ?=
PLIC_PROVIDER ?= native
KNOWN_PLIC_PROVIDERS := native linux-object
UNKNOWN_PLIC_PROVIDER := $(filter-out $(KNOWN_PLIC_PROVIDERS),$(PLIC_PROVIDER))
ifneq ($(UNKNOWN_PLIC_PROVIDER),)
$(error unknown PLIC_PROVIDER: $(PLIC_PROVIDER))
endif
ifneq ($(PROBE_FILE),)
ifeq ($(wildcard $(PROBE_FILE)),)
$(error PROBE_FILE not found: $(PROBE_FILE))
endif
endif
PROBE_FILE_LIST := $(if $(PROBE_FILE),$(strip $(shell sed -e 's/#.*//' "$(PROBE_FILE)")),)
PROBE_LIST := $(strip $(subst $(comma),$(space),$(PROBE)) $(PROBE_FILE_LIST))
KNOWN_PROBES := memblock page-allocator earlycon kernel-init-task of-platform scheduler-action console-handoff uart-irq-chain virtio-bus virtio-blk virtio-rng vmalloc-mapping linux-plic smoke
UNKNOWN_PROBES := $(filter-out $(KNOWN_PROBES),$(PROBE_LIST))
ifneq ($(UNKNOWN_PROBES),)
$(error unknown PROBE handler(s): $(UNKNOWN_PROBES))
endif
PROBE_CFGS := $(foreach probe,$(PROBE_LIST),--cfg checkpoint_handler_$(subst -,_,$(probe)))
PROBE_BUILD_LABEL := $(subst $(space),+,$(PROBE_LIST))
PROBE_BUILD_SUFFIX := $(if $(PROBE_LIST),/probe-$(subst /,_,$(subst .,_,$(PROBE_BUILD_LABEL))))
PLIC_PROVIDER_SUFFIX := $(if $(filter native,$(PLIC_PROVIDER)),,/plic-$(PLIC_PROVIDER))
BUILD_DIR := build/$(TARGET)/$(PROFILE)/$(APP)$(PLIC_PROVIDER_SUFFIX)$(PROBE_BUILD_SUFFIX)
KERNEL_ELF := $(BUILD_DIR)/arceos_ex.elf
KERNEL_BIN := $(BUILD_DIR)/arceos_ex.bin
TRACE_KERNEL_BIN := build/$(TARGET)/trace/$(APP)$(PLIC_PROVIDER_SUFFIX)$(PROBE_BUILD_SUFFIX)/arceos_ex.bin
LINKER := linker/riscv64.lds
SOURCES := $(shell find src -name '*.rs')
ROOT := ../..
LINUX_PLIC_OBJ := $(ROOT)/impl/third_party/linux/6.12.37/riscv64-lp64/objects/drivers/irqchip/irq-sifive-plic.o
RUSTC ?= rustc +nightly-2025-05-20
RUST_OBJCOPY ?= rustup run nightly-2025-05-20 rust-objcopy
QEMU ?= qemu-system-riscv64
QEMU_SMP ?= 8
QEMU_APPEND ?= earlycon=sbi
VIRTIO_BLK_IMAGE ?= build/virtio-blk.raw
VIRTIO_BLK_IMAGE_SIZE ?= 1M
QEMU_DEVICES ?= -object rng-random,id=rng0,filename=/dev/urandom -device virtio-rng-device,rng=rng0 -drive file=$(VIRTIO_BLK_IMAGE),if=none,format=raw,id=blk0 -device virtio-blk-device,drive=blk0
PYVERI ?= $(ROOT)/tools/pyveri/bin/pyveri
CODEGEN_PYTHONPATH := $(ROOT)/tools/common/src:$(ROOT)/tools/codegen/src
MODEL_JSON := $(ROOT)/tools/out/codegen/main.model.json
MODEL_SPECS := $(shell find $(ROOT)/spec/model -name '*.spec')
CODEGEN_SOURCES := $(shell find $(ROOT)/tools/codegen/src $(ROOT)/tools/common/src -name '*.py')
EXTRA_RUSTFLAGS ?=
APP_CFG := app_$(subst -,_,$(APP))
PLIC_PROVIDER_CFG := --cfg plic_provider_$(subst -,_,$(PLIC_PROVIDER))
PLIC_PROVIDER_LINK_ARGS := $(if $(filter linux-object,$(PLIC_PROVIDER)),-C link-arg=$(LINUX_PLIC_OBJ),)

RUSTFLAGS := \
	$(if $(filter hello,$(APP)),-A dead_code,) \
	-C opt-level=2 \
	-C panic=abort \
	-C relocation-model=static \
	-C code-model=medium \
	-C link-arg=-T$(LINKER) \
	-C link-arg=-static \
	--cfg $(APP_CFG) \
	$(PLIC_PROVIDER_CFG) \
	$(PROBE_CFGS) \
	$(PLIC_PROVIDER_LINK_ARGS) \
	$(EXTRA_RUSTFLAGS)

.PHONY: build run generate disk clean

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

disk:
	mkdir -p $(dir $(VIRTIO_BLK_IMAGE))
	truncate -s $(VIRTIO_BLK_IMAGE_SIZE) $(VIRTIO_BLK_IMAGE)

run: disk
ifeq ($(LOG),trace)
	$(MAKE) PROFILE=trace APP=$(APP) PROBE="$(PROBE)" PROBE_FILE="$(PROBE_FILE)" PLIC_PROVIDER=$(PLIC_PROVIDER) EXTRA_RUSTFLAGS="--cfg checkpoint_sbi_char" build
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp $(QEMU_SMP) -nographic -serial mon:stdio $(QEMU_DEVICES) -append "$(QEMU_APPEND)" -kernel $(TRACE_KERNEL_BIN)
else
	$(MAKE) build APP=$(APP) PROBE="$(PROBE)" PROBE_FILE="$(PROBE_FILE)" PLIC_PROVIDER=$(PLIC_PROVIDER)
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp $(QEMU_SMP) -nographic -serial mon:stdio $(QEMU_DEVICES) -append "$(QEMU_APPEND)" -kernel $(KERNEL_BIN)
endif

clean:
	rm -rf build
