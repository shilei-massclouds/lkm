PYTHON ?= python3
NPM ?= npm
PYTHONPATH := common/src:parse/src:model/src:derive/src:check/src:view/src:render/src:animate/src:pyveri/src
FRONTEND := animate/frontend

.PHONY: test test-python test-focused test-frontend test-browser test-all frontend-build bundle-check

test: test-python

test-python:
	PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=$(PYTHONPATH) $(PYTHON) -m unittest discover -s tests -v

test-focused:
	PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=$(PYTHONPATH) $(PYTHON) -m unittest tests.test_signal_pipeline.SignalPipelineTests.test_drives_and_emits_order -v

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
