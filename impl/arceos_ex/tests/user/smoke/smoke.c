#include <stddef.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int say_status(const char *prefix, size_t prefix_len, int status, const char *suffix,
		      size_t suffix_len)
{
	char digits[16];
	size_t digit_count = 0;
	unsigned int value;

	if (say(prefix, prefix_len) < 0) {
		return -1;
	}
	if (status < 0) {
		if (SAY_LITERAL("-") < 0) {
			return -1;
		}
		value = (unsigned int)(-status);
	} else {
		value = (unsigned int)status;
	}
	do {
		digits[digit_count++] = (char)('0' + value % 10);
		value /= 10;
	} while (value != 0);
	while (digit_count > 0) {
		if (say(&digits[--digit_count], 1) < 0) {
			return -1;
		}
	}
	return say(suffix, suffix_len);
}

#define SAY_STATUS(prefix, status, suffix) \
	say_status(prefix, sizeof(prefix) - 1, status, suffix, sizeof(suffix) - 1)

static int run_case(const char *name, size_t name_len, int (*case_fn)(void))
{
	int rc;

	if (say("user-smoke: case ", sizeof("user-smoke: case ") - 1) < 0 ||
	    say(name, name_len) < 0 ||
	    SAY_LITERAL(": begin\n") < 0) {
		return 31;
	}
	rc = case_fn();
	if (say("user-smoke: case ", sizeof("user-smoke: case ") - 1) < 0 ||
	    say(name, name_len) < 0 ||
	    SAY_STATUS(": end status=", rc, "\n\n") < 0) {
		return rc != 0 ? rc : 32;
	}
	return rc;
}

#define RUN_CASE(name, fn) run_case(name, sizeof(name) - 1, fn)

int main(int argc, char **argv)
{
	static const char message[] = "user hello\n";
	int rc;

	if (argc < 1 || argv == NULL || argv[0] == NULL) {
		return 30;
	}

	if (SAY_LITERAL("user-smoke: begin\n\n") < 0) {
		return 31;
	}

	rc = RUN_CASE("fileio", smoke_fileio);
	if (rc != 0) {
		(void)SAY_STATUS("user-smoke: end status=", rc, "\n");
		return rc;
	}

	rc = RUN_CASE("signal", smoke_signal);
	if (rc != 0) {
		(void)SAY_STATUS("user-smoke: end status=", rc, "\n");
		return rc;
	}

	rc = RUN_CASE("sh_probe", smoke_sh_probe);
	if (rc != 0) {
		(void)SAY_STATUS("user-smoke: end status=", rc, "\n");
		return rc;
	}

	if (say(message, sizeof(message) - 1) < 0) {
		return 17;
	}
	if (SAY_STATUS("user-smoke: end status=", 0, "\n") < 0) {
		return 18;
	}

	return 0;
}
