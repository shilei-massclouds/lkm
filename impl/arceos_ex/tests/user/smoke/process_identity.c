#include <errno.h>
#include <stddef.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_process_identity(void)
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

	errno = 0;
	rc = syscall(SYS_setsid);
	if (rc != -1 || errno != EPERM) {
		return 85;
	}
	if (SAY_LITERAL("syscall setsid pgrp leader eperm ok\n") < 0) {
		return 86;
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
