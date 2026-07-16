SMOKE_SRC := smoke.c \
	trap_registers.S \
	fileio.c \
	signal.c \
	stdin.c \
	fpu_mmap.c \
	credentials.c \
	process_identity.c \
	tty_termios.c \
	uts_cwd.c \
	time_random.c \
	stack.c
SMOKE_DEPS := smoke.h Makefile

.PHONY: build

build: $(USER_TEST_OUT)

$(USER_TEST_OUT): $(SMOKE_SRC) $(SMOKE_DEPS)
	mkdir -p $(dir $@)
	$(USER_TEST_CC) $(USER_TEST_CFLAGS) -o $@ $(SMOKE_SRC)
