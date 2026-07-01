#include <stddef.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <sys/utsname.h>
#include <time.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int str_eq(const char *left, const char *right)
{
	while (*left != '\0' && *right != '\0') {
		if (*left != *right) {
			return 0;
		}
		left++;
		right++;
	}
	return *left == '\0' && *right == '\0';
}

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
	uid_t ruid;
	uid_t euid;
	uid_t suid;
	gid_t rgid;
	gid_t egid;
	gid_t sgid;
	long rc;

	rc = syscall(SYS_getuid);
	if (rc != 0) {
		return 44;
	}
	if (SAY_LITERAL("syscall getuid ok\n") < 0) {
		return 45;
	}

	rc = syscall(SYS_geteuid);
	if (rc != 0) {
		return 65;
	}
	if (SAY_LITERAL("syscall geteuid ok\n") < 0) {
		return 66;
	}

	rc = syscall(SYS_getgid);
	if (rc != 0) {
		return 46;
	}
	if (SAY_LITERAL("syscall getgid ok\n") < 0) {
		return 47;
	}

	rc = syscall(SYS_getegid);
	if (rc != 0) {
		return 67;
	}
	if (SAY_LITERAL("syscall getegid ok\n") < 0) {
		return 68;
	}

	ruid = 1;
	euid = 1;
	suid = 1;
	rc = syscall(SYS_getresuid, &ruid, &euid, &suid);
	if (rc != 0 || ruid != 0 || euid != 0 || suid != 0) {
		return 69;
	}
	if (SAY_LITERAL("syscall getresuid ok\n") < 0) {
		return 70;
	}

	rgid = 1;
	egid = 1;
	sgid = 1;
	rc = syscall(SYS_getresgid, &rgid, &egid, &sgid);
	if (rc != 0 || rgid != 0 || egid != 0 || sgid != 0) {
		return 71;
	}
	if (SAY_LITERAL("syscall getresgid ok\n") < 0) {
		return 72;
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

static int smoke_process_identity(void)
{
	int pgrp;
	long rc;

	rc = syscall(SYS_getpid);
	if (rc != 1) {
		return 73;
	}
	if (SAY_LITERAL("syscall getpid ok\n") < 0) {
		return 74;
	}

	rc = syscall(SYS_getppid);
	if (rc != 0) {
		return 75;
	}
	if (SAY_LITERAL("syscall getppid ok\n") < 0) {
		return 76;
	}

	rc = syscall(SYS_getpgid, 0);
	if (rc != 1) {
		return 81;
	}
	if (SAY_LITERAL("syscall getpgid ok\n") < 0) {
		return 82;
	}

	rc = syscall(SYS_setpgid, 0, 1);
	if (rc != 0) {
		return 83;
	}
	if (SAY_LITERAL("syscall setpgid ok\n") < 0) {
		return 84;
	}

	pgrp = 1;
	rc = ioctl(STDOUT_FILENO, TIOCSPGRP, &pgrp);
	if (rc != 0) {
		return 85;
	}
	if (SAY_LITERAL("syscall ioctl TIOCSPGRP ok\n") < 0) {
		return 86;
	}

	return 0;
}

static int smoke_uts_and_cwd(void)
{
	struct utsname uts;
	char cwd[8];
	long rc;

	rc = syscall(SYS_uname, &uts);
	if (rc != 0 || !str_eq(uts.sysname, "Linux") ||
	    !str_eq(uts.machine, "riscv64")) {
		return 77;
	}
	if (SAY_LITERAL("syscall uname ok\n") < 0) {
		return 78;
	}

	cwd[0] = 0;
	cwd[1] = 0;
	rc = syscall(SYS_getcwd, cwd, sizeof(cwd));
	if (rc != 2 || cwd[0] != '/' || cwd[1] != '\0') {
		return 79;
	}
	if (SAY_LITERAL("syscall getcwd ok\n") < 0) {
		return 80;
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
	status = smoke_credentials();
	if (status != 0) {
		return status;
	}
	status = smoke_process_identity();
	if (status != 0) {
		return status;
	}
	status = smoke_uts_and_cwd();
	if (status != 0) {
		return status;
	}
	status = smoke_time_syscalls();
	if (status != 0) {
		return status;
	}
	status = smoke_getrandom();
	if (status != 0) {
		return status;
	}
	return 0;
}
