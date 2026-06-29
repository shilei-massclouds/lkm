#include <fcntl.h>
#include <errno.h>
#include <stddef.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/ioctl.h>
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
	struct winsize ws;

	if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) < 0) {
		return 33;
	}
	if (ws.ws_row == 0 || ws.ws_col == 0) {
		return 34;
	}
	if (SAY_LITERAL("syscall ioctl TIOCGWINSZ stdout ok\n") < 0) {
		return 35;
	}

	if (faccessat(AT_FDCWD, path, F_OK | R_OK, 0) < 0) {
		return 36;
	}
	if (SAY_LITERAL("syscall faccessat F_OK R_OK regular ok\n") < 0) {
		return 37;
	}
	errno = 0;
	if (faccessat(AT_FDCWD, path, W_OK, 0) == 0 || errno != EACCES) {
		return 38;
	}
	if (SAY_LITERAL("syscall faccessat W_OK denied ok\n") < 0) {
		return 39;
	}

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

	if (fcntl(fd, F_GETFL, 0) < 0) {
		return 28;
	}
	if (SAY_LITERAL("syscall fcntl F_GETFL regular ok\n") < 0) {
		return 29;
	}

	if (lseek(fd, 0, SEEK_SET) != 0) {
		return 30;
	}
	if (SAY_LITERAL("syscall lseek regular ok\n") < 0) {
		return 31;
	}
	read_len = read(fd, read_buf, 2);
	if (read_len != 2 || read_buf[0] != '3' || read_buf[1] != '.') {
		return 32;
	}

	if (syscall(SYS_fstat, fd, &st) < 0) {
		return 25;
	}
	if (!S_ISREG(st.st_mode) || st.st_size <= 0) {
		return 26;
	}
	if (SAY_LITERAL("syscall fstat regular ok\n") < 0) {
		return 27;
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
