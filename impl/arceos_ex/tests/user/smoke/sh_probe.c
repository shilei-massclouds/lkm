#include <stddef.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

int smoke_sh_probe(void)
{
	if (SAY_LITERAL("sh_probe entry ok\n") < 0) {
		return 41;
	}
	return 0;
}
