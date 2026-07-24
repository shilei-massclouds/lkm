PYTHON ?= python3
PYTHONPATH := common/src:parse/src:model/src:derive/src:check/src:view/src:render/src:animate/src:pyveri/src

.PHONY: test test-focused

test:
	PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=$(PYTHONPATH) $(PYTHON) -m unittest discover -s tests -v

test-focused:
	PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=$(PYTHONPATH) $(PYTHON) -m unittest tests.test_signal_pipeline.SignalPipelineTests.test_drives_and_emits_order -v
