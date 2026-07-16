ARCH ?= riscv64
# Fixture naming:
# - *.S is pure assembly.
# - *_nolibc.c is freestanding/no-libc C.
# - unsuffixed .c names are reserved for libc-linked tests.
USER_TEST ?= init_hello
USER_TEST_TOOLCHAIN ?= gnu
USER_TEST_LINK ?= static
USER_TEST_OUT ?= ../../build/rootfs-user-tests/$(USER_TEST)-$(USER_TEST_TOOLCHAIN)-$(USER_TEST_LINK)

GNU_CC ?= riscv64-linux-gnu-gcc
MUSL_CC ?= riscv64-linux-musl-gcc

USER_TEST_MARCH ?= rv64imac
USER_TEST_MABI ?= lp64
USER_TEST_LIBC_MARCH ?= rv64gc
USER_TEST_LIBC_MABI ?= lp64d
USER_TEST_ENTRY ?= _start
USER_TEST_TEXT ?= 0x10000

COMMON_CFLAGS ?= -march=$(USER_TEST_MARCH) -mabi=$(USER_TEST_MABI)
LIBC_COMMON_CFLAGS ?= -march=$(USER_TEST_LIBC_MARCH) -mabi=$(USER_TEST_LIBC_MABI)
STATIC_LDFLAGS ?= -static
DYNAMIC_LDFLAGS ?=

ifeq ($(USER_TEST_TOOLCHAIN),gnu)
USER_TEST_CC := $(GNU_CC)
else ifeq ($(USER_TEST_TOOLCHAIN),musl)
USER_TEST_CC := $(MUSL_CC)
else
$(error unknown USER_TEST_TOOLCHAIN: $(USER_TEST_TOOLCHAIN))
endif

ifeq ($(USER_TEST_LINK),static)
USER_TEST_LINK_FLAGS := $(STATIC_LDFLAGS)
else ifeq ($(USER_TEST_LINK),dynamic)
USER_TEST_LINK_FLAGS := $(DYNAMIC_LDFLAGS)
else
$(error unknown USER_TEST_LINK: $(USER_TEST_LINK))
endif

NO_LIBC_CFLAGS ?= \
	-nostdlib \
	$(COMMON_CFLAGS) \
	-Wl,-e,$(USER_TEST_ENTRY) \
	-Wl,-Ttext=$(USER_TEST_TEXT) \
	-Wl,--build-id=none \
	$(USER_TEST_LINK_FLAGS)

LIBC_CFLAGS ?= \
	$(LIBC_COMMON_CFLAGS) \
	-Os \
	-fstack-protector-strong \
	-ffunction-sections \
	-fdata-sections \
	-no-pie \
	-Wl,--gc-sections \
	-Wl,--build-id=none \
	$(USER_TEST_LINK_FLAGS)

ifeq ($(USER_TEST),user_smoke)
USER_TEST_CFLAGS ?= $(LIBC_CFLAGS)
else
USER_TEST_SRC ?= $(firstword $(wildcard $(USER_TEST).c) $(wildcard $(USER_TEST).S) $(USER_TEST).S)
USER_TEST_DEPS ?=
USER_TEST_NOLIBC := $(filter %.S %_nolibc.c,$(USER_TEST_SRC))

ifeq ($(USER_TEST_NOLIBC),)
USER_TEST_CFLAGS ?= $(LIBC_CFLAGS)
else
USER_TEST_CFLAGS ?= $(NO_LIBC_CFLAGS)
ifeq ($(suffix $(USER_TEST_SRC)),.c)
USER_TEST_CFLAGS += -ffreestanding -fno-stack-protector -no-pie
endif
endif
endif

.PHONY: build clean

ifeq ($(USER_TEST),user_smoke)
build:
	$(MAKE) -C smoke build \
		USER_TEST_OUT="$(abspath $(USER_TEST_OUT))" \
		USER_TEST_CC="$(USER_TEST_CC)" \
		USER_TEST_CFLAGS="$(USER_TEST_CFLAGS)"
else
build: $(USER_TEST_OUT)

$(USER_TEST_OUT): $(USER_TEST_SRC) $(USER_TEST_DEPS) Makefile
	mkdir -p $(dir $@)
	$(USER_TEST_CC) $(USER_TEST_CFLAGS) -o $@ $(USER_TEST_SRC)
endif

clean:
	rm -rf ../../build/rootfs-user-tests
