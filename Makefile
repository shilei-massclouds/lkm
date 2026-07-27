ROOT := .
include $(ROOT)/impl/providers/linux-6.12.mk
KERNEL ?= arceos_ex
TEST ?= hello-native
ROOTFS ?= canonical
PYTHON ?= python3
LOG ?= info
REPORT ?= text
VERBOSE ?= 0
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
STRESS_BASELINE ?=
DIFFTEST_CASE ?= impl/arceos_ex/tests/stress/cases/rc-local-difftest.toml
DIFFTEST_RUNS ?= 1

KERNEL_DIR := impl/$(KERNEL)
BASIC_TEST_RUNNER ?= $(KERNEL_DIR)/tests/basic/runner.py
BASIC_TEST_OUT_ROOT ?= $(KERNEL_DIR)/tests/basic/out
BASIC_TEST_BEHAVIOR_VARIABLES := LOG PROFILE PLIC_PROVIDER PROBE PROBE_FILE STRESS_MEM_BYTES QEMU_APPEND QEMU_SMP QEMU_DEVICES VIRTIO_BLK_IMAGE VIRTIO_BLK_IMAGE_SIZE ROOTFS_OVERLAY ROOTFS_OVERLAY_MAP ROOTFS_FILE_OVERLAY_DIR ROOTFS_LTP_OVERLAY FORCE
BASIC_TEST_COMMAND_LINE_OVERRIDES := $(strip $(foreach variable,$(BASIC_TEST_BEHAVIOR_VARIABLES),$(if $(filter command line,$(origin $(variable))),$(variable))))
BASIC_TEST_EXPLICIT_TEST := $(or $(findstring command line,$(origin TEST)),$(findstring environment,$(origin TEST)))
BASIC_TEST_EXPLICIT_APP := $(or $(findstring command line,$(origin APP)),$(findstring environment,$(origin APP)))
BASIC_TEST_SELECTION_CONFLICT := $(and $(BASIC_TEST_EXPLICIT_TEST),$(BASIC_TEST_EXPLICIT_APP))
BASIC_TEST_REQUEST := $(if $(BASIC_TEST_EXPLICIT_APP),$(APP),$(TEST))
PYVERI ?= tools/pyveri/bin/pyveri
STRESS_RUNNER ?= impl/arceos_ex/tests/stress/runner.py
PROBE_FILE_ARG := $(if $(PROBE_FILE),PROBE_FILE="$(abspath $(PROBE_FILE))",)
STRESS_BASELINE_ARG := $(if $(STRESS_BASELINE),--baseline "$(STRESS_BASELINE)",)

ifeq ($(VERBOSE),1)
VERIFY_TEXT_ARGS := --derive --strict
else
VERIFY_TEXT_ARGS := --strict
endif

.PHONY: build run disk disk-clean fmt fmt-check clippy-check coding-spec-check charter-lock-check verify checkpoints-inventory checkpoints-map-linux checkpoints-coverage checkpoints-instrumentation-plan checkpoints checkpoints-linux-check test test-charter-lock test-basic test-composite test-verify test-checkpoints test-kunit test-smoke test-stress stress-test difftest-preflight difftest clean

build:
	@if [ -n "$(BASIC_TEST_SELECTION_CONFLICT)" ]; then \
		echo "basic-test selection is ambiguous; pass either TEST=$(TEST) or APP=$(APP), not both" >&2; \
		exit 2; \
	fi
	@if [ -n "$(BASIC_TEST_COMMAND_LINE_OVERRIDES)" ]; then \
		echo "basic-test behavior must come from TEST=$(BASIC_TEST_REQUEST) TOML; remove Make override(s): $(BASIC_TEST_COMMAND_LINE_OVERRIDES)" >&2; \
		exit 2; \
	fi
	$(PYTHON) $(BASIC_TEST_RUNNER) build "$(BASIC_TEST_REQUEST)" --out-root "$(BASIC_TEST_OUT_ROOT)"

disk:
	$(MAKE) -C $(KERNEL_DIR) disk ROOTFS="$(ROOTFS)" FORCE=$(FORCE)

disk-clean:
	$(MAKE) -C $(KERNEL_DIR) disk-clean ROOTFS="$(ROOTFS)"

run:
	@if [ -n "$(BASIC_TEST_SELECTION_CONFLICT)" ]; then \
		echo "basic-test selection is ambiguous; pass either TEST=$(TEST) or APP=$(APP), not both" >&2; \
		exit 2; \
	fi
	@if [ -n "$(BASIC_TEST_COMMAND_LINE_OVERRIDES)" ]; then \
		echo "basic-test behavior must come from TEST=$(BASIC_TEST_REQUEST) TOML; remove Make override(s): $(BASIC_TEST_COMMAND_LINE_OVERRIDES)" >&2; \
		exit 2; \
	fi
	$(PYTHON) $(BASIC_TEST_RUNNER) run "$(BASIC_TEST_REQUEST)" --out-root "$(BASIC_TEST_OUT_ROOT)"

fmt:
	$(MAKE) -C $(KERNEL_DIR) fmt

fmt-check:
	$(MAKE) -C $(KERNEL_DIR) fmt-check

clippy-check:
	@set -e; \
	for provider in native $(TEST_PLIC_PROVIDERS); do \
		$(MAKE) -C $(KERNEL_DIR) clippy-check APP=$(SMOKE_APP) PLIC_PROVIDER="$$provider"; \
		$(MAKE) -C $(KERNEL_DIR) clippy-check APP=hello PLIC_PROVIDER="$$provider"; \
		$(MAKE) -C $(KERNEL_DIR) clippy-check APP=$(KUNIT_APP) PROBE_FILE="$(abspath $(KUNIT_HANDLERS))" PLIC_PROVIDER="$$provider"; \
		$(MAKE) -C $(KERNEL_DIR) clippy-check APP=user-boot PLIC_PROVIDER="$$provider"; \
	done

coding-spec-check:
	@files="$$(find spec/coding -type f -name '*.spec' -print | sort)"; \
	if [ -n "$$files" ]; then \
		echo "coding .spec files are forbidden; move authoritative rules to Markdown:" >&2; \
		printf '%s\n' "$$files" >&2; \
		exit 1; \
	fi

charter-lock-check:
	$(PYTHON) tools/charter_lock.py enforce
	$(PYTHON) tools/charter_lock.py check

verify:
ifeq ($(REPORT),graph)
	$(PYVERI) $(SPEC) -T --trace-annotations state,transition --trace-hide-contexts "$(TRACE_HIDE_CONTEXTS)"
else
	$(PYVERI) $(SPEC) $(VERIFY_TEXT_ARGS)
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

test:
	$(MAKE) charter-lock-check
	$(MAKE) test-charter-lock
	$(MAKE) fmt-check
	$(MAKE) clippy-check
	$(MAKE) coding-spec-check
	@bash tools/test_summary.sh "$(MAKE)" "$(SPEC)" "$(KERNEL_DIR)" "$(KUNIT_APP)" "$(abspath $(KUNIT_HANDLERS))" "$(SMOKE_APP)" "$(TEST_PLIC_PROVIDERS)"

test-verify:
	$(MAKE) verify REPORT=text SPEC="$(SPEC)"

test-charter-lock:
	PYTHONDONTWRITEBYTECODE=1 $(PYTHON) -m unittest tools.tests.test_charter_lock

test-basic:
	PYTHONDONTWRITEBYTECODE=1 $(PYTHON) -m unittest impl.arceos_ex.tests.basic.test_runner

test-composite:
	PYTHONDONTWRITEBYTECODE=1 $(PYTHON) -m unittest impl.arceos_ex.tests.stress.test_runner

test-checkpoints:
	python3 -m unittest tools.checkpoints.tests.test_list_checkpoints tools.checkpoints.tests.test_map_linux_checkpoints tools.checkpoints.tests.test_summarize_linux_checkpoint_mapping tools.checkpoints.tests.test_plan_linux_instrumentation
	python3 tools/checkpoints/list_checkpoints.py --check
	python3 tools/checkpoints/map_linux_checkpoints.py --check
	python3 tools/checkpoints/summarize_linux_checkpoint_mapping.py --check
	python3 tools/checkpoints/plan_linux_instrumentation.py --check

test-kunit:
	$(MAKE) run TEST=checkpoint-kunit-native

test-smoke:
	$(MAKE) run TEST=kernel-smoke-native

test-stress:
	$(STRESS_RUNNER) $(STRESS_CASES) --runs $(STRESS_RUNS) $(STRESS_BASELINE_ARG)

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
	$(STRESS_RUNNER) $(DIFFTEST_CASE) --runs $(DIFFTEST_RUNS)

clean:
	$(MAKE) -C $(KERNEL_DIR) clean
	rm -rf tools/build
	@if [ -d tools/out ]; then find tools/out -mindepth 1 \( -path 'tools/out/checkpoints' -o -path 'tools/out/checkpoints/*' \) -prune -o -exec rm -rf {} +; fi
	@if [ -d impl/arceos_ex/tests/stress/out ]; then find impl/arceos_ex/tests/stress/out -mindepth 1 -maxdepth 1 ! -name '.gitignore' -exec rm -rf {} +; fi
	@if [ -d impl/arceos_ex/tests/basic/out ]; then find impl/arceos_ex/tests/basic/out -mindepth 1 -maxdepth 1 ! -name '.gitignore' -exec rm -rf {} +; fi
	find . \( -path ./.git -o -path ./.venv -o -path ./venv \) -prune -o -type d \( -name __pycache__ -o -name .pytest_cache -o -name .mypy_cache -o -name .ruff_cache \) -prune -exec rm -rf {} +
	find . \( -path ./.git -o -path ./.venv -o -path ./venv \) -prune -o -type f \( -name '*.pyc' -o -name '*.pyo' \) -exec rm -f {} +
