#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static volatile unsigned long initialized_data = 0x1122334455667788UL;
static volatile unsigned long zero_bss;

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_fork_mm(void)
{
	volatile unsigned long stack_value = 0x8877665544332211UL;
	volatile unsigned long *anonymous;
	uintptr_t parent_brk;
	long mapping;
	long child;
	long waited;
	int status = 0;

	mapping = syscall(SYS_mmap, NULL, 4096, PROT_READ | PROT_WRITE,
			  MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
	if (mapping < 0) {
		return 106;
	}
	anonymous = (volatile unsigned long *)(uintptr_t)mapping;
	*anonymous = 0xa5a5a5a55a5a5a5aUL;
	parent_brk = (uintptr_t)syscall(SYS_brk, 0);
	if (parent_brk == 0) {
		return 107;
	}

	child = syscall(SYS_clone, SIGCHLD, 0, 0, 0, 0);
	if (child < 0) {
		return 108;
	}
	if (child == 0) {
		uintptr_t child_brk = (parent_brk + 8191UL) & ~4095UL;

		initialized_data = 0x0102030405060708UL;
		zero_bss = 0xfedcba9876543210UL;
		stack_value = 0x1020304050607080UL;
		*anonymous = 0x55aa55aaaa55aa55UL;
		if (child_brk <= parent_brk ||
		    syscall(SYS_brk, child_brk) != (long)child_brk) {
			syscall(SYS_exit, 24);
		}
		*(volatile unsigned char *)(child_brk - 1) = 0x6e;
		if (SAY_LITERAL("user fork child private mm ok\n") < 0) {
			syscall(SYS_exit, 25);
		}
		syscall(SYS_exit, 23);
		__builtin_unreachable();
	}

	waited = syscall(SYS_wait4, -1, &status, 0, NULL);
	if (waited != child || status != (23 << 8)) {
		return 109;
	}
	if (initialized_data != 0x1122334455667788UL || zero_bss != 0 ||
	    stack_value != 0x8877665544332211UL ||
	    *anonymous != 0xa5a5a5a55a5a5a5aUL ||
	    (uintptr_t)syscall(SYS_brk, 0) != parent_brk) {
		return 110;
	}
	if (syscall(SYS_munmap, (void *)(uintptr_t)mapping, 4096) != 0) {
		return 111;
	}
	if (SAY_LITERAL("user fork parent isolation ok\n") < 0) {
		return 112;
	}
	return 0;
}
