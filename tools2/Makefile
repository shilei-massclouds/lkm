PYTHON ?= python3
NPM ?= npm
FRONTEND := animate/frontend
DEFAULT_FOCUSED_TEST := tools2.tests.test_signal_pipeline.SignalPipelineTests.test_drives_and_emits_order

.PHONY: test test-python test-focused test-frontend test-browser test-all frontend-build bundle-check

test: test-python

test-python:
	PYTHONDONTWRITEBYTECODE=1 $(PYTHON) bin/test-python

test-focused:
	PYTHONDONTWRITEBYTECODE=1 $(PYTHON) bin/test-python $(if $(TEST),$(TEST),$(DEFAULT_FOCUSED_TEST)) -v

test-frontend:
	$(NPM) --prefix $(FRONTEND) run check
	$(NPM) --prefix $(FRONTEND) test

test-browser:
	$(NPM) --prefix $(FRONTEND) run test:e2e

frontend-build:
	$(NPM) --prefix $(FRONTEND) run build

bundle-check:
	$(NPM) --prefix $(FRONTEND) run bundle-check

test-all: test-python test-frontend bundle-check test-browser
