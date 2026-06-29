#include <fcntl.h>
#include <stddef.h>
#include <sys/stat.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_init_fileio(void)
{
	static const char path[] = "/etc/alpine-release";
	char read_buf[32];
	struct stat st;

	int fd = open(path, O_RDONLY);
	if (fd < 0) {
		return 20;
	}
	if (SAY_LITERAL("syscall openat ok\n") < 0) {
		return 21;
	}

	ssize_t read_len = read(fd, read_buf, sizeof(read_buf));
	if (read_len <= 0 || read_buf[0] != '3' || read_buf[1] != '.') {
		return 11;
	}
	if (SAY_LITERAL("syscall read ok\n") < 0) {
		return 22;
	}

	if (close(fd) < 0) {
		return 12;
	}
	if (SAY_LITERAL("syscall close ok\n") < 0) {
		return 23;
	}

	if (fstatat(AT_FDCWD, path, &st, 0) < 0) {
		return 13;
	}
	if (!S_ISREG(st.st_mode) || st.st_size <= 0) {
		return 15;
	}
	if (SAY_LITERAL("syscall newfstatat ok\n") < 0) {
		return 24;
	}

	if (SAY_LITERAL("syscall write ok\n") < 0) {
		return 14;
	}

	return 0;
}
