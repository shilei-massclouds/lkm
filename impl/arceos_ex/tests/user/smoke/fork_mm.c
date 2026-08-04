#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static volatile unsigned long initialized_data = 0x1122334455667788UL;
static volatile unsigned long zero_bss;
static const unsigned char readonly_probe = 0x5a;

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

typedef void (*fault_fn_t)(void);

__attribute__((noinline, noreturn)) static void fault_unmapped_load(void)
{
	const void *address = (const void *)(uintptr_t)0x1000;

	__asm__ volatile("lb zero, 0(%0)" : : "r"(address) : "memory");
	syscall(SYS_exit, 90);
	__builtin_unreachable();
}

__attribute__((noinline, noreturn)) static void fault_readonly_store(void)
{
	__asm__ volatile("sb zero, 0(%0)" : : "r"(&readonly_probe) : "memory");
	syscall(SYS_exit, 91);
	__builtin_unreachable();
}

__attribute__((noinline, noreturn)) static void fault_nx_execute(void)
{
	long mapping = syscall(SYS_mmap, NULL, 4096, PROT_READ | PROT_WRITE,
			       MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);

	if (mapping < 0) {
		syscall(SYS_exit, 92);
	}
	*(volatile uint16_t *)(uintptr_t)mapping = 0x8082;
	__asm__ volatile("jalr ra, 0(%0)" : : "r"((uintptr_t)mapping) : "ra", "memory");
	syscall(SYS_exit, 93);
	__builtin_unreachable();
}

static int expect_sigsegv(fault_fn_t fault)
{
	long child = syscall(SYS_clone, SIGCHLD, 0, 0, 0, 0);
	long waited;
	int status = 0;

	if (child < 0) {
		return -1;
	}
	if (child == 0) {
		fault();
		__builtin_unreachable();
	}
	waited = syscall(SYS_wait4, -1, &status, 0, NULL);
	return waited == child && status == SIGSEGV ? 0 : -1;
}

int smoke_fork_mm(void)
{
	volatile unsigned long stack_value = 0x8877665544332211UL;
	volatile unsigned long *anonymous;
	volatile unsigned char *heap_probe;
	uintptr_t initial_brk;
	uintptr_t parent_brk;
	long mapping;
	long child;
	long waited;
	int status = 0;
	unsigned int round;

	mapping = syscall(SYS_mmap, NULL, 4096, PROT_READ | PROT_WRITE,
			  MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
	if (mapping < 0) {
		return 106;
	}
	anonymous = (volatile unsigned long *)(uintptr_t)mapping;
	*anonymous = 0xa5a5a5a55a5a5a5aUL;
	initial_brk = (uintptr_t)syscall(SYS_brk, 0);
	parent_brk = (initial_brk + 8191UL) & ~4095UL;
	if (initial_brk == 0 || parent_brk <= initial_brk ||
	    syscall(SYS_brk, parent_brk) != (long)parent_brk) {
		return 107;
	}
	heap_probe = (volatile unsigned char *)(parent_brk - 1);
	*heap_probe = 0x3c;

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
		*heap_probe = 0x7d;
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
	    *heap_probe != 0x3c ||
	    (uintptr_t)syscall(SYS_brk, 0) != parent_brk) {
		return 110;
	}
	initialized_data = 0x2233445566778899UL;
	zero_bss = 0xabcdef0123456789UL;
	stack_value = 0x21436587a9cbed0fUL;
	*anonymous = 0x5a5a5a5aa5a5a5a5UL;
	*heap_probe = 0x4e;
	if (initialized_data != 0x2233445566778899UL ||
	    zero_bss != 0xabcdef0123456789UL ||
	    stack_value != 0x21436587a9cbed0fUL ||
	    *anonymous != 0x5a5a5a5aa5a5a5a5UL || *heap_probe != 0x4e) {
		return 111;
	}
	if (SAY_LITERAL("user fork parent COW unique fast path ok\n") < 0) {
		return 112;
	}
	for (round = 0; round < 8; ++round) {
		unsigned long expected_data = 0x3300000000000000UL | round;
		unsigned long expected_bss = 0x4400000000000000UL | round;
		unsigned long expected_anon = 0x5500000000000000UL | round;
		unsigned char expected_heap = (unsigned char)(0x60U + round);
		volatile unsigned long round_stack = 0x6600000000000000UL | round;

		initialized_data = expected_data;
		zero_bss = expected_bss;
		*anonymous = expected_anon;
		*heap_probe = expected_heap;
		child = syscall(SYS_clone, SIGCHLD, 0, 0, 0, 0);
		if (child < 0) {
			return 115;
		}
		if (child == 0) {
			initialized_data = ~expected_data;
			zero_bss = ~expected_bss;
			round_stack = ~round_stack;
			*anonymous = ~expected_anon;
			*heap_probe = (unsigned char)~expected_heap;
			syscall(SYS_exit, 40 + round);
			__builtin_unreachable();
		}
		status = 0;
		waited = syscall(SYS_wait4, -1, &status, 0, NULL);
		if (waited != child || status != ((int)(40 + round) << 8) ||
		    initialized_data != expected_data || zero_bss != expected_bss ||
		    round_stack != (0x6600000000000000UL | round) ||
		    *anonymous != expected_anon || *heap_probe != expected_heap) {
			return 116;
		}
		initialized_data ^= 0x101UL;
		zero_bss ^= 0x202UL;
		round_stack ^= 0x303UL;
		*anonymous ^= 0x404UL;
		*heap_probe ^= 0x5U;
	}
	if (SAY_LITERAL("user fork COW reuse stress ok\n") < 0) {
		return 117;
	}
	if (expect_sigsegv(fault_unmapped_load) < 0) {
		return 120;
	}
	if (SAY_LITERAL("user SIGSEGV MAPERR wait ok\n") < 0) {
		return 121;
	}
	initialized_data ^= 0x1001UL;
	zero_bss ^= 0x2002UL;
	stack_value ^= 0x3003UL;
	*anonymous ^= 0x4004UL;
	*heap_probe ^= 0x5U;
	if (expect_sigsegv(fault_readonly_store) < 0 || readonly_probe != 0x5a) {
		return 122;
	}
	if (SAY_LITERAL("user SIGSEGV ACCERR readonly wait ok\n") < 0) {
		return 123;
	}
	initialized_data ^= 0x6006UL;
	zero_bss ^= 0x7007UL;
	stack_value ^= 0x8008UL;
	*anonymous ^= 0x9009UL;
	*heap_probe ^= 0xaU;
	if (expect_sigsegv(fault_nx_execute) < 0) {
		return 124;
	}
	if (SAY_LITERAL("user SIGSEGV ACCERR NX wait ok\n") < 0) {
		return 125;
	}
	initialized_data ^= 0xb00bUL;
	zero_bss ^= 0xc00cUL;
	stack_value ^= 0xd00dUL;
	*anonymous ^= 0xe00eUL;
	*heap_probe ^= 0xfU;
	if (readonly_probe != 0x5a || initialized_data == 0 || zero_bss == 0 ||
	    stack_value == 0 || *anonymous == 0 || *heap_probe == 0) {
		return 126;
	}
	if (SAY_LITERAL("user SIGSEGV parent mm survives ok\n") < 0) {
		return 127;
	}
	if (syscall(SYS_munmap, (void *)(uintptr_t)mapping, 4096) != 0) {
		return 118;
	}
	if (SAY_LITERAL("user fork parent isolation ok\n") < 0) {
		return 119;
	}
	return 0;
}
