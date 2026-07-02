#include <stddef.h>
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

int smoke_fpu_mmap(void)
{
	int status = smoke_user_fpu();

	if (status != 0) {
		return status;
	}
	return smoke_mmap_fixed_prot_none();
}
