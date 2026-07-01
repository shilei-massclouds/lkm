#include <errno.h>
#include <signal.h>
#include <stddef.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

#define FIRST_SIG_WORD(sig) (1UL << ((sig) - 1))
#define SA_UNSUPPORTED_FLAG 0x00000400UL

struct kernel_sigaction_smoke {
	void (*handler)(int);
	unsigned long flags;
	unsigned long mask;
};

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static void clear_action(struct kernel_sigaction_smoke *action)
{
	action->handler = SIG_DFL;
	action->flags = 0;
	action->mask = 0;
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
		return 81;
	}

	new_mask = FIRST_SIG_WORD(SIGUSR1) | FIRST_SIG_WORD(SIGKILL) |
		   FIRST_SIG_WORD(SIGSTOP);
	old_mask = ~0UL;
	rc = syscall(SYS_rt_sigprocmask, SIG_BLOCK, &new_mask, &old_mask,
		     sizeof(unsigned long));
	if (rc != 0 || old_mask != 0) {
		return 82;
	}

	new_mask = 0;
	old_mask = 0;
	rc = syscall(SYS_rt_sigprocmask, SIG_SETMASK, &new_mask, &old_mask,
		     sizeof(unsigned long));
	if (rc != 0 || old_mask != FIRST_SIG_WORD(SIGUSR1)) {
		return 83;
	}

	if (SAY_LITERAL("syscall rt_sigprocmask ok\n") < 0) {
		return 84;
	}
	return 0;
}

static int smoke_rt_sigaction(void)
{
	struct kernel_sigaction_smoke action;
	struct kernel_sigaction_smoke old_action;
	long rc;

	old_action.handler = (void (*)(int))1;
	old_action.flags = ~0UL;
	old_action.mask = ~0UL;
	rc = syscall(SYS_rt_sigaction, SIGUSR1, NULL, &old_action,
		     sizeof(unsigned long));
	if (rc != 0 || old_action.handler != SIG_DFL || old_action.flags != 0 ||
	    old_action.mask != 0) {
		return 85;
	}

	action.handler = SIG_IGN;
	action.flags = SA_RESTART | SA_UNSUPPORTED_FLAG;
	action.mask = FIRST_SIG_WORD(SIGUSR2) | FIRST_SIG_WORD(SIGKILL) |
		      FIRST_SIG_WORD(SIGSTOP);
	old_action.handler = (void (*)(int))1;
	old_action.flags = ~0UL;
	old_action.mask = ~0UL;
	rc = syscall(SYS_rt_sigaction, SIGUSR1, &action, &old_action,
		     sizeof(unsigned long));
	if (rc != 0 || old_action.handler != SIG_DFL || old_action.flags != 0 ||
	    old_action.mask != 0) {
		return 86;
	}

	clear_action(&old_action);
	old_action.handler = (void (*)(int))1;
	rc = syscall(SYS_rt_sigaction, SIGUSR1, NULL, &old_action,
		     sizeof(unsigned long));
	if (rc != 0 || old_action.handler != SIG_IGN ||
	    old_action.flags != SA_RESTART ||
	    old_action.mask != FIRST_SIG_WORD(SIGUSR2)) {
		return 87;
	}

	errno = 0;
	rc = syscall(SYS_rt_sigaction, SIGUSR1, NULL, NULL,
		     sizeof(unsigned long) * 2);
	if (rc != -1 || errno != EINVAL) {
		return 88;
	}

	errno = 0;
	rc = syscall(SYS_rt_sigaction, SIGKILL, &action, NULL,
		     sizeof(unsigned long));
	if (rc != -1 || errno != EINVAL) {
		return 89;
	}

	clear_action(&action);
	rc = syscall(SYS_rt_sigaction, SIGUSR1, &action, NULL,
		     sizeof(unsigned long));
	if (rc != 0) {
		return 90;
	}

	if (SAY_LITERAL("syscall rt_sigaction ok\n") < 0) {
		return 91;
	}
	return 0;
}

int smoke_signal(void)
{
	int status;

	status = smoke_rt_sigprocmask();
	if (status != 0) {
		return status;
	}
	status = smoke_rt_sigaction();
	if (status != 0) {
		return status;
	}
	return 0;
}
