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
	if (fcntl(fd, F_SETFL, O_RDWR | O_NONBLOCK | O_LARGEFILE) < 0) {
		return 95;
	}
	flags = fcntl(fd, F_GETFL, 0);
	if (flags < 0 || (flags & O_NONBLOCK) == 0 ||
	    (flags & O_CLOEXEC) != 0 || (flags & O_ACCMODE) != O_RDWR) {
		return 96;
	}
	if (fcntl(fd, F_SETFL, O_RDWR | O_LARGEFILE) < 0) {
		return 97;
	}
	flags = fcntl(fd, F_GETFL, 0);
	if (flags < 0 || (flags & O_NONBLOCK) != 0 ||
	    (flags & O_CLOEXEC) != 0 || (flags & O_ACCMODE) != O_RDWR) {
		return 98;
	}
	if (close(fd) < 0) {
		return 99;
	}

	fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty",
		     O_RDWR | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd < 0) {
		return 100;
	}
	if (close(fd) < 0) {
		return 101;
	}

	errno = 0;
	fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty1",
		     O_RDONLY | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd >= 0) {
		close(fd);
		return 102;
	}

	errno = 0;
	fd = syscall(SYS_openat, AT_FDCWD, "/dev/tty1",
		     O_RDWR | O_DIRECTORY | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd >= 0) {
		close(fd);
		return 103;
	}
	if (errno != EINVAL) {
		return 104;
	}

	errno = 0;
	fd = syscall(SYS_openat, AT_FDCWD, "/etc/alpine-release",
		     O_RDONLY | O_NONBLOCK | O_LARGEFILE, 0);
	if (fd >= 0) {
		close(fd);
		return 105;
	}
	if (errno != EINVAL) {
		return 106;
	}

	fd = syscall(SYS_openat, AT_FDCWD, "/etc/alpine-release",
		     O_RDONLY | O_LARGEFILE, 0);
	if (fd < 0) {
		return 107;
	}
	errno = 0;
	if (fcntl(fd, F_SETFL, O_RDONLY | O_NONBLOCK | O_LARGEFILE) >= 0) {
		close(fd);
		return 108;
	}
	if (errno != EINVAL) {
		close(fd);
		return 109;
	}
	if (close(fd) < 0) {
		return 110;
	}

	fd = syscall(SYS_openat, AT_FDCWD, "/",
		     O_RDONLY | O_DIRECTORY | O_CLOEXEC, 0);
	if (fd < 0) {
		return 111;
	}
	errno = 0;
	if (fcntl(fd, F_SETFL,
		  O_RDONLY | O_DIRECTORY | O_NONBLOCK | O_LARGEFILE) >= 0) {
		close(fd);
		return 112;
	}
	if (errno != EINVAL) {
		close(fd);
		return 113;
	}
	if (close(fd) < 0) {
		return 114;
	}

	errno = 0;
	if (fcntl(99, F_SETFL, O_NONBLOCK) >= 0) {
		return 115;
	}
	if (errno != EBADF) {
		return 116;
	}

	if (SAY_LITERAL("syscall fcntl F_SETFL tty nonblock ok\n") < 0) {
		return 117;
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
