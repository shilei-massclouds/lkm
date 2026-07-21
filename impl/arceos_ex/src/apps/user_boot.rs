use crate::{
    context::Context,
    objects::{printk, state::EventResult, user_boot},
};

pub(crate) fn prepare(ctx: &mut Context) -> EventResult {
    printk::write_str("arceos_ex user boot start\n");
    user_boot::prepare_first_user_init(ctx)
}

pub(crate) fn enter(ctx: &mut Context) -> ! {
    user_boot::enter_first_user_init(
        &ctx.user_boot_payload,
        &ctx.user_address_space,
        &ctx.user_trap_frame,
        &ctx.pid1_user_app_flow,
        &ctx.kernel_init_user_state,
    )
}
