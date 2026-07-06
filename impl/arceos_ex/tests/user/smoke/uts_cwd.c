#include <stddef.h>
#include <sys/syscall.h>
#include <sys/utsname.h>
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

int smoke_uts_cwd(void)
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

	rc = syscall(SYS_chdir, "/");
	if (rc != 0) {
		return 81;
	}
	if (SAY_LITERAL("syscall chdir root ok\n") < 0) {
		return 82;
	}

	cwd[0] = 0;
	cwd[1] = 0;
	rc = syscall(SYS_getcwd, cwd, sizeof(cwd));
	if (rc != 2 || cwd[0] != '/' || cwd[1] != '\0') {
		return 83;
	}

	return 0;
}
