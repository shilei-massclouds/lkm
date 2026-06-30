#include <stddef.h>
#include <signal.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

#define FIRST_SIG_WORD(sig) (1UL << ((sig) - 1))

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

static int smoke_credentials(void)
{
	long rc;

	rc = syscall(SYS_getuid);
	if (rc != 0) {
		return 44;
	}
	if (SAY_LITERAL("syscall getuid ok\n") < 0) {
		return 45;
	}

	rc = syscall(SYS_getgid);
	if (rc != 0) {
		return 46;
	}
	if (SAY_LITERAL("syscall getgid ok\n") < 0) {
		return 47;
	}

	rc = syscall(SYS_setgid, 0);
	if (rc != 0) {
		return 48;
	}
	if (SAY_LITERAL("syscall setgid ok\n") < 0) {
		return 49;
	}

	rc = syscall(SYS_setuid, 0);
	if (rc != 0) {
		return 50;
	}
	if (SAY_LITERAL("syscall setuid ok\n") < 0) {
		return 51;
	}

	return 0;
}

static int smoke_rt_sigprocmask(void)
{
	unsigned long new_mask;
	unsigned long old_mask;
	long rc;

	new_mask = 0;
	old_mask = ~0UL;
	rc = syscall(SYS_rt_sigprocmask, SIG_SETMASK, &new_mask, &old_mask,
		     sizeof(unsigned long));
	if (rc != 0 || old_mask != 0) {
		return 52;
	}

	new_mask = FIRST_SIG_WORD(SIGUSR1) | FIRST_SIG_WORD(SIGKILL) |
		   FIRST_SIG_WORD(SIGSTOP);
	old_mask = ~0UL;
	rc = syscall(SYS_rt_sigprocmask, SIG_BLOCK, &new_mask, &old_mask,
		     sizeof(unsigned long));
	if (rc != 0 || old_mask != 0) {
		return 53;
	}

	new_mask = 0;
	old_mask = 0;
	rc = syscall(SYS_rt_sigprocmask, SIG_SETMASK, &new_mask, &old_mask,
		     sizeof(unsigned long));
	if (rc != 0 || old_mask != FIRST_SIG_WORD(SIGUSR1)) {
		return 54;
	}

	if (SAY_LITERAL("syscall rt_sigprocmask ok\n") < 0) {
		return 55;
	}
	return 0;
}

static int smoke_time_syscalls(void)
{
	struct timespec ts;
	struct timeval tv;
	struct timezone tz;
	long rc;

	ts.tv_nsec = -1;
	rc = syscall(SYS_clock_gettime, CLOCK_MONOTONIC, &ts);
	if (rc != 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1000000000L) {
		return 56;
	}
	if (SAY_LITERAL("syscall clock_gettime monotonic ok\n") < 0) {
		return 57;
	}

	ts.tv_nsec = -1;
	rc = syscall(SYS_clock_gettime, CLOCK_REALTIME, &ts);
	if (rc != 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1000000000L) {
		return 58;
	}
	if (SAY_LITERAL("syscall clock_gettime realtime ok\n") < 0) {
		return 59;
	}

	tv.tv_usec = -1;
	tz.tz_minuteswest = -1;
	tz.tz_dsttime = -1;
	rc = syscall(SYS_gettimeofday, &tv, &tz);
	if (rc != 0 || tv.tv_usec < 0 || tv.tv_usec >= 1000000L ||
	    tz.tz_minuteswest != 0 || tz.tz_dsttime != 0) {
		return 60;
	}
	if (SAY_LITERAL("syscall gettimeofday ok\n") < 0) {
		return 61;
	}

	return 0;
}

static int smoke_getrandom(void)
{
	unsigned char random_bytes[16];
	long rc;

	rc = syscall(SYS_getrandom, random_bytes, sizeof(random_bytes), 0);
	if (rc != (long)sizeof(random_bytes)) {
		return 42;
	}
	if (SAY_LITERAL("syscall getrandom ok\n") < 0) {
		return 43;
	}
	return 0;
}

int smoke_sh_probe(void)
{
	int status;

	if (SAY_LITERAL("sh_probe entry ok\n") < 0) {
		return 41;
	}
	status = smoke_user_fpu();
	if (status != 0) {
		return status;
	}
	if (smoke_credentials() != 0) {
		return 44;
	}
	if (smoke_rt_sigprocmask() != 0) {
		return 52;
	}
	status = smoke_time_syscalls();
	if (status != 0) {
		return status;
	}
	if (smoke_getrandom() != 0) {
		return 42;
	}
	return 0;
}
