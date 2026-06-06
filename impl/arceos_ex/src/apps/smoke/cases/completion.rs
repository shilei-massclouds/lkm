use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        completion::{Completion, CompletionExtState},
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let live = ctx.kthreadd_ready_gate.completion();

    if live.state() != State::Online
        || live.ext_state() != CompletionExtState::Completed
        || live.done_count() != 1
        || !live.storage_bound()
        || !live.owns_wait_queue()
        || !live.handle_published()
        || !live.complete_committed()
        || !live.token_available()
        || !live.wakes_one_waiter()
        || live.wakes_all_waiters()
        || live.wait_queue().state() != State::Ready
    {
        printk::write_str("live kthreadd completion facts invalid\n");
        return SmokeResult::Failed;
    }

    let mut completion = Completion::new();
    if completion.setup().is_err()
        || completion.state() != State::Ready
        || !completion.pending()
        || completion.done_count() != 0
        || completion.wait_queue().state() != State::Ready
        || completion.enable().is_err()
        || completion.state() != State::Online
        || completion.complete().is_err()
        || completion.ext_state() != CompletionExtState::Completed
        || completion.done_count() != 1
        || !completion.wakes_one_waiter()
        || completion.done() != 1
        || !completion.done_observed()
    {
        printk::write_str("completion setup/complete flow invalid\n");
        return SmokeResult::Failed;
    }

    if completion.wait().is_err()
        || !completion.pending()
        || completion.done_count() != 0
        || !completion.waiter_enqueued()
        || !completion.waiter_finished()
        || !completion.wait_queue().waiter_enqueued()
        || !completion.wait_queue().waiter_finished()
        || completion.try_wait().is_ok()
    {
        printk::write_str("completion token consume flow invalid\n");
        return SmokeResult::Failed;
    }

    if completion.complete_all().is_err()
        || !completion.completed_all()
        || !completion.token_available()
        || !completion.complete_all_committed()
        || !completion.wakes_all_waiters()
        || completion.reinit().is_err()
        || !completion.pending()
        || completion.done_count() != 0
    {
        printk::write_str("completion complete_all/reinit flow invalid\n");
        return SmokeResult::Failed;
    }

    let mut preset_completion = Completion::new();
    if preset_completion.preset().is_err()
        || !preset_completion.storage_bound()
        || preset_completion.setup().is_err()
        || preset_completion.state() != State::Ready
        || preset_completion.enable().is_err()
        || preset_completion.state() != State::Online
    {
        printk::write_str("completion preset/setup flow invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "completion live_done={} local_state={}\n",
        live.done_count(),
        completion.ext_state() as u8
    ));
    SmokeResult::Passed
}
