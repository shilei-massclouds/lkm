#include <stddef.h>
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
	ssize_t read_len;
	size_t i;

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

	return 0;
}
