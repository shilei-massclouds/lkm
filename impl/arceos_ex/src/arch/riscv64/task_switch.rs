use core::arch::global_asm;

#[repr(C)]
pub struct TaskSwitchContext {
    ra: usize,
    sp: usize,
    tp: usize,
    s: [usize; 12],
}

impl TaskSwitchContext {
    pub const fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            tp: 0,
            s: [0; 12],
        }
    }

    pub const fn ra(&self) -> usize {
        self.ra
    }

    pub const fn initialized(&self) -> bool {
        self.ra != 0 && self.sp != 0
    }

    pub fn init(&mut self, entry: extern "C" fn() -> !, stack_top: usize, tp_value: usize) {
        self.ra = entry as usize;
        self.sp = stack_top & !0xf;
        self.tp = tp_value;
        self.s = [0; 12];
    }

    /// Mark the context as initialized without providing real register values.
    /// The actual register state will be captured on the first save (when this
    /// task acts as the *prev* party in a context switch).
    pub fn init_with_dummy(&mut self) {
        self.ra = 1;
        self.sp = 1;
        self.tp = 0;
        self.s = [0; 12];
    }

    pub fn set_tp(&mut self, tp_value: usize) {
        self.tp = tp_value;
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
     *
     * Cooperative task context switch: save/restore ra, sp, tp and the
     * callee-saved s-registers. tp is saved/restored per Linux convention
     * (move tp, a1 / current task_struct).
     * Does not switch address spaces, trap frames, FPU/vector state or
     * task-local metadata.
     */
    sd      ra, 0(a0)
    sd      sp, 8(a0)
    sd      tp, 16(a0)
    sd      s0, 24(a0)
    sd      s1, 32(a0)
    sd      s2, 40(a0)
    sd      s3, 48(a0)
    sd      s4, 56(a0)
    sd      s5, 64(a0)
    sd      s6, 72(a0)
    sd      s7, 80(a0)
    sd      s8, 88(a0)
    sd      s9, 96(a0)
    sd      s10, 104(a0)
    sd      s11, 112(a0)

    ld      ra, 0(a1)
    ld      sp, 8(a1)
    ld      tp, 16(a1)
    ld      s0, 24(a1)
    ld      s1, 32(a1)
    ld      s2, 40(a1)
    ld      s3, 48(a1)
    ld      s4, 56(a1)
    ld      s5, 64(a1)
    ld      s6, 72(a1)
    ld      s7, 80(a1)
    ld      s8, 88(a1)
    ld      s9, 96(a1)
    ld      s10, 104(a1)
    ld      s11, 112(a1)
    ret
"#
);

unsafe extern "C" {
    fn arceos_ex_task_switch(prev: *mut TaskSwitchContext, next: *const TaskSwitchContext);
}

pub unsafe fn switch(prev: &mut TaskSwitchContext, next: &TaskSwitchContext) {
    unsafe {
        arceos_ex_task_switch(
            prev as *mut TaskSwitchContext,
            next as *const TaskSwitchContext,
        );
    }
}
