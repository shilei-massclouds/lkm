#include <stddef.h>
#include <stdint.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int smoke_user_fpu(void)
{
#if defined(__riscv) && defined(__riscv_flen) && __riscv_flen >= 64
	unsigned long pattern = 0x3ff0000000000000UL;
	unsigned long out = 0;

	__asm__ volatile("fmv.d.x ft0, %1\n\t"
			 "fmv.x.d %0, ft0\n\t"
			 : "=r"(out)
			 : "r"(pattern)
			 : "ft0");
	if (out != pattern) {
		return 62;
	}
	if (SAY_LITERAL("user fpu ok\n") < 0) {
		return 63;
	}
	return 0;
#else
	return 64;
#endif
}

static int smoke_mmap_fixed_prot_none(void)
{
	void *fixed = (void *)0x30000000UL;
	long rc;

	rc = syscall(SYS_mmap, fixed, 4096, PROT_NONE,
		     MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS, -1, 0);
	if (rc != (long)(unsigned long)fixed) {
		return 95;
	}
	if (SAY_LITERAL("syscall mmap MAP_FIXED PROT_NONE ok\n") < 0) {
		return 96;
	}
	return 0;
}

static int smoke_anonymous_demand_faults(void)
{
	volatile unsigned char *bytes;
	long rc;

	rc = syscall(SYS_mmap, NULL, 8192, PROT_READ | PROT_WRITE,
		     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
	if (rc < 0) {
		return 97;
	}
	bytes = (volatile unsigned char *)(uintptr_t)rc;
	if (bytes[0] != 0 || bytes[4096] != 0) {
		return 98;
	}
	bytes[0] = 0x35;
	bytes[4096] = 0xa7;
	if (bytes[0] != 0x35 || bytes[4096] != 0xa7) {
		return 99;
	}
	if (SAY_LITERAL("user anonymous demand faults ok\n") < 0) {
		return 100;
	}
	return 0;
}

static int smoke_brk_demand_fault(void)
{
	volatile unsigned char *last;
	uintptr_t current;
	uintptr_t target;
	long rc;

	rc = syscall(SYS_brk, 0);
	if (rc <= 0) {
		return 101;
	}
	current = (uintptr_t)rc;
	target = ((current + 4095UL) & ~4095UL) + 4096UL;
	if (target <= current || syscall(SYS_brk, target) != (long)target) {
		return 102;
	}
	last = (volatile unsigned char *)(target - 1);
	if (*last != 0) {
		return 103;
	}
	*last = 0x6d;
	if (*last != 0x6d) {
		return 104;
	}
	if (SAY_LITERAL("user brk demand fault ok\n") < 0) {
		return 105;
	}
	return 0;
}

int smoke_fpu_mmap(void)
{
	int status = smoke_user_fpu();

	if (status != 0) {
		return status;
	}
	status = smoke_mmap_fixed_prot_none();
	if (status != 0) {
		return status;
	}
	status = smoke_anonymous_demand_faults();
	if (status != 0) {
		return status;
	}
	return smoke_brk_demand_fault();
}
