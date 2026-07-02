#include <stddef.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#include "smoke.h"

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static int smoke_time_syscalls(void)
{
	struct timespec ts;
	struct timeval tv;
	struct timezone tz;
	long rc;

	ts.tv_nsec = -1;
	rc = syscall(SYS_clock_gettime, CLOCK_MONOTONIC, &ts);
	if (rc != 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1000000000L) {
		return 56;
	}
	if (SAY_LITERAL("syscall clock_gettime monotonic ok\n") < 0) {
		return 57;
	}

	ts.tv_nsec = -1;
	rc = syscall(SYS_clock_gettime, CLOCK_REALTIME, &ts);
	if (rc != 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1000000000L) {
		return 58;
	}
	if (SAY_LITERAL("syscall clock_gettime realtime ok\n") < 0) {
		return 59;
	}

	tv.tv_usec = -1;
	tz.tz_minuteswest = -1;
	tz.tz_dsttime = -1;
	rc = syscall(SYS_gettimeofday, &tv, &tz);
	if (rc != 0 || tv.tv_usec < 0 || tv.tv_usec >= 1000000L ||
	    tz.tz_minuteswest != 0 || tz.tz_dsttime != 0) {
		return 60;
	}
	if (SAY_LITERAL("syscall gettimeofday ok\n") < 0) {
		return 61;
	}

	ts.tv_sec = 0;
	ts.tv_nsec = 1000000L;
	rc = syscall(SYS_nanosleep, &ts, &ts);
	if (rc != 0) {
		return 93;
	}
	if (SAY_LITERAL("syscall nanosleep ok\n") < 0) {
		return 94;
	}

	return 0;
}

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

int smoke_time_random(void)
{
	int status = smoke_time_syscalls();

	if (status != 0) {
		return status;
	}
	return smoke_getrandom();
}
