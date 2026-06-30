#include <stddef.h>
#include <signal.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

#define FIRST_SIG_WORD(sig) (1UL << ((sig) - 1))

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
	if (SAY_LITERAL("sh_probe entry ok\n") < 0) {
		return 41;
	}
	if (smoke_credentials() != 0) {
		return 44;
	}
	if (smoke_rt_sigprocmask() != 0) {
		return 52;
	}
	if (smoke_getrandom() != 0) {
		return 42;
	}
	return 0;
}
