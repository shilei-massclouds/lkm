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
DIFFTEST_CASE ?= impl/arceos_ex/tests/stress/cases/rc-local-difftest.toml
DIFFTEST_RUNS ?= 1
DIFFTEST_TIMEOUT ?=

KERNEL_DIR := impl/$(KERNEL)
PYVERI ?= tools/pyveri/bin/pyveri
STRESS_RUNNER ?= impl/arceos_ex/tests/stress/runner.py
PROBE_FILE_ARG := $(if $(PROBE_FILE),PROBE_FILE="$(abspath $(PROBE_FILE))",)
STRESS_TIMEOUT_ARG := $(if $(STRESS_TIMEOUT),--timeout $(STRESS_TIMEOUT),)
DIFFTEST_TIMEOUT_ARG := $(if $(DIFFTEST_TIMEOUT),--timeout $(DIFFTEST_TIMEOUT),)

.PHONY: build run disk disk-clean fmt fmt-check verify checkpoints-inventory checkpoints-map-linux checkpoints-coverage checkpoints-instrumentation-plan checkpoints checkpoints-linux-check test test-verify test-checkpoints test-kunit test-smoke test-stress stress-test difftest-preflight difftest clean

build:
	$(MAKE) -C $(KERNEL_DIR) build APP=$(APP) PROBE="$(PROBE)" PLIC_PROVIDER="$(PLIC_PROVIDER)" $(PROBE_FILE_ARG)

disk:
	$(MAKE) -C $(KERNEL_DIR) disk FORCE=$(FORCE)

disk-clean:
	$(MAKE) -C $(KERNEL_DIR) disk-clean

run:
	$(MAKE) -C $(KERNEL_DIR) run LOG="$(LOG)" APP=$(APP) PROBE="$(PROBE)" PLIC_PROVIDER="$(PLIC_PROVIDER)" $(PROBE_FILE_ARG)

fmt:
	$(MAKE) -C $(KERNEL_DIR) fmt

fmt-check:
	$(MAKE) -C $(KERNEL_DIR) fmt-check

verify:
ifeq ($(REPORT),graph)
	$(PYVERI) $(SPEC) -T --trace-annotations state,transition --trace-hide-contexts "$(TRACE_HIDE_CONTEXTS)"
else
	$(PYVERI) $(SPEC) --derive --strict
endif

checkpoints-inventory:
	python3 tools/checkpoints/list_checkpoints.py

checkpoints-map-linux: checkpoints-inventory
	python3 tools/checkpoints/map_linux_checkpoints.py

checkpoints-coverage: checkpoints-map-linux
	python3 tools/checkpoints/summarize_linux_checkpoint_mapping.py

checkpoints-instrumentation-plan: checkpoints-map-linux checkpoints-coverage
	python3 tools/checkpoints/plan_linux_instrumentation.py

checkpoints: checkpoints-instrumentation-plan

checkpoints-linux-check:
	@python3 tools/checkpoints/plan_linux_instrumentation.py --check --check-markers || { \
		status=$$?; \
		echo "Linux checkpoint synchronization requires manual review." >&2; \
		echo "Review mapping/semantics and sibling Linux instrumentation, then run 'make checkpoints' and 'make checkpoints-linux-check'." >&2; \
		exit $$status; \
	}

test: fmt-check
	@bash tools/test_summary.sh "$(MAKE)" "$(SPEC)" "$(KERNEL_DIR)" "$(KUNIT_APP)" "$(abspath $(KUNIT_HANDLERS))" "$(SMOKE_APP)" "$(TEST_PLIC_PROVIDERS)"

test-verify:
	$(MAKE) verify REPORT=text SPEC="$(SPEC)"

test-checkpoints:
	python3 -m unittest tools.checkpoints.tests.test_list_checkpoints tools.checkpoints.tests.test_map_linux_checkpoints tools.checkpoints.tests.test_summarize_linux_checkpoint_mapping tools.checkpoints.tests.test_plan_linux_instrumentation
	python3 tools/checkpoints/list_checkpoints.py --check
	python3 tools/checkpoints/map_linux_checkpoints.py --check
	python3 tools/checkpoints/summarize_linux_checkpoint_mapping.py --check
	python3 tools/checkpoints/plan_linux_instrumentation.py --check

test-kunit:
	$(MAKE) -C $(KERNEL_DIR) run APP=$(KUNIT_APP) PROBE_FILE="$(abspath $(KUNIT_HANDLERS))"

test-smoke:
	$(MAKE) run APP=$(SMOKE_APP)

test-stress:
	$(STRESS_RUNNER) $(STRESS_CASES) --runs $(STRESS_RUNS) $(STRESS_TIMEOUT_ARG)

stress-test: test-stress

difftest-preflight:
	@$(MAKE) test-checkpoints || { \
		status=$$?; \
		echo "Checkpoint artifacts require manual review; difftest did not regenerate them." >&2; \
		echo "Review mapping/semantics and sibling Linux instrumentation, then run 'make checkpoints' and 'make checkpoints-linux-check'." >&2; \
		exit $$status; \
	}
	@$(MAKE) checkpoints-linux-check

difftest: difftest-preflight
	$(STRESS_RUNNER) $(DIFFTEST_CASE) --runs $(DIFFTEST_RUNS) $(DIFFTEST_TIMEOUT_ARG)

clean:
	$(MAKE) -C $(KERNEL_DIR) clean
	rm -rf tools/build
	@if [ -d tools/out ]; then find tools/out -mindepth 1 \( -path 'tools/out/checkpoints' -o -path 'tools/out/checkpoints/*' \) -prune -o -exec rm -rf {} +; fi
	find . \( -path ./.git -o -path ./.venv -o -path ./venv \) -prune -o -type d \( -name __pycache__ -o -name .pytest_cache -o -name .mypy_cache -o -name .ruff_cache \) -prune -exec rm -rf {} +
	find . \( -path ./.git -o -path ./.venv -o -path ./venv \) -prune -o -type f \( -name '*.pyc' -o -name '*.pyo' \) -exec rm -f {} +
