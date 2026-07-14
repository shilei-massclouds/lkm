#[cfg(checkpoint_handler_announce)]
pub mod announce;
#[cfg(checkpoint_handler_console_handoff)]
mod console_handoff;
#[cfg(checkpoint_handler_earlycon)]
mod earlycon;
#[cfg(checkpoint_handler_kernel_init_task)]
mod kernel_init_task;
#[cfg(all(checkpoint_handler_linux_plic, plic_provider_linux_object))]
mod linux_plic;
#[cfg(checkpoint_handler_memblock_api)]
compile_error!("checkpoint handler memblock-api was renamed to memblock");
#[cfg(checkpoint_handler_memblock)]
compile_error!(
    "checkpoint handler memblock mutates state; cover it through app smoke or an explicit action-level probe"
);
#[cfg(checkpoint_handler_vmalloc_mapping)]
compile_error!(
    "checkpoint handler vmalloc-mapping mutates state; cover it through app smoke or an explicit action-level probe"
);
#[cfg(checkpoint_handler_smoke)]
compile_error!(
    "checkpoint handler smoke runs app smoke cases; app smoke must remain outside checkpoint KUnit"
);
#[cfg(checkpoint_handler_of_platform)]
mod of_platform;
#[cfg(checkpoint_handler_page_allocator)]
mod page_allocator;
#[cfg(checkpoint_handler_scheduler_action)]
mod scheduler_action;
#[cfg(checkpoint_handler_slub)]
mod slub;
#[cfg(checkpoint_handler_uart_irq_chain)]
mod uart_irq_chain;
#[cfg(checkpoint_handler_user_boot)]
mod user_boot;
#[cfg(all(checkpoint_handler_user_boot_failure, app_user_boot))]
mod user_boot_failure;
#[cfg(checkpoint_handler_virtio_blk)]
mod virtio_blk;
#[cfg(checkpoint_handler_virtio_bus)]
mod virtio_bus;
#[cfg(checkpoint_handler_virtio_rng)]
mod virtio_rng;

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
))]
use crate::checkpoint::kunit::{KtapSink, Sink};
use crate::{checkpoint::Checkpoint, context::Context};

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum CheckpointOutcome {
    Continue,
    Passed,
    FailAndShutdown,
    StopAndShutdown,
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub enum HandlerScope {
    All,
    Only(&'static [Checkpoint]),
}

// Checkpoint handlers are observers. Do not add a `Write` variant, pass
// `&mut Context`, or otherwise let handlers mutate ordinary Context objects.
// Writable capability is intentionally limited to `Sink`.
#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
))]
#[allow(dead_code)]
pub enum HandlerRun {
    Observe(fn(Checkpoint, &Context, &mut dyn Sink) -> CheckpointOutcome),
}

#[allow(dead_code)]
pub struct Handler {
    pub name: &'static str,
    pub priority: i16,
    pub scope: HandlerScope,
    #[cfg(any(
        checkpoint_handler_announce,
        checkpoint_handler_page_allocator,
        checkpoint_handler_earlycon,
        checkpoint_handler_kernel_init_task,
        checkpoint_handler_linux_plic,
        checkpoint_handler_of_platform,
        checkpoint_handler_scheduler_action,
        checkpoint_handler_slub,
        checkpoint_handler_console_handoff,
        checkpoint_handler_uart_irq_chain,
        checkpoint_handler_virtio_bus,
        checkpoint_handler_virtio_blk,
        checkpoint_handler_virtio_rng,
        checkpoint_handler_user_boot,
        checkpoint_handler_user_boot_failure
    ))]
    pub run: HandlerRun,
}

const POST_VM_HANDLERS: &[Handler] = &[
    #[cfg(checkpoint_handler_announce)]
    announce::HANDLER,
    #[cfg(checkpoint_handler_page_allocator)]
    page_allocator::HANDLER,
    #[cfg(checkpoint_handler_of_platform)]
    of_platform::HANDLER,
    #[cfg(checkpoint_handler_earlycon)]
    earlycon::HANDLER,
    #[cfg(checkpoint_handler_kernel_init_task)]
    kernel_init_task::HANDLER,
    #[cfg(all(checkpoint_handler_linux_plic, plic_provider_linux_object))]
    linux_plic::HANDLER,
    #[cfg(checkpoint_handler_scheduler_action)]
    scheduler_action::HANDLER,
    #[cfg(checkpoint_handler_slub)]
    slub::HANDLER,
    #[cfg(checkpoint_handler_console_handoff)]
    console_handoff::HANDLER,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    uart_irq_chain::HANDLER,
    #[cfg(checkpoint_handler_virtio_bus)]
    virtio_bus::HANDLER,
    #[cfg(checkpoint_handler_virtio_blk)]
    virtio_blk::HANDLER,
    #[cfg(checkpoint_handler_virtio_rng)]
    virtio_rng::HANDLER,
    #[cfg(checkpoint_handler_user_boot)]
    user_boot::HANDLER,
    #[cfg(all(checkpoint_handler_user_boot_failure, app_user_boot))]
    user_boot_failure::HANDLER,
];

pub const fn has_post_vm_handlers() -> bool {
    !POST_VM_HANDLERS.is_empty()
}

#[cfg(any(
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
))]
pub const fn kunit_case_count() -> usize {
    let mut count = 0usize;
    #[cfg(checkpoint_handler_page_allocator)]
    {
        count += page_allocator::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_of_platform)]
    {
        count += of_platform::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_earlycon)]
    {
        count += earlycon::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_kernel_init_task)]
    {
        count += kernel_init_task::KUNIT_CASE_COUNT;
    }
    #[cfg(all(checkpoint_handler_linux_plic, plic_provider_linux_object))]
    {
        count += linux_plic::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_scheduler_action)]
    {
        count += scheduler_action::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_slub)]
    {
        count += slub::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_console_handoff)]
    {
        count += console_handoff::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_uart_irq_chain)]
    {
        count += uart_irq_chain::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_virtio_bus)]
    {
        count += virtio_bus::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_virtio_blk)]
    {
        count += virtio_blk::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_virtio_rng)]
    {
        count += virtio_rng::KUNIT_CASE_COUNT;
    }
    #[cfg(checkpoint_handler_user_boot)]
    {
        count += user_boot::KUNIT_CASE_COUNT;
    }
    #[cfg(all(checkpoint_handler_user_boot_failure, app_user_boot))]
    {
        count += user_boot_failure::KUNIT_CASE_COUNT;
    }
    count
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
))]
pub fn dispatch(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    dispatch_filtered(checkpoint, ctx, true)
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
))]
pub fn dispatch_without_announce(checkpoint: Checkpoint, ctx: &Context) -> CheckpointOutcome {
    dispatch_filtered(checkpoint, ctx, false)
}

#[cfg(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
))]
fn dispatch_filtered(
    checkpoint: Checkpoint,
    ctx: &Context,
    include_announce: bool,
) -> CheckpointOutcome {
    let mut current_priority = next_priority(checkpoint, None);
    while let Some(priority) = current_priority {
        let mut index = 0usize;
        while index < POST_VM_HANDLERS.len() {
            let handler = &POST_VM_HANDLERS[index];
            if (!include_announce && handler.name == "announce")
                || handler.priority != priority
                || !handler_matches(handler, checkpoint)
            {
                index += 1;
                continue;
            }
            {
                let outcome = match handler.run {
                    HandlerRun::Observe(run) => {
                        let mut sink = KtapSink;
                        run(checkpoint, ctx, &mut sink)
                    }
                };
                if outcome != CheckpointOutcome::Continue {
                    return outcome;
                }
            }
            index += 1;
        }
        current_priority = next_priority(checkpoint, Some(priority));
    }

    CheckpointOutcome::Continue
}

#[cfg(not(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
)))]
pub fn dispatch(_checkpoint: Checkpoint, _ctx: &Context) -> CheckpointOutcome {
    CheckpointOutcome::Continue
}

#[cfg(not(any(
    checkpoint_handler_announce,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_of_platform,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_slub,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_virtio_blk,
    checkpoint_handler_virtio_rng,
    checkpoint_handler_user_boot,
    checkpoint_handler_user_boot_failure
)))]
pub fn dispatch_without_announce(_checkpoint: Checkpoint, _ctx: &Context) -> CheckpointOutcome {
    CheckpointOutcome::Continue
}

#[allow(dead_code)]
fn next_priority(checkpoint: Checkpoint, after: Option<i16>) -> Option<i16> {
    let mut next = None;
    let mut index = 0usize;
    while index < POST_VM_HANDLERS.len() {
        let handler = &POST_VM_HANDLERS[index];
        if handler_matches(handler, checkpoint)
            && after.is_none_or(|priority| handler.priority > priority)
            && next.is_none_or(|priority| handler.priority < priority)
        {
            next = Some(handler.priority);
        }
        index += 1;
    }
    next
}

#[allow(dead_code)]
fn handler_matches(handler: &Handler, checkpoint: Checkpoint) -> bool {
    match handler.scope {
        HandlerScope::All => true,
        HandlerScope::Only(checkpoints) => {
            let mut index = 0usize;
            while index < checkpoints.len() {
                if checkpoints[index] == checkpoint {
                    return true;
                }
                index += 1;
            }
            false
        }
    }
}
