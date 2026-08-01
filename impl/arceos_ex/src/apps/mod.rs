#[cfg(app_hello)]
pub mod hello;
#[cfg(app_smoke)]
pub mod smoke;
#[cfg(app_user_boot)]
pub mod user_boot;

use crate::{
    context::Context,
    objects::{
        config::SelectedPayloadKind,
        state::{EventResult, State},
    },
};

#[cfg(app_hello)]
pub(crate) fn setup_selected_payload(ctx: &mut Context) -> EventResult {
    ctx.selected_payload_handoff.setup(&ctx.config, true)
}

#[cfg(app_smoke)]
pub(crate) fn setup_selected_payload(ctx: &mut Context) -> EventResult {
    ctx.selected_payload_handoff.setup(&ctx.config, true)
}

#[cfg(app_user_boot)]
pub(crate) fn setup_selected_payload(ctx: &mut Context) -> EventResult {
    ctx.user_boot_payload
        .setup(&ctx.kernel_init_task, &ctx.exec_sync_boundaries)?;
    ctx.selected_payload_handoff
        .setup(&ctx.config, ctx.user_boot_payload.state() == State::Ready)
}

#[cfg(app_hello)]
pub(crate) fn prepare_selected_payload(ctx: &mut Context) -> EventResult {
    ctx.selected_payload_handoff.enable(&ctx.config, true)
}

#[cfg(app_smoke)]
pub(crate) fn prepare_selected_payload(ctx: &mut Context) -> EventResult {
    ctx.selected_payload_handoff.enable(&ctx.config, true)
}

#[cfg(app_user_boot)]
pub(crate) fn prepare_selected_payload(ctx: &mut Context) -> EventResult {
    user_boot::prepare_handoff(ctx)?;
    ctx.selected_payload_handoff.enable(
        &ctx.config,
        ctx.user_boot_payload.state() == State::Ready
            && ctx.kernel_init_user_runtime.state() == State::Ready,
    )
}

#[cfg(any(app_hello, app_smoke))]
pub(crate) fn commit_selected_payload(_ctx: &mut Context) -> EventResult {
    Ok(())
}

#[cfg(app_user_boot)]
pub(crate) fn commit_selected_payload(ctx: &mut Context) -> EventResult {
    user_boot::commit_handoff(ctx)
}

#[cfg(app_hello)]
pub(crate) fn enter_selected_payload(ctx: &mut Context) -> ! {
    require_selected_entry(ctx, SelectedPayloadKind::Hello);
    hello::run()
}

#[cfg(app_smoke)]
pub(crate) fn enter_selected_payload(ctx: &mut Context) -> ! {
    require_selected_entry(ctx, SelectedPayloadKind::Smoke);
    smoke::run()
}

#[cfg(app_user_boot)]
pub(crate) fn enter_selected_payload(ctx: &mut Context) -> ! {
    require_selected_entry(ctx, SelectedPayloadKind::UserBoot);
    user_boot::enter(ctx)
}

fn require_selected_entry(ctx: &Context, expected_kind: SelectedPayloadKind) {
    if ctx.config.selected_payload_kind() != expected_kind
        || ctx.selected_payload_handoff.kind() != expected_kind
        || ctx.selected_payload_handoff.state() != State::Online
        || !ctx.selected_payload_handoff.kind_bound()
        || !ctx.selected_payload_handoff.variant_setup_ready()
        || !ctx.selected_payload_handoff.variant_prepare_ready()
        || !ctx.selected_payload_handoff.no_return_entry_bound()
        || !crate::phases::payload::prepare::is_online()
        || !crate::phases::payload::handoff_prepare::is_online()
        || !ctx.kernel_init_flow.payload_handoff_committed()
        || !crate::systems::kernel::is_online()
    {
        crate::arch::riscv64::sbi::putstr("selected payload entry invariant failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

#[cfg(not(any(app_smoke, app_hello, app_user_boot)))]
compile_error!("unsupported APP selection; build with APP=smoke, APP=hello or APP=user-boot");
