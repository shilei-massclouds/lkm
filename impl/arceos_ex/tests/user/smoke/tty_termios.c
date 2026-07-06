#include <errno.h>
#include <fcntl.h>
#include <stddef.h>
#include <sys/syscall.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>

#include "smoke.h"

#ifndef O_LARGEFILE
#define O_LARGEFILE 0
#endif

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int smoke_tty_open_alias(void)
{
	int fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty1",
			 O_RDWR | O_NONBLOCK | O_LARGEFILE | O_CLOEXEC, 0);
	if (fd < 0) {
		return 93;
	}
	int flags = fcntl(fd, F_GETFL, 0);
	if (flags < 0 || (flags & O_NONBLOCK) == 0 ||
	    (flags & O_CLOEXEC) != 0) {
		return 94;
	}
	if (close(fd) < 0) {
		return 95;
	}

	fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty",
		     O_RDWR | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd < 0) {
		return 96;
	}
	if (close(fd) < 0) {
		return 97;
	}

	errno = 0;
	fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty1",
		     O_RDONLY | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd >= 0) {
		close(fd);
		return 98;
	}

	errno = 0;
	fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty1",
		     O_RDWR | O_DIRECTORY | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd >= 0) {
		close(fd);
		return 99;
	}
	if (errno != EINVAL) {
		return 100;
	}

	errno = 0;
	fd = syscall(SYS_openat, AT_FDCWD, "/etc/alpine-release",
		     O_RDONLY | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd >= 0) {
		close(fd);
		return 101;
	}
	if (errno != EINVAL) {
		return 102;
	}

	if (SAY_LITERAL("syscall openat tty alias nonblock ok\n") < 0) {
		return 103;
	}

	return 0;
}

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

	return smoke_tty_open_alias();
}
