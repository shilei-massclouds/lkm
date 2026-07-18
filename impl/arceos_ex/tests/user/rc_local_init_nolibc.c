typedef unsigned long size_t;
typedef long ssize_t;

enum {
    SYS_WRITE = 64,
    SYS_EXIT = 93,
    SYS_EXECVE = 221,
};

static long syscall1(long number, long arg0)
{
    register long a0 __asm__("a0") = arg0;
    register long a7 __asm__("a7") = number;

    __asm__ volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
    return a0;
}

static long syscall3(long number, long arg0, long arg1, long arg2)
{
    register long a0 __asm__("a0") = arg0;
    register long a1 __asm__("a1") = arg1;
    register long a2 __asm__("a2") = arg2;
    register long a7 __asm__("a7") = number;

    __asm__ volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a2), "r"(a7) : "memory");
    return a0;
}

__attribute__((noreturn)) void _start(void)
{
    static char const shell[] = "/bin/sh";
    static char const name[] = "sh";
    static char const script[] = "/opt/lkm/tests/rc-local.sh";
    static char const path[] = "PATH=/sbin:/bin:/usr/sbin:/usr/bin";
    static char const home[] = "HOME=/root";
    static char const term[] = "TERM=linux";
    static char const diagnostic[] = "lkm-rc-local-init: execve failed\n";
    static char const *const argv[] = {name, script, 0};
    static char const *const envp[] = {path, home, term, 0};

    syscall3(SYS_EXECVE, (long)shell, (long)argv, (long)envp);
    syscall3(SYS_WRITE, 2, (long)diagnostic, (long)(sizeof(diagnostic) - 1));
    syscall1(SYS_EXIT, 127);
    __builtin_unreachable();
}
