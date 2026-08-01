TARGET := riscv64imac-unknown-none-elf
LOG ?= info
PROFILE ?= release
APP ?= smoke
ROOTFS ?= canonical
ROOT := ../..
include $(ROOT)/impl/providers/linux-6.12.mk
comma := ,
empty :=
space := $(empty) $(empty)
PROBE ?=
PROBE_FILE ?=
STRESS_MEM_BYTES ?= 65536
PLIC_PROVIDER ?= native
PLIC_PROVIDER_CHOICES := native $(strip $(foreach provider,$(PROVIDER_NAMES),$(if $(filter plic,$(PROVIDER_$(provider)_KIND)),$(provider))))
UNKNOWN_PLIC_PROVIDER := $(filter-out $(PLIC_PROVIDER_CHOICES),$(PLIC_PROVIDER))
ifneq ($(UNKNOWN_PLIC_PROVIDER),)
$(error unknown PLIC_PROVIDER: $(PLIC_PROVIDER) (expected one of: $(PLIC_PROVIDER_CHOICES)))
endif
PLIC_PROVIDER_OBJECTS := $(if $(filter native,$(PLIC_PROVIDER)),,$(PROVIDER_$(PLIC_PROVIDER)_OBJECTS))
MISSING_PLIC_PROVIDER_OBJECTS := $(strip $(foreach obj,$(PLIC_PROVIDER_OBJECTS),$(if $(wildcard $(obj)),,$(obj))))
ifneq ($(MISSING_PLIC_PROVIDER_OBJECTS),)
$(error missing object(s) for PLIC_PROVIDER=$(PLIC_PROVIDER): $(MISSING_PLIC_PROVIDER_OBJECTS). Expected sibling Linux tree at $(LINUX_PROVIDER_DIR), or set LINUX_PROVIDER_DIR=/path/to/linux-6.12)
endif
ifneq ($(PROBE_FILE),)
ifeq ($(wildcard $(PROBE_FILE)),)
$(error PROBE_FILE not found: $(PROBE_FILE))
endif
endif
PROBE_FILE_LIST := $(if $(PROBE_FILE),$(strip $(shell sed -e 's/#.*//' "$(PROBE_FILE)")),)
LOG_TRACE_PROBE := $(if $(filter trace,$(LOG)),announce,)
RUN_PROBE := $(strip $(PROBE) $(LOG_TRACE_PROBE))
PROBE_LIST_RAW := $(strip $(subst $(comma),$(space),$(PROBE)) $(PROBE_FILE_LIST) $(LOG_TRACE_PROBE))
PROBE_LIST := $(strip $(sort $(PROBE_LIST_RAW)))
KNOWN_PROBES := announce memblock page-allocator earlycon kernel-init-task of-platform scheduler-action slub console-handoff uart-irq-chain virtio-bus virtio-blk virtio-rng user-boot user-boot-failure user-syscall-error user-read-trace user-syscall-trace vmalloc-mapping linux-plic smoke stress-mem
UNKNOWN_PROBES := $(filter-out $(KNOWN_PROBES),$(PROBE_LIST))
ifneq ($(UNKNOWN_PROBES),)
$(error unknown PROBE handler(s): $(UNKNOWN_PROBES))
endif
KNOWN_STRESS_MEM_BYTES := 32768 65536 131072 262144 524288
UNKNOWN_STRESS_MEM_BYTES := $(filter-out $(KNOWN_STRESS_MEM_BYTES),$(STRESS_MEM_BYTES))
ifneq ($(filter stress-mem,$(PROBE_LIST)),)
ifneq ($(UNKNOWN_STRESS_MEM_BYTES),)
$(error unknown STRESS_MEM_BYTES: $(STRESS_MEM_BYTES))
endif
endif
PROBE_CFGS := $(foreach probe,$(PROBE_LIST),--cfg checkpoint_handler_$(subst -,_,$(probe)))
STRESS_MEM_CFG := $(if $(filter stress-mem,$(PROBE_LIST)),--cfg stress_mem_bytes_$(STRESS_MEM_BYTES),)
PROBE_BUILD_LABEL := $(subst $(space),+,$(PROBE_LIST))
PROBE_BUILD_SUFFIX := $(if $(PROBE_LIST),/probe-$(subst /,_,$(subst .,_,$(PROBE_BUILD_LABEL)))$(if $(filter stress-mem,$(PROBE_LIST)),-bytes$(STRESS_MEM_BYTES),))
PLIC_PROVIDER_SUFFIX := $(if $(filter native,$(PLIC_PROVIDER)),,/plic-$(PLIC_PROVIDER))
BUILD_DIR := build/$(TARGET)/$(PROFILE)/$(APP)$(PLIC_PROVIDER_SUFFIX)$(PROBE_BUILD_SUFFIX)
KERNEL_ELF := $(BUILD_DIR)/arceos_ex.elf
KERNEL_BIN := $(BUILD_DIR)/arceos_ex.bin
RUN_PROFILE := $(if $(filter trace,$(LOG)),trace,$(PROFILE))
RUN_EXTRA_RUSTFLAGS := $(strip $(EXTRA_RUSTFLAGS))
RUN_BUILD_DIR := build/$(TARGET)/$(RUN_PROFILE)/$(APP)$(PLIC_PROVIDER_SUFFIX)$(PROBE_BUILD_SUFFIX)
RUN_KERNEL_BIN := $(RUN_BUILD_DIR)/arceos_ex.bin
LINKER := linker/riscv64.lds
SOURCES := $(sort $(shell find src -type f -name '*.rs'))
RUSTC ?= rustc
RUSTFMT ?= rustfmt
CLIPPY ?= clippy-driver
RUSTFMT_EDITION ?= 2024
RUSTFMT_CONFIG ?= skip_children=true
CLIPPY_FLAGS ?= -D warnings -D clippy::all
RUST_SYSROOT ?= $(shell $(RUSTC) --print sysroot)
RUST_HOST ?= $(shell $(RUSTC) --version --verbose | sed -n 's/^host: //p')
RUST_OBJCOPY ?= $(RUST_SYSROOT)/lib/rustlib/$(RUST_HOST)/bin/rust-objcopy
RUST_OBJDUMP ?= $(RUST_SYSROOT)/lib/rustlib/$(RUST_HOST)/bin/llvm-objdump
QEMU ?= qemu-system-riscv64
QEMU_SMP ?= 8
DEFAULT_QEMU_APPEND := earlycon=sbi
ifeq ($(APP),user-boot)
DEFAULT_QEMU_APPEND := earlycon=sbi init=/bin/sh
endif
QEMU_APPEND ?= $(DEFAULT_QEMU_APPEND)
FS_TYPE ?= ext2
EXT2_BLOCK_SIZE ?=
ROOTFS_URL ?= https://mirrors.tuna.tsinghua.edu.cn/alpine/v3.24/releases/riscv64/alpine-minirootfs-3.24.1-riscv64.tar.gz
ROOTFS_TARBALL ?= build/rootfs-cache/alpine-minirootfs-3.24.1-riscv64.tar.gz
CANONICAL_ROOTFS_IMAGE ?= build/rootfs/canonical.raw
CANONICAL_ROOTFS_SIZE ?= 320M
CANONICAL_ROOTFS_BUILDER ?= tests/basic/rootfs_builder.py
CANONICAL_ROOTFS_CONFIG_DIR ?= tests/rootfs/canonical
ROOTFS_PROFILES := canonical
UNKNOWN_ROOTFS_PROFILE := $(filter-out $(ROOTFS_PROFILES),$(ROOTFS))
ifneq ($(UNKNOWN_ROOTFS_PROFILE),)
$(error unknown ROOTFS profile: $(ROOTFS) (expected one of: $(ROOTFS_PROFILES)))
endif
ROOTFS_LTP_DIR ?= $(abspath $(ROOT)/../ltp/build-riscv64-musl-syscalls/rootfs)
PYTHON ?= python3
ROOTFS_STAGING_DIR ?= build/rootfs-staging
WGET ?= wget
TAR ?= tar
MKFS_EXT2 ?= mkfs.ext2
EXT2_MKFS_FEATURE_ARGS ?= -O ^dir_index
EXT2_MKFS_BLOCK_ARGS := $(if $(strip $(EXT2_BLOCK_SIZE)),-b $(EXT2_BLOCK_SIZE),)
ROOTFS_OVERLAY ?= default
ROOTFS_OVERLAY_MAP ?= tests/user/rootfs-overlay.map
ROOTFS_FILE_OVERLAY_DIR ?=
ROOTFS_OVERLAY_OUTPUT_DIR ?= build/rootfs-user-tests
ROOTFS_OVERLAY_TOOLCHAIN ?= gnu
ROOTFS_OVERLAY_LINK ?= static
ROOTFS_OVERLAY_GNU_CC ?= riscv64-linux-gnu-gcc
ROOTFS_OVERLAY_MUSL_CC ?= riscv64-linux-musl-gcc
ROOTFS_LTP_OVERLAY ?= $(if $(filter user-boot,$(APP)),default,none)
ROOTFS_LTP_OVERLAY_DIR ?= $(abspath $(ROOT)/../ltp/build-riscv64-musl-syscalls/rootfs)
ifneq ($(words $(ROOTFS_LTP_OVERLAY)),1)
$(error unknown ROOTFS_LTP_OVERLAY: $(ROOTFS_LTP_OVERLAY) (expected one of: default none))
endif
ifneq ($(filter-out default none,$(ROOTFS_LTP_OVERLAY)),)
$(error unknown ROOTFS_LTP_OVERLAY: $(ROOTFS_LTP_OVERLAY) (expected one of: default none))
endif
ifeq ($(ROOTFS_LTP_OVERLAY),default)
VIRTIO_BLK_IMAGE ?= build/virtio-blk-ltp.raw
VIRTIO_BLK_IMAGE_SIZE ?= 320M
else
VIRTIO_BLK_IMAGE ?= build/virtio-blk.raw
VIRTIO_BLK_IMAGE_SIZE ?= 64M
endif
QEMU_DEVICES ?= -object rng-random,id=rng0,filename=/dev/urandom -device virtio-rng-device,rng=rng0 -drive file=$(VIRTIO_BLK_IMAGE),if=none,format=raw,id=blk0 -device virtio-blk-device,drive=blk0
PYVERI ?= $(ROOT)/tools/pyveri/bin/pyveri
CODEGEN_PYTHONPATH := $(ROOT)/tools/common/src:$(ROOT)/tools/codegen/src
TOOLS2_PYTHONPATH := $(shell find $(ROOT)/tools2 -mindepth 2 -maxdepth 2 -type d -name src -printf '%p:')
MODEL_AST := $(ROOT)/tools/out/codegen/main.ast.json
MODEL_JSON := $(ROOT)/tools/out/codegen/main.model.json
MODEL_SPECS := $(shell find $(ROOT)/spec/model -name '*.spec')
CODEGEN_SOURCES := $(shell find $(ROOT)/tools/codegen/src $(ROOT)/tools/common/src $(ROOT)/tools2/parse/src $(ROOT)/tools2/model/src $(ROOT)/tools2/common/src -name '*.py')
EXTRA_RUSTFLAGS ?=
APP_CFG := app_$(subst -,_,$(APP))
PLIC_PROVIDER_CFG := --cfg $(if $(filter native,$(PLIC_PROVIDER)),plic_provider_native,$(PROVIDER_$(PLIC_PROVIDER)_CFG))
PLIC_PROVIDER_LINK_ARGS := $(foreach obj,$(PLIC_PROVIDER_OBJECTS),-C link-arg=$(obj))

RUSTFLAGS := \
	-C opt-level=2 \
	-C panic=abort \
	-C relocation-model=static \
	-C code-model=medium \
	-C link-arg=-T$(LINKER) \
	-C link-arg=-static \
	--cfg $(APP_CFG) \
	$(PLIC_PROVIDER_CFG) \
	$(PROBE_CFGS) \
	$(STRESS_MEM_CFG) \
	$(PLIC_PROVIDER_LINK_ARGS) \
	$(EXTRA_RUSTFLAGS)

.PHONY: build run legacy-run generate fmt fmt-check clippy-check test-vm-activation-order test-trap-setup-order test-boot-init-flow-entry disk legacy-disk disk-clean legacy-disk-clean rootfs-tarball rootfs-overlay rootfs-file-overlay rootfs-ltp-overlay clean

build: generate $(KERNEL_BIN)

generate: $(LINKER)

fmt:
	@echo "rustfmt: formatting $(words $(SOURCES)) Rust source files"
	@$(RUSTFMT) --edition $(RUSTFMT_EDITION) --config $(RUSTFMT_CONFIG) $(SOURCES)

fmt-check:
	@echo "rustfmt: checking $(words $(SOURCES)) Rust source files"
	@$(RUSTFMT) --edition $(RUSTFMT_EDITION) --config $(RUSTFMT_CONFIG) --check $(SOURCES)

clippy-check:
	@echo "clippy: checking APP=$(APP) PLIC_PROVIDER=$(PLIC_PROVIDER)"
	@output="$${TMPDIR:-/tmp}/arceos-ex-clippy.$$$$.rmeta"; \
	trap 'rm -f "$$output"' EXIT; \
	$(CLIPPY) src/main.rs \
		--crate-name arceos_ex \
		--crate-type bin \
		--edition 2024 \
		--target $(TARGET) \
		$(RUSTFLAGS) \
		--emit=metadata \
		$(CLIPPY_FLAGS) \
		-o "$$output"

test-vm-activation-order: $(KERNEL_ELF)
	$(PYTHON) tests/check_vm_activation_order.py --objdump "$(RUST_OBJDUMP)" --elf "$(KERNEL_ELF)"

test-trap-setup-order: $(KERNEL_ELF)
	$(PYTHON) tests/check_trap_setup_order.py --objdump "$(RUST_OBJDUMP)" --elf "$(KERNEL_ELF)"

test-boot-init-flow-entry: $(KERNEL_ELF)
	$(PYTHON) tests/check_boot_init_flow_entry.py --objdump "$(RUST_OBJDUMP)" --elf "$(KERNEL_ELF)" --source-root src

$(LINKER): $(MODEL_SPECS) $(CODEGEN_SOURCES) Makefile
	mkdir -p $(ROOT)/tools/out/codegen
	PYTHONPATH=$(TOOLS2_PYTHONPATH) $(PYTHON) -m parse_tool $(ROOT)/spec/model/main.spec -o $(MODEL_AST)
	PYTHONPATH=$(TOOLS2_PYTHONPATH) $(PYTHON) -m model_tool $(MODEL_AST) -o $(MODEL_JSON)
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
	$(PYTHON) $(CANONICAL_ROOTFS_BUILDER) \
		--repo-root "$(abspath $(ROOT))" \
		--image "$(abspath $(CANONICAL_ROOTFS_IMAGE))" \
		--size "$(CANONICAL_ROOTFS_SIZE)" \
		--tarball "$(abspath $(ROOTFS_TARBALL))" \
		--url "$(ROOTFS_URL)" \
		--ltp-dir "$(ROOTFS_LTP_DIR)" \
		--config-dir "$(abspath $(CANONICAL_ROOTFS_CONFIG_DIR))" \
		--wget "$(WGET)" \
		--tar "$(TAR)" \
		--mkfs-ext2 "$(MKFS_EXT2)" \
		--mkfs-feature-args "$(EXT2_MKFS_FEATURE_ARGS)" \
		--ext2-block-size "$(EXT2_BLOCK_SIZE)" \
		--gnu-cc "$(ROOTFS_OVERLAY_GNU_CC)" \
		--musl-cc "$(ROOTFS_OVERLAY_MUSL_CC)" \
		--make "$(MAKE)" \
		$(if $(filter 1,$(FORCE)),--force,)

legacy-disk:
ifeq ($(FORCE),1)
	rm -f $(VIRTIO_BLK_IMAGE)
endif
	$(MAKE) $(VIRTIO_BLK_IMAGE)

disk-clean:
	rm -f $(CANONICAL_ROOTFS_IMAGE) $(CANONICAL_ROOTFS_IMAGE).inputs.json

legacy-disk-clean:
	rm -f $(VIRTIO_BLK_IMAGE)

rootfs-tarball: $(ROOTFS_TARBALL)

$(ROOTFS_TARBALL):
	mkdir -p $(dir $@)
	$(WGET) -O $@.tmp $(ROOTFS_URL)
	mv $@.tmp $@

rootfs-overlay:
	set -e; \
	case "$(ROOTFS_OVERLAY)" in none) exit 0;; esac; \
	test -f "$(ROOTFS_OVERLAY_MAP)"; \
	while read -r target test toolchain link extra; do \
		case "$$target" in ""|\#*) continue;; esac; \
		if [ -n "$$extra" ]; then echo "too many fields in $(ROOTFS_OVERLAY_MAP): $$target $$test $$toolchain $$link $$extra"; exit 1; fi; \
		case "$$target" in /*) ;; *) echo "overlay target must be absolute: $$target"; exit 1;; esac; \
		if [ -z "$$test" ]; then echo "overlay test missing for $$target"; exit 1; fi; \
		if [ "$$test" = "__absent__" ]; then \
			if [ -n "$$toolchain$$link" ]; then echo "__absent__ overlay row must not set toolchain/link: $$target"; exit 1; fi; \
			rm -f "$(ROOTFS_STAGING_DIR)$$target"; \
			continue; \
		fi; \
		toolchain="$${toolchain:-$(ROOTFS_OVERLAY_TOOLCHAIN)}"; \
		link="$${link:-$(ROOTFS_OVERLAY_LINK)}"; \
		output="$(abspath $(ROOTFS_OVERLAY_OUTPUT_DIR))/$$test-$$toolchain-$$link"; \
		$(MAKE) -B -C tests/user build \
			USER_TEST="$$test" \
			USER_TEST_OUT="$$output" \
			USER_TEST_TOOLCHAIN="$$toolchain" \
			USER_TEST_LINK="$$link" \
			GNU_CC="$(ROOTFS_OVERLAY_GNU_CC)" \
			MUSL_CC="$(ROOTFS_OVERLAY_MUSL_CC)"; \
		mkdir -p "$(ROOTFS_STAGING_DIR)$$(dirname "$$target")"; \
		rm -f "$(ROOTFS_STAGING_DIR)$$target"; \
		cp "$$output" "$(ROOTFS_STAGING_DIR)$$target"; \
		chmod 0755 "$(ROOTFS_STAGING_DIR)$$target"; \
	done < "$(ROOTFS_OVERLAY_MAP)"

rootfs-file-overlay:
	set -e; \
	overlay_dir="$(ROOTFS_FILE_OVERLAY_DIR)"; \
	if [ -z "$$overlay_dir" ]; then exit 0; fi; \
	overlay_dir="$${overlay_dir%/}"; \
	test -d "$$overlay_dir"; \
	find "$$overlay_dir" -mindepth 1 | sort | while IFS= read -r src; do \
		rel="$${src#$$overlay_dir/}"; \
		dst="$(ROOTFS_STAGING_DIR)/$$rel"; \
		if [ -d "$$src" ] && [ ! -L "$$src" ]; then \
			mkdir -p "$$dst"; \
		else \
			mkdir -p "$$(dirname "$$dst")"; \
			rm -rf "$$dst"; \
			cp -a "$$src" "$$dst"; \
			case "$$rel" in etc/shadow) chmod 0640 "$$dst";; esac; \
		fi; \
	done

rootfs-ltp-overlay:
	set -e; \
	case "$(ROOTFS_LTP_OVERLAY)" in none) exit 0;; default) ;; *) echo "unknown ROOTFS_LTP_OVERLAY: $(ROOTFS_LTP_OVERLAY) (expected one of: default none)" >&2; exit 1;; esac; \
	overlay_dir="$(ROOTFS_LTP_OVERLAY_DIR)"; \
	if [ ! -d "$$overlay_dir" ]; then \
		echo "LTP rootfs overlay directory not found: $$overlay_dir" >&2; \
		exit 1; \
	fi; \
	runner="$$overlay_dir/opt/ltp/run-syscalls.sh"; \
	if [ ! -f "$$runner" ]; then \
		echo "LTP rootfs overlay is invalid: missing opt/ltp/run-syscalls.sh in $$overlay_dir" >&2; \
		exit 1; \
	fi; \
	if [ ! -x "$$runner" ]; then \
		echo "LTP rootfs overlay is invalid: opt/ltp/run-syscalls.sh is not executable in $$overlay_dir" >&2; \
		exit 1; \
	fi; \
	$(MAKE) rootfs-file-overlay \
		ROOTFS_FILE_OVERLAY_DIR="$$overlay_dir" \
		ROOTFS_STAGING_DIR="$(ROOTFS_STAGING_DIR)"

$(VIRTIO_BLK_IMAGE):
	mkdir -p $(dir $@)
ifeq ($(FS_TYPE),ext2)
	$(MAKE) rootfs-tarball ROOTFS_TARBALL="$(ROOTFS_TARBALL)" ROOTFS_URL="$(ROOTFS_URL)" WGET="$(WGET)"
	rm -rf $(ROOTFS_STAGING_DIR)
	mkdir -p $(ROOTFS_STAGING_DIR)
	$(TAR) --no-same-owner -xzf $(ROOTFS_TARBALL) -C $(ROOTFS_STAGING_DIR)
ifneq ($(ROOTFS_OVERLAY),none)
	$(MAKE) rootfs-overlay
endif
ifneq ($(strip $(ROOTFS_FILE_OVERLAY_DIR)),)
	$(MAKE) rootfs-file-overlay ROOTFS_FILE_OVERLAY_DIR="$(ROOTFS_FILE_OVERLAY_DIR)" ROOTFS_STAGING_DIR="$(ROOTFS_STAGING_DIR)"
endif
ifneq ($(ROOTFS_LTP_OVERLAY),none)
	$(MAKE) rootfs-ltp-overlay ROOTFS_LTP_OVERLAY="$(ROOTFS_LTP_OVERLAY)" ROOTFS_LTP_OVERLAY_DIR="$(ROOTFS_LTP_OVERLAY_DIR)" ROOTFS_STAGING_DIR="$(ROOTFS_STAGING_DIR)"
endif
	truncate -s $(VIRTIO_BLK_IMAGE_SIZE) $@
	$(MKFS_EXT2) -q -F $(EXT2_MKFS_FEATURE_ARGS) $(EXT2_MKFS_BLOCK_ARGS) -d $(ROOTFS_STAGING_DIR) $@
else ifeq ($(FS_TYPE),none)
	truncate -s $(VIRTIO_BLK_IMAGE_SIZE) $@
else
	$(error unknown FS_TYPE: $(FS_TYPE))
endif

run: legacy-run

legacy-run: legacy-disk
	$(MAKE) build APP=$(APP) PROBE="$(RUN_PROBE)" PROBE_FILE="$(PROBE_FILE)" PLIC_PROVIDER=$(PLIC_PROVIDER) PROFILE=$(RUN_PROFILE) EXTRA_RUSTFLAGS="$(RUN_EXTRA_RUSTFLAGS)"
	$(QEMU) -machine virt -cpu rv64 -m 128M -smp $(QEMU_SMP) -nographic -serial mon:stdio $(QEMU_DEVICES) -append "$(QEMU_APPEND)" -kernel $(RUN_KERNEL_BIN)

clean:
	rm -rf build
