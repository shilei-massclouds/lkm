//! KernelInitFlow payload leaf phases.

pub mod handoff_prepare;
pub mod prepare;

use crate::{context::Context, objects::state::State};

pub(super) fn mainline_ready(ctx: &Context) -> bool {
    ctx.kernel_init_task.state() == State::OnCpu
        && ctx.boot_cpu_current_task.current_is_kernel_init()
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
        && ctx.kernel_init_task.entry_started_count() == 1
        && ctx.kernel_init_task.entry_stack_verified()
        && ctx.kernel_init_task.current_stack_pointer_in_range()
}

pub(super) fn common_dependencies_ready() -> bool {
    crate::systems::kernel::is_online()
        && crate::phases::prepare::is_online()
        && crate::flows::boot_init_flow::is_online()
        && crate::phases::boot::core_prepare::is_online()
        && crate::phases::boot::mm_core_init::is_online()
        && crate::objects::printk::is_ready()
        && (crate::objects::printk::boot_console_online()
            || crate::objects::printk::console_handoff_complete())
}
