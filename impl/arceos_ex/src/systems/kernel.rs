#![allow(dead_code)]

//! Runtime `Kernel.Enable` adoption for the already-built kernel image.

use super::{MappingStatus, SpecPath, SystemMapping};

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

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

// This is an execution-context fact, not an additional lifecycle state. Kernel
// stays Ready from the OpenSBI handoff until the application environment is
// completely prepared.
#[unsafe(link_section = ".data.phase")]
static KERNEL_ENABLE_ACCEPTED: AtomicBool = AtomicBool::new(false);

/// Accepts the OpenSBI handoff after the Rust entry has adopted its immutable
/// ABI inputs. The live SATP check occurs before the early-VM switch; OpenSBI's
/// stable handoff record cannot substitute for it. All checks precede recording
/// the accepted Enable context; the Kernel.Online commit happens only after
/// application-environment readiness.
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
        || crate::arch::riscv64::csr::read_satp() != 0
        || ctx.boot_task.state() != State::OnCpu
        || ctx.boot_task.task_ref() != TaskRef::BOOT
        || ctx.boot_task.pid() != 0
        || ctx.boot_task.task().active_flow().is_valid()
        || ctx.boot_init_flow.state() != State::Base
        || ctx.cpu_group.state() != State::Prepared
        || ctx.cpu_group.boot_cpu_state() != State::Prepared
        || ctx.boot_init_flow.cpu_ref() != ctx.cpu_group.boot_cpu_ref()
        || KERNEL_ENABLE_ACCEPTED.load(Ordering::Acquire)
    {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }

    if KERNEL_ENABLE_ACCEPTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }
    Ok(())
}

pub fn enable_in_progress() -> bool {
    state() == State::Ready && KERNEL_ENABLE_ACCEPTED.load(Ordering::Acquire)
}

/// Commits the target state after every lower-layer process driven by
/// Kernel.Enable has reached its application-environment-ready boundary.
pub fn commit_online_after_application_environment_ready() -> EventResult {
    let actual = state();
    let ctx = crate::context::context_ref();
    if actual != State::Ready
        || !KERNEL_ENABLE_ACCEPTED.load(Ordering::Acquire)
        || !crate::flows::boot_init_flow::is_online()
        || ctx.scheduler.schedule_passes() != 1
        || ctx.scheduler.kernel_init_stack_switch_started_count() != 1
        || ctx.kernel_init_task.state() != State::OnCpu
        || ctx.kernel_init_task.entry_started_count() != 1
        || !ctx.kernel_init_task.entry_stack_verified()
        || !ctx.kernel_init_task.current_stack_pointer_in_range()
        || !ctx.boot_cpu_current_task().current_is_kernel_init()
        || ctx.kernel_init_flow.state() != State::Online
        || ctx.kernel_init_flow.released()
        || !ctx.kernel_init_flow.active()
        || !crate::phases::payload::prepare::is_online()
        || !crate::phases::payload::handoff_prepare::is_online()
        || ctx.selected_payload_handoff.state() != State::Online
        || !ctx.selected_payload_handoff.kind_bound()
        || !ctx.selected_payload_handoff.variant_setup_ready()
        || !ctx.selected_payload_handoff.variant_prepare_ready()
        || !ctx.selected_payload_handoff.no_return_entry_bound()
    {
        return failed_condition(LifecycleEvent::Enable, actual, State::Ready, State::Online);
    }

    crate::phases::state::mark(
        &KERNEL_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::KernelOnline,
    )
}

pub fn state() -> State {
    crate::phases::state::load(&KERNEL_STATE)
}

pub fn is_online() -> bool {
    state() == State::Online
}
