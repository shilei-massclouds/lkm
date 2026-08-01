use crate::{
    context::Context,
    objects::{printk, state::EventResult, user_boot},
};

pub(crate) fn prepare_handoff(ctx: &mut Context) -> EventResult {
    printk::write_str("arceos_ex user boot start\n");
    user_boot::prepare_first_user_init_handoff(ctx)
}

pub(crate) fn commit_handoff(ctx: &mut Context) -> EventResult {
    user_boot::commit_first_user_init_handoff(ctx)
}

pub(crate) fn enter(ctx: &mut Context) -> ! {
    let trap_stack_base = user_boot::user_kernel_trap_stack_base();
    let trap_stack_top = user_boot::user_kernel_trap_stack_top();
    let task_identity = crate::arch::riscv64::csr::read_tp();
    let trap_entry_context = {
        let Some(trap) = ctx.cpu_group.boot_cpu_trap_mut() else {
            crate::arch::riscv64::sbi::putstr("boot CPU trap entry context is missing\n");
            crate::arch::riscv64::sbi::system_shutdown()
        };
        if !trap.refresh_entry_task_stack(task_identity, trap_stack_base, trap_stack_top) {
            crate::arch::riscv64::sbi::putstr("boot CPU trap entry context refresh failed\n");
            crate::arch::riscv64::sbi::system_shutdown()
        }
        trap.entry_context_address()
    };
    user_boot::enter_first_user_init(
        &ctx.user_boot_payload,
        &ctx.user_address_space,
        &ctx.user_trap_frame,
        &ctx.kernel_init_user_runtime,
        &ctx.kernel_init_user_state,
        trap_entry_context,
    )
}
