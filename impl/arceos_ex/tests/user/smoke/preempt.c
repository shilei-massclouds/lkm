#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "smoke.h"

enum {
	CHILD_COUNT = 17,
	TARGET_A_INDEX = 0,
	TARGET_B_INDEX = 16,
	TARGET_ROUNDS = 4,
	SPIN_ITERATIONS = 10000000,
};

static int say(const char *message, size_t len)
{
	return write(STDOUT_FILENO, message, len) == (ssize_t)len ? 0 : -1;
}

#define SAY_LITERAL(message) say(message, sizeof(message) - 1)

static void pure_user_spin(void)
{
	unsigned long iteration;

	for (iteration = 0; iteration < SPIN_ITERATIONS; ++iteration) {
		__asm__ volatile("" : "+r"(iteration) : : "memory");
	}
}

static uint64_t read_time_counter(void)
{
	uint64_t value;

	__asm__ volatile("rdtime %0" : "=r"(value));
	return value;
}

static int say_spin_calibration(uint64_t ticks)
{
	char record[96];
	int len = snprintf(record, sizeof(record),
			   "user preempt spin calibration ticks=%llu\n",
			   (unsigned long long)ticks);

	return len > 0 && (size_t)len < sizeof(record) ? say(record, (size_t)len) : -1;
}

static int say_placement(long coordinator_pid, long target_a_pid, long target_b_pid)
{
	char record[160];
	int len = snprintf(record, sizeof(record),
			   "user preempt placement coordinator_pid=%ld target_a_pid=%ld target_b_pid=%ld delta=%ld\n",
			   coordinator_pid, target_a_pid, target_b_pid,
			   target_b_pid - target_a_pid);

	return len > 0 && (size_t)len < sizeof(record) ? say(record, (size_t)len) : -1;
}

__attribute__((noreturn)) static void child_run(int read_fd, int write_fd,
						 int target_ordinal)
{
	char start;
	char marker;
	int round;

	if (target_ordinal < 0) {
		(void)close(read_fd);
		(void)close(write_fd);
		syscall(SYS_exit, 0);
	}
	if (target_ordinal > 1) {
		(void)close(read_fd);
		(void)close(write_fd);
		syscall(SYS_exit, 72);
	}

	marker = target_ordinal == 0 ? 'A' : 'B';
	if ((marker == 'A' && SAY_LITERAL("user preempt child A compute begin\n") < 0) ||
	    (marker == 'B' && SAY_LITERAL("user preempt child B compute begin\n") < 0)) {
		(void)close(read_fd);
		(void)close(write_fd);
		syscall(SYS_exit, 74);
	}
	if (read(read_fd, &start, 1) != 1 || start != 'S' || close(read_fd) != 0) {
		(void)close(write_fd);
		syscall(SYS_exit, 76);
	}
	for (round = 0; round < TARGET_ROUNDS; ++round) {
		pure_user_spin();
		if (write(write_fd, &marker, 1) != 1) {
			(void)close(write_fd);
			syscall(SYS_exit, 73);
		}
	}
	if ((marker == 'A' && SAY_LITERAL("user preempt child A writes complete\n") < 0) ||
	    (marker == 'B' && SAY_LITERAL("user preempt child B writes complete\n") < 0)) {
		(void)close(write_fd);
		syscall(SYS_exit, 75);
	}
	(void)close(write_fd);
	syscall(SYS_exit, 0);
	__builtin_unreachable();
}

static int coordinator_run(void)
{
	long children[CHILD_COUNT];
	long coordinator_pid = syscall(SYS_getpid);
	char order[TARGET_ROUNDS * 2];
	int pipefd[2];
	int target_count = 0;
	int child_index;
	int collected = 0;
	int count_a = 0;
	int count_b = 0;
	int transitions = 0;
	uint64_t calibration_start;
	uint64_t calibration_ticks;

	if (coordinator_pid < 0) {
		return 129;
	}
	if (SAY_LITERAL("user preempt coordinator begin\n") < 0) {
		return 128;
	}
	calibration_start = read_time_counter();
	pure_user_spin();
	calibration_ticks = read_time_counter() - calibration_start;
	if (say_spin_calibration(calibration_ticks) < 0) {
		return 121;
	}
	if (pipe(pipefd) != 0) {
		return 130;
	}

	for (child_index = 0; child_index < CHILD_COUNT; ++child_index) {
		int target_ordinal = child_index == TARGET_A_INDEX ? 0 :
				     child_index == TARGET_B_INDEX ? 1 : -1;
		long child = syscall(SYS_clone, SIGCHLD, 0, 0, 0, 0);

		if (child < 0) {
			(void)close(pipefd[0]);
			(void)close(pipefd[1]);
			return 131;
		}
		if (child == 0) {
			child_run(pipefd[0], pipefd[1], target_ordinal);
		}
		children[child_index] = child;
		if (target_ordinal >= 0) {
			++target_count;
		}
	}
	if (SAY_LITERAL("user preempt children published\n") < 0) {
		return 127;
	}
	if (say_placement(coordinator_pid, children[TARGET_A_INDEX],
			  children[TARGET_B_INDEX]) < 0) {
		return 120;
	}

	if (target_count != 2) {
		(void)close(pipefd[0]);
		(void)close(pipefd[1]);
		return 132;
	}
	if (write(pipefd[1], "SS", 2) != 2 || close(pipefd[1]) != 0) {
		(void)close(pipefd[0]);
		return 123;
	}
	for (child_index = 0; child_index < CHILD_COUNT; ++child_index) {
		int status = 0;
		long waited = syscall(SYS_wait4, children[child_index], &status, 0,
					    NULL);

		if (waited != children[child_index] || status != 0) {
			(void)close(pipefd[0]);
			return 122;
		}
	}
	while (collected < (int)sizeof(order)) {
		ssize_t amount = read(pipefd[0], order + collected,
				      sizeof(order) - (size_t)collected);

		if (amount <= 0) {
			(void)close(pipefd[0]);
			return 133;
		}
		collected += (int)amount;
	}
	if (close(pipefd[0]) != 0) {
		return 134;
	}
	if (SAY_LITERAL("user preempt bytes collected order=") < 0 ||
	    write(STDOUT_FILENO, order, sizeof(order)) != (ssize_t)sizeof(order) ||
	    SAY_LITERAL("\n") < 0) {
		return 126;
	}
	for (child_index = 0; child_index < (int)sizeof(order); ++child_index) {
		if (order[child_index] == 'A') {
			++count_a;
		} else if (order[child_index] == 'B') {
			++count_b;
		} else {
			return 136;
		}
		if (child_index > 0 && order[child_index] != order[child_index - 1]) {
			++transitions;
		}
	}
	if (count_a != TARGET_ROUNDS || count_b != TARGET_ROUNDS || transitions < 2) {
		return 137;
	}

	if (SAY_LITERAL("user preempt children reaped\n") < 0) {
		return 125;
	}
	if (SAY_LITERAL("user timer preemption A-B-A round robin ok\n") < 0) {
		return 138;
	}
	return 0;
}

int smoke_preempt(void)
{
	long coordinator = syscall(SYS_clone, SIGCHLD, 0, 0, 0, 0);
	long waited;
	int status = 0;

	if (coordinator < 0) {
		return 139;
	}
	if (coordinator == 0) {
		int rc = coordinator_run();

		syscall(SYS_exit, rc);
		__builtin_unreachable();
	}
	waited = syscall(SYS_wait4, coordinator, &status, 0, NULL);
	if (waited != coordinator) {
		return 140;
	}
	if (status != 0) {
		return 141;
	}
	return 0;
}
