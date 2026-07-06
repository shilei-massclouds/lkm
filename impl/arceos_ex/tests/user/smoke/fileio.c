#include <fcntl.h>
#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/ioctl.h>
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

struct linux_dirent64_probe {
	uint64_t d_ino;
	int64_t d_off;
	unsigned short d_reclen;
	unsigned char d_type;
	char d_name[];
};

static int name_is_dot(const char *name)
{
	return name[0] == '.' && name[1] == '\0';
}

static int name_is_dotdot(const char *name)
{
	return name[0] == '.' && name[1] == '.' && name[2] == '\0';
}

static int smoke_directory_fileio(void)
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

	if (fstatat(AT_FDCWD, ".", &st, 0) < 0) {
		return 64;
	}
	if (!S_ISDIR(st.st_mode) || st.st_size <= 0) {
		return 65;
	}
	if (SAY_LITERAL("syscall newfstatat cwd dot ok\n") < 0) {
		return 66;
	}

	if (fstatat(AT_FDCWD, "./bin", &st, AT_SYMLINK_NOFOLLOW) < 0) {
		return 71;
	}
	if (!S_ISDIR(st.st_mode) || st.st_size <= 0) {
		return 72;
	}
	if (SAY_LITERAL("syscall newfstatat cwd dot component ok\n") < 0) {
		return 73;
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

	int flags = fcntl(dir_fd, F_GETFL, 0);
	if (flags < 0 || (flags & O_DIRECTORY) == 0) {
		return 55;
	}
	if (SAY_LITERAL("syscall fcntl F_GETFL directory ok\n") < 0) {
		return 56;
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

	if (lseek(dir_fd, 0, SEEK_SET) != 0) {
		return 57;
	}
	if (SAY_LITERAL("syscall lseek directory ok\n") < 0) {
		return 58;
	}
	long bytes_again = syscall(SYS_getdents64, dir_fd, dir_buf, sizeof(dir_buf));
	if (bytes_again <= 0) {
		return 59;
	}

	if (close(dir_fd) < 0) {
		return 47;
	}
	if (SAY_LITERAL("syscall close directory ok\n") < 0) {
		return 48;
	}

	int plain_dir_fd = syscall(SYS_openat, AT_FDCWD, "/",
				   O_RDONLY | O_LARGEFILE, 0);
	if (plain_dir_fd < 0) {
		return 83;
	}
	if (syscall(SYS_fstat, plain_dir_fd, &st) < 0) {
		return 84;
	}
	if (!S_ISDIR(st.st_mode) || st.st_size <= 0) {
		return 85;
	}
	long plain_bytes = syscall(SYS_getdents64, plain_dir_fd, dir_buf,
				   sizeof(dir_buf));
	if (plain_bytes <= 0) {
		return 86;
	}
	if (close(plain_dir_fd) < 0) {
		return 87;
	}
	if (SAY_LITERAL("syscall openat directory target ok\n") < 0) {
		return 88;
	}

	int dot_dir_fd = syscall(SYS_openat, AT_FDCWD, ".",
				 O_RDONLY | O_DIRECTORY | O_CLOEXEC, 0);
	if (dot_dir_fd < 0) {
		return 67;
	}
	int dot_flags = fcntl(dot_dir_fd, F_GETFL, 0);
	if (dot_flags < 0 || (dot_flags & O_DIRECTORY) == 0 ||
	    (dot_flags & O_CLOEXEC) != 0) {
		return 68;
	}
	int dot_fd_flags = fcntl(dot_dir_fd, F_GETFD, 0);
	if (dot_fd_flags < 0 || (dot_fd_flags & FD_CLOEXEC) == 0) {
		return 74;
	}
	if (fcntl(dot_dir_fd, F_SETFD, 0) < 0) {
		return 75;
	}
	dot_fd_flags = fcntl(dot_dir_fd, F_GETFD, 0);
	if (dot_fd_flags < 0 || (dot_fd_flags & FD_CLOEXEC) != 0) {
		return 76;
	}
	if (fcntl(dot_dir_fd, F_SETFD, FD_CLOEXEC) < 0) {
		return 77;
	}
	dot_fd_flags = fcntl(dot_dir_fd, F_GETFD, 0);
	if (dot_fd_flags < 0 || (dot_fd_flags & FD_CLOEXEC) == 0) {
		return 78;
	}
	if (SAY_LITERAL("syscall fcntl F_GETFD F_SETFD cloexec ok\n") < 0) {
		return 79;
	}
	if (close(dot_dir_fd) < 0) {
		return 69;
	}
	if (SAY_LITERAL("syscall openat cwd dot cloexec ok\n") < 0) {
		return 70;
	}

	return 0;
}

static int smoke_readlinkat(void)
{
	char link_buf[64];
	struct stat st;
	ssize_t link_len = syscall(SYS_readlinkat, AT_FDCWD, "/bin/cat",
				   link_buf, sizeof(link_buf));

	if (link_len <= 0 || link_len >= (ssize_t)sizeof(link_buf)) {
		return 60;
	}
	if (link_len < 7) {
		return 61;
	}
	if (link_buf[link_len - 7] != 'b' ||
	    link_buf[link_len - 6] != 'u' ||
	    link_buf[link_len - 5] != 's' ||
	    link_buf[link_len - 4] != 'y' ||
	    link_buf[link_len - 3] != 'b' ||
	    link_buf[link_len - 2] != 'o' ||
	    link_buf[link_len - 1] != 'x') {
		return 62;
	}
	if (SAY_LITERAL("syscall readlinkat symlink ok\n") < 0) {
		return 63;
	}

	if (fstatat(AT_FDCWD, "/bin/cat", &st, AT_SYMLINK_NOFOLLOW) < 0) {
		return 80;
	}
	if (!S_ISLNK(st.st_mode) || st.st_size <= 0) {
		return 81;
	}
	if (SAY_LITERAL("syscall newfstatat symlink nofollow ok\n") < 0) {
		return 82;
	}

	return 0;
}

int smoke_fileio(void)
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

	int dir_status = smoke_directory_fileio();
	if (dir_status != 0) {
		return dir_status;
	}

	int readlink_status = smoke_readlinkat();
	if (readlink_status != 0) {
		return readlink_status;
	}

	if (SAY_LITERAL("syscall write ok\n") < 0) {
		return 14;
	}

	return 0;
}
