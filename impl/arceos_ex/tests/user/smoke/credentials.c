#include <stddef.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_credentials(void)
{
	uid_t ruid;
	uid_t euid;
	uid_t suid;
	gid_t rgid;
	gid_t egid;
	gid_t sgid;
	gid_t groups[1];
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

	rc = syscall(SYS_setgid, 100);
	if (rc != 0) {
		return 48;
	}
	if (SAY_LITERAL("syscall setgid ok\n") < 0) {
		return 49;
	}

	rgid = 0;
	egid = 0;
	sgid = 0;
	rc = syscall(SYS_getresgid, &rgid, &egid, &sgid);
	if (rc != 0 || rgid != 100 || egid != 100 || sgid != 100) {
		return 75;
	}
	if (SAY_LITERAL("syscall getresgid setgid ok\n") < 0) {
		return 76;
	}

	rc = syscall(SYS_setgid, 0);
	if (rc != 0) {
		return 77;
	}
	if (SAY_LITERAL("syscall setgid restore ok\n") < 0) {
		return 78;
	}

	groups[0] = 0;
	rc = syscall(SYS_setgroups, 1, groups);
	if (rc != 0) {
		return 73;
	}
	if (SAY_LITERAL("syscall setgroups ok\n") < 0) {
		return 74;
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
