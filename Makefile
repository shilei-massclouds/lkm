KERNEL ?= arceos_ex
LOG ?= info
REPORT ?= text
SPEC ?= spec/model/main.spec
APP ?= smoke
PROBE ?=
PROBE_FILE ?=
KUNIT_HANDLERS ?= impl/arceos_ex/tests/kunit.handlers

KERNEL_DIR := impl/$(KERNEL)
PYVERI ?= tools/pyveri/bin/pyveri
PROBE_FILE_ARG := $(if $(PROBE_FILE),PROBE_FILE="$(abspath $(PROBE_FILE))",)

.PHONY: build run verify test-kunit clean

build:
	$(MAKE) -C $(KERNEL_DIR) build APP=$(APP) PROBE="$(PROBE)" $(PROBE_FILE_ARG)

run:
ifeq ($(LOG),trace)
	$(MAKE) -C $(KERNEL_DIR) run LOG=trace APP=$(APP) PROBE="$(PROBE)" $(PROBE_FILE_ARG)
else
	$(MAKE) -C $(KERNEL_DIR) run APP=$(APP) PROBE="$(PROBE)" $(PROBE_FILE_ARG)
endif

verify:
ifeq ($(REPORT),graph)
	$(PYVERI) $(SPEC) -T --trace-annotations state,event
else
	$(PYVERI) $(SPEC) --derive --strict
endif

test-kunit:
	$(MAKE) -C $(KERNEL_DIR) run APP=hello PROBE_FILE="$(abspath $(KUNIT_HANDLERS))"

clean:
	$(MAKE) -C $(KERNEL_DIR) clean
