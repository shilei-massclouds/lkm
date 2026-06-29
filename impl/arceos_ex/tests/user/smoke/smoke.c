#include <stddef.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int main(int argc, char **argv)
{
	static const char message[] = "user hello\n";
	int rc;

	if (argc < 1 || argv == NULL || argv[0] == NULL) {
		return 30;
	}

	rc = smoke_init_fileio();
	if (rc != 0) {
		return rc;
	}

	rc = smoke_sh_probe();
	if (rc != 0) {
		return rc;
	}

	if (say(message, sizeof(message) - 1) < 0) {
		return 16;
	}
	if (SAY_LITERAL("user smoke ok\n") < 0) {
		return 17;
	}

	return 0;
}
