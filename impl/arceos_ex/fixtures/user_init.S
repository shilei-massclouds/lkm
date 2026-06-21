.section .text
.global _start
_start:
	li a7, 64
	li a0, 1
	la a1, message
	li a2, 11
	ecall

	li a7, 93
	li a0, 0
	ecall

1:
	j 1b

.section .rodata
message:
	.ascii "user hello\n"
