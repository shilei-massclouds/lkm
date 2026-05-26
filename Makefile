KERNEL ?= arceos_ex
LOG ?= info
REPORT ?= text
SPEC ?= spec/model/main.spec

KERNEL_DIR := impl/$(KERNEL)
PYVERI ?= tools/pyveri/bin/pyveri

.PHONY: build run verify clean

build:
	$(MAKE) -C $(KERNEL_DIR) build

run:
ifeq ($(LOG),trace)
	$(MAKE) -C $(KERNEL_DIR) run LOG=trace
else
	$(MAKE) -C $(KERNEL_DIR) run
endif

verify:
ifeq ($(REPORT),graph)
	$(PYVERI) $(SPEC) -T --trace-annotations state,event
else
	$(PYVERI) $(SPEC) --derive --strict
endif

clean:
	$(MAKE) -C $(KERNEL_DIR) clean
