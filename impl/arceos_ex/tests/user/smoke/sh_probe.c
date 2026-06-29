#include <fcntl.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

struct linux_dirent64_probe {
	uint64_t d_ino;
	int64_t d_off;
	unsigned short d_reclen;
	unsigned char d_type;
	char d_name[];
};

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int name_is_dot(const char *name)
{
	return name[0] == '.' && name[1] == '\0';
}

static int name_is_dotdot(const char *name)
{
	return name[0] == '.' && name[1] == '.' && name[2] == '\0';
}

int smoke_sh_probe(void)
{
	char dir_buf[512];
	int saw_dot = 0;
	int saw_dotdot = 0;
	struct stat st;
	int dir_fd = syscall(SYS_openat, AT_FDCWD, "/", O_RDONLY | O_DIRECTORY, 0);

	if (dir_fd < 0) {
		return 40;
	}
	if (SAY_LITERAL("syscall openat directory ok\n") < 0) {
		return 41;
	}

	if (fstatat(AT_FDCWD, "/", &st, 0) < 0) {
		return 49;
	}
	if (!S_ISDIR(st.st_mode) || st.st_size <= 0) {
		return 50;
	}
	if (SAY_LITERAL("syscall newfstatat directory ok\n") < 0) {
		return 51;
	}

	if (syscall(SYS_fstat, dir_fd, &st) < 0) {
		return 52;
	}
	if (!S_ISDIR(st.st_mode) || st.st_size <= 0) {
		return 53;
	}
	if (SAY_LITERAL("syscall fstat directory ok\n") < 0) {
		return 54;
	}

	long bytes = syscall(SYS_getdents64, dir_fd, dir_buf, sizeof(dir_buf));
	if (bytes <= 0) {
		return 42;
	}
	if (SAY_LITERAL("syscall getdents64 ok\n") < 0) {
		return 43;
	}

	for (long offset = 0; offset < bytes;) {
		struct linux_dirent64_probe *entry =
			(struct linux_dirent64_probe *)(void *)(dir_buf + offset);

		if (entry->d_reclen < offsetof(struct linux_dirent64_probe, d_name) + 1 ||
		    offset + entry->d_reclen > bytes) {
			return 44;
		}
		if (name_is_dot(entry->d_name)) {
			saw_dot = 1;
		}
		if (name_is_dotdot(entry->d_name)) {
			saw_dotdot = 1;
		}
		offset += entry->d_reclen;
	}

	if (!saw_dot || !saw_dotdot) {
		return 45;
	}
	if (SAY_LITERAL("syscall getdents64 parse ok\n") < 0) {
		return 46;
	}
	if (close(dir_fd) < 0) {
		return 47;
	}
	if (SAY_LITERAL("syscall close directory ok\n") < 0) {
		return 48;
	}

	return 0;
}
