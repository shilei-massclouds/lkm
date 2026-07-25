#![allow(dead_code)]

//! Runtime `Kernel.Enable` adoption for the already-built kernel image.

use super::{MappingStatus, SpecPath, SystemMapping};

use core::sync::atomic::AtomicU8;

use crate::{
    checkpoint::Checkpoint,
    objects::{
        boot_args::BootArgs,
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::TaskRef,
    },
};

pub const KERNEL_SYSTEM_MAPPING: SystemMapping = SystemMapping {
    object_name: "Kernel",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/systems/kernel.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/systems/kernel.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/systems/kernel.md",
    },
    implementation: "impl/arceos_ex/src/systems/kernel.rs",
    status: MappingStatus::RuntimeImplemented,
};

// The executable is entered from the canonical Kernel.Enable pre-send
// boundary. Design-time Preset/Setup and Config/Lds publication have already
// happened outside this runtime image and are not replayed here.
#[unsafe(link_section = ".data.phase")]
static KERNEL_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Ready));

/// Accepts the OpenSBI handoff after the Rust entry has adopted its immutable
/// ABI inputs. All checks precede the single Kernel.Online commit.
pub fn accept_enable_at_entry(boot_args: &BootArgs) -> EventResult {
    let state = crate::phases::state::load(&KERNEL_STATE);
    let ctx = crate::context::context_ref();
    let kernel_start = ctx.lds.kernel_start();
    let kernel_link_addr = ctx.config.kernel_link_addr();

    if state != State::Ready
        || boot_args.state() != State::Online
        || !crate::phases::prepare::is_online()
        || ctx.config.state() != State::Online
        || !ctx.config.entry_prelude_ready()
        || ctx.lds.state() != State::Online
        || !ctx.lds.entry_layout_ready()
        || kernel_start == 0
        || kernel_start >= kernel_link_addr
        || !kernel_start.is_multiple_of(ctx.config.pmd_size())
        || ctx.boot_task.state() != State::OnCpu
        || ctx.boot_task.task_ref() != TaskRef::BOOT
        || ctx.boot_task.pid() != 0
        || ctx.boot_task.task().active_flow().is_valid()
        || ctx.boot_init_flow.state() != State::Base
    {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }

    crate::phases::state::mark(
        &KERNEL_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::KernelOnline,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&KERNEL_STATE) == State::Online
}
