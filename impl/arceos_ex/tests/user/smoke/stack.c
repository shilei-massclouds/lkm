#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/random.h>

#ifndef AT_RANDOM
#define AT_RANDOM 25
#endif
#ifndef AT_EXECFN
#define AT_EXECFN 31
#endif

#define RISCV_HWCAP(letter) (1UL << ((letter) - 'A'))
#define RISCV_G_HWCAP (RISCV_HWCAP('I') | RISCV_HWCAP('M') | \
			 RISCV_HWCAP('A') | RISCV_HWCAP('F') | \
			 RISCV_HWCAP('D'))
#define RISCV_REQUIRED_HWCAP (RISCV_G_HWCAP | RISCV_HWCAP('C'))

#define PAGE_SIZE 4096
#define STACK_PROBE_BYTES (256 * 1024)
#define USERCOPY_STACK_BYTES (192 * 1024)

static const char *expected_execfn;

void smoke_stack_expect_execfn(const char *execfn)
{
	expected_execfn = execfn;
}

__attribute__((noinline)) static int probe_stack_usercopy(void)
{
	volatile unsigned char usercopy_probe[USERCOPY_STACK_BYTES];
	ssize_t copied;

	copied = getrandom((void *)&usercopy_probe[0], 16, 0);
	if (copied != 16) {
		return 3;
	}
	return usercopy_probe[0] == 0 && usercopy_probe[1] == 0 &&
	       usercopy_probe[2] == 0 && usercopy_probe[3] == 0 ? 4 : 0;
}

__attribute__((noinline)) static int probe_growing_stack(void)
{
	volatile unsigned char stack_probe[STACK_PROBE_BYTES];
	size_t offset;
	unsigned int checksum = 0;

	for (offset = 0; offset < sizeof(stack_probe); offset += PAGE_SIZE) {
		stack_probe[offset] = (unsigned char)(offset / PAGE_SIZE + 1);
	}
	for (offset = 0; offset < sizeof(stack_probe); offset += PAGE_SIZE) {
		checksum += stack_probe[offset];
	}
	return checksum == 2080 ? 0 : 3;
}

int smoke_stack(void)
{
	const unsigned char *random = (const unsigned char *)(uintptr_t)getauxval(AT_RANDOM);
	const char *execfn = (const char *)(uintptr_t)getauxval(AT_EXECFN);
	unsigned long hwcap = getauxval(AT_HWCAP);
	size_t index;
	unsigned char combined = 0;

	if (execfn == NULL || expected_execfn == NULL ||
	    strcmp(execfn, expected_execfn) != 0) {
		return 1;
	}
	if ((hwcap & RISCV_REQUIRED_HWCAP) != RISCV_REQUIRED_HWCAP) {
		return 2;
	}
	if (getauxval(AT_UID) != 0 || getauxval(AT_EUID) != 0 ||
	    getauxval(AT_GID) != 0 || getauxval(AT_EGID) != 0) {
		return 3;
	}
	if (random == NULL) {
		return 4;
	}
	for (index = 0; index < 16; ++index) {
		combined |= random[index];
	}
	if (combined == 0) {
		return 5;
	}
	if (probe_stack_usercopy() != 0) {
		return 6;
	}
	return probe_growing_stack();
}
