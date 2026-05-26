#![no_std]
#![no_main]

mod arch;
mod objects;
mod phases;
mod trace;

use core::panic::PanicInfo;
use core::sync::atomic::AtomicU8;

use objects::state::{EventErrorCode, EventResult, LifecycleEvent, State};
use trace::Checkpoint;

#[unsafe(link_section = ".data.phase")]
static STARTUP_TIMELINE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn startup_timeline_continue_after_head_prefix(boot_args: &objects::boot_args::BootArgs) -> ! {
    require_startup_event(phases::prepare::adopt_head_prefix(boot_args));
    phases::boot::setup_after_head_prefix(boot_args)
}

pub fn startup_timeline_ready() -> ! {
    if !phases::prepare::is_online() || !phases::boot::is_ready() {
        arch::riscv64::sbi::putstr("arceos_ex startup invariant failed\n");
        arch::riscv64::sbi::system_shutdown()
    }

    require_startup_event(crate::phases::state::mark(
        &STARTUP_TIMELINE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::StartupTimelineReady,
    ));
    app_main();
    arch::riscv64::sbi::system_shutdown()
}

fn require_startup_event(result: EventResult) {
    match result {
        EventResult::Success => {}
        EventResult::Failed(error) | EventResult::Blocked(error) => {
            arch::riscv64::sbi::putstr("arceos_ex startup event failed:");
            arch::riscv64::sbi::putchar(match error.code {
                EventErrorCode::DuplicateLifecycleEvent => b'D',
                EventErrorCode::InvalidTransition => b'I',
                EventErrorCode::ConditionFailed => b'C',
                EventErrorCode::UnexpectedState => b'U',
            });
            arch::riscv64::sbi::putchar(b'\n');
            arch::riscv64::sbi::system_shutdown()
        }
    }
}

pub fn app_main() {
    objects::printk::write_str("Hello, world!\n");
    objects::earlycon::drain_printk();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    arch::riscv64::sbi::putstr("arceos_ex panic\n");
    arch::riscv64::sbi::system_shutdown()
}
