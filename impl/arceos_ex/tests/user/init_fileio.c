typedef unsigned long usize;
typedef long isize;

#define AT_FDCWD (-100L)
#define SYS_OPENAT 56
#define SYS_CLOSE 57
#define SYS_READ 63
#define SYS_WRITE 64
#define SYS_NEWFSTATAT 79
#define SYS_EXIT 93

static inline isize syscall0(usize nr)
{
	register usize a7 __asm__("a7") = nr;
	register usize a0 __asm__("a0");
	__asm__ volatile("ecall" : "=r"(a0) : "r"(a7) : "memory");
	return (isize)a0;
}

static inline isize syscall1(usize nr, usize arg0)
{
	register usize a7 __asm__("a7") = nr;
	register usize a0 __asm__("a0") = arg0;
	__asm__ volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
	return (isize)a0;
}

static inline isize syscall3(usize nr, usize arg0, usize arg1, usize arg2)
{
	register usize a7 __asm__("a7") = nr;
	register usize a0 __asm__("a0") = arg0;
	register usize a1 __asm__("a1") = arg1;
	register usize a2 __asm__("a2") = arg2;
	__asm__ volatile("ecall"
			 : "+r"(a0)
			 : "r"(a7), "r"(a1), "r"(a2)
			 : "memory");
	return (isize)a0;
}

static inline isize syscall4(usize nr, usize arg0, usize arg1, usize arg2, usize arg3)
{
	register usize a7 __asm__("a7") = nr;
	register usize a0 __asm__("a0") = arg0;
	register usize a1 __asm__("a1") = arg1;
	register usize a2 __asm__("a2") = arg2;
	register usize a3 __asm__("a3") = arg3;
	__asm__ volatile("ecall"
			 : "+r"(a0)
			 : "r"(a7), "r"(a1), "r"(a2), "r"(a3)
			 : "memory");
	return (isize)a0;
}

static void exit_with(usize status)
{
	(void)syscall1(SYS_EXIT, status);
	for (;;) {
	}
}

void _start(void)
{
	static const char path[] = "/etc/alpine-release";
	static const char message[] = "user hello\n";
	char read_buf[32];
	char stat_buf[128];

	isize fd = syscall4(SYS_OPENAT, (usize)AT_FDCWD, (usize)path, 0, 0);
	if (fd < 0) {
		exit_with(20 + (usize)(-fd));
	}

	isize read_len = syscall3(SYS_READ, (usize)fd, (usize)read_buf, sizeof(read_buf));
	if (read_len <= 0 || read_buf[0] != '3' || read_buf[1] != '.') {
		exit_with(11);
	}

	if (syscall1(SYS_CLOSE, (usize)fd) < 0) {
		exit_with(12);
	}

	if (syscall4(SYS_NEWFSTATAT, (usize)AT_FDCWD, (usize)path, (usize)stat_buf, 0) < 0) {
		exit_with(13);
	}

	if (syscall3(SYS_WRITE, 1, (usize)message, sizeof(message) - 1) != (isize)(sizeof(message) - 1)) {
		exit_with(14);
	}

	exit_with(0);
}
