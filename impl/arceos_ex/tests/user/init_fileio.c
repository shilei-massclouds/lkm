#include <fcntl.h>
#include <stddef.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void)
{
	static const char path[] = "/etc/alpine-release";
	static const char message[] = "user hello\n";
	char read_buf[32];
	struct stat st;

	int fd = open(path, O_RDONLY);
	if (fd < 0) {
		return 20;
	}

	ssize_t read_len = read(fd, read_buf, sizeof(read_buf));
	if (read_len <= 0 || read_buf[0] != '3' || read_buf[1] != '.') {
		return 11;
	}

	if (close(fd) < 0) {
		return 12;
	}

	if (fstatat(AT_FDCWD, path, &st, 0) < 0) {
		return 13;
	}
	if (!S_ISREG(st.st_mode) || st.st_size <= 0) {
		return 15;
	}

	if (write(STDOUT_FILENO, message, sizeof(message) - 1) != (ssize_t)(sizeof(message) - 1)) {
		return 14;
	}

	return 0;
}
