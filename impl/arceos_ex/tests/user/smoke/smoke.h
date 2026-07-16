#ifndef USER_SMOKE_H
#define USER_SMOKE_H

int smoke_fileio(void);
int smoke_signal(void);
int smoke_stdin(void);
int smoke_fpu_mmap(void);
int smoke_credentials(void);
int smoke_process_identity(void);
int smoke_tty_termios(void);
int smoke_uts_cwd(void);
int smoke_time_random(void);
void smoke_stack_expect_execfn(const char *execfn);
int smoke_stack(void);

#endif
