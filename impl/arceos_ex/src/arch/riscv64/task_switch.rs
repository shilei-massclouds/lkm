use core::arch::global_asm;

#[repr(C)]
pub struct TaskSwitchContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskSwitchContext {
    pub const fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub const fn initialized(&self) -> bool {
        self.ra != 0 && self.sp != 0
    }

    pub fn init(&mut self, entry: extern "C" fn() -> !, stack_top: usize) {
        self.ra = entry as usize;
        self.sp = stack_top & !0xf;
        self.s = [0; 12];
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
     * This is the minimal RISC-V cooperative task context: ra, sp and the
     * callee-saved s-registers. It intentionally does not switch address
     * spaces, trap frames, FPU/vector state or task-local metadata.
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
