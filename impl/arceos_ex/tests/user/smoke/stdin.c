#include <errno.h>
#include <stddef.h>
#include <poll.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_stdin(void)
{
	char buffer[16];
	static const char expected[] = "stdin\n";
	struct pollfd pfd;
	struct timespec timeout;
	long poll_rc;
	ssize_t read_len;
	size_t i;

	pfd.fd = STDIN_FILENO;
	pfd.events = POLLIN;
	pfd.revents = 0;
	timeout.tv_sec = 0;
	timeout.tv_nsec = 0;
	poll_rc = syscall(SYS_ppoll, &pfd, 1, &timeout, NULL,
			  sizeof(unsigned long));
	if (poll_rc != 1 || (pfd.revents & POLLIN) == 0) {
		return 84;
	}
	if (SAY_LITERAL("syscall ppoll stdin ok\n") < 0) {
		return 85;
	}

	read_len = read(STDIN_FILENO, buffer, sizeof(buffer));
	if (read_len != (ssize_t)(sizeof(expected) - 1)) {
		return 81;
	}

	for (i = 0; i < sizeof(expected) - 1; i++) {
		if (buffer[i] != expected[i]) {
			return 82;
		}
	}

	if (SAY_LITERAL("syscall read stdin ok\n") < 0) {
		return 83;
	}

	pfd.fd = STDIN_FILENO;
	pfd.events = POLLIN;
	pfd.revents = 0;
	errno = 0;
	poll_rc = syscall(SYS_ppoll, &pfd, 1, NULL, NULL,
			  sizeof(unsigned long));
	if (poll_rc != -1 || errno != ENOSYS) {
		return 86;
	}
	if (pfd.revents != 0) {
		return 87;
	}
	if (SAY_LITERAL("syscall ppoll stdin no-ready out-of-slice ok\n") < 0) {
		return 88;
	}

	return 0;
}
