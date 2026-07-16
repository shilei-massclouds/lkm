	.section .text
	.align 2
	.globl smoke_trap_registers
	.type smoke_trap_registers, @function
smoke_trap_registers:
	addi sp, sp, -96
	sd s0, 0(sp)
	sd s1, 8(sp)
	sd s2, 16(sp)
	sd s3, 24(sp)
	sd s4, 32(sp)
	sd s5, 40(sp)
	sd s6, 48(sp)
	sd s7, 56(sp)
	sd s8, 64(sp)
	sd s9, 72(sp)
	sd s10, 80(sp)
	sd s11, 88(sp)

	li s0, 0x510
	li s1, 0x511
	li s2, 0x512
	li s3, 0x513
	li s4, 0x514
	li s5, 0x515
	li s6, 0x516
	li s7, 0x517
	li s8, 0x518
	li s9, 0x519
	li s10, 0x51a
	li s11, 0x51b
	li a7, 172
	ecall

	li t0, 1
	bne a0, t0, 1f
	li t0, 0x510
	bne s0, t0, 1f
	li t0, 0x511
	bne s1, t0, 1f
	li t0, 0x512
	bne s2, t0, 1f
	li t0, 0x513
	bne s3, t0, 1f
	li t0, 0x514
	bne s4, t0, 1f
	li t0, 0x515
	bne s5, t0, 1f
	li t0, 0x516
	bne s6, t0, 1f
	li t0, 0x517
	bne s7, t0, 1f
	li t0, 0x518
	bne s8, t0, 1f
	li t0, 0x519
	bne s9, t0, 1f
	li t0, 0x51a
	bne s10, t0, 1f
	li t0, 0x51b
	bne s11, t0, 1f
	li a0, 0
	j 2f
1:
	li a0, 101
2:
	ld s0, 0(sp)
	ld s1, 8(sp)
	ld s2, 16(sp)
	ld s3, 24(sp)
	ld s4, 32(sp)
	ld s5, 40(sp)
	ld s6, 48(sp)
	ld s7, 56(sp)
	ld s8, 64(sp)
	ld s9, 72(sp)
	ld s10, 80(sp)
	ld s11, 88(sp)
	addi sp, sp, 96
	ret
	.size smoke_trap_registers, .-smoke_trap_registers
