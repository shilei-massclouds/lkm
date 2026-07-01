SMOKE_SRC := smoke.c fileio.c signal.c stdin.c sh_probe.c
SMOKE_DEPS := smoke.h Makefile

.PHONY: build

build: $(USER_TEST_OUT)

$(USER_TEST_OUT): $(SMOKE_SRC) $(SMOKE_DEPS)
	mkdir -p $(dir $@)
	$(USER_TEST_CC) $(USER_TEST_CFLAGS) -o $@ $(SMOKE_SRC)
