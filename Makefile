ROOT := .
include $(ROOT)/impl/providers/linux-6.12.mk
KERNEL ?= arceos_ex
LOG ?= info
REPORT ?= text
SPEC ?= spec/model/main.spec
TRACE_HIDE_CONTEXTS ?= SingleTaskContext,SingleTaskInterruptStreamContext,BootIdleStartupContext
APP ?= hello
PLIC_PROVIDER ?= native
PROBE ?=
PROBE_FILE ?=
KUNIT_HANDLERS ?= impl/arceos_ex/tests/kunit.handlers
KUNIT_APP ?= hello
SMOKE_APP ?= smoke
TEST_PLIC_PROVIDERS ?= $(strip $(foreach provider,$(PROVIDER_NAMES),$(if $(filter plic,$(PROVIDER_$(provider)_KIND)),$(provider))))
STRESS_RUNS ?= 10
STRESS_CASES ?=
STRESS_TIMEOUT ?=

KERNEL_DIR := impl/$(KERNEL)
PYVERI ?= tools/pyveri/bin/pyveri
STRESS_RUNNER ?= impl/arceos_ex/tests/stress/runner.py
PROBE_FILE_ARG := $(if $(PROBE_FILE),PROBE_FILE="$(abspath $(PROBE_FILE))",)
STRESS_TIMEOUT_ARG := $(if $(STRESS_TIMEOUT),--timeout $(STRESS_TIMEOUT),)

.PHONY: build run disk disk-clean verify test test-verify test-kunit test-smoke stress-test clean

build:
	$(MAKE) -C $(KERNEL_DIR) build APP=$(APP) PROBE="$(PROBE)" PLIC_PROVIDER="$(PLIC_PROVIDER)" $(PROBE_FILE_ARG)

disk:
	$(MAKE) -C $(KERNEL_DIR) disk FORCE=$(FORCE)

disk-clean:
	$(MAKE) -C $(KERNEL_DIR) disk-clean

run:
	$(MAKE) -C $(KERNEL_DIR) run LOG="$(LOG)" APP=$(APP) PROBE="$(PROBE)" PLIC_PROVIDER="$(PLIC_PROVIDER)" $(PROBE_FILE_ARG)

verify:
ifeq ($(REPORT),graph)
	$(PYVERI) $(SPEC) -T --trace-annotations state,transition --trace-hide-contexts "$(TRACE_HIDE_CONTEXTS)"
else
	$(PYVERI) $(SPEC) --derive --strict
endif

test:
	@bash tools/test_summary.sh "$(MAKE)" "$(SPEC)" "$(KERNEL_DIR)" "$(KUNIT_APP)" "$(abspath $(KUNIT_HANDLERS))" "$(SMOKE_APP)" "$(TEST_PLIC_PROVIDERS)"

test-verify:
	$(MAKE) verify REPORT=text SPEC="$(SPEC)"

test-kunit:
	$(MAKE) -C $(KERNEL_DIR) run APP=$(KUNIT_APP) PROBE_FILE="$(abspath $(KUNIT_HANDLERS))"

test-smoke:
	$(MAKE) run APP=$(SMOKE_APP)

stress-test:
	$(STRESS_RUNNER) $(STRESS_CASES) --runs $(STRESS_RUNS) $(STRESS_TIMEOUT_ARG)

clean:
	$(MAKE) -C $(KERNEL_DIR) clean
