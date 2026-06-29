typedef unsigned long usize;
typedef long isize;

#define SYS_WRITE 64
#define SYS_EXIT 93
#define SYS_UNSUPPORTED_PROBE 451

static inline isize syscall1(usize nr, usize arg0)
{
	register usize a7 __asm__("a7") = nr;
	register usize a0 __asm__("a0") = arg0;
	__asm__ volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
	return (isize)a0;
}

static inline isize syscall6(usize nr, usize arg0, usize arg1, usize arg2,
			     usize arg3, usize arg4, usize arg5)
{
	register usize a7 __asm__("a7") = nr;
	register usize a0 __asm__("a0") = arg0;
	register usize a1 __asm__("a1") = arg1;
	register usize a2 __asm__("a2") = arg2;
	register usize a3 __asm__("a3") = arg3;
	register usize a4 __asm__("a4") = arg4;
	register usize a5 __asm__("a5") = arg5;
	__asm__ volatile("ecall"
			 : "+r"(a0)
			 : "r"(a7), "r"(a1), "r"(a2), "r"(a3), "r"(a4),
			   "r"(a5)
			 : "memory");
	return (isize)a0;
}

static void say(const char *message, usize len)
{
	(void)syscall6(SYS_WRITE, 1, (usize)message, len, 0, 0, 0);
}

static void exit_with(usize status)
{
	(void)syscall1(SYS_EXIT, status);
	for (;;) {
	}
}

void _start(void)
{
	static const char ok[] = "unsupported syscall enosys ok\n";
	isize rc = syscall6(SYS_UNSUPPORTED_PROBE, 0x11, 0x22, 0x33, 0x44,
			    0x55, 0x66);

	if (rc != -38) {
		exit_with(10);
	}

	say(ok, sizeof(ok) - 1);
	exit_with(0);
}
