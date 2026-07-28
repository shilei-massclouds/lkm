use core::arch::global_asm;

#[repr(C)]
pub struct TaskSwitchContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
    kernel_stack_base: usize,
    kernel_stack_top: usize,
    root_trap_flow_address: usize,
    root_trap_flow_generation: usize,
}

const _: () = {
    assert!(core::mem::size_of::<TaskSwitchContext>() == 18 * core::mem::size_of::<usize>());
    assert!(core::mem::offset_of!(TaskSwitchContext, ra) == 0);
    assert!(core::mem::offset_of!(TaskSwitchContext, sp) == core::mem::size_of::<usize>());
    assert!(core::mem::offset_of!(TaskSwitchContext, s) == 2 * core::mem::size_of::<usize>());
    assert!(core::mem::offset_of!(TaskSwitchContext, kernel_stack_base) == 112);
    assert!(core::mem::offset_of!(TaskSwitchContext, kernel_stack_top) == 120);
    assert!(core::mem::offset_of!(TaskSwitchContext, root_trap_flow_address) == 128);
    assert!(core::mem::offset_of!(TaskSwitchContext, root_trap_flow_generation) == 136);
};

impl TaskSwitchContext {
    pub const fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
            kernel_stack_base: 0,
            kernel_stack_top: 0,
            root_trap_flow_address: 0,
            root_trap_flow_generation: 0,
        }
    }

    pub const fn ra(&self) -> usize {
        self.ra
    }

    pub const fn initialized(&self) -> bool {
        self.ra != 0 && self.sp != 0
    }

    pub const fn physical_switch_ready(&self) -> bool {
        self.initialized()
            && self.kernel_stack_base() != 0
            && self.kernel_stack_top() > self.kernel_stack_base()
            && self.sp >= self.kernel_stack_base()
            && self.sp <= self.kernel_stack_top()
    }

    #[cfg(app_smoke)]
    pub const fn sp(&self) -> usize {
        self.sp
    }

    #[cfg(app_smoke)]
    pub const fn saved_register(&self, index: usize) -> Option<usize> {
        if index < self.s.len() {
            Some(self.s[index])
        } else {
            None
        }
    }

    pub const fn kernel_stack_base(&self) -> usize {
        self.kernel_stack_base
    }

    pub const fn kernel_stack_top(&self) -> usize {
        self.kernel_stack_top
    }

    pub const fn root_trap_flow_ref(&self) -> crate::objects::trap_flow_type::TrapFlowRef {
        crate::objects::trap_flow_type::TrapFlowRef::new(
            self.root_trap_flow_address,
            self.root_trap_flow_generation as u32,
        )
    }

    pub fn set_root_trap_flow_ref(
        &mut self,
        root_ref: crate::objects::trap_flow_type::TrapFlowRef,
    ) {
        self.root_trap_flow_address = root_ref.address();
        self.root_trap_flow_generation = root_ref.generation() as usize;
    }

    pub fn init(
        &mut self,
        entry: extern "C" fn() -> !,
        kernel_stack_base: usize,
        kernel_stack_top: usize,
    ) {
        self.ra = entry as usize;
        self.sp = kernel_stack_top & !0xf;
        self.s = [0; 12];
        self.kernel_stack_base = kernel_stack_base;
        self.kernel_stack_top = kernel_stack_top;
        self.root_trap_flow_address = 0;
        self.root_trap_flow_generation = 0;
    }

    /// Mark the context as initialized without providing real register values.
    /// The actual register state will be captured on the first save (when this
    /// task acts as the *prev* party in a context switch).
    pub fn init_with_dummy(&mut self) {
        self.ra = 1;
        self.sp = 1;
        self.s = [0; 12];
        self.root_trap_flow_address = 0;
        self.root_trap_flow_generation = 0;
    }

    pub fn set_kernel_stack_bounds(&mut self, base: usize, top: usize) -> bool {
        if base == 0 || top <= base {
            return false;
        }
        self.kernel_stack_base = base;
        self.kernel_stack_top = top;
        if self.sp == 1 {
            self.sp = top & !0xf;
        }
        true
    }
}

global_asm!(
    r#"
    .section .text.task_switch, "ax"
    .align 2
    .globl arceos_ex_task_switch
arceos_ex_task_switch:
    /*
     * a0 = prev TaskSwitchContext*
     * a1 = next TaskSwitchContext*
     * a2 = next Task identity pointer (the independent tp/current carrier)
     *
     * Cooperative task context switch: save/restore ra, sp and the
     * callee-saved s-registers. tp is deliberately not context: it is
     * established from the selected Task identity after restoring registers.
     * Does not switch address spaces, trap frames, FPU/vector state or
     * task-local metadata.
     */
    sd      ra, 0(a0)
    sd      sp, 8(a0)
    sd      s0, 16(a0)
    sd      s1, 24(a0)
    sd      s2, 32(a0)
    sd      s3, 40(a0)
    sd      s4, 48(a0)
    sd      s5, 56(a0)
    sd      s6, 64(a0)
    sd      s7, 72(a0)
    sd      s8, 80(a0)
    sd      s9, 88(a0)
    sd      s10, 96(a0)
    sd      s11, 104(a0)

    ld      ra, 0(a1)
    ld      sp, 8(a1)
    ld      s0, 16(a1)
    ld      s1, 24(a1)
    ld      s2, 32(a1)
    ld      s3, 40(a1)
    ld      s4, 48(a1)
    ld      s5, 56(a1)
    ld      s6, 64(a1)
    ld      s7, 72(a1)
    ld      s8, 80(a1)
    ld      s9, 88(a1)
    ld      s10, 96(a1)
    ld      s11, 104(a1)

    /* Commit the selected Task's CPU-local formal trap-entry authority. */
    csrr    t0, sscratch
    beqz    t0, .Ltask_switch_entry_context_rejected
    sd      a2, {trap_context_task_offset}(t0)
    ld      t1, 112(a1)
    sd      t1, {trap_context_stack_base_offset}(t0)
    ld      t1, 120(a1)
    sd      t1, {trap_context_stack_top_offset}(t0)
    ld      t1, 128(a1)
    sd      t1, {trap_context_root_address_offset}(t0)
    ld      t1, 136(a1)
    sd      t1, {trap_context_root_generation_offset}(t0)
    mv      tp, a2
    ret

.Ltask_switch_entry_context_rejected:
    li      a7, 0x53525354
    li      a6, 0
    li      a0, 0
    li      a1, 1
    ecall
1:
    wfi
    j       1b
"#
    ,
    trap_context_task_offset = const crate::objects::trap_type::TRAP_ENTRY_CONTEXT_TASK_OFFSET,
    trap_context_stack_base_offset = const crate::objects::trap_type::TRAP_ENTRY_CONTEXT_STACK_BASE_OFFSET,
    trap_context_stack_top_offset = const crate::objects::trap_type::TRAP_ENTRY_CONTEXT_STACK_TOP_OFFSET,
    trap_context_root_address_offset = const crate::objects::trap_type::TRAP_ENTRY_CONTEXT_ROOT_ADDRESS_OFFSET,
    trap_context_root_generation_offset = const crate::objects::trap_type::TRAP_ENTRY_CONTEXT_ROOT_GENERATION_OFFSET,
);

#[allow(dead_code)] // The smoke sentinel reaches this symbol from assembly.
unsafe extern "C" {
    fn arceos_ex_task_switch(
        prev: *mut TaskSwitchContext,
        next: *const TaskSwitchContext,
        next_task_identity: usize,
    );
}

#[cfg(app_smoke)]
global_asm!(
    r#"
    .section .text.task_switch_sentinel, "ax"
    .align 2
    .globl arceos_ex_task_switch_sentinel
arceos_ex_task_switch_sentinel:
    addi    sp, sp, -112
    sd      ra, 0(sp)
    sd      s0, 8(sp)
    sd      s1, 16(sp)
    sd      s2, 24(sp)
    sd      s3, 32(sp)
    sd      s4, 40(sp)
    sd      s5, 48(sp)
    sd      s6, 56(sp)
    sd      s7, 64(sp)
    sd      s8, 72(sp)
    sd      s9, 80(sp)
    sd      s10, 88(sp)
    sd      s11, 96(sp)

    li      s0, 0x110
    li      s1, 0x221
    li      s2, 0x332
    li      s3, 0x443
    li      s4, 0x554
    li      s5, 0x665
    li      s6, 0x776
    li      s7, 0x887
    li      s8, 0x998
    li      s9, 0xaa9
    li      s10, 0xbba
    li      s11, 0xccb
    call    arceos_ex_task_switch

    li      a0, 0
    li      t0, 0x110
    beq     s0, t0, 1f
    ori     a0, a0, 0x001
1:  li      t0, 0x221
    beq     s1, t0, 2f
    ori     a0, a0, 0x002
2:  li      t0, 0x332
    beq     s2, t0, 3f
    ori     a0, a0, 0x004
3:  li      t0, 0x443
    beq     s3, t0, 4f
    ori     a0, a0, 0x008
4:  li      t0, 0x554
    beq     s4, t0, 5f
    ori     a0, a0, 0x010
5:  li      t0, 0x665
    beq     s5, t0, 6f
    ori     a0, a0, 0x020
6:  li      t0, 0x776
    beq     s6, t0, 7f
    ori     a0, a0, 0x040
7:  li      t0, 0x887
    beq     s7, t0, 8f
    ori     a0, a0, 0x080
8:  li      t0, 0x998
    beq     s8, t0, 9f
    ori     a0, a0, 0x100
9:  li      t0, 0xaa9
    beq     s9, t0, 10f
    ori     a0, a0, 0x200
10: li      t0, 0xbba
    beq     s10, t0, 11f
    ori     a0, a0, 0x400
11: li      t0, 0xccb
    beq     s11, t0, 12f
    li      t1, 0x800
    or      a0, a0, t1
12:
    li      t0, 0xfff
    xor     a0, a0, t0
    ld      ra, 0(sp)
    ld      s0, 8(sp)
    ld      s1, 16(sp)
    ld      s2, 24(sp)
    ld      s3, 32(sp)
    ld      s4, 40(sp)
    ld      s5, 48(sp)
    ld      s6, 56(sp)
    ld      s7, 64(sp)
    ld      s8, 72(sp)
    ld      s9, 80(sp)
    ld      s10, 88(sp)
    ld      s11, 96(sp)
    addi    sp, sp, 112
    ret
"#
);

#[cfg(app_smoke)]
unsafe extern "C" {
    fn arceos_ex_task_switch_sentinel(
        prev: *mut TaskSwitchContext,
        next: *const TaskSwitchContext,
        next_task_identity: usize,
    ) -> usize;
}

pub unsafe fn switch(
    prev: &mut TaskSwitchContext,
    next: &TaskSwitchContext,
    next_task_identity: usize,
) -> usize {
    #[cfg(app_smoke)]
    unsafe {
        arceos_ex_task_switch_sentinel(
            prev as *mut TaskSwitchContext,
            next as *const TaskSwitchContext,
            next_task_identity,
        )
    }

    #[cfg(not(app_smoke))]
    unsafe {
        arceos_ex_task_switch(
            prev as *mut TaskSwitchContext,
            next as *const TaskSwitchContext,
            next_task_identity,
        );
        0xfff
    }
}
