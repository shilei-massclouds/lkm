#include <errno.h>
#include <stddef.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

#ifndef SYS_getsid
#define SYS_getsid 156
#endif

#ifndef SYS_sched_getaffinity
#define SYS_sched_getaffinity 123
#endif

#ifndef TIOCSCTTY
#define TIOCSCTTY 0x540e
#endif

#ifndef TIOCGSID
#define TIOCGSID 0x5429
#endif

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_process_identity(void)
{
	unsigned long affinity[2] = { 0, 0xa5a5a5a5a5a5a5a5UL };
	int pgrp;
	int sid;
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

	rc = syscall(SYS_sched_getaffinity, 0, sizeof(affinity), affinity);
	if (rc != (long)sizeof(affinity[0]) || affinity[0] != 0xffUL ||
	    affinity[1] != 0xa5a5a5a5a5a5a5a5UL) {
		return 98;
	}
	affinity[0] = 0;
	rc = syscall(SYS_sched_getaffinity, 1, sizeof(affinity[0]), affinity);
	if (rc != (long)sizeof(affinity[0]) || affinity[0] != 0xffUL) {
		return 99;
	}
	errno = 0;
	rc = syscall(SYS_sched_getaffinity, 0, 4, affinity);
	if (rc != -1 || errno != EINVAL) {
		return 100;
	}
	errno = 0;
	rc = syscall(SYS_sched_getaffinity, 999, sizeof(affinity[0]), affinity);
	if (rc != -1 || errno != ESRCH) {
		return 101;
	}
	errno = 0;
	rc = syscall(SYS_sched_getaffinity, 0, sizeof(affinity[0]), (void *)1);
	if (rc != -1 || errno != EFAULT) {
		return 102;
	}
	if (SAY_LITERAL("syscall sched_getaffinity ok\n") < 0) {
		return 103;
	}

	rc = syscall(SYS_getpgid, 0);
	if (rc != 1) {
		return 81;
	}
	if (SAY_LITERAL("syscall getpgid ok\n") < 0) {
		return 82;
	}

	rc = syscall(SYS_getsid, 0);
	if (rc != 1) {
		return 89;
	}
	rc = syscall(SYS_getsid, 1);
	if (rc != 1) {
		return 90;
	}
	errno = 0;
	rc = syscall(SYS_getsid, 999);
	if (rc != -1 || errno != ESRCH) {
		return 91;
	}
	if (SAY_LITERAL("syscall getsid ok\n") < 0) {
		return 92;
	}

	rc = syscall(SYS_setpgid, 0, 1);
	if (rc != 0) {
		return 83;
	}
	if (SAY_LITERAL("syscall setpgid ok\n") < 0) {
		return 84;
	}

	errno = 0;
	rc = syscall(SYS_setsid);
	if (rc != -1 || errno != EPERM) {
		return 85;
	}
	if (SAY_LITERAL("syscall setsid pgrp leader eperm ok\n") < 0) {
		return 86;
	}

	sid = 0;
	rc = ioctl(STDOUT_FILENO, TIOCGSID, &sid);
	if (rc != 0 || sid != 1) {
		return 93;
	}
	errno = 0;
	rc = ioctl(STDOUT_FILENO, TIOCGSID, (void *)1);
	if (rc != -1 || errno != EFAULT) {
		return 94;
	}
	if (SAY_LITERAL("syscall ioctl TIOCGSID ok\n") < 0) {
		return 95;
	}

	errno = 0;
	rc = ioctl(STDOUT_FILENO, TIOCSCTTY, 1);
	if (rc != -1 || errno != EPERM) {
		return 96;
	}
	if (SAY_LITERAL("syscall ioctl TIOCSCTTY eperm ok\n") < 0) {
		return 97;
	}

	pgrp = 1;
	rc = ioctl(STDOUT_FILENO, TIOCSPGRP, &pgrp);
	if (rc != 0) {
		return 87;
	}
	if (SAY_LITERAL("syscall ioctl TIOCSPGRP ok\n") < 0) {
		return 88;
	}

	return 0;
}
