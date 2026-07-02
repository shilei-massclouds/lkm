#include <stddef.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_tty_termios(void)
{
	struct termios original;
	struct termios updated;
	struct termios observed;
	long rc;

	rc = ioctl(STDOUT_FILENO, TCGETS, &original);
	if (rc != 0) {
		return 87;
	}

	updated = original;
	updated.c_lflag ^= ECHO;
	rc = ioctl(STDOUT_FILENO, TCSETS, &updated);
	if (rc != 0) {
		return 88;
	}

	rc = ioctl(STDOUT_FILENO, TCGETS, &observed);
	if (rc != 0) {
		return 89;
	}
	if ((observed.c_lflag & ECHO) != (updated.c_lflag & ECHO)) {
		return 90;
	}

	rc = ioctl(STDOUT_FILENO, TCSETS, &original);
	if (rc != 0) {
		return 91;
	}
	if (SAY_LITERAL("syscall ioctl TCSETS ok\n") < 0) {
		return 92;
	}

	return 0;
}
