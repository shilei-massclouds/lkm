KERNEL ?= arceos_ex
LOG ?= info
REPORT ?= text
SPEC ?= spec/model/main.spec
APP ?= hello

KERNEL_DIR := impl/$(KERNEL)
PYVERI ?= tools/pyveri/bin/pyveri

.PHONY: build run verify clean

build:
	$(MAKE) -C $(KERNEL_DIR) build APP=$(APP)

run:
ifeq ($(LOG),trace)
	$(MAKE) -C $(KERNEL_DIR) run LOG=trace APP=$(APP)
else
	$(MAKE) -C $(KERNEL_DIR) run APP=$(APP)
endif

verify:
ifeq ($(REPORT),graph)
	$(PYVERI) $(SPEC) -T --trace-annotations state,event
else
	$(PYVERI) $(SPEC) --derive --strict
endif

clean:
	$(MAKE) -C $(KERNEL_DIR) clean
