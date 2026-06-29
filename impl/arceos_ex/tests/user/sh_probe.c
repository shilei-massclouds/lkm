#include <fcntl.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

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

static int probe_directory_enumeration(void)
{
	char dir_buf[512];
	int saw_dot = 0;
	int saw_dotdot = 0;
	int dir_fd = syscall(SYS_openat, AT_FDCWD, "/", O_RDONLY | O_DIRECTORY, 0);

	if (dir_fd < 0) {
		return 40;
	}
	if (SAY_LITERAL("syscall openat directory ok\n") < 0) {
		return 41;
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

int main(int argc, char **argv)
{
	static const char path[] = "/etc/alpine-release";
	static const char message[] = "user hello\n";
	char read_buf[32];
	struct stat st;

	if (argc < 1 || argv == NULL || argv[0] == NULL) {
		return 30;
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
	if (say(message, sizeof(message) - 1) < 0) {
		return 16;
	}

	return probe_directory_enumeration();
}
