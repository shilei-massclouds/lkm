#include <stddef.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int smoke_getrandom(void)
{
	unsigned char random_bytes[16];
	long rc;

	rc = syscall(SYS_getrandom, random_bytes, sizeof(random_bytes), 0);
	if (rc != (long)sizeof(random_bytes)) {
		return 42;
	}
	if (SAY_LITERAL("syscall getrandom ok\n") < 0) {
		return 43;
	}
	return 0;
}

int smoke_sh_probe(void)
{
	if (SAY_LITERAL("sh_probe entry ok\n") < 0) {
		return 41;
	}
	if (smoke_getrandom() != 0) {
		return 42;
	}
	return 0;
}
