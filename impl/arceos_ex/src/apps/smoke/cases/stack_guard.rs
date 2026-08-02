use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        task::{TASK_STACK_GUARD_VALUE, Task},
    },
};

#[repr(align(16))]
struct TestStack([usize; 4]);

pub fn run() -> SmokeResult {
    {
        let ctx = context();
        if !ctx.boot_task.task().stack_guard_installed()
            || !ctx.boot_task.task().stack_guard_intact()
        {
            printk::write_str("boot task stack guard is not intact\n");
            return SmokeResult::Failed;
        }
    }

    let mut storage = TestStack([0xA5A5_A5A5_A5A5_A5A5; 4]);
    let base = storage.0.as_mut_ptr() as usize;
    let top = base + core::mem::size_of_val(&storage.0);
    let word_size = core::mem::size_of::<usize>();
    let mut task = Task::new();
    if !task.set_kernel_stack_bounds(base, top) {
        printk::write_str("test task stack bounds rejected\n");
        return SmokeResult::Failed;
    }

    let invalid_ranges = [
        (0, 0),
        (top, base),
        (base, base + word_size - 1),
        (base + 1, top),
        (base + word_size, top),
    ];
    for (invalid_base, invalid_top) in invalid_ranges {
        if task.enable_stack_guard(invalid_base, invalid_top).is_ok()
            || task.stack_guard_installed()
            || unsafe { core::ptr::read_volatile(base as *const usize) } != 0xA5A5_A5A5_A5A5_A5A5
        {
            printk::write_str("invalid stack guard range mutated storage\n");
            return SmokeResult::Failed;
        }
    }

    if task.enable_stack_guard(base, top).is_err()
        || !task.stack_guard_installed()
        || !task.stack_guard_intact()
        || unsafe { core::ptr::read_volatile(base as *const usize) } != TASK_STACK_GUARD_VALUE
    {
        printk::write_str("stack guard installation failed\n");
        return SmokeResult::Failed;
    }
    if task.enable_stack_guard(base, top).is_err() || !task.stack_guard_intact() {
        printk::write_str("stack guard repeat was not idempotent\n");
        return SmokeResult::Failed;
    }

    let corrupted = TASK_STACK_GUARD_VALUE ^ 1;
    unsafe { core::ptr::write_volatile(base as *mut usize, corrupted) };
    if task.stack_guard_intact()
        || task.enable_stack_guard(base, top).is_ok()
        || !task.stack_guard_installed()
        || unsafe { core::ptr::read_volatile(base as *const usize) } != corrupted
    {
        printk::write_str("stack guard corruption was repaired or missed\n");
        return SmokeResult::Failed;
    }

    SmokeResult::Passed
}
